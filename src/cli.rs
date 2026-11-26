//! Command-line parsing.

pub const USAGE: &str = "\
cono - build C++ projects with strict safety flags

Usage:
  cono                     Build and run the project in this directory
  cono run [-- <args>...]  Build and run, passing <args> to the program
  cono build               Build only
  cono init <name>         Create a new project in ./<name>
  cono help                Show this message

  Credits: 
  Github                   https://github.com/aditya-gupta-dev/cono.git

Settings are read from the .cono file in the current directory.";

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Init(String),
    Build,
    /// Arguments to pass to the built program.
    Run(Vec<String>),
    Help,
}

/// Parses the arguments after the program name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Command, String> {
    let mut args = args.into_iter();

    let Some(cmd) = args.next() else {
        return Ok(Command::Run(Vec::new()));
    };

    let command = match cmd.as_str() {
        "help" | "-h" | "--help" => Command::Help,
        "build" => Command::Build,
        "run" => {
            return match args.next().as_deref() {
                None => Ok(Command::Run(Vec::new())),
                Some("--") => Ok(Command::Run(args.collect())),
                Some(other) => Err(format!(
                    "unexpected argument `{other}`; pass program arguments after `--`, e.g. `cono run -- {other}`"
                )),
            };
        }
        "init" => match args.next() {
            Some(name) if !name.starts_with('-') => Command::Init(name),
            _ => return Err("missing project name: `cono init <name>`".into()),
        },
        other => return Err(format!("unknown command `{other}`")),
    };

    match args.next() {
        Some(extra) => Err(format!("unexpected argument `{extra}`")),
        None => Ok(command),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(args: &[&str]) -> Result<Command, String> {
        parse(args.iter().map(|s| s.to_string()))
    }

    #[test]
    fn no_args_runs() {
        assert_eq!(p(&[]), Ok(Command::Run(vec![])));
    }

    #[test]
    fn commands() {
        assert_eq!(p(&["build"]), Ok(Command::Build));
        assert_eq!(p(&["run"]), Ok(Command::Run(vec![])));
        assert_eq!(p(&["init", "game"]), Ok(Command::Init("game".into())));
        assert_eq!(p(&["--help"]), Ok(Command::Help));
    }

    #[test]
    fn run_passes_args_after_double_dash() {
        assert_eq!(
            p(&["run", "--", "a", "--flag"]),
            Ok(Command::Run(vec!["a".into(), "--flag".into()]))
        );
    }

    #[test]
    fn errors() {
        assert!(p(&["init"]).is_err());
        assert!(p(&["init", "--release"]).is_err());
        assert!(p(&["init", "a", "b"]).is_err());
        assert!(p(&["build", "x"]).is_err());
        assert!(p(&["run", "x"]).is_err());
        assert!(p(&["frobnicate"]).is_err());
    }
}
