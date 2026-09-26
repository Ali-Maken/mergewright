//! Times `blame` on a long file with interleaved history: 5 000 lines, 500
//! commits, each rewriting ~100 lines scattered through the file, and every
//! tenth line never touched after the first commit, so blame walks the whole
//! history for those (m7-power-tools/18b, review Major 3). The repository is
//! built with `git fast-import` in a temporary directory.
//!
//! Run by `scripts/m7-perf-blame-longfile.sh` in release; prints one line
//! per run and a `blame-longfile p95=<ms>` summary line.

use std::fmt::Write as _;
use std::io::Write as _;
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant};

use graft_core::oid::Oid;
use graft_git::blame::{BlameOptions, Rev, blame};
use graft_git::repo::RepoHandle;

const RUNS: usize = 10;
const FILE: &str = "long.txt";
const LINES: usize = 5_000;
const COMMITS: usize = 500;
/// One line in this many is rewritten by a given commit (roughly).
const SPREAD: usize = 50;

fn main() -> ExitCode {
    match measure() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("blame_longfile: {why}");
            ExitCode::FAILURE
        }
    }
}

fn measure() -> Result<(), String> {
    let scratch = tempfile::tempdir().map_err(|why| why.to_string())?;
    let dir = scratch.path();
    build(dir)?;
    let head = head(dir)?;
    let repo = RepoHandle::open(dir).map_err(|why| why.to_string())?;
    let path = Path::new(FILE);
    let options = BlameOptions::default();

    let first =
        blame(&repo, &Rev::Commit(head), path, &options, &()).map_err(|why| why.to_string())?;
    let commits = first.commits.len();
    println!(
        "# long file: {FILE}, {} lines, {COMMITS} commits each rewriting ~1 line in {SPREAD}; every 10th line untouched since the first; blame names {commits} commits",
        first.lines.len()
    );

    let mut times = Vec::with_capacity(RUNS);
    for run in 1..=RUNS {
        let started = Instant::now();
        blame(&repo, &Rev::Commit(head), path, &options, &()).map_err(|why| why.to_string())?;
        let took = started.elapsed();
        println!("run {run}: blame-longfile={}", ms(took));
        times.push(took);
    }
    println!("blame-longfile p95={}", ms(p95(&mut times)));
    Ok(())
}

/// Whether commit `commit` (1-based) rewrites line `line`. Every tenth line
/// is left alone; the rest are picked by a spread that differs per commit.
fn touches(commit: usize, line: usize) -> bool {
    !line.is_multiple_of(10) && (line * 7_919 + commit * 104_729).is_multiple_of(SPREAD)
}

/// The repository, written in one `git fast-import` stream.
fn build(dir: &Path) -> Result<(), String> {
    run(dir, &["init", "-q", "-b", "main"])?;
    let mut last = vec![0usize; LINES];
    let mut stream = String::new();
    for commit in 0..COMMITS {
        for (line, rev) in last.iter_mut().enumerate() {
            if commit > 0 && touches(commit, line) {
                *rev = commit;
            }
        }
        let mut content = String::with_capacity(LINES * 48);
        for (line, rev) in last.iter().enumerate() {
            writeln!(
                content,
                "line {line:05} last written by commit {rev:04} ..."
            )
            .map_err(|why| why.to_string())?;
        }
        let when = 1_600_000_000 + commit * 60;
        let message = format!("commit {commit}");
        write!(
            stream,
            "commit refs/heads/main\nmark :{}\ncommitter Perf <perf@example.invalid> {when} +0000\ndata {}\n{message}\n",
            commit + 1,
            message.len()
        )
        .map_err(|why| why.to_string())?;
        if commit > 0 {
            writeln!(stream, "from :{commit}").map_err(|why| why.to_string())?;
        }
        write!(
            stream,
            "M 100644 inline {FILE}\ndata {}\n{content}\n",
            content.len()
        )
        .map_err(|why| why.to_string())?;
    }
    let mut child = Command::new("git")
        .args(["fast-import", "--quiet"])
        .current_dir(dir)
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|why| format!("running git fast-import: {why}"))?;
    child
        .stdin
        .take()
        .ok_or("fast-import has no stdin")?
        .write_all(stream.as_bytes())
        .map_err(|why| why.to_string())?;
    let status = child.wait().map_err(|why| why.to_string())?;
    if !status.success() {
        return Err(format!("git fast-import: {status}"));
    }
    run(dir, &["reset", "-q", "--hard", "main"])
}

fn run(dir: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .args(args)
        .current_dir(dir)
        .status()
        .map_err(|why| format!("running git: {why}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {args:?}: {status}"))
    }
}

fn head(dir: &Path) -> Result<Oid, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(dir)
        .output()
        .map_err(|why| format!("running git: {why}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into_owned());
    }
    let hex = String::from_utf8_lossy(&output.stdout);
    Oid::from_hex(hex.trim()).map_err(|why| format!("{hex:?}: {why}"))
}

/// The nearest-rank 95th percentile: with 10 runs, the slowest.
fn p95(times: &mut [Duration]) -> Duration {
    times.sort();
    let rank = (times.len() * 95).div_ceil(100).max(1);
    times[rank - 1]
}

fn ms(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1000.0)
}
