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
//! - `branch-pane-cost: candidates=<n> local=<n> remote-only=<n> trunk=<n> integration=<n> other=<n>`
//! - `branch-pane-cost: focus runs=<n> p50_ms=… p95_ms=… spawns=<n>`
//! - `branch-pane-cost: breakdown <key>=<n> …` — one focus's spawns by kind
//! - `branch-pane-cost: window rows=40 tips=<n> total_ms=… spawns=<n>`
//! - `branch-pane-cost: scaling others=10 spawns=<a> others=500 spawns=<b>`
//!   (ease-pass 09p): one focus on a small built repository, with N stale
//!   ordinary branches (tips on `main`) and N siblings (off `main`, not under
//!   the focus). Candidates that cannot win must not cost a spawn: `a == b`.
//!
//! `spawns` counts lines appended to `$MWR_SPAWN_LOG` (the script's `git`
//! shim writes one per spawn, keyed by subcommand); `-` when it is unset.
//! The script runs the timings without the shim and the counts with it, so
//! the shim's own cost never lands in a time.

use std::collections::BTreeMap;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use mergewright_core::oid::Oid;
use mergewright_fixtures::team::{self, TeamSpec};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_ops::branch_status::{Candidate, CandidateKind, branch_status};
use mergewright_ops::operation::Tip;

const FOCUS_RUNS: usize = 10;
const WINDOW: usize = 40;
/// The two sizes the scaling line compares.
const SCALING: [usize; 2] = [10, 500];

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

/// The lines of `$MWR_SPAWN_LOG` so far, one per spawn; `None` when it is
/// unset.
fn spawn_log() -> Result<Option<Vec<String>>, String> {
    let Some(path) = std::env::var_os("MWR_SPAWN_LOG") else {
        return Ok(None);
    };
    match std::fs::read_to_string(&path) {
        Ok(text) => Ok(Some(text.lines().map(str::to_owned).collect())),
        // Not written yet: no spawn yet.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Ok(Some(Vec::new())),
        Err(why) => Err(format!("{}: {why}", path.to_string_lossy())),
    }
}

/// Lines of `$MWR_SPAWN_LOG` so far, or `None` when it is unset.
fn spawns() -> Result<Option<usize>, String> {
    Ok(spawn_log()?.map(|log| log.len()))
}

fn spawned(before: Option<usize>) -> Result<String, String> {
    Ok(match (before, spawns()?) {
        (Some(before), Some(after)) => (after - before).to_string(),
        _ => "-".to_owned(),
    })
}

/// The spawns since `before`, tallied by the shim's key (the subcommand,
/// and rev-list's first option): `for-each-ref=1 rev-list:--count=2`; `-`
/// when the log is unset.
fn breakdown(before: Option<usize>) -> Result<String, String> {
    let (Some(before), Some(log)) = (before, spawn_log()?) else {
        return Ok("-".to_owned());
    };
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    for key in log.iter().skip(before) {
        *tally.entry(key.as_str()).or_default() += 1;
    }
    Ok(tally
        .iter()
        .map(|(key, n)| format!("{key}={n}"))
        .collect::<Vec<_>>()
        .join(" "))
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

/// Every branch as a candidate, sorted by name: `refs/heads/main` the
/// trunk, every other an ordinary branch (the fixtures have no integration
/// branch and no `origin/main`).
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

fn count(all: &[Candidate], kind: CandidateKind) -> usize {
    all.iter().filter(|c| c.kind == kind).count()
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
        "branch-pane-cost: candidates={} local={local} remote-only={} trunk={} integration={} other={}",
        all.len(),
        all.len() - local,
        count(&all, CandidateKind::Trunk),
        count(&all, CandidateKind::Integration),
        count(&all, CandidateKind::Other),
    );

    // One focus: the newest local feature branch, read FOCUS_RUNS times.
    let focus = all
        .iter()
        .filter(|c| c.name.starts_with("refs/heads/feature/"))
        .max_by_key(|c| c.name.clone())
        .ok_or("no local feature branch")?;
    let mut times = Vec::with_capacity(FOCUS_RUNS);
    let mut one_spawn = String::new();
    let mut one_breakdown = String::new();
    for _ in 0..FOCUS_RUNS {
        let before = spawns()?;
        let started = Instant::now();
        branch_status(&repo, focus, &all, None, &Never).map_err(|why| why.to_string())?;
        times.push(started.elapsed());
        one_spawn = spawned(before)?;
        one_breakdown = breakdown(before)?;
    }
    times.sort();
    println!(
        "branch-pane-cost: focus runs={FOCUS_RUNS} p50_ms={} p95_ms={} spawns={one_spawn}",
        ms(percentile(&times, 50)),
        ms(percentile(&times, 95))
    );
    println!("branch-pane-cost: breakdown {one_breakdown}");

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
    let before = spawns()?;
    let started = Instant::now();
    for branch in &tips {
        branch_status(&repo, branch, &all, own_upstream(branch).as_ref(), &Never)
            .map_err(|why| why.to_string())?;
    }
    println!(
        "branch-pane-cost: window rows={WINDOW} tips={} total_ms={} spawns={}",
        tips.len(),
        ms(started.elapsed()),
        spawned(before)?
    );

    print_scaling()
}

fn print_scaling() -> Result<(), String> {
    let mut scaling = Vec::with_capacity(SCALING.len());
    for others in SCALING {
        scaling.push(format!(
            "others={others} spawns={}",
            scaling_spawns(others)?
        ));
    }
    println!("branch-pane-cost: scaling {}", scaling.join(" "));
    Ok(())
}

/// The spawns of one `branch_status` for `sara` in a repository built for
/// it: `main` (5 commits), `sara` (2 commits on `main`), `others` stale
/// ordinary branches with tips on `main` (under `sara`, and under the
/// trunk), and `others` siblings (one commit off `main` each, not under
/// `sara`). Neither kind can ever be `sara`'s base.
fn scaling_spawns(others: usize) -> Result<String, String> {
    let dir = tempfile::tempdir().map_err(|why| why.to_string())?;
    git(dir.path(), &["init", "-q", "-b", "main"])?;
    fast_import(dir.path(), &scaling_stream(others))?;
    let repo = RepoHandle::open(dir.path()).map_err(|why| why.to_string())?;
    let all = candidates(dir.path())?;
    let sara = all
        .iter()
        .find(|c| c.name == "refs/heads/sara")
        .ok_or("no sara in the scaling repository")?;
    let before = spawns()?;
    branch_status(&repo, sara, &all, None, &Never).map_err(|why| why.to_string())?;
    spawned(before)
}

/// A `git fast-import` stream for [`scaling_spawns`]. Empty-tree commits:
/// only the shape matters.
fn scaling_stream(others: usize) -> String {
    const MAIN: usize = 5;
    let mut out = String::new();
    let mut when = 1_700_000_000_u64;
    let mut commit = |out: &mut String, branch: &str, mark: usize, from: Option<usize>| {
        when += 60;
        let message = format!("{branch} {mark}");
        out.push_str(&format!(
            "commit refs/heads/{branch}\nmark :{mark}\ncommitter A <a@example.com> {when} +0000\ndata {}\n{message}\n",
            message.len()
        ));
        if let Some(from) = from {
            out.push_str(&format!("from :{from}\n"));
        }
        out.push('\n');
    };
    for mark in 1..=MAIN {
        commit(&mut out, "main", mark, (mark > 1).then(|| mark - 1));
    }
    commit(&mut out, "sara", MAIN + 1, Some(MAIN));
    commit(&mut out, "sara", MAIN + 2, Some(MAIN + 1));
    for n in 0..others {
        let mark = MAIN + 3 + n;
        commit(&mut out, &format!("sib/{n:05}"), mark, Some(1 + n % MAIN));
        out.push_str(&format!(
            "reset refs/heads/stale/{n:05}\nfrom :{}\n\n",
            1 + n % MAIN
        ));
    }
    out
}

fn fast_import(repo: &Path, stream: &str) -> Result<(), String> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["fast-import", "--quiet"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|why| format!("git fast-import: {why}"))?;
    child
        .stdin
        .take()
        .ok_or("fast-import has no stdin")?
        .write_all(stream.as_bytes())
        .map_err(|why| format!("git fast-import: {why}"))?;
    let output = child
        .wait_with_output()
        .map_err(|why| format!("git fast-import: {why}"))?;
    if !output.status.success() {
        return Err(format!(
            "git fast-import: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}
