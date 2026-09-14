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
//!
//! `--gate` runs only the gated path (open → first paint → `G` → `g`) and
//! exits, so `/usr/bin/time -l`'s process-wide numbers describe that path
//! and nothing else; the other measurements below lay history out again
//! (m1-the-graph/56).

#[path = "../tests/it/budget_run.rs"]
mod budget_run;
#[path = "budgets/no_cg.rs"]
mod no_cg;
#[path = "budgets/phases.rs"]
mod phases;

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let Some((mode, repo)) = arguments() else {
        eprintln!("usage: budgets [--phases | --gate] <repository>");
        return ExitCode::FAILURE;
    };
    if mode == Mode::Phases {
        return run_phases(&repo);
    }
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
    println!(
        "footprint now: {} after G, {} at end (m1-the-graph/57)",
        format_footprint(numbers.footprint_at_bottom),
        format_footprint(numbers.footprint_at_end),
    );
    if mode == Mode::Gate {
        return ExitCode::SUCCESS;
    }
    counters(&repo)
}

/// The counters that lay history out again after the gated path: order
/// bytes, what a cold `G` reads, two far jumps.
fn counters(repo: &Path) -> ExitCode {
    match budget_run::order_bytes(repo) {
        Ok((bytes, commits)) => {
            let per_commit = bytes.div_ceil(commits.max(1));
            println!(
                "EmittedOrder: {bytes} B total over {commits} commits, {per_commit} B/commit (m1-the-graph/51c); peak RSS now {}",
                peak_now()
            );
            println!("{}", footprint_now());
        }
        Err(why) => {
            eprintln!("budgets: EmittedOrder bytes: {why}");
            return ExitCode::FAILURE;
        }
    }
    match budget_run::cold_jump(repo) {
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
            println!("peak RSS after the cold G measurement {}", peak_now());
            println!("{}", footprint_now());
        }
        Err(why) => {
            eprintln!("budgets: cold jump: {why}");
            return ExitCode::FAILURE;
        }
    }
    // A `Find` after a cold `G`, half way down and nine tenths of the way
    // down: checkpoints thin to an even grid rather than evicting
    // least-recently-used (m1-the-graph/60), so any jump replays at most one
    // stride's worth of links, wherever in history it lands.
    for (name, at) in [
        ("half way", (|rows| rows / 2) as fn(usize) -> usize),
        ("nine tenths", |rows| rows / 10 * 9),
    ] {
        match budget_run::far_jump(repo, at) {
            Ok(jump) => println!(
                "far jump {name}, to row {} of {}: {:?}, replayed {} rows, {} chunks rebuilt, hydrated {}, checkpoint stride {} (m1-the-graph/56, 60)",
                jump.row,
                jump.rows,
                jump.took,
                jump.rows_replayed,
                jump.chunks_rebuilt,
                jump.hydrated,
                jump.checkpoint_stride,
            ),
            Err(why) => {
                eprintln!("budgets: far jump {name}: {why}");
                return ExitCode::FAILURE;
            }
        }
    }
    println!("peak RSS after the far jump measurements {}", peak_now());
    println!("{}", footprint_now());
    ExitCode::SUCCESS
}

/// `--phases`: first paint cut into phases, serially (m1-the-graph/55).
fn run_phases(repo: &std::path::Path) -> ExitCode {
    match phases::run(repo).and_then(|()| no_cg::run(repo)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("budgets --phases: {why}");
            ExitCode::FAILURE
        }
    }
}

/// What to measure.
#[derive(PartialEq, Eq, Clone, Copy)]
enum Mode {
    /// Every measurement.
    All,
    /// First paint cut into phases (`--phases`).
    Phases,
    /// Only the gated path, then exit (`--gate`).
    Gate,
}

/// `[--phases | --gate] <repository>`: the mode, and the repository.
fn arguments() -> Option<(Mode, PathBuf)> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    let mode = match args.first() {
        Some(flag) if flag == "--phases" => Mode::Phases,
        Some(flag) if flag == "--gate" => Mode::Gate,
        _ => Mode::All,
    };
    if mode != Mode::All {
        args.remove(0);
    }
    args.first().map(|repo| (mode, PathBuf::from(repo)))
}

/// This process's peak RSS so far, in MiB, or why it could not be read.
fn peak_now() -> String {
    match graft_fixtures::peak_rss() {
        Ok(bytes) => format!("{} MiB", mib(bytes)),
        Err(why) => format!("unreadable: {why}"),
    }
}

/// This process's physical footprint right now, `<current MiB> / peak
/// <peak MiB>` (m1-the-graph/57), or why it could not be read — macOS only,
/// see `graft_fixtures::MemoryError::Unsupported`.
fn footprint_now() -> String {
    format!(
        "footprint now: {}",
        format_footprint(graft_fixtures::footprint())
    )
}

/// `<current MiB> / peak <peak MiB>` for an already-taken reading, or why it
/// could not be read.
fn format_footprint(
    footprint: Result<graft_fixtures::Footprint, graft_fixtures::MemoryError>,
) -> String {
    match footprint {
        Ok(footprint) => format!(
            "{} MiB / peak {} MiB",
            mib(footprint.current),
            mib(footprint.peak)
        ),
        Err(why) => format!("unreadable: {why}"),
    }
}

fn mib(bytes: u64) -> String {
    format!(
        "{}.{}",
        bytes / (1024 * 1024),
        (bytes % (1024 * 1024)) * 10 / (1024 * 1024)
    )
}
