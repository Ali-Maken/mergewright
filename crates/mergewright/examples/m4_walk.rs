//! The M4 acceptance walk: one command that walks "you never leave Mergewright to
//! fetch, pull or push — not for a password, a rejection or a hang"
//! (m4-remotes/07).
//!
//! ```sh
//! cargo run -p mergewright --example m4_walk                              # every scenario
//! cargo run -p mergewright --example m4_walk -- --only push-plain         # one
//! cargo run -p mergewright --example m4_walk -- --list                    # the names
//! cargo run -p mergewright --example m4_walk -- --stems                   # capture stems
//! cargo run -p mergewright --example m4_walk -- --capture push-plain      # one scenario, live, for the pty script
//! cargo run -p mergewright --example m4_walk -- --matrix                  # docs/shots/35-coverage-matrix.txt
//! ```
//!
//! Each scenario builds a real remote — a bare over `file://`, smart HTTP
//! with Basic auth, or a port that never answers — drives the real app by
//! keys over the real job pool, then reads both repositories back by running
//! git. One line per scenario: `ok <name>` or `FAIL <name>: <what differs>`
//! (20 lines at most); exit 1 on any failure. `--capture` runs the
//! scenario's setup and its headless prefix, then opens the real interactive
//! loop on the work clone with the capture's keys as the prelude — what
//! `.scratch/m4-remotes/pty-walk-m4.py` records. The pure half is
//! `tests/it/m4_walk/`, which `cargo test` runs as one test per scenario.
//!
//! This binary is also git's **askpass helper** while it runs: the job pool
//! names `current_exe` as the helper, so when git asks for a password it
//! runs this example with `MWR_ASKPASS_SOCKET` set, and the process
//! answers the way `graft` itself does — never touching the terminal.

use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[allow(
    dead_code,
    reason = "the harness is shared with the integration tests, which use more of it"
)]
#[path = "../tests/it/app_harness.rs"]
mod app_harness;
#[allow(dead_code, reason = "only `--matrix` is used here")]
#[path = "../tests/it/coverage.rs"]
mod coverage;
#[path = "../tests/it/deadline.rs"]
mod deadline;
#[allow(
    dead_code,
    reason = "only its coverage tables are used here; the gallery uses the rest"
)]
#[path = "../tests/it/gallery.rs"]
mod gallery;
#[path = "../tests/it/m4_walk/mod.rs"]
mod m4_walk;
#[allow(
    dead_code,
    reason = "shared with the integration tests and the M3 walk, which use more of it"
)]
#[path = "../tests/it/repo_truth.rs"]
mod repo_truth;

use mergewright::cli::{self, Invocation};
use mergewright::event_loop::{self, Exit};
use mergewright::session::Startup;
use mergewright::update::Harness;
use m4_walk::{Scenario, capture_prelude, prepare_capture, run_scenario, scenarios, stems_line};

const USAGE: &str =
    "usage: m4_walk [--only <scenario> | --list | --stems | --capture <scenario> | --matrix]";

fn main() -> ExitCode {
    // Askpass mode first, before anything else: git ran this binary as its
    // credential helper during a captured push.
    let raw: Vec<OsString> = std::env::args_os().skip(1).collect();
    if let Some(Invocation::Askpass { socket, prompt }) =
        cli::askpass(std::env::var_os(cli::ASKPASS_SOCKET_VAR), &raw)
    {
        return askpass(&socket, &prompt);
    }

    let args: Vec<String> = raw
        .iter()
        .map(|arg| arg.to_string_lossy().into_owned())
        .collect();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    match (args.first().map(String::as_str), args.get(1)) {
        (None, _) => walk(None),
        (Some("--only"), Some(name)) => walk(Some(name)),
        (Some("--list"), None) => {
            for scenario in scenarios() {
                println!("{}", scenario.name);
            }
            ExitCode::SUCCESS
        }
        (Some("--stems"), None) => {
            for scenario in scenarios() {
                if let Some(capture) = &scenario.capture {
                    println!("{}", stems_line(&scenario, capture));
                }
            }
            ExitCode::SUCCESS
        }
        (Some("--capture"), Some(name)) => capture(name),
        (Some("--matrix"), None) => write_matrix(&workspace),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}

/// Askpass mode, as `graft`'s own `main` does it: pass git's `prompt` to
/// the running walk over `socket`, print its answer for git, exit. 0
/// answered, 1 cancelled, 2 anything else.
fn askpass(socket: &Path, prompt: &str) -> ExitCode {
    use mergewright_git::run::askpass::{RespondError, respond};
    let mut out = std::io::stdout().lock();
    match respond(socket, prompt, &mut out).and_then(|()| out.flush().map_err(RespondError::Io)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(RespondError::Cancelled) => ExitCode::from(1),
        Err(why) => {
            eprintln!("m4_walk askpass: {why}");
            ExitCode::from(2)
        }
    }
}

/// Run every scenario, or the one named, and print a line for each.
fn walk(only: Option<&str>) -> ExitCode {
    let all = scenarios();
    let chosen: Vec<&Scenario> = all
        .iter()
        .filter(|scenario| only.is_none_or(|name| name == scenario.name))
        .collect();
    if chosen.is_empty() {
        eprintln!(
            "no scenario named {}; --list names them",
            only.unwrap_or("")
        );
        return ExitCode::FAILURE;
    }
    let mut failed = 0;
    for scenario in &chosen {
        match run_scenario(scenario) {
            Ok(()) => println!("ok {}", scenario.name),
            Err(why) => {
                failed += 1;
                println!("FAIL {}: {why}", scenario.name);
            }
        }
    }
    println!("{} ok, {failed} failed", chosen.len() - failed);
    if failed == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Set the scenario up, then run Mergewright on its work clone interactively
/// with the capture's keys as the prelude, until `q` or a signal.
fn capture(name: &str) -> ExitCode {
    let all = scenarios();
    let Some(scenario) = all.iter().find(|scenario| scenario.name == name) else {
        eprintln!("no scenario named {name}; --stems names the ones with a capture");
        return ExitCode::FAILURE;
    };
    let Some(prelude) = scenario.capture.as_ref().map(capture_prelude) else {
        eprintln!("{name} has no capture");
        return ExitCode::FAILURE;
    };
    match show(scenario, prelude) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("m4_walk --capture {name}: {why}");
            ExitCode::FAILURE
        }
    }
}

fn show(
    scenario: &Scenario,
    prelude: Vec<mergewright_core::event::KeyPress>,
) -> Result<ExitCode, String> {
    // The fixture lives as long as the run: it holds the scratch and, for an
    // HTTP scenario, the server git talks to.
    let (_fixture, repo) = prepare_capture(scenario)?;
    let config = mergewright::config::load().map_err(|error| error.to_string())?;
    let startup = Startup {
        repo_path: repo,
        config,
        harness: Harness {
            prelude,
            // The work clone is a throwaway: never offer to write a
            // commit-graph into it, and pin nothing to the screen.
            offer_commit_graph: false,
            // Ages drawn against the pinned instant, so a re-shoot never
            // rolls with the calendar (m8/19d-b2).
            clock: mergewright::session::Clock::Fixed(mergewright_fixtures::PINNED_NOW),
            ..Harness::default()
        },
    };
    match event_loop::run(startup).map_err(|error| error.to_string())? {
        Exit::Quit | Exit::HandedBack(_) => Ok(ExitCode::SUCCESS),
        Exit::Signal(signal) => {
            // Normally does not return: the process dies of the signal.
            event_loop::signals::reraise(signal)
                .map_err(|error| format!("could not re-raise signal {signal}: {error}"))?;
            Ok(ExitCode::SUCCESS)
        }
        Exit::Fatal(reason) => Err(reason),
        // A walk never stops a write half-way; if one did, say so (its
        // exit line was printed when the pool stopped).
        Exit::StoppedHalfWay { what, after, .. } => Err(format!(
            "the stop box stopped {what} after {} s",
            after.as_secs()
        )),
        // `run` gives the terminal back after one repository; only `drive`
        // opens another place, so this would be a broken promise, said.
        Exit::Open(open) => Err(format!("`run` asked to open {}", open.path().display())),
        // The tour is reached only through `drive`; `run` never starts one.
        Exit::Practise { .. } | Exit::LeaveTour { .. } => {
            Err("`run` asked to start or leave the tour".to_owned())
        }
    }
}

/// Print the coverage matrix and write it where the ticket wants it.
fn write_matrix(workspace: &Path) -> ExitCode {
    match coverage::write_matrix(workspace) {
        Ok(matrix) => {
            print!("{matrix}");
            eprintln!(
                "written to {}",
                workspace.join(coverage::MATRIX_PATH).display()
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("could not write the coverage matrix: {why}");
            ExitCode::FAILURE
        }
    }
}
