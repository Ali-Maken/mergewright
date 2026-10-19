//! What the branch pane's read costs (ease-pass 09 Cost): `branch_status`
//! for one focused branch, and for every branch tip in a 40-row window (the
//! input 09b's `not merged` label needs). Run by
//! `scripts/branch-pane-cost.sh`; never in a helper's context.
//!
//! The repository is the team fixture (`mergewright_fixtures::team`):
//! `MONOREPO` under `heavy-tests`, `SMALL` without. Candidates are built as
//! the app builds them (ruling R): every local branch, and every
//! remote-tracking branch no local branch tracks — the fixture configures
//! no upstreams, so all of `refs/remotes/origin/*` — `main` the trunk.
//!
//! Lines:
//! - `branch-pane-cost: candidates=<n> local=<n> remote-only=<n>`
//! - `branch-pane-cost: focus runs=<n> p50_ms=… p95_ms=… spawns=<n>`
//! - `branch-pane-cost: window rows=40 tips=<n> total_ms=… spawns=<n>`
//!
//! `spawns` counts lines appended to `$MWR_SPAWN_LOG` (the script's `git`
//! shim writes one per spawn); `-` when it is unset. The script runs the
//! timings without the shim and the counts with it, so the shim's own cost
//! never lands in a time.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use mergewright_core::oid::Oid;
use mergewright_fixtures::team::{self, TeamSpec};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_ops::branch_status::{Candidate, CandidateKind, branch_status};
use mergewright_ops::operation::Tip;

const FOCUS_RUNS: usize = 10;
const WINDOW: usize = 40;

/// Never asks a git call to stop.
struct Never;

impl Stop for Never {
    fn should_stop(&self) -> bool {
        false
    }
}

fn main() -> ExitCode {
    match measure() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("branch-pane-cost: {why}");
            ExitCode::FAILURE
        }
    }
}

fn spec() -> TeamSpec {
    if cfg!(feature = "heavy-tests") {
        TeamSpec::MONOREPO
    } else {
        TeamSpec::SMALL
    }
}

/// Lines of `$MWR_SPAWN_LOG` so far, or `None` when it is unset.
fn spawns() -> Option<usize> {
    let path = std::env::var_os("MWR_SPAWN_LOG")?;
    // A log not yet written means no spawn yet.
    Some(std::fs::read_to_string(path).map_or(0, |text| text.lines().count()))
}

fn spawned(before: Option<usize>) -> String {
    match (before, spawns()) {
        (Some(before), Some(after)) => (after - before).to_string(),
        _ => "-".to_owned(),
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(|why| format!("git {args:?}: {why}"))?;
    if !output.status.success() {
        return Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|why| format!("git {args:?}: {why}"))
}

fn candidates(repo: &Path) -> Result<Vec<Candidate>, String> {
    let listed = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(objectname) %(refname)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;
    let mut all = Vec::new();
    for line in listed.lines() {
        let (hex, name) = line.split_once(' ').ok_or("a for-each-ref line")?;
        if name.ends_with("/HEAD") {
            continue;
        }
        let tip = Oid::from_hex(hex).map_err(|why| format!("{hex}: {why}"))?;
        let kind = if name == "refs/heads/main" {
            CandidateKind::Trunk
        } else {
            CandidateKind::Other
        };
        all.push(Candidate {
            name: name.to_owned(),
            tip,
            kind,
        });
    }
    all.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(all)
}

/// A remote-only branch is its own upstream, as the app's job passes it
/// (ruling R); the fixture's local branches track nothing.
fn own_upstream(branch: &Candidate) -> Option<Tip> {
    branch.name.starts_with("refs/remotes/").then(|| Tip {
        name: branch.name.clone(),
        oid: branch.tip,
    })
}

fn ms(duration: Duration) -> String {
    format!("{:.1}", duration.as_secs_f64() * 1000.0)
}

fn percentile(sorted: &[Duration], p: usize) -> Duration {
    let at = (sorted.len() * p).div_ceil(100).saturating_sub(1);
    sorted[at.min(sorted.len() - 1)]
}

fn measure() -> Result<(), String> {
    let path = team::borrow(&spec()).map_err(|why| why.to_string())?;
    let repo = RepoHandle::open(&path).map_err(|why| why.to_string())?;
    let all = candidates(&path)?;
    let local = all
        .iter()
        .filter(|c| c.name.starts_with("refs/heads/"))
        .count();
    println!(
        "branch-pane-cost: candidates={} local={local} remote-only={}",
        all.len(),
        all.len() - local
    );

    // One focus: the newest local feature branch, read FOCUS_RUNS times.
    let focus = all
        .iter()
        .filter(|c| c.name.starts_with("refs/heads/feature/"))
        .max_by_key(|c| c.name.clone())
        .ok_or("no local feature branch")?;
    let mut times = Vec::with_capacity(FOCUS_RUNS);
    let mut one_spawn = String::new();
    for _ in 0..FOCUS_RUNS {
        let before = spawns();
        let started = Instant::now();
        branch_status(&repo, focus, &all, None, &Never).map_err(|why| why.to_string())?;
        times.push(started.elapsed());
        one_spawn = spawned(before);
    }
    times.sort();
    println!(
        "branch-pane-cost: focus runs={FOCUS_RUNS} p50_ms={} p95_ms={} spawns={one_spawn}",
        ms(percentile(&times, 50)),
        ms(percentile(&times, 95))
    );

    // Every branch whose tip is one of the 40 newest rows of the walk.
    let rows = git(
        &path,
        &["rev-list", "--topo-order", "--all", &format!("-n{WINDOW}")],
    )?;
    let window: Vec<&str> = rows.lines().collect();
    let tips: Vec<&Candidate> = all
        .iter()
        .filter(|c| window.contains(&c.tip.to_string().as_str()))
        .collect();
    let before = spawns();
    let started = Instant::now();
    for branch in &tips {
        branch_status(&repo, branch, &all, own_upstream(branch).as_ref(), &Never)
            .map_err(|why| why.to_string())?;
    }
    println!(
        "branch-pane-cost: window rows={WINDOW} tips={} total_ms={} spawns={}",
        tips.len(),
        ms(started.elapsed()),
        spawned(before)
    );
    Ok(())
}
