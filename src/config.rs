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
