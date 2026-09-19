//! The M3 acceptance walk: one command that walks "you cannot lose work"
//! (m3-working-tree-undo-and-stash/07).
//!
//! ```sh
//! cargo run -p graft --example m3_walk                       # every scenario
//! cargo run -p graft --example m3_walk -- --only stash-drop  # one
//! cargo run -p graft --example m3_walk -- --list             # the names
//! cargo run -p graft --example m3_walk -- --stems            # capture stems
//! cargo run -p graft --example m3_walk -- --capture commit   # one scenario, live, for the pty script
//! cargo run -p graft --example m3_walk -- --matrix           # docs/shots/35-coverage-matrix.txt
//! ```
//!
//! Each scenario copies a corpus shape into a scratch directory, scripts a
//! dirty state, drives the real app by keys over the real job pool, then
//! reads the repository back by running git. One line per scenario:
//! `ok <name>` or `FAIL <name>: <what differs>` (20 lines at most); exit 1
//! on any failure. `--capture` runs the scenario's setup and its headless
//! prefix, then opens the real interactive loop on the scratch repository
//! with the scenario's capture keys as the prelude — that is what
//! `.scratch/m3-working-tree-undo-and-stash/pty-walk-m3.py` records.
//! The pure half is `tests/it/m3_walk.rs`, which `cargo test` runs as one
//! test per scenario.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[allow(
    dead_code,
    reason = "the harness is shared with the integration tests, which use more of it"
)]
#[path = "../tests/it/app_harness.rs"]
mod app_harness;
#[allow(
    dead_code,
    reason = "only the coverage matrix is used here; the gallery uses the rest"
)]
#[path = "../tests/it/gallery.rs"]
mod gallery;
#[path = "../tests/it/m3_walk.rs"]
mod m3_walk;
#[allow(
    dead_code,
    reason = "the coverage matrix lists the M4 walk's captures; its scenarios are not run here"
)]
#[path = "../tests/it/m4_walk.rs"]
mod m4_walk;

use graft::event_loop::{self, Exit};
use graft::session::Startup;
use graft::update::Harness;
use m3_walk::{Scenario, prepare_capture, run_scenario, scenarios};

const USAGE: &str =
    "usage: m3_walk [--only <scenario> | --list | --stems | --capture <scenario> | --matrix]";

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
                    println!(
                        "{}\t{}\t{}\t{}",
                        capture.stem,
                        scenario.name,
                        capture.on_screen,
                        m3_walk::keys_label(&capture.keys)
                    );
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

/// Set the scenario up, then run Graft on it interactively with the
/// capture's keys as the prelude, until `q` or a signal.
fn capture(name: &str) -> ExitCode {
    let all = scenarios();
    let Some(scenario) = all.iter().find(|scenario| scenario.name == name) else {
        eprintln!("no scenario named {name}; --stems names the ones with a capture");
        return ExitCode::FAILURE;
    };
    let Some(keys) = scenario
        .capture
        .as_ref()
        .map(|capture| capture.keys.clone())
    else {
        eprintln!("{name} has no capture");
        return ExitCode::FAILURE;
    };
    match show(scenario, keys) {
        Ok(code) => code,
        Err(why) => {
            eprintln!("m3_walk --capture {name}: {why}");
            ExitCode::FAILURE
        }
    }
}

fn show(
    scenario: &Scenario,
    prelude: Vec<graft_core::event::KeyPress>,
) -> Result<ExitCode, String> {
    // The scratch lives as long as the run: dropping it deletes the repository.
    let (_scratch, repo) = prepare_capture(scenario)?;
    let config = graft::config::load().map_err(|error| error.to_string())?;
    let startup = Startup {
        repo_path: repo,
        config,
        harness: Harness {
            prelude,
            // The scratch is a throwaway copy: never offer to write a
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

/// Every coverage table (the gallery's two, the working-tree one, the
/// remotes one), written where the gallery writes them.
fn write_matrix(workspace: &Path) -> ExitCode {
    let matrix = gallery::coverage_matrix(workspace);
    print!("{matrix}");
    let out = workspace.join("docs/shots/35-coverage-matrix.txt");
    match std::fs::write(&out, &matrix) {
        Ok(()) => {
            eprintln!("written to {}", out.display());
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("m3_walk: could not write {}: {error}", out.display());
            ExitCode::FAILURE
        }
    }
}
