//! Runs the heavy tier's budget measurement by hand against any repository
//! (ticket m1-the-graph/27) — for `git/git`, which no test may touch
//! (TESTING.md "Getting `git/git`"):
//!
//! ```sh
//! cargo run --release -p graft --example budgets -- "$(scripts/clone-git-git.sh --where)"
//! ```
//!
//! Run it alone: the peak RSS is this process's, and the timings are wall
//! clock.

#[path = "../tests/it/budget_run.rs"]
mod budget_run;

use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some(repo) = std::env::args_os().nth(1).map(PathBuf::from) else {
        eprintln!("usage: budgets <repository>");
        return ExitCode::FAILURE;
    };
    match budget_run::run(&repo) {
        Ok(numbers) => {
            println!(
                "first paint {:?}, G {:?}, g {:?}, {} rows, provisional order: {}, peak RSS at paint {} MiB, after G {} MiB, end {} MiB",
                numbers.first_paint,
                numbers.bottom,
                numbers.top,
                numbers.rows,
                numbers.provisional,
                mib(numbers.rss_at_paint),
                mib(numbers.rss_at_bottom),
                mib(numbers.peak_rss),
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("budgets: {why}");
            ExitCode::FAILURE
        }
    }
}

fn mib(bytes: u64) -> String {
    format!(
        "{}.{}",
        bytes / (1024 * 1024),
        (bytes % (1024 * 1024)) * 10 / (1024 * 1024)
    )
}
