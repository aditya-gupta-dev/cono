use std::process::ExitCode;

use cono::{
    cli::{self, Command},
    config::Config,
};

fn main() -> ExitCode {
    let command = match cli::parse(std::env::args().skip(1)) {
        Ok(command) => command,
        Err(err) => {
            eprintln!("error: {err}\n\n{}", cli::USAGE);
            return ExitCode::FAILURE;
        }
    };

    let result = match command {
        Command::Help => {
            println!("{}", cli::USAGE);
            Ok(0)
        }
        Command::Init(name) => cono::init(&name).map(|_| 0),
        Command::Build => cono::build(&Config::new()).map(|_| 0),
        Command::Run(args) => cono::run(&Config::new(), &args),
    };

    match result {
        Ok(code) => ExitCode::from(code),
        Err(err) => {
            eprintln!("error: {err}");
            ExitCode::FAILURE
        }
    }
}
