//! The graph gallery (ticket m1-the-graph/35): one command that walks every
//! test repository so the graph can be judged by eye against TESTING.md's
//! "Graph gallery" section. Dev-only; nothing here is in the shipped binary.
//!
//! ```sh
//! cargo run -p mergewright --example gallery            # start at the first stop
//! cargo run -p mergewright --example gallery -- --start foxtrot-merge
//! cargo run -p mergewright --example gallery -- --list   # the stops, in order
//! cargo run -p mergewright --example gallery -- --matrix # coverage matrix, also
//!                                                  # written to docs/shots/
//! cargo run -p mergewright --example gallery -- --group details          # M2's stops
//! cargo run -p mergewright --example gallery -- --stems  # details capture stems
//! ```
//!
//! Inside: `]` next repository, `[` previous, `q` quit; every graph key and
//! `F3` work as in Graft. A banner under the header names the stop, its
//! position, its group, the keys and the tier, then what right looks like
//! and what sets the stop apart from its neighbours (m1-the-graph/38: the
//! footer alone did not orient a first-time viewer). Each step restarts the
//! run on the next repository, so the screen is re-probed and repainted (a
//! blink), which a dev tool can afford. A heavy repository that is not on
//! disk gets a page saying how to build it; on that page only `]`, `[` and
//! `q` do anything.

#[allow(
    dead_code,
    reason = "the walks' tables are theirs; `--matrix` writes the whole file"
)]
#[path = "../tests/it/coverage.rs"]
mod coverage;
#[path = "../tests/it/gallery.rs"]
mod gallery;

use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use gallery::{
    Availability, Entry, Group, banner, capture_stem, listing, missing_page, resolve, roots, stops,
};
use graft::event_loop::{self, Exit};
use graft::session::Startup;
use graft::terminal::{SessionOptions, TerminalSession};
use graft::update::Harness;
use graft_core::event::KeyCode;

const NEXT: KeyCode = KeyCode::Char(']');
const PREVIOUS: KeyCode = KeyCode::Char('[');
const USAGE: &str =
    "usage: gallery [--group graph|details] [--start <repo> | --list | --matrix | --stems]";

/// Where the walk goes after a stop.
enum Step {
    Next,
    Previous,
    Quit,
}

fn main() -> ExitCode {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // `--group` first, before whatever else the line says; the default
    // stays M1's so its checklist does not move.
    let mut group = Group::Graph;
    if args.first().map(String::as_str) == Some("--group") {
        let Some(chosen) = args.get(1).and_then(|word| Group::parse(word)) else {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        };
        group = chosen;
        args.drain(..2);
    }
    let all = stops(group);
    let start = match args.first().map(String::as_str) {
        None => 0,
        Some("--list") => {
            print!("{}", listing(&stops(Group::Graph)));
            print!("{}", listing(&stops(Group::Details)));
            return ExitCode::SUCCESS;
        }
        Some("--matrix") => return write_matrix(&workspace),
        Some("--stems") => {
            // `<nn> <stem>` per details stop: the capture script's file
            // names and the matrix's row keys come from the same function.
            for (index, entry) in stops(Group::Details).iter().enumerate() {
                println!("{:02} {}", index + 1, capture_stem(entry, index));
            }
            return ExitCode::SUCCESS;
        }
        Some("--start") => {
            let Some(name) = args.get(1) else {
                eprintln!("{USAGE}");
                return ExitCode::FAILURE;
            };
            // A details name repeats (five feature-branches stops): the
            // first is the one `--start` means; `]` reaches the rest.
            let Some(index) = all.iter().position(|entry| entry.name == name) else {
                eprintln!(
                    "gallery: no stop named {name} in the {} group; --list has them",
                    group.name()
                );
                return ExitCode::FAILURE;
            };
            index
        }
        Some(_) => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match walk(start, &all, &workspace) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gallery: {error}");
            ExitCode::FAILURE
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

/// Walk the stops from `index` until the user quits.
fn walk(mut index: usize, all: &[Entry], workspace: &Path) -> Result<(), String> {
    let roots = roots(workspace);
    loop {
        let entry = &all[index];
        let step = match resolve(entry, workspace, &roots) {
            Availability::Ready(path) => show(path, entry, index, all.len())?,
            Availability::Missing { build_with } => {
                show_missing(&missing_page(entry, index, all.len(), &build_with))?
            }
        };
        index = match step {
            Step::Next => (index + 1) % all.len(),
            Step::Previous => (index + all.len() - 1) % all.len(),
            Step::Quit => return Ok(()),
        };
    }
}

/// Run Graft on one repository until a step key or `q`.
fn show(repo_path: PathBuf, entry: &Entry, index: usize, total: usize) -> Result<Step, String> {
    let config = graft::config::load().map_err(|error| error.to_string())?;
    let startup = Startup {
        repo_path,
        config,
        harness: Harness {
            banner: Some(banner(entry, index, total)),
            hand_back: vec![NEXT, PREVIOUS],
            prelude: entry.prelude.clone(),
            clock: entry.clock(),
            // The corpus is borrowed read-only: never offer to write to it.
            offer_commit_graph: false,
        },
    };
    match event_loop::run(startup).map_err(|error| error.to_string())? {
        Exit::Quit => Ok(Step::Quit),
        Exit::HandedBack(key) if key == NEXT => Ok(Step::Next),
        Exit::HandedBack(_) => Ok(Step::Previous),
        Exit::Signal(signal) => {
            // Normally does not return: the process dies of the signal.
            event_loop::signals::reraise(signal)
                .map_err(|error| format!("could not re-raise signal {signal}: {error}"))?;
            Ok(Step::Quit)
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

/// The page for a repository that is not on disk. Raw mode, so lines end
/// in `\r\n`; escape sequences (arrow keys) are skipped rather than read.
fn show_missing(lines: &[String]) -> Result<Step, String> {
    let session =
        TerminalSession::on_stdout(SessionOptions::default()).map_err(|error| error.to_string())?;
    let shown = page(lines);
    let restored = session.restore();
    match (shown, restored) {
        (Ok(step), Ok(())) => Ok(step),
        (Err(error), _) | (_, Err(error)) => Err(error.to_string()),
    }
}

fn page(lines: &[String]) -> io::Result<Step> {
    let mut out = io::stdout();
    write!(out, "\x1b[2J\x1b[H")?;
    for line in lines {
        write!(out, " {line}\r\n")?;
    }
    out.flush()?;
    let mut stdin = io::stdin();
    let mut byte = [0u8; 1];
    loop {
        if stdin.read(&mut byte)? == 0 {
            return Ok(Step::Quit);
        }
        match byte[0] {
            b']' => return Ok(Step::Next),
            b'[' => return Ok(Step::Previous),
            b'q' | 0x03 => return Ok(Step::Quit),
            // ESC: swallow the rest of a CSI/SS3 sequence up to its final byte.
            0x1b => while stdin.read(&mut byte)? == 1 && !(0x40..=0x7e).contains(&byte[0]) {},
            _ => {}
        }
    }
}
