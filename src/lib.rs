use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

use crate::{
    cmd::{CLANG_DEBUG_FLAGS, CLANG_FLAGS, DEBUG_FLAGS, GPP_FLAGS, RELEASE_FLAGS, WARN_FLAGS},
    config::{CompilationMode, Compiler, Config},
    deps::Sources,
};

pub mod cli;
pub mod cmd;
pub mod config;
pub mod deps;

const MAIN_CPP: &str = r#"#include <iostream>

int main() {
    std::cout << "Hello, world!\n";
    return 0;
}
"#;

const DEFAULT_CONO: &str = "\
entry_point = main.cpp
compiler = g++
mode = debug
include_dir = include
lib_dir = lib
libs =
";

/// Creates `name/` holding `include/`, `lib/`, `main.cpp` and `.cono`.
/// Refuses to touch a directory that already exists.
pub fn init(name: &str) -> io::Result<()> {
    let root = Path::new(name);
    fs::create_dir(root).map_err(|e| match e.kind() {
        io::ErrorKind::AlreadyExists => {
            io::Error::new(e.kind(), format!("`{name}` already exists"))
        }
        _ => io::Error::new(e.kind(), format!("could not create `{name}`: {e}")),
    })?;

    fs::create_dir(root.join("include"))?;
    fs::create_dir(root.join("lib"))?;
    fs::write(root.join("main.cpp"), MAIN_CPP)?;
    fs::write(root.join(".cono"), DEFAULT_CONO)?;

    println!("     Created project `{name}`");
    println!("\n  cd {name} && cono");
    Ok(())
}

/// Builds the project, then runs the executable with `args`.
/// Returns the program's exit code.
pub fn run(config: &Config, args: &[String]) -> io::Result<u8> {
    let output = build(config)?;

    let exe = if output.is_relative() {
        Path::new(".").join(&output)
    } else {
        output
    };

    println!("     Running {}", exe.display());
    let status = Command::new(&exe)
        .args(args)
        .status()
        .map_err(|e| io::Error::new(e.kind(), format!("could not run {}: {e}", exe.display())))?;

    match status.code() {
        Some(code) => Ok(u8::try_from(code).unwrap_or(1)),
        // Killed by a signal, e.g. SIGABRT from a sanitizer.
        None => Err(io::Error::other(format!("program terminated ({status})"))),
    }
}

/// Finds every source file reachable from the entry point and compiles them
/// into one executable. Returns the executable's path.
pub fn build(config: &Config) -> io::Result<PathBuf> {
    if !Path::new(&config.entry_point).is_file() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "entry point `{}` not found (create a project with `cono init <name>`, or set entry_point in .cono)",
                config.entry_point
            ),
        ));
    }

    let sources = deps::collect(
        Path::new(&config.entry_point),
        &[Path::new(&config.include_dir)],
        &[Path::new(&config.src_dir)],
    )?;

    for (file, name) in &sources.unresolved {
        eprintln!(
            "warning: {}: \"{name}\" not found in the project",
            file.display()
        );
    }

    let output = config.output_path();
    if let Some(dir) = output.parent() {
        fs::create_dir_all(dir)?;
    }

    let mut cmd = build_command(config, &sources, &output);
    println!("   Compiling {}", display_command(&cmd));

    let status = cmd.status().map_err(|e| {
        io::Error::new(
            e.kind(),
            format!("could not run {:?}: {e}", cmd.get_program()),
        )
    })?;
    if !status.success() {
        return Err(io::Error::other(format!("compilation failed ({status})")));
    }

    println!(
        "    Finished {:?} -> {}",
        config.compilation_mode,
        output.display()
    );
    Ok(output)
}

fn build_command(config: &Config, sources: &Sources, output: &Path) -> Command {
    let debug = matches!(config.compilation_mode, CompilationMode::DEBUG);

    let mut cmd = match config.compiler {
        Compiler::ClANG => {
            let mut cmd = Command::new("clang++");
            cmd.args(CLANG_FLAGS);
            if debug {
                cmd.args(CLANG_DEBUG_FLAGS);
            }
            cmd
        }
        Compiler::GPP => {
            let mut cmd = Command::new("g++");
            cmd.args(GPP_FLAGS);
            cmd
        }
    };

    cmd.args(WARN_FLAGS);
    cmd.args(if debug { DEBUG_FLAGS } else { RELEASE_FLAGS });

    cmd.arg(format!("-I{}", config.include_dir));
    cmd.args(&sources.units);
    cmd.arg("-o").arg(output);

    if Path::new(&config.lib_dir).is_dir() {
        cmd.arg(format!("-L{}", config.lib_dir));
    }
    cmd.args(config.libs.iter().map(|lib| format!("-l{lib}")));

    cmd
}

fn display_command(cmd: &Command) -> String {
    std::iter::once(cmd.get_program())
        .chain(cmd.get_args())
        .map(|a| a.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(" ")
}
