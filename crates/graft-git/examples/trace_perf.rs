//! Times `blame`, `line_history` and `file_history` on the heavy synthetic
//! repository's busiest file, `trunk.txt` (rewritten by every trunk commit,
//! so its history is the deepest there is). Run by
//! `scripts/m7-perf-blame.sh` in release; prints one line per run and a
//! `trace-readers:` summary line (m7-power-tools/18).
//!
//! Needs `--features heavy-tests`: the synthetic repository is behind it.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant};

use graft_core::oid::Oid;
use graft_fixtures::Scale;
use graft_git::blame::{BlameOptions, Rev, blame};
use graft_git::repo::RepoHandle;
use graft_git::trace::{LineRange, TraceEnd, file_history, line_history};

const RUNS: usize = 10;
const FILE: &str = "trunk.txt";
/// The row cap the traces run with here: a screenful many times over.
const CAP: usize = 1000;

fn main() -> ExitCode {
    match measure() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("trace_perf: {why}");
            ExitCode::FAILURE
        }
    }
}

fn measure() -> Result<(), String> {
    let dir = graft_fixtures::synthetic::borrow(Scale::HEAVY).map_err(|why| why.to_string())?;
    let head = head(&dir)?;
    let repo = RepoHandle::open(&dir).map_err(|why| why.to_string())?;
    let path = Path::new(FILE);
    let options = BlameOptions::default();

    let lines = blame(&repo, &Rev::Commit(head), path, &options, &())
        .map_err(|why| why.to_string())?
        .lines
        .len();
    let last = u32::try_from(lines).map_err(|why| why.to_string())?;
    let range = LineRange::Block {
        start: 1,
        end: last,
    };
    println!(
        "# synthetic {} commits, {FILE}: {lines} lines, cap {CAP}",
        Scale::HEAVY.commits()
    );

    let (mut blames, mut lines_, mut files) = (Vec::new(), Vec::new(), Vec::new());
    for run in 1..=RUNS {
        let started = Instant::now();
        blame(&repo, &Rev::Commit(head), path, &options, &()).map_err(|why| why.to_string())?;
        let b = started.elapsed();

        let started = Instant::now();
        let mut rows = 0;
        let line_end = line_history(
            &repo,
            head,
            path,
            &range,
            CAP,
            &mut |batch| rows += batch.len(),
            &(),
        )
        .map_err(|why| why.to_string())?;
        let l = started.elapsed();

        let started = Instant::now();
        let mut file_rows = 0;
        let file_end = file_history(
            &repo,
            head,
            path,
            CAP,
            &mut |batch| file_rows += batch.len(),
            &(),
        )
        .map_err(|why| why.to_string())?;
        let f = started.elapsed();

        println!(
            "run {run}: blame={} line_history={} ({rows} rows, {}) file_history={} ({file_rows} rows, {})",
            ms(b),
            ms(l),
            end(&line_end),
            ms(f),
            end(&file_end)
        );
        blames.push(b);
        lines_.push(l);
        files.push(f);
    }
    println!(
        "trace-readers: blame p95={} line_history p95={} file_history p95={}",
        ms(p95(&mut blames)),
        ms(p95(&mut lines_)),
        ms(p95(&mut files))
    );
    Ok(())
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

fn end(end: &TraceEnd) -> String {
    match end {
        TraceEnd::Start { .. } => "start".to_owned(),
        TraceEnd::GitStops(why) => format!("git stops: {why}"),
        TraceEnd::Capped { shown } => format!("capped at {shown}"),
        TraceEnd::Stopped => "stopped".to_owned(),
    }
}
