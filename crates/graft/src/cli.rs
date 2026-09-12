//! The command line: one optional path, and the two flags every program has.
//!
//! Hand-parsed rather than pulled from a dependency, because this is the whole
//! surface — Graft is a terminal application, not a CLI, and its arguments are
//! "which repository". When that stops being true, this is the file that gets
//! replaced by a parser.

use std::env;
use std::ffi::OsString;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// What the user asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invocation {
    /// Open this repository.
    Open(PathBuf),
    /// Print this and exit successfully.
    Usage(&'static str),
}

/// What the arguments could not be made sense of.
#[derive(Debug)]
pub enum CliError {
    /// A flag Graft has never heard of. Not ignored: a user who types
    /// `--themes dark` deserves to be told, not to get the default silently.
    UnknownOption(String),
    /// More than one path. Graft opens one repository.
    TooManyPaths,
    /// No path was given and the current directory could not be read — it has
    /// been deleted, or its permissions changed, under the running shell.
    NoWorkingDirectory(io::Error),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::UnknownOption(option) => {
                write!(f, "unknown option `{option}`\n\n{USAGE}")
            }
            CliError::TooManyPaths => write!(f, "Graft opens one repository\n\n{USAGE}"),
            CliError::NoWorkingDirectory(error) => write!(
                f,
                "cannot tell which directory Graft was started in: {error}"
            ),
        }
    }
}

impl std::error::Error for CliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CliError::NoWorkingDirectory(error) => Some(error),
            CliError::UnknownOption(_) | CliError::TooManyPaths => None,
        }
    }
}

/// What `--help` prints.
pub const USAGE: &str = "\
usage: graft [<path>]

  <path>    the repository to open (default: the current directory)

  -h, --help    print this and exit";

/// Read the process's own arguments.
///
/// # Errors
/// [`CliError`], with a message that is ready to print.
pub fn from_process() -> Result<Invocation, CliError> {
    parse(env::args_os().skip(1))
}

/// Parse arguments that have already had the program name removed.
///
/// # Errors
/// [`CliError`], with a message that is ready to print.
pub fn parse(arguments: impl IntoIterator<Item = OsString>) -> Result<Invocation, CliError> {
    let mut path: Option<PathBuf> = None;
    // Everything after `--` is a path, however it is spelled. A repository can
    // legitimately be in a directory called `--help`.
    let mut only_paths = false;

    for argument in arguments {
        let flag = argument.to_str().filter(|_| !only_paths);

        match flag {
            Some("--") => only_paths = true,
            Some("-h" | "--help") => return Ok(Invocation::Usage(USAGE)),
            // A lone `-` is a path on every shell that produces it; anything
            // else starting with a dash is a flag Graft does not have.
            Some(option) if option.starts_with('-') && option != "-" => {
                return Err(CliError::UnknownOption(option.to_owned()));
            }
            _ => {
                if path.is_some() {
                    return Err(CliError::TooManyPaths);
                }
                path = Some(PathBuf::from(argument));
            }
        }
    }

    match path {
        Some(path) => Ok(Invocation::Open(path)),
        // Graft is usually started inside the repository it is meant to show.
        None => env::current_dir()
            .map(Invocation::Open)
            .map_err(CliError::NoWorkingDirectory),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_words(words: &[&str]) -> Result<Invocation, CliError> {
        parse(words.iter().map(OsString::from))
    }

    #[test]
    fn a_path_is_the_repository_to_open() {
        let parsed = parse_words(&["/code/api"]).expect("a path is valid");

        assert_eq!(parsed, Invocation::Open(PathBuf::from("/code/api")));
    }

    #[test]
    fn no_arguments_means_the_current_directory() {
        let parsed = parse_words(&[]).expect("the test process has a working directory");
        let here = env::current_dir().expect("the test process has a working directory");

        assert_eq!(parsed, Invocation::Open(here));
    }

    #[test]
    fn help_is_asked_for_in_both_spellings() {
        for spelling in ["-h", "--help"] {
            let parsed = parse_words(&[spelling]).expect("help is always valid");

            assert_eq!(parsed, Invocation::Usage(USAGE));
        }
    }

    #[test]
    fn help_wins_over_a_path_it_was_given_with() {
        let parsed = parse_words(&["/code/api", "--help"]).expect("help is always valid");

        assert_eq!(parsed, Invocation::Usage(USAGE));
    }

    #[test]
    fn an_unknown_option_is_refused_rather_than_ignored() {
        let error = parse_words(&["--theme=dark"]).expect_err("Graft has no --theme");

        assert!(matches!(error, CliError::UnknownOption(ref option) if option == "--theme=dark"));
        assert!(error.to_string().contains("usage: graft"));
    }

    #[test]
    fn two_paths_are_refused() {
        let error = parse_words(&["/code/api", "/code/web"]).expect_err("one repository at a time");

        assert!(matches!(error, CliError::TooManyPaths));
    }

    #[test]
    fn a_path_after_two_dashes_is_a_path_whatever_it_looks_like() {
        let parsed = parse_words(&["--", "--help"]).expect("a directory may be called --help");

        assert_eq!(parsed, Invocation::Open(PathBuf::from("--help")));
    }

    #[test]
    fn a_lone_dash_is_a_path() {
        let parsed = parse_words(&["-"]).expect("a directory may be called -");

        assert_eq!(parsed, Invocation::Open(PathBuf::from("-")));
    }
}
