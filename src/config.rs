use std::{
    ffi::OsStr,
    fmt::Debug,
    fs::{self},
    path::{Path, PathBuf},
};

pub enum Compiler {
    GPP,
    ClANG,
}

pub enum CompilationMode {
    DEBUG,
    RELEASE,
}

impl Debug for CompilationMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DEBUG => write!(f, "debug"),
            Self::RELEASE => write!(f, "release"),
        }
    }
}

impl Debug for Compiler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GPP => write!(f, "g++"),
            Self::ClANG => write!(f, "clang"),
        }
    }
}

const CONFIG_FILE_PATH: &str = ".cono";

/// Case-insensitive match without allocating a lowercased copy.
fn is_any(value: &str, names: &[&str]) -> bool {
    names.iter().any(|name| value.eq_ignore_ascii_case(name))
}

fn parse_compiler(value: &str) -> Option<Compiler> {
    if is_any(value, &["gcc", "g++", "gpp"]) {
        Some(Compiler::GPP)
    } else if is_any(value, &["clang", "clang++"]) {
        Some(Compiler::ClANG)
    } else {
        None
    }
}

fn parse_mode(value: &str) -> Option<CompilationMode> {
    if is_any(value, &["debug", "d"]) {
        Some(CompilationMode::DEBUG)
    } else if is_any(value, &["release", "r"]) {
        Some(CompilationMode::RELEASE)
    } else {
        None
    }
}

#[derive(Debug)]
pub struct Config {
    pub include_dir: String,
    pub src_dir: String,
    pub lib_dir: String,
    pub entry_point: String,
    pub compilation_mode: CompilationMode,
    pub compiler: Compiler,
    pub libs: Vec<String>,
    pub output: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            lib_dir: String::from("lib"),
            libs: Vec::new(),
            compiler: Compiler::GPP,
            compilation_mode: CompilationMode::DEBUG,
            include_dir: String::from("include"),
            src_dir: String::from("src"),
            entry_point: String::from("main.cpp"),
            output: None,
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let mut config = Config::default();
        let Ok(content) = fs::read_to_string(CONFIG_FILE_PATH) else {
            return config;
        };

        // Later lines win, so a repeated key overrides an earlier one.
        for (key, value) in content.lines().filter_map(|line| line.split_once('=')) {
            let value = value.trim();
            match key.trim() {
                "compiler" => {
                    if let Some(compiler) = parse_compiler(value) {
                        config.compiler = compiler;
                    }
                }
                "mode" => {
                    if let Some(mode) = parse_mode(value) {
                        config.compilation_mode = mode;
                    }
                }
                "libs" => {
                    config.libs = value
                        .split(',')
                        .map(str::trim)
                        .filter(|lib| !lib.is_empty())
                        .map(String::from)
                        .collect();
                }
                "include_dir" => config.include_dir = value.into(),
                "src_dir" => config.src_dir = value.into(),
                "lib_dir" => config.lib_dir = value.into(),
                "entry_point" => config.entry_point = value.into(),
                "output" => config.output = (!value.is_empty()).then(|| value.into()),
                _ => {}
            }
        }
        config
    }

    pub fn output_path(&self) -> PathBuf {
        match &self.output {
            Some(out) => PathBuf::from(out),
            None => {
                let name = Path::new(&self.entry_point)
                    .file_stem()
                    .unwrap_or(OsStr::new("main"));
                Path::new("build")
                    .join(format!("{:?}", self.compilation_mode))
                    .join(name)
            }
        }
    }
}
