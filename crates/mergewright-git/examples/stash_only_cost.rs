//! Times `history_reads::only_in_stash` on the fixture `lost_cost` builds
//! (heavy synthetic, 50 000 refs, no commit-graph; m7-power-tools/14a).
//! Run by `scripts/m7-stash-only-cost.sh` in release.
//!
//! Stash-shaped commits are made with `git commit-tree`, one per fixture
//! lost commit, each with that lost commit as first parent (and a second
//! parent, as a real entry's index commit): so every entry's base is off
//! every branch, the case the reader exists for. Two cases, 1 entry and
//! 10 entries, and the bound itself: `unreached` over 500 reflog
//! candidates, as `lost_cost` times it. 10 warm runs, the three cases
//! interleaved in each, so the bound is measured in the same run on the
//! same machine state, not remembered (review m7-20k-to-20l). One
//! `stash-only-cost:` summary line per case; each stash case's per-entry
//! p95 against the same run's `unreached` p95.
//!
//! `stash_only_cost [commits refs]` runs a smaller repository.

use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_fixtures::Scale;
use mergewright_fixtures::synthetic::{self, Reflogged};
use mergewright_git::history::previous_tips;
use mergewright_git::history_reads::{only_in_stash, unreached};
use mergewright_git::repo::RepoHandle;

const RUNS: usize = 10;
const CAP: usize = 1000;
/// Reflog candidates `unreached` is timed over, as in `lost_cost`.
const CANDIDATES: usize = 500;
/// The stash cases, by entry count.
const COUNTS: [u8; 2] = [1, 10];

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
    let candidates = candidates(&repo, &fixture)?;
    println!("# unreached over {} reflog candidates", candidates.len());

    let mut stash: Vec<Vec<Duration>> = vec![Vec::new(); COUNTS.len()];
    let mut bound = Vec::new();
    for run in 1..=RUNS {
        for (case, &count) in COUNTS.iter().enumerate() {
            let entries = &entries[..usize::from(count).min(entries.len())];
            let started = Instant::now();
            let found = only_in_stash(&repo, entries, CAP, &()).map_err(|why| why.to_string())?;
            let took = started.elapsed();
            println!(
                "entries={count} run {run}: {} ms, {} commits",
                ms(took),
                found.commits.len()
            );
            stash[case].push(took);
        }
        let started = Instant::now();
        let found =
            unreached(&repo, &candidates, CANDIDATES, &()).map_err(|why| why.to_string())?;
        let took = started.elapsed();
        println!(
            "unreached run {run}: {} ms, {} lost runs",
            ms(took),
            found.lost.len()
        );
        bound.push(took);
    }

    report(&mut stash, &mut bound, candidates.len(), refs);
    Ok(())
}

/// One `stash-only-cost:` line for the bound, then one per stash case.
fn report(stash: &mut [Vec<Duration>], bound: &mut [Duration], candidates: usize, refs: usize) {
    let bound_p95 = p95(bound);
    println!(
        "stash-only-cost: unreached cands={} p50={} p95={} refs={refs} · the bound, this run",
        candidates,
        ms(bound[bound.len() / 2]),
        ms(bound_p95)
    );
    let bound_ms = bound_p95.as_secs_f64() * 1000.0;
    for (times, &count) in stash.iter_mut().zip(&COUNTS) {
        let p95 = p95(times);
        let per_entry = p95.as_secs_f64() * 1000.0 / f64::from(count);
        println!(
            "stash-only-cost: entries={count} p50={} p95={} per-entry={per_entry:.1} refs={refs} · bound {bound_ms:.1} ms per entry (unreached p95, same run) · {}",
            ms(times[times.len() / 2]),
            ms(p95),
            if per_entry <= bound_ms {
                "held"
            } else {
                "missed"
            }
        );
    }
}

/// Nearest-rank p95; sorts `times`.
fn p95(times: &mut [Duration]) -> Duration {
    times.sort();
    times[(times.len() * 95).div_ceil(100) - 1]
}

/// The newest distinct old values of every fixture reflog, as `lost_cost`
/// picks them.
fn candidates(repo: &RepoHandle, fixture: &Reflogged) -> Result<Vec<Oid>, String> {
    let mut all = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for log in &fixture.logs {
        let name = RefName::new(log.as_bytes()).map_err(|why| why.to_string())?;
        for id in previous_tips(repo, &name, usize::MAX).map_err(|why| why.to_string())? {
            if all.len() < CANDIDATES && seen.insert(id) {
                all.push(id);
            }
        }
    }
    Ok(all)
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
