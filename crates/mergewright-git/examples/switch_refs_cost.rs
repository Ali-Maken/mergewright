//! Splits `switch_refs` (the `C` picker's and the palette's branch read) by
//! phase on one repository (m8-polish-and-ship/12c). Run by
//! `scripts/m8-switch-refs.sh` in release; no product code is changed.
//!
//! `switch_refs_cost <repo>`: 10 warm runs of each, interleaved,
//! - `git`: the `for-each-ref` process alone, with `switch_refs`' own
//!   format and patterns (`switch_refs.rs:58-66`), run by this program;
//! - `previous`: the two `@{-1}` processes (`switch_refs.rs:76-97`);
//! - `total`: `switch_refs` itself — the three processes plus
//!   `parse_listing` (`switch_refs.rs:127`), which is private, so its time
//!   is printed as `total - git - previous` and labelled derived.
//!
//! One `switch-refs-cost:` line per phase, milliseconds, numbers only.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::Instant;

use mergewright_git::repo::RepoHandle;
use mergewright_git::switch_refs::switch_refs;

const RUNS: usize = 10;

/// `switch_refs`' `--format`, copied: the process timed alone must be
/// asked for the same fields.
const FORMAT: &str = "--format=%(refname)%00%(objectname)%00%(symref)%00%(upstream)%00\
%(subject)%00%(authorname)%00%(committerdate:unix)";

fn main() -> ExitCode {
    let Some(repo) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: switch_refs_cost <repo>");
        return ExitCode::FAILURE;
    };
    match measure(&repo) {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("switch_refs_cost: {why}");
            ExitCode::FAILURE
        }
    }
}

/// One git process: its wall time, its stdout's length, and whether it
/// exited 0.
fn git(repo: &Path, args: &[&str]) -> Result<(f64, usize, bool), String> {
    let started = Instant::now();
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|why| format!("git {}: {why}", args.join(" ")))?;
    let ms = started.elapsed().as_secs_f64() * 1000.0;
    // `rev-parse --verify --quiet` exits 1 when there is no `@{-1}`, which
    // is an answer, not a failure (`switch_refs::has_previous`).
    if !output.status.success() && output.status.code() != Some(1) {
        return Err(format!("git {} exited {}", args.join(" "), output.status));
    }
    Ok((ms, output.stdout.len(), output.status.success()))
}

fn measure(repo: &Path) -> Result<(), String> {
    let handle = RepoHandle::open(repo).map_err(|why| why.to_string())?;
    let (mut git_ms, mut previous_ms, mut total_ms) = (Vec::new(), Vec::new(), Vec::new());
    let mut records = 0;
    let mut bytes = 0;
    for _ in 0..RUNS {
        let (ms, out, _) = git(
            repo,
            &["for-each-ref", FORMAT, "refs/heads", "refs/remotes"],
        )?;
        git_ms.push(ms);
        bytes = out;

        // As `switch_refs::previous_branch`: the name is asked for only
        // when `rev-parse` found a previous checkout.
        let (verify, _, found) = git(repo, &["rev-parse", "--verify", "--quiet", "@{-1}"])?;
        let named = if found {
            git(repo, &["check-ref-format", "--branch", "@{-1}"])?.0
        } else {
            0.0
        };
        previous_ms.push(verify + named);

        let started = Instant::now();
        let read = switch_refs(&handle, &()).map_err(|why| why.to_string())?;
        total_ms.push(started.elapsed().as_secs_f64() * 1000.0);
        records = read.refs.len();
    }
    let git = summary(&mut git_ms);
    let previous = summary(&mut previous_ms);
    let total = summary(&mut total_ms);
    println!("switch-refs-cost: records={records} for_each_ref_bytes={bytes} runs={RUNS}");
    println!(
        "switch-refs-cost: phase=git for_each_ref p50_ms={:.1} p95_ms={:.1}",
        git.0, git.1
    );
    println!(
        "switch-refs-cost: phase=previous rev_parse+check_ref_format p50_ms={:.1} p95_ms={:.1}",
        previous.0, previous.1
    );
    println!(
        "switch-refs-cost: phase=total switch_refs p50_ms={:.1} p95_ms={:.1}",
        total.0, total.1
    );
    println!(
        "switch-refs-cost: phase=parse derived p50_ms={:.1}",
        total.0 - git.0 - previous.0
    );
    Ok(())
}

/// p50 and p95, nearest rank.
fn summary(values: &mut [f64]) -> (f64, f64) {
    values.sort_by(f64::total_cmp);
    let rank = |p: usize| values[((p * values.len()).div_ceil(100)).max(1) - 1];
    (rank(50), rank(95))
}
