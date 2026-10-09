//! The command line: one optional path, and the flags in [`FLAGS`].
//!
//! Hand-parsed rather than pulled from a dependency, because this is the whole
//! surface — Graft is a terminal application, not a CLI, and its arguments are
//! "which repository". When that stops being true, this is the file that gets
//! replaced by a parser.

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::PathBuf;

mod config_keys;
#[cfg(test)]
mod docs_tests;
mod flags;
mod man;

pub use config_keys::{CONFIG_KEYS, ConfigKeyDoc};
pub use flags::{FLAGS, Flag, usage};
pub use man::man_page;

use flags::FlagAction;

/// What the user asked for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Invocation {
    /// Open this repository.
    Open(PathBuf),
    /// Print this and exit successfully: `--help`, [`usage`] of [`FLAGS`].
    Usage(String),
    /// Print the man page and exit successfully: `--generate-man`.
    GenerateMan,
    /// Askpass mode: git (or ssh) ran Graft as its credential helper. Pass
    /// `prompt` to the running Graft listening on `socket`, print its answer
    /// for git, exit — never touching the terminal the running Graft owns.
    Askpass {
        /// The running Graft's socket, from `GRAFT_ASKPASS_SOCKET`.
        socket: PathBuf,
        /// git's question, its one argument (`Password for '…':`).
        prompt: String,
    },
}

/// The variable that names the running Graft's askpass socket. git passes it
/// through to the helper; its presence is what selects askpass mode (design
/// note m4, choice 2: an argument cannot ride in `GIT_ASKPASS` reliably).
pub const ASKPASS_SOCKET_VAR: &str = "GRAFT_ASKPASS_SOCKET";

/// What the arguments could not be made sense of.
#[derive(Debug, thiserror::Error)]
pub enum CliError {
    /// A flag Graft has never heard of. Not ignored: a user who types
    /// `--themes dark` deserves to be told, not to get the default silently.
    #[error("unknown option `{0}`\n\n{usage}", usage = usage(FLAGS))]
    UnknownOption(String),
    /// More than one path. Graft opens one repository.
    #[error("Graft opens one repository\n\n{usage}", usage = usage(FLAGS))]
    TooManyPaths,
    /// No path was given and the current directory could not be read — it has
    /// been deleted, or its permissions changed, under the running shell.
    #[error("cannot tell which directory Graft was started in: {0}")]
    NoWorkingDirectory(#[source] io::Error),
}

/// Read the process's own arguments.
///
/// # Errors
/// [`CliError`], with a message that is ready to print.
pub fn from_process() -> Result<Invocation, CliError> {
    let arguments: Vec<OsString> = env::args_os().skip(1).collect();
    if let Some(askpass) = askpass(env::var_os(ASKPASS_SOCKET_VAR), &arguments) {
        return Ok(askpass);
    }
    parse(arguments)
}

/// Askpass mode, when `socket` (the value of [`ASKPASS_SOCKET_VAR`]) is set
/// and there is exactly one argument: git and ssh pass the prompt as the
/// single argument. Anything else is an ordinary command line.
///
/// A prompt that is not valid UTF-8 is converted lossily: it is only ever
/// shown to the user, and the answer does not depend on it.
#[must_use]
pub fn askpass(socket: Option<OsString>, arguments: &[OsString]) -> Option<Invocation> {
    let socket = socket.filter(|socket| !socket.is_empty())?;
    match arguments {
        [prompt] => Some(Invocation::Askpass {
            socket: PathBuf::from(socket),
            prompt: prompt.to_string_lossy().into_owned(),
        }),
        _ => None,
    }
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
            // A lone `-` is a path on every shell that produces it; anything
            // else starting with a dash is a flag, and a flag with no row in
            // `FLAGS` is one Graft does not have.
            Some(option) if option.starts_with('-') && option != "-" => {
                let action = flags::find(FLAGS, option).and_then(|flag| FlagAction::of(flag.long));
                match action {
                    Some(FlagAction::Usage) => return Ok(Invocation::Usage(usage(FLAGS))),
                    Some(FlagAction::GenerateMan) => return Ok(Invocation::GenerateMan),
                    None => return Err(CliError::UnknownOption(option.to_owned())),
                }
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

            assert_eq!(parsed, Invocation::Usage(usage(FLAGS)));
        }
    }

    #[test]
    fn help_wins_over_a_path_it_was_given_with() {
        let parsed = parse_words(&["/code/api", "--help"]).expect("help is always valid");

        assert_eq!(parsed, Invocation::Usage(usage(FLAGS)));
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
    fn askpass_mode_needs_the_socket_and_exactly_one_argument() {
        let socket = Some(OsString::from("/tmp/graft-askpass-1/ask.sock"));
        let prompt = [OsString::from("Password for 'http://u@h':")];

        assert_eq!(
            askpass(socket.clone(), &prompt),
            Some(Invocation::Askpass {
                socket: PathBuf::from("/tmp/graft-askpass-1/ask.sock"),
                prompt: "Password for 'http://u@h':".to_owned(),
            })
        );
        assert_eq!(askpass(None, &prompt), None);
        assert_eq!(askpass(Some(OsString::new()), &prompt), None);
        assert_eq!(askpass(socket.clone(), &[]), None);
        let two = [OsString::from("a"), OsString::from("b")];
        assert_eq!(askpass(socket, &two), None);
    }

    /// `--help`'s words as they were when `USAGE` was a hand-written const
    /// (m8/23): generating it from `FLAGS` changed nothing he sees.
    const USAGE_BEFORE_FLAGS: &str = "\
usage: graft [<path>]

  <path>    the repository to open (default: the current directory)

  -h, --help    print this and exit";

    #[test]
    fn help_prints_the_words_it_always_did() {
        assert_eq!(usage(FLAGS), USAGE_BEFORE_FLAGS);
    }

    #[test]
    fn refusals_keep_their_words_and_carry_the_usage() {
        let unknown = parse_words(&["--themes"]).expect_err("no such flag");
        assert_eq!(
            unknown.to_string(),
            format!("unknown option `--themes`\n\n{USAGE_BEFORE_FLAGS}")
        );

        let two = parse_words(&["/a", "/b"]).expect_err("one repository");
        assert_eq!(
            two.to_string(),
            format!("Graft opens one repository\n\n{USAGE_BEFORE_FLAGS}")
        );
    }

    #[test]
    fn generate_man_is_asked_for_and_hidden_from_help() {
        let parsed = parse_words(&["--generate-man"]).expect("a flag in FLAGS");

        assert_eq!(parsed, Invocation::GenerateMan);
        assert!(!usage(FLAGS).contains("generate-man"));
    }

    #[test]
    fn a_short_spelling_parse_does_not_know_is_refused() {
        for word in ["-x", "-hh", "--h"] {
            let error = parse_words(&[word]).expect_err("not a flag");
            assert!(matches!(error, CliError::UnknownOption(ref option) if option == word));
        }
    }

    #[test]
    fn a_lone_dash_is_a_path() {
        let parsed = parse_words(&["-"]).expect("a directory may be called -");

        assert_eq!(parsed, Invocation::Open(PathBuf::from("-")));
    }
}
