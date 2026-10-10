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
use graft::update::Harness;

fn main() -> ExitCode {
    // Settled first, before any config load, terminal probe or log line: in
    // askpass mode (git ran Graft's own binary as its credential helper during
    // a fetch, pull or push — design note m4, choice 2) this process must
    // never touch the terminal the running Graft owns.
    let repo_path = match cli::from_process() {
        Ok(Invocation::Askpass { socket, prompt }) => return askpass(&socket, &prompt),
        Ok(Invocation::Open(path)) => path,
        Ok(Invocation::Usage(usage) | Invocation::Version(usage)) => {
            return print_to_stdout(&format!("{usage}\n"));
        }
        // For packagers and `docs/man/graft.1`: the page from the live
        // tables, nothing else on stdout.
        Ok(Invocation::GenerateMan) => {
            return print_to_stdout(&cli::man_page(
                cli::FLAGS,
                graft_ui::keymap::defaults(),
                cli::CONFIG_KEYS,
            ));
        }
        Err(error) => {
            report(&error.to_string());
            return ExitCode::FAILURE;
        }
    };

    // Before the terminal session, like the config: a bad clock pin is a
    // plain line and exit 2, never a screen (m8/19d-b).
    let clock = match graft::session::launch_clock() {
        Ok(clock) => clock,
        Err(words) => {
            report(&words);
            return ExitCode::from(2);
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

    match start(repo_path, config, clock) {
        // The harness is the default, so `HandedBack` cannot happen; if it
        // did, the loop stopped where it was told to, which is a success.
        // `Open`, `Practise` and `LeaveTour` are handled by `drive` and never
        // returned; `run` never produces them. The stop box's `y` (m8/13b review fix) is a success
        // too: he asked for it, and its one line, saying where git was left,
        // was printed when the pool stopped (`event_loop::exit_wait`, 13d),
        // which is when its answer is known.
        Ok(
            Exit::Quit
            | Exit::HandedBack(_)
            | Exit::Open(_)
            | Exit::Practise { .. }
            | Exit::LeaveTour { .. }
            | Exit::StoppedHalfWay { .. },
        ) => ExitCode::SUCCESS,
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

/// Where to start, decided before the terminal is touched
/// (m8-polish-and-ship/08): a stdout that is not a terminal runs as it
/// always did (outside a repository that is today's one-line error, exit 1:
/// a pipe gets an error, not a screen); otherwise the path is located — a
/// broken repository is still a plain line — and the terminal opened once
/// for every place that follows.
fn start(
    repo_path: std::path::PathBuf,
    config: config::Loaded,
    clock: graft::session::Clock,
) -> std::io::Result<Exit> {
    use std::io::IsTerminal;
    if !io::stdout().is_terminal() {
        return event_loop::run(Startup {
            repo_path,
            config,
            harness: Harness {
                clock,
                ..Harness::default()
            },
        });
    }
    let place = match event_loop::locate(&repo_path, &config) {
        Ok(place) => place,
        Err(reason) => return Ok(Exit::Fatal(reason)),
    };
    let terminal = event_loop::Terminal::open()?;
    event_loop::drive(terminal, place, &config, clock)
}

/// Write `text` to stdout for `--help`, `--version` or `--generate-man`.
/// A reader that closed early (`graft --generate-man | head`) is a clean
/// exit: it asked for no more, and Rust ignores SIGPIPE so the write would
/// otherwise panic. Safe to treat as success because nothing else is lost —
/// this text is all the process does. Any other write failure is exit 1.
fn print_to_stdout(text: &str) -> ExitCode {
    let mut stdout = io::stdout().lock();
    match stdout
        .write_all(text.as_bytes())
        .and_then(|()| stdout.flush())
    {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(error) => {
            report(&format!("cannot write to stdout: {error}"));
            ExitCode::FAILURE
        }
    }
}

/// Say what went wrong on stderr, now that the terminal is the user's again.
fn report(message: &str) {
    let mut stderr = io::stderr();
    // If even stderr has gone there is nowhere left to complain to, and the
    // exit code still carries the failure.
    writeln!(stderr, "{}: {message}", cli::COMMAND)
        .and_then(|()| stderr.flush())
        .unwrap_or(());
}

/// Askpass mode: pass git's `prompt` to the running Graft over `socket`,
/// print its answer for git, exit. The answer goes to stdout and nowhere
/// else. Exit 0 answered, 1 cancelled, 2 anything else, with one line on
/// stderr naming it (git shows it to no one and reports "could not read
/// Password", which Graft classifies as an authentication failure).
fn askpass(socket: &std::path::Path, prompt: &str) -> ExitCode {
    use graft_git::run::askpass::{RespondError, respond};
    let mut out = io::stdout().lock();
    match respond(socket, prompt, &mut out).and_then(|()| out.flush().map_err(RespondError::Io)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(RespondError::Cancelled) => ExitCode::from(1),
        Err(why) => {
            report(&why.to_string());
            ExitCode::from(2)
        }
    }
}
