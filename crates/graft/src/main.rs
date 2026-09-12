//! The Graft binary. Deliberately thin: everything it runs lives in the
//! `graft` library (`lib.rs`) so the same code can be driven from tests and
//! from the `terminal_probe` example.
//!
//! All this file decides is how the process dies. A signalled run has to die
//! *as if it were signalled* — the terminal is already restored by the time
//! the loop returns, so re-raising here costs nothing and keeps `$?` honest
//! for whatever script is watching.

use std::io::{self, Write};
use std::process::ExitCode;

use graft::event_loop::{self, Exit};
use graft::terminal::SessionOptions;

fn main() -> ExitCode {
    // Capability detection decides the keyboard protocol; ticket 11 wires it
    // in. Until then the legacy encoding is used, which every terminal has.
    match event_loop::run(SessionOptions::default()) {
        Ok(Exit::Quit) => ExitCode::SUCCESS,
        Ok(Exit::Signal(signal)) => {
            // Normally this does not return: the process dies of the signal.
            if let Err(error) = event_loop::signals::reraise(signal) {
                report(&format!("could not re-raise signal {signal}: {error}"));
            }
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
