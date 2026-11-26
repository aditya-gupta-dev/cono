use std::process::ExitCode;

use cono::{config::Config, run};

fn main() -> ExitCode {
    let config = Config::new();
    match run(&config) {
        Ok(_) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
