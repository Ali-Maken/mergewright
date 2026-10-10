//! `cargo run -p graft-fixtures` — build the fixture corpus, or confirm it is
//! already built.
//!
//! Prints where the corpus is, whether this run built it, how long it took,
//! how much disk it uses, and any shape this machine cannot hold. Running it
//! twice is the check that the second run is a no-op.
//!
//! Two subcommands put the capture rules where a program that is not a Rust
//! test can reach them (m5-history-and-ghost-preview/41):
//!
//! ```text
//! graft-fixtures capture <path>     # the drawn frame on stdin
//! graft-fixtures redact <path>...
//! ```
//!
//! `capture` is [`graft_fixtures::capture`] exactly — redact, compare, fail
//! on drift, rewrite only under `TRIB_UPDATE_SHOTS=1` and stay red when it
//! does. It exists so `pty-walk-*.py`, which draws its frames under a real
//! pty and so cannot be a Rust test, asserts on the same terms as every other
//! capture rather than inventing a second convention.
//!
//! `redact` is the same redaction applied to a file that was written before
//! the rule existed. It is not a sweep with a different instrument: it is the
//! writers' own function, run once over what predates them.

#![forbid(unsafe_code)]

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args_os().skip(1);
    let first = args.next();
    match first.as_deref().and_then(std::ffi::OsStr::to_str) {
        None => corpus(),
        Some("capture") => match args.next() {
            Some(path) => capture(Path::new(&path)),
            None => usage("capture wants the path of the file to assert"),
        },
        Some("redact") => {
            let paths: Vec<PathBuf> = args.map(PathBuf::from).collect();
            if paths.is_empty() {
                usage("redact wants at least one path")
            } else {
                redact(&paths)
            }
        }
        Some(other) => usage(&format!("no such subcommand: {other}")),
    }
}

fn usage(why: &str) -> ExitCode {
    eprintln!("error: {why}");
    eprintln!("usage: graft-fixtures [capture <path> | redact <path>...]");
    ExitCode::FAILURE
}

/// One capture, its drawn body on stdin, on `graft_fixtures::capture`'s terms.
fn capture(path: &Path) -> ExitCode {
    let mut drawn = Vec::new();
    if let Err(why) = std::io::stdin().read_to_end(&mut drawn) {
        eprintln!("error: the drawn capture could not be read from stdin: {why}");
        return ExitCode::FAILURE;
    }
    match String::from_utf8(drawn) {
        Ok(text) => match graft_fixtures::check(path, &text) {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("{why}");
                ExitCode::FAILURE
            }
        },
        Err(why) => match graft_fixtures::check_bytes(path, why.as_bytes()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(why) => {
                eprintln!("{why}");
                ExitCode::FAILURE
            }
        },
    }
}

/// Rewrites each file with every home directory, scratch path and ephemeral
/// port replaced by a placeholder of the same width.
///
/// Prints one line per file that changed and one count at the end; a file
/// that was already clean is left untouched, so re-running it is a no-op.
fn redact(paths: &[PathBuf]) -> ExitCode {
    let mut changed = 0usize;
    let mut failed = false;
    for path in paths {
        let before = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(why) => {
                eprintln!("error: {} could not be read: {why}", path.display());
                failed = true;
                continue;
            }
        };
        let after = graft_fixtures::redacted_bytes(&before);
        if after == before {
            continue;
        }
        let found = graft_fixtures::found(&before).len();
        if let Err(why) = std::fs::write(path, &after) {
            eprintln!("error: {} could not be written: {why}", path.display());
            failed = true;
            continue;
        }
        changed += 1;
        println!("{}: {found} redacted", path.display());
    }
    println!("{changed} of {} files changed", paths.len());
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn corpus() -> ExitCode {
    match graft_fixtures::ensure() {
        Ok(report) => {
            let action = if report.built {
                "built"
            } else {
                "already built"
            };
            println!("corpus   {}", report.root.display());
            println!("state    {action} in {:.2?}", report.elapsed);
            println!("size     {:.1} MiB", bytes_to_mib(report.bytes));
            if report.unavailable.is_empty() {
                println!("skipped  none");
            } else {
                for skipped in &report.unavailable {
                    println!("skipped  {}: {}", skipped.shape, skipped.reason);
                }
            }
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("error: {why}");
            ExitCode::FAILURE
        }
    }
}

fn bytes_to_mib(bytes: u64) -> f64 {
    #[allow(
        clippy::cast_precision_loss,
        reason = "a corpus size is reported to one decimal place; f64 is exact far past that"
    )]
    let bytes = bytes as f64;
    bytes / (1024.0 * 1024.0)
}
