//! Times three ways of finding lost commits among 500 reflog candidates on
//! the heavy synthetic repository with 50 000 refs (m7-power-tools/19). Run
//! by `scripts/m7-perf-lost.sh` in release; prints one line per run, the
//! p50/p95/max per method, and a `lost-commit-cost:` summary line naming
//! the fastest method that found exactly the fixture's lost commits.
//!
//! - **(a) rev-list** — `history_reads::unreached`, the shipped reader.
//! - **(b) per candidate** — `contained::contained_in` once per candidate.
//!   Expected to lose; if one run would take over [`B_FULL_LIMIT`], it is
//!   timed on the first [`B_SAMPLE`] candidates and scaled to 500, and the
//!   line says `extrapolated`. It does not read stashes: timing only.
//! - **(c) in process** — a `Walk` in commit-date order from every held tip
//!   (every peeled ref, `HEAD`), marking candidates, stopping once all are
//!   seen. Without a commit-graph every generation is infinite, so the
//!   generation cut-off the ticket names never fires here: a lost candidate
//!   keeps the walk going to the root. This is that method's cost on this
//!   fixture, not its best case.
//!
//! `lost_cost [commits refs]` runs a smaller synthetic repository, to check
//! the harness quickly. Needs `--features heavy-tests`.

use std::collections::HashSet;
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_fixtures::Scale;
use graft_fixtures::synthetic::{self, Reflogged};
use graft_git::contained::contained_in;
use graft_git::history::{CHUNK, CommitTable, Order, Walk, previous_tips};
use graft_git::history_reads::unreached;
use graft_git::repo::{HeadState, RepoHandle};

const RUNS: usize = 10;
const CANDIDATES: usize = 500;
const BUDGET_MS: f64 = 1000.0;
/// One (b) run projected past this is sampled instead of run whole.
const B_FULL_LIMIT: Duration = Duration::from_secs(30);
/// Candidates a sampled (b) run times.
const B_SAMPLE: usize = 25;

fn main() -> ExitCode {
    match measure() {
        Ok(()) => ExitCode::SUCCESS,
        Err(why) => {
            eprintln!("lost_cost: {why}");
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
        _ => Err("usage: lost_cost [commits refs]".to_owned()),
    }
}

fn measure() -> Result<(), String> {
    let scale = scale()?;
    let fixture = synthetic::with_reflog(scale).map_err(|why| why.to_string())?;
    let repo = RepoHandle::open(fixture.scratch.path()).map_err(|why| why.to_string())?;
    let candidates = candidates(&repo, &fixture)?;
    let expected: HashSet<Oid> = fixture
        .lost
        .iter()
        .map(|hex| Oid::from_hex(hex).map_err(|why| format!("{hex}: {why}")))
        .collect::<Result<_, _>>()?;
    let refs = repo.references().map_err(|why| why.to_string())?.len();
    println!(
        "# synthetic {} ({} commits), {refs} refs, {} reflog records ({} lost), {} candidates",
        scale.name(),
        scale.commits(),
        synthetic::RECORDS,
        synthetic::LOST,
        candidates.len()
    );

    let results = vec![
        time(
            "a-rev-list",
            RUNS,
            || method_a(&repo, &candidates),
            &expected,
        )?,
        time_b(&repo, &candidates, &expected)?,
        time(
            "c-in-process",
            RUNS,
            || method_c(&repo, &candidates),
            &expected,
        )?,
    ];
    report(&results, candidates.len(), refs)
}

/// One line per method, then the summary line.
fn report(results: &[Timed], candidates: usize, refs: usize) -> Result<(), String> {
    for result in results {
        println!(
            "method={} p50={} p95={} max={} runs={}{} found={}",
            result.name,
            ms(result.p50),
            ms(result.p95),
            ms(result.max),
            result.runs,
            if result.extrapolated {
                " extrapolated"
            } else {
                ""
            },
            match (result.extrapolated, result.correct) {
                // A sample cannot say whether all 500 were judged right.
                (true, _) => "not-judged",
                (false, true) => "exact",
                (false, false) => "WRONG",
            },
        );
    }
    let best = results
        .iter()
        .filter(|result| result.correct && !result.extrapolated)
        .min_by_key(|result| result.p95)
        .ok_or("no method found exactly the fixture's lost commits")?;
    let p95 = best.p95.as_secs_f64() * 1000.0;
    println!(
        "lost-commit-cost: method={} p95={p95:.1} cands={} refs={refs} · budget {BUDGET_MS:.0} ms · {}",
        best.name,
        candidates,
        if p95 <= BUDGET_MS { "held" } else { "missed" }
    );
    Ok(())
}

/// The newest distinct old values: `HEAD`'s reflog, then each branch's.
fn candidates(repo: &RepoHandle, fixture: &Reflogged) -> Result<Vec<Oid>, String> {
    let mut all = Vec::new();
    let mut seen = HashSet::new();
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

fn method_a(repo: &RepoHandle, candidates: &[Oid]) -> Result<HashSet<Oid>, String> {
    let found = unreached(repo, candidates, CANDIDATES, &()).map_err(|why| why.to_string())?;
    // Every fixture lost commit sits on a held parent: one run each.
    Ok(found.lost.iter().map(|run| run.tip).collect())
}

fn method_b(repo: &RepoHandle, candidates: &[Oid]) -> Result<HashSet<Oid>, String> {
    let mut lost = HashSet::new();
    for &candidate in candidates {
        let contained = contained_in(repo, candidate, &()).map_err(|why| why.to_string())?;
        if contained.branches.is_empty()
            && contained.remotes.is_empty()
            && contained.tags.is_empty()
            && contained.more == 0
        {
            lost.insert(candidate);
        }
    }
    Ok(lost)
}

fn method_c(repo: &RepoHandle, candidates: &[Oid]) -> Result<HashSet<Oid>, String> {
    let table = Arc::new(CommitTable::open(repo).map_err(|why| why.to_string())?);
    let mut tips: Vec<Oid> = repo
        .peeled_references(table.as_ref())
        .map_err(|why| why.to_string())?
        .into_iter()
        .map(|(_, id)| id)
        .collect();
    if let HeadState::Detached(head) = repo.head().map_err(|why| why.to_string())? {
        // An attached `HEAD` is its branch's tip, already among the refs.
        tips.push(head);
    }
    let mut unseen: HashSet<_> = candidates.iter().map(|id| table.intern(id)).collect();
    let mut walk = Walk::from_tips(repo, Order::CommitDate, &tips, Arc::clone(&table))
        .map_err(|why| why.to_string())?;
    while !unseen.is_empty() {
        let chunk = walk
            .next_links(CHUNK, &|| false)
            .map_err(|why| why.to_string())?;
        for link in &chunk.links {
            unseen.remove(&link.commit);
        }
        if chunk.done {
            break;
        }
    }
    Ok(unseen
        .into_iter()
        .map(|handle| table.id_of(handle))
        .collect())
}

struct Timed {
    name: &'static str,
    p50: Duration,
    p95: Duration,
    max: Duration,
    runs: usize,
    extrapolated: bool,
    correct: bool,
}

fn time(
    name: &'static str,
    runs: usize,
    mut method: impl FnMut() -> Result<HashSet<Oid>, String>,
    expected: &HashSet<Oid>,
) -> Result<Timed, String> {
    let mut times = Vec::new();
    let mut correct = true;
    for run in 1..=runs {
        let started = Instant::now();
        let found = method()?;
        let took = started.elapsed();
        let exact = &found == expected;
        correct &= exact;
        println!(
            "{name} run {run}: {} ms, {} lost{}",
            ms(took),
            found.len(),
            if exact { "" } else { " (not the fixture's)" }
        );
        times.push(took);
    }
    Ok(summarise(name, times, false, correct))
}

fn time_b(repo: &RepoHandle, candidates: &[Oid], expected: &HashSet<Oid>) -> Result<Timed, String> {
    let sample = &candidates[..B_SAMPLE.min(candidates.len())];
    let started = Instant::now();
    method_b(repo, sample)?;
    let per = started.elapsed() / u32::try_from(sample.len().max(1)).unwrap_or(1);
    let projected = per * u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    if projected <= B_FULL_LIMIT {
        return time(
            "b-per-candidate",
            RUNS,
            || method_b(repo, candidates),
            expected,
        );
    }
    println!(
        "# b per-candidate: projected {} ms a run, over {} s; timing {B_SAMPLE} candidates and scaling",
        ms(projected),
        B_FULL_LIMIT.as_secs()
    );
    let scale = u32::try_from(candidates.len()).unwrap_or(u32::MAX);
    let sample_len = u32::try_from(sample.len().max(1)).unwrap_or(1);
    let mut times = Vec::new();
    for run in 1..=RUNS {
        let started = Instant::now();
        method_b(repo, sample)?;
        let took = started.elapsed() / sample_len * scale;
        println!("b per-candidate run {run}: {} ms (scaled)", ms(took));
        times.push(took);
    }
    // Correctness cannot be judged from a sample: the method is timed only.
    Ok(summarise("b-per-candidate", times, true, false))
}

fn summarise(
    name: &'static str,
    mut times: Vec<Duration>,
    extrapolated: bool,
    correct: bool,
) -> Timed {
    times.sort();
    let rank = |percent: usize| {
        let at = (times.len() * percent).div_ceil(100).max(1);
        times[at - 1]
    };
    Timed {
        name,
        p50: rank(50),
        p95: rank(95),
        max: times[times.len() - 1],
        runs: times.len(),
        extrapolated,
        correct,
    }
}

fn ms(time: Duration) -> String {
    format!("{:.1}", time.as_secs_f64() * 1000.0)
}
