//! The Graft binary. Deliberately thin: everything it runs lives in the
//! `graft` library (`lib.rs`) so the same code can be driven from tests and
//! from the `terminal_probe` example.
//!
//! What it owns is the order of startup and the way the process dies. The
//! order matters: the command line and the config file are settled *before*
//! the terminal is touched, so a bad argument or an unreadable config prints
//! a plain line on a plain terminal instead of flashing an alternate screen at
//! the user. And a signalled run has to die *as if it were signalled* — the
//! terminal is restored by the time the loop returns, so re-raising here costs
//! nothing and keeps `$?` honest for whatever script is watching.

use std::io::{self, Write};
use std::process::ExitCode;

use graft::cli::{self, Invocation};
use graft::config;
use graft::event_loop::{self, Exit};
use graft::session::Startup;

fn main() -> ExitCode {
    let repo_path = match cli::from_process() {
        Ok(Invocation::Open(path)) => path,
        Ok(Invocation::Usage(usage)) => {
            println!("{usage}");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            report(&error.to_string());
            return ExitCode::FAILURE;
        }
    };

    // Before the terminal session: a config file that cannot be read is a
    // message, and a message is only readable on a terminal Graft has not
    // taken over yet.
    let config = match config::load() {
        Ok(loaded) => loaded,
        Err(error) => {
            report(&error.to_string());
            return ExitCode::FAILURE;
        }
    };

    match event_loop::run(Startup { repo_path, config }) {
        Ok(Exit::Quit) => ExitCode::SUCCESS,
        Ok(Exit::Signal(signal)) => {
            // Normally this does not return: the process dies of the signal.
            if let Err(error) = event_loop::signals::reraise(signal) {
                report(&format!("could not re-raise signal {signal}: {error}"));
            }
            ExitCode::FAILURE
        }
        // Reported here rather than in the loop: this is the first moment the
        // terminal is the user's again, so the line survives.
        Ok(Exit::Fatal(reason)) => {
            report(&reason);
            ExitCode::FAILURE
        }
        Err(error) => {
            report(&error.to_string());
            ExitCode::FAILURE
        }
    }
}

/// Say what went wrong on stderr, now that the terminal is the user's again.
fn report(message: &str) {
    let mut stderr = io::stderr();
    // If even stderr has gone there is nowhere left to complain to, and the
    // exit code still carries the failure.
    writeln!(stderr, "graft: {message}")
        .and_then(|()| stderr.flush())
        .unwrap_or(());
}
