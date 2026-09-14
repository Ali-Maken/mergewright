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
    let numbers = match budget_run::run(&repo) {
        Ok(numbers) => numbers,
        Err(why) => {
            eprintln!("budgets: {why}");
            return ExitCode::FAILURE;
        }
    };
    println!(
        "first paint {:?}, G {:?}, g {:?}, {} rows, provisional order: {}, peak RSS at paint {} MiB, after G {} MiB, end {} MiB, overflow_len {} (m1-the-graph/51b 2x check)",
        numbers.first_paint,
        numbers.bottom,
        numbers.top,
        numbers.rows,
        numbers.provisional,
        mib(numbers.rss_at_paint),
        mib(numbers.rss_at_bottom),
        mib(numbers.peak_rss),
        numbers.overflow_len,
    );
    match budget_run::order_bytes(&repo) {
        Ok((bytes, commits)) => {
            let per_commit = bytes.div_ceil(commits.max(1));
            println!(
                "EmittedOrder: {bytes} B total over {commits} commits, {per_commit} B/commit (m1-the-graph/51c)"
            );
        }
        Err(why) => {
            eprintln!("budgets: EmittedOrder bytes: {why}");
            return ExitCode::FAILURE;
        }
    }
    match budget_run::cold_jump(&repo) {
        Ok(jump) => {
            let work = jump.work;
            println!(
                "cold G, viewport at the top: hydrated {} (bound {}), merge_reads {} (bound {}), links_read {}, pulled {}, rows {} (m1-the-graph/52b)",
                work.hydrated,
                jump.hydrated_bound,
                work.merge_reads,
                jump.merge_bound,
                work.links_read,
                work.commits_pulled,
                jump.rows,
            );
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("budgets: cold jump: {why}");
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
