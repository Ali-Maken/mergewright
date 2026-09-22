//! The M5 acceptance walk: one command that walks "you see the result
//! before you confirm" (m5-history-and-ghost-preview/11).
//!
//! ```sh
//! cargo run -p graft --example m5_walk                                          # every scenario
//! cargo run -p graft --example m5_walk -- --only rebase-two-commits-ghost-equals-git   # one
//! cargo run -p graft --example m5_walk -- --list                                # the names
//! cargo run -p graft --example m5_walk -- --stems                               # capture stems
//! cargo run -p graft --example m5_walk -- --capture merge-no-ff-ghost-equals-git   # one scenario, live, for the pty script
//! cargo run -p graft --example m5_walk -- --matrix                              # docs/shots/35-coverage-matrix.txt
//! ```
//!
//! Each scenario copies a corpus shape to scratch, scripts a history into
//! it with git, drives the real app by keys (and, for the drop menu, the
//! mouse) over the real job pool, then reads the repository back by running
//! git. The one thing it reads from the app is the plan behind the preview
//! box, so it can say the ghost equals what git did. One line per
//! scenario: `ok <name>` or `FAIL <name>: <what differs>` (20 lines at
//! most); exit 1 on any failure. `--capture` runs the scenario's setup and
//! its headless prefix, then opens the real interactive loop on the scratch
//! with the capture's keys as the prelude — what
//! `.scratch/m5-history-and-ghost-preview/pty-walk-m5.py` records. The pure
//! half is `tests/it/m5_walk/`, which `cargo test` runs as one test per
//! scenario.

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
#[allow(
    dead_code,
    reason = "only its coverage tables are used here; the gallery uses the rest"
)]
#[path = "../tests/it/gallery.rs"]
mod gallery;
#[path = "../tests/it/m5_walk/mod.rs"]
mod m5_walk;
#[allow(
    dead_code,
    reason = "shared with the integration tests and the other walks, which use more of it"
)]
#[path = "../tests/it/repo_truth.rs"]
mod repo_truth;

use graft::event_loop::{self, Exit};
use graft::session::Startup;
use graft::update::Harness;
use m5_walk::{Scenario, capture_prelude, prepare_capture, run_scenario, scenarios, stems_line};

const USAGE: &str =
    "usage: m5_walk [--only <scenario> | --list | --stems | --capture <scenario> | --matrix]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
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

/// Set the scenario up, then run Graft on its scratch interactively with
/// the capture's keys as the prelude, until `q` or a signal.
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
            eprintln!("m5_walk --capture {name}: {why}");
            ExitCode::FAILURE
        }
    }
}

fn show(
    scenario: &Scenario,
    prelude: Vec<graft_core::event::KeyPress>,
) -> Result<ExitCode, String> {
    // The fixture lives as long as the run: it holds the scratch and, for a
    // remote scenario, the bare.
    let (_fixture, repo) = prepare_capture(scenario)?;
    let config = graft::config::load().map_err(|error| error.to_string())?;
    let startup = Startup {
        repo_path: repo,
        config,
        harness: Harness {
            prelude,
            // The scratch is a throwaway: never offer to write a
            // commit-graph into it, and pin nothing to the screen.
            offer_commit_graph: false,
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
