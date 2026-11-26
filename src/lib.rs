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

pub mod cmd;
pub mod config;
pub mod deps;

pub fn run(config: &Config) -> io::Result<PathBuf> {
    let sources = deps::collect(
        Path::new(&config.entry_point),
        &[PathBuf::from(&config.include_dir)],
        &[PathBuf::from(&config.src_dir)],
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
