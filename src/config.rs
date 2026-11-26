use std::{
    collections::HashMap,
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

const CONFIG_FILE_PATH: &'static str = ".cono";

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
        let default_config = Config::default();

        if let Ok(content) = fs::read_to_string(CONFIG_FILE_PATH) {
            let mut hash_map: HashMap<String, String> =
                HashMap::with_capacity(content.lines().count());
            for line in content.lines() {
                if let Some(pair) = line.split_once("=").map(|(k, v)| (k.trim(), v.trim())) {
                    hash_map.insert(pair.0.into(), pair.1.into());
                }
            }

            Config {
                compiler: hash_map
                    .get("compiler")
                    .and_then(|c| match c.trim().to_ascii_lowercase().as_str() {
                        "gcc" | "g++" | "gpp" => Some(Compiler::GPP),
                        "clang" | "clang++" => Some(Compiler::ClANG),
                        _ => None,
                    })
                    .unwrap_or(default_config.compiler),
                compilation_mode: hash_map
                    .get("mode")
                    .and_then(|c| match c.trim().to_ascii_lowercase().as_str() {
                        "debug" | "d" => Some(CompilationMode::DEBUG),
                        "release" | "r" => Some(CompilationMode::RELEASE),
                        _ => None,
                    })
                    .unwrap_or(default_config.compilation_mode),
                libs: hash_map
                    .get("libs")
                    .map(|libs| {
                        libs.split(",")
                            .map(|f| f.trim().to_string())
                            .filter(|f| !f.is_empty())
                            .collect()
                    })
                    .unwrap_or(default_config.libs),
                include_dir: hash_map
                    .get("include_dir")
                    .unwrap_or(&default_config.include_dir)
                    .to_string(),
                src_dir: hash_map
                    .get("src_dir")
                    .unwrap_or(&default_config.src_dir)
                    .to_string(),
                lib_dir: hash_map
                    .get("lib_dir")
                    .unwrap_or(&default_config.lib_dir)
                    .to_string(),
                entry_point: hash_map
                    .get("entry_point")
                    .unwrap_or(&default_config.entry_point)
                    .to_string(),
                output: hash_map.get("output").filter(|o| !o.is_empty()).cloned(),
            }
        } else {
            default_config
        }
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
