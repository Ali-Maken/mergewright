//! What the before-snapshot costs since it keeps raw bytes
//! (m7-power-tools/30q-c, ADR-0052, ticket 30q R2's cost).
//! Run by `scripts/m7-30q-perf.sh` in release; never in a helper's context.
//!
//! mergewright-fixtures' `Scale` grows history, not the tree (its final tree is
//! three files), so no fixture scale reaches 5 000 files. The example builds
//! its own: one commit of `files` small LF text files, in a temp directory.
//! Three cases, each in its own copy of that repository:
//!
//! 1. no conversion, 5 changed files;
//! 2. no conversion, clean worktree;
//! 3. `core.autocrlf=true`, 50 changed LF files (every one kept raw).
//!
//! Per case, `runs` timed calls of `snapshot::take` and, apart, of
//! `snapshot::raw_record` over the same worktree tree. One line per case:
//! `case<k> files=… changed=… raw=<kept|none> take_p50_us=… take_p99_us=…
//! raw_p50_us=… raw_p99_us=… raw_spawns=<n>`, then `snapshot_cost: done`.
//! `raw_spawns` is how many `git` the last `raw_record` ran (its `commands`,
//! m7-power-tools/30r): a count, the same on every machine.
//!
//! `snapshot_cost [runs files]` runs smaller (the builder's smoke run).

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use mergewright_core::journal::EntryId;
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_ops::snapshot;

const RUNS: usize = 20;
const FILES: usize = 5_000;

/// Never asks a git call to stop.
struct Never;

impl Stop for Never {
    fn should_stop(&self) -> bool {
        false
    }
}

/// One case: how many files change, and whether git converts them.
struct Case {
    changed: usize,
    autocrlf: bool,
}

const CASES: [Case; 3] = [
    Case {
        changed: 5,
        autocrlf: false,
    },
    Case {
        changed: 0,
        autocrlf: false,
    },
    Case {
        changed: 50,
        autocrlf: true,
    },
];

fn main() -> ExitCode {
    match measure() {
        Ok(()) => {
            println!("snapshot_cost: done");
            ExitCode::SUCCESS
        }
        Err(why) => {
            eprintln!("snapshot_cost: {why}");
            ExitCode::FAILURE
        }
    }
}

fn sizes() -> Result<(usize, usize), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [] => Ok((RUNS, FILES)),
        [runs, files] => {
            let runs = runs.parse().map_err(|why| format!("{runs}: {why}"))?;
            let files = files.parse().map_err(|why| format!("{files}: {why}"))?;
            Ok((runs, files))
        }
        _ => Err("usage: snapshot_cost [runs files]".to_owned()),
    }
}

/// A temp directory removed on drop. git makes loose objects read-only,
/// which `remove_dir_all` handles on unix (it needs the directory's write
/// bit, not the file's).
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        // Dropping is cleanup after the numbers are printed; a failure here
        // leaves a temp directory behind and changes no measurement.
        if let Err(why) = std::fs::remove_dir_all(&self.0) {
            eprintln!("snapshot_cost: {} not removed: {why}", self.0.display());
        }
    }
}

fn git(dir: &Path, args: &[&str]) -> Result<String, String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_SYSTEM", "/dev/null")
        .env("GIT_AUTHOR_NAME", snapshot::BACKUP_AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", snapshot::BACKUP_AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", snapshot::BACKUP_AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", snapshot::BACKUP_AUTHOR_EMAIL)
        .output()
        .map_err(|why| format!("git {args:?}: {why}"))?;
    if !out.status.success() {
        return Err(format!(
            "git {args:?}: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// The case's repository: `files` committed LF files, then its config and
/// its changes on disk.
fn build(root: &Path, k: u8, files: usize, case: &Case) -> Result<PathBuf, String> {
    let dir = root.join(format!("case{k}"));
    let src = dir.join("src");
    std::fs::create_dir_all(&src).map_err(|why| format!("{}: {why}", src.display()))?;
    git(&dir, &["init", "-q", "-b", "main", "."])?;
    for i in 0..files {
        let path = src.join(format!("f{i}.txt"));
        std::fs::write(&path, format!("line one of file {i}\nline two\n"))
            .map_err(|why| format!("{}: {why}", path.display()))?;
    }
    git(&dir, &["add", "-A"])?;
    git(&dir, &["commit", "-qm", "the tracked tree"])?;
    if case.autocrlf {
        git(&dir, &["config", "core.autocrlf", "true"])?;
    }
    for i in 0..case.changed.min(files) {
        let path = src.join(format!("f{i}.txt"));
        std::fs::write(&path, format!("line one of file {i}\nchanged\nline two\n"))
            .map_err(|why| format!("{}: {why}", path.display()))?;
    }
    Ok(dir)
}

fn percentile(sorted: &[Duration], p: usize) -> u128 {
    if sorted.is_empty() {
        return 0;
    }
    let at = (sorted.len() * p).div_ceil(100).saturating_sub(1);
    sorted[at.min(sorted.len() - 1)].as_micros()
}

fn measure() -> Result<(), String> {
    let (runs, files) = sizes()?;
    let root = std::env::temp_dir().join(format!("graft-30q-{}", std::process::id()));
    std::fs::create_dir_all(&root).map_err(|why| format!("{}: {why}", root.display()))?;
    let _cleanup = TempDir(root.clone());
    for (k, case) in (1u8..).zip(CASES.iter()) {
        let dir = build(&root, k, files, case)?;
        let repo = RepoHandle::open(&dir).map_err(|why| format!("open case{k}: {why}"))?;
        let scratch = dir.join(".git/graft");
        std::fs::create_dir_all(&scratch).map_err(|why| format!("{}: {why}", scratch.display()))?;
        let head = Oid::from_hex(&git(&dir, &["rev-parse", "HEAD"])?)
            .map_err(|why| format!("HEAD of case{k}: {why}"))?;
        let timed = time_runs(&repo, &scratch, head, k, runs)?;
        let label = match timed.kept {
            Some(true) => "kept",
            Some(false) | None => "none",
        };
        println!(
            "case{k} files={files} changed={} autocrlf={} raw={label} runs={runs} take_p50_us={} take_p99_us={} raw_p50_us={} raw_p99_us={} raw_spawns={}",
            case.changed.min(files),
            case.autocrlf,
            percentile(&timed.take, 50),
            percentile(&timed.take, 99),
            percentile(&timed.raw, 50),
            percentile(&timed.raw, 99),
            timed.spawns,
        );
    }
    Ok(())
}

/// One case's runs: `take` and `raw_record` times (sorted), whether the last
/// run kept `raw/`, and how many `git` spawns its `raw_record` made.
struct Timed {
    take: Vec<Duration>,
    raw: Vec<Duration>,
    kept: Option<bool>,
    spawns: usize,
}

fn time_runs(
    repo: &RepoHandle,
    scratch: &Path,
    head: Oid,
    k: u8,
    runs: usize,
) -> Result<Timed, String> {
    let mut timed = Timed {
        take: Vec::with_capacity(runs),
        raw: Vec::with_capacity(runs),
        kept: None,
        spawns: 0,
    };
    for run in (0i64..).take(runs) {
        let id = EntryId::new(1_700_000_000_000 + run, [k; 10]);
        let started = Instant::now();
        let taken = snapshot::take(repo, scratch, &id, "main", "perf", &[], Some(head), &Never)
            .map_err(|why| format!("take case{k}: {why}"))?;
        timed.take.push(started.elapsed());
        let mut commands = Vec::new();
        let started = Instant::now();
        let raw = snapshot::raw_record(
            repo,
            scratch,
            &Never,
            Some(head),
            taken.worktree_tree,
            &mut commands,
        )
        .map_err(|why| format!("raw_record case{k}: {why}"))?;
        timed.raw.push(started.elapsed());
        timed.kept = Some(raw.is_some());
        timed.spawns = commands.len();
    }
    timed.take.sort();
    timed.raw.sort();
    Ok(timed)
}
