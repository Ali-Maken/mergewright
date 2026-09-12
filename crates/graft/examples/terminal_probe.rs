//! Manual check for "quitting leaves the terminal clean" (PROMPT.md §9).
//!
//! The fast tier asserts the escape *bytes* against a buffer; only a real
//! terminal (or a pty) can show that the terminal is genuinely handed back —
//! raw mode off, alternate screen left, cursor visible. This example opens a
//! [`TerminalSession`] exactly as the binary will, then leaves in one of three
//! ways, so each exit path can be watched from outside:
//!
//! ```text
//! cargo run -p graft --example terminal_probe -- quit    # normal drop
//! cargo run -p graft --example terminal_probe -- panic   # panic hook, then drop
//! cargo run -p graft --example terminal_probe -- hang    # sit still until signalled
//! ```
//!
//! After `quit` or `panic` the shell must behave normally (echo on, Ctrl+C
//! works) and the screen must hold whatever it held before. `hang` is there to
//! be killed: it shows what a signal does to *this* example, which still takes
//! the default action and dies without restoring. The product binary does not
//! — `event_loop::signals` turns a signal into an ordinary wake-up (m0/09).

use std::io::{self, Write};
use std::process::ExitCode;
use std::thread;
use std::time::Duration;

use graft::crash;
use graft::terminal::{SessionOptions, TerminalSession};

fn main() -> ExitCode {
    let Some(mode) = std::env::args().nth(1) else {
        eprintln!("usage: terminal_probe <quit|panic|hang>");
        return ExitCode::FAILURE;
    };

    let session = match TerminalSession::on_stdout(SessionOptions {
        kitty_keyboard: true,
    }) {
        Ok(session) => session,
        Err(error) => {
            eprintln!("terminal_probe: could not set up the terminal: {error}");
            return ExitCode::FAILURE;
        }
    };
    crash::install(session.restorer());

    let mut stdout = io::stdout();
    if let Err(error) = write!(stdout, "terminal_probe: {mode}\r\n").and_then(|()| stdout.flush()) {
        eprintln!("terminal_probe: could not write to the terminal: {error}");
        return ExitCode::FAILURE;
    }

    match mode.as_str() {
        "quit" => ExitCode::SUCCESS,
        "panic" => panic!("terminal_probe was asked to panic"),
        "hang" => {
            thread::sleep(Duration::from_secs(30));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("terminal_probe: unknown mode {other}");
            ExitCode::FAILURE
        }
    }
}
