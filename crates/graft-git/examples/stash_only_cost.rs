//! Times `history_reads::only_in_stash` on the fixture `lost_cost` builds
//! (heavy synthetic, 50 000 refs, no commit-graph; m7-power-tools/14a).
//! Run by `scripts/m7-stash-only-cost.sh` in release.
//!
//! Stash-shaped commits are made with `git commit-tree`, one per fixture
//! lost commit, each with that lost commit as first parent (and a second
//! parent, as a real entry's index commit): so every entry's base is off
//! every branch, the case the reader exists for. Two cases, 1 entry and
//! 10 entries, 10 warm runs each; one `stash-only-cost:` summary line per
//! case against `unreached`'s measured 190 ms p95.
//!
//! `stash_only_cost [commits refs]` runs a smaller repository.

use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use graft_core::oid::Oid;
use graft_fixtures::Scale;
use graft_fixtures::synthetic;
use graft_git::history_reads::only_in_stash;
use graft_git::repo::RepoHandle;

const RUNS: usize = 10;
const CAP: usize = 1000;
/// `unreached`'s p95 for 500 candidates (`docs/perf/lost-commit-cost.txt`).
const BOUND_MS: f64 = 190.0;

fn main() -> ExitCode {
    match measure() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("stash_only_cost: {why}");
            ExitCode::FAILURE
        }
    }
}

fn scale() -> Result<Scale, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Ok(Scale::HEAVY.with_refs(50_000)),
        [commits, refs] => {
            let commits = commits.parse().map_err(|why| format!("{commits}: {why}"))?;
            let refs = refs.parse().map_err(|why| format!("{refs}: {why}"))?;
            Ok(Scale::of(commits).with_refs(refs))
        }
        _ => Err("usage: stash_only_cost [commits refs]".to_owned()),
    }
}

fn measure() -> Result<(), String> {
    let scale = scale()?;
    let fixture = synthetic::with_reflog(scale).map_err(|why| why.to_string())?;
    let dir = fixture.scratch.path();
    let repo = RepoHandle::open(dir).map_err(|why| why.to_string())?;
    let refs = repo.references().map_err(|why| why.to_string())?.len();
    let mut entries = Vec::new();
    for lost in fixture.lost.iter().take(10) {
        entries.push(stash_shaped(dir, lost)?);
    }
    println!(
        "# synthetic {} ({} commits), {refs} refs, {} stash-shaped entries on lost bases",
        scale.name(),
        scale.commits(),
        entries.len()
    );
    for count in [1u8, 10] {
        let entries = &entries[..usize::from(count).min(entries.len())];
        let mut times = Vec::new();
        for run in 1..=RUNS {
            let started = Instant::now();
            let found = only_in_stash(&repo, entries, CAP, &()).map_err(|why| why.to_string())?;
            let took = started.elapsed();
            println!(
                "entries={count} run {run}: {} ms, {} commits",
                ms(took),
                found.commits.len()
            );
            times.push(took);
        }
        times.sort();
        let p95 = times[(times.len() * 95).div_ceil(100) - 1];
        let per_entry = p95.as_secs_f64() * 1000.0 / f64::from(count);
        println!(
            "stash-only-cost: entries={count} p95={} per-entry={per_entry:.1} refs={refs} · bound {BOUND_MS:.0} ms per entry · {}",
            ms(p95),
            if per_entry <= BOUND_MS {
                "held"
            } else {
                "missed"
            }
        );
    }
    Ok(())
}

/// A commit shaped like a stash entry: `lost` as first parent.
fn stash_shaped(dir: &std::path::Path, lost: &str) -> Result<Oid, String> {
    let output = Command::new("git")
        .args([
            "commit-tree",
            &format!("{lost}^{{tree}}"),
            "-p",
            lost,
            "-p",
            lost,
            "-m",
            "WIP on perf",
        ])
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Graft Perf")
        .env("GIT_AUTHOR_EMAIL", "perf@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Perf")
        .env("GIT_COMMITTER_EMAIL", "perf@example.invalid")
        .output()
        .map_err(|why| format!("git commit-tree: {why}"))?;
    if !output.status.success() {
        return Err(format!(
            "git commit-tree: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let hex = String::from_utf8_lossy(&output.stdout);
    Oid::from_hex(hex.trim()).map_err(|why| format!("{hex:?}: {why}"))
}

fn ms(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1000.0)
}
