use std::{
    collections::HashMap,
    fs::{self},
};

const CONFIG_FILE_PATH: &'static str = ".cono";

#[derive(Debug)]
pub struct Config {
    pub include_dir: String,
    pub lib_dir: String,
    pub entry_point: String,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            include_dir: String::from("include"),
            lib_dir: String::from("lib"),
            entry_point: String::from("main.cpp"),
        }
    }
}

impl Config {
    pub fn new() -> Self {
        let default_config = Config::default();

        if let Ok(content) = fs::read_to_string(CONFIG_FILE_PATH) {
            let mut hash_map: HashMap<String, String> = HashMap::with_capacity(content.lines().count());
            for line in content.lines() {
                if let Some(pair) = line.split_once("=").map(|(k, v)| (k.trim(), v.trim())) {
                    hash_map.insert(pair.0.into(), pair.1.into());
                }
            }

            Config {
                include_dir: hash_map.get("include_dir").unwrap_or(&default_config.include_dir).to_string(),
                lib_dir: hash_map.get("lib_dir").unwrap_or(&default_config.lib_dir).to_string(),
                entry_point: hash_map.get("entry_point").unwrap_or(&default_config.entry_point).to_string(),
            }
        } else { 
            default_config
        }
    }

}
