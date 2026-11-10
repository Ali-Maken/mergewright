//! The put-in check's measurements (m7-power-tools/09b): `a` on `HEAD`
//! through the dependents check, timed. The heavy tier runs them (review
//! m7-09..21c Minor 12); `scripts/m7-09-perf.sh` runs them in release and
//! writes `docs/perf/09b.txt`. Lines start `m7-09b`.

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use super::{Fixture, open_on_head, perf_repo, read};
use crate::app_harness::App;

/// The dependents check `a` on `HEAD` waits on when another local branch
/// exists (ticket 09 "Decided", 09b A1: under one frame on the fixture).
/// Run by `scripts/m7-09-perf.sh` in release; lines start `m7-09b`.
/// A measurement: it prints, and gates nothing yet.
///
/// Phase 1: `a` to the amend editor open, as Mergewright runs it, 3 rounds × 7,
/// with the check (HEAD on `feature-a`, `main`, and `spike` — a branch that
/// is not the default and not built on his) and without it (`feature-a`
/// alone: today's amend at once). Phase 2: each call the plan job makes on
/// the `HEAD` path, timed alone in the order it makes them (the job's own
/// `repos.shared()` and `repos.branches()` are cached reads of the store;
/// `Branches::resolve` is timed cold, the upper bound). Phase 3 is
/// [`perf_dependents_spawns`]. The 300-branch line is not gated.
#[test]
fn perf_dependents_check() {
    const ROUNDS: usize = 3;
    let mut with_check = Vec::new();
    let mut without = Vec::new();
    for round in 1..=ROUNDS {
        let with = a_to_editor(Fixture::Spike);
        let base = a_to_editor(Fixture::OnlyBranch);
        report("phase 1: a to the editor, with the check", round, &with);
        report("phase 1: a to the editor, today's amend", round, &base);
        with_check.push(median(&with));
        without.push(median(&base));
    }
    for round in 1..=ROUNDS {
        for (call, took) in calls_alone() {
            report(&format!("phase 2: {call}"), round, &took);
        }
    }
    let wide = a_to_editor(Fixture::Wide);
    report("300 unrelated branches, a to the editor", 1, &wide);

    let ms = |took: std::time::Duration| took.as_secs_f64() * 1e3;
    let rounds: Vec<String> = with_check
        .iter()
        .map(|took| format!("{:.1}", ms(*took)))
        .collect();
    let (with, base) = (median(&with_check), median(&without));
    println!(
        "m7-09b summary: a to the amend editor, median of 3\u{d7}7: {:.1} ms through the dependents check (rounds {}), {:.1} ms today's amend; the check adds {:.1} ms (budget 16.7 ms)",
        ms(with),
        rounds.join("/"),
        ms(base),
        ms(with.saturating_sub(base))
    );
}

/// Seven `a`s on `HEAD`, each to the amend editor open and `Esc`, sorted.
fn a_to_editor(fixture: Fixture) -> Vec<std::time::Duration> {
    let dir = perf_repo(fixture);
    let (app, mut state) = open_on_head(dir.path());
    let mut took = Vec::new();
    for _ in 0..7 {
        let started = std::time::Instant::now();
        state = a_once(&app, &state);
        took.push(started.elapsed());
    }
    took.sort();
    drop(state);
    app.close();
    took
}

/// One `a` to the editor, then `Esc`.
fn a_once(app: &App, state: &AppState) -> AppState {
    let asked = app.key(state, KeyCode::Char('a'));
    let opened = app.pump(asked, "the amend editor", |state| {
        state.commit_editor.is_some()
    });
    let closed = app.key(&opened, KeyCode::Escape);
    assert!(closed.commit_editor.is_none(), "Esc closes the editor");
    closed
}

/// Phase 2: the `HEAD` path's calls, seven times each, in the job's order
/// (`jobs/rewrite.rs::plan`), then `HEAD`'s message (`commit_message.rs`).
fn calls_alone() -> Vec<(&'static str, Vec<std::time::Duration>)> {
    use mergewright_git::branches::Branches;
    use mergewright_git::repo::RepoHandle;
    use mergewright_git::version::system_git;
    let dir = perf_repo(Fixture::Spike);
    let handle = RepoHandle::open(dir.path()).unwrap_or_else(|why| panic!("open: {why}"));
    let head = mergewright_ops::operation::Tip {
        name: "refs/heads/feature-a".to_owned(),
        oid: mergewright_core::oid::Oid::from_hex(&read(dir.path(), "feature-a"))
            .unwrap_or_else(|why| panic!("oid: {why}")),
    };
    let time = |run: &dyn Fn()| {
        let mut took: Vec<std::time::Duration> = (0..7)
            .map(|_| {
                let started = std::time::Instant::now();
                run();
                started.elapsed()
            })
            .collect();
        took.sort();
        took
    };
    let git = system_git()
        .unwrap_or_else(|why| panic!("git: {why}"))
        .version();
    let default = Branches::resolve(&handle, None)
        .unwrap_or_else(|why| panic!("branches: {why}"))
        .default_branch()
        .cloned();
    vec![
        (
            "system_git (git --version)",
            time(&|| {
                system_git().unwrap_or_else(|why| panic!("git: {why}"));
            }),
        ),
        (
            "Branches::resolve, cold",
            time(&|| {
                Branches::resolve(&handle, None).unwrap_or_else(|why| panic!("branches: {why}"));
            }),
        ),
        (
            "rewrite::dependents",
            time(&|| {
                mergewright_ops::rewrite::dependents(&handle, &git, &head, default.as_ref(), &())
                    .unwrap_or_else(|why| panic!("dependents: {why:?}"));
            }),
        ),
        (
            "HEAD's message (in process)",
            time(&|| {
                mergewright_git::history::read_commit(&handle, head.oid)
                    .unwrap_or_else(|why| panic!("read: {why}"));
            }),
        ),
    ]
}

/// Phase 3, its own run so the trace's cost stays out of phases 1 and 2:
/// seven `a`s with git tracing into the directory `GIT_TRACE2_PERF` names
/// (the script sets it; Mergewright's spawns keep the environment); every trace
/// file that appeared during them is one git process: argv and time.
///
/// Without `GIT_TRACE2_PERF` (the heavy tier), it runs itself again as a
/// child of this test binary with a directory of its own: the variable must
/// be in the environment Mergewright's git processes inherit from the start.
#[test]
fn perf_dependents_spawns() {
    if let Some(dir) = std::env::var_os("GIT_TRACE2_PERF") {
        let dir = std::path::PathBuf::from(dir);
        assert!(dir.is_dir(), "GIT_TRACE2_PERF names no directory");
        spawns(&dir);
        return;
    }
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "m7_walk::amend_into::perf::perf_dependents_spawns",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("GIT_TRACE2_PERF", dir.path())
        .status()
        .expect("the child runs");
    assert!(status.success(), "the traced run failed: {status:?}");
}

fn spawns(dir: &Path) {
    let dir_files = || -> std::collections::BTreeSet<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .unwrap_or_else(|why| panic!("read {}: {why}", dir.display()))
            .map(|entry| entry.unwrap_or_else(|why| panic!("entry: {why}")).path())
            .collect()
    };
    let repo = perf_repo(Fixture::Spike);
    let (app, mut state) = open_on_head(repo.path());
    // Let the opening reads finish tracing before the window starts.
    state = a_once(&app, &state);
    let before = dir_files();
    for _ in 0..7 {
        state = a_once(&app, &state);
    }
    let after = dir_files();
    drop(state);
    app.close();
    let mut by_argv: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for file in after.difference(&before) {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|why| panic!("read {}: {why}", file.display()));
        let mut argv = None;
        let mut took = None;
        for line in text.lines() {
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            match fields.get(3).copied() {
                Some("start") => argv = fields.get(8).map(|argv| (*argv).to_owned()),
                Some("atexit") => took = fields.get(5).and_then(|at| at.parse::<f64>().ok()),
                _ => {}
            }
        }
        let (Some(argv), Some(took)) = (argv, took) else {
            println!("m7-09b phase 3: unreadable trace {}", file.display());
            continue;
        };
        // The fixture's scratch path names this machine and changes every
        // run: said as `<repo>` (the repo may be published).
        let argv = [repo.path().to_path_buf(), canonical(repo.path())]
            .iter()
            .fold(argv, |argv, path| {
                argv.replace(&path.display().to_string(), "<repo>")
            });
        by_argv.entry(argv).or_default().push(took * 1e3);
    }
    println!(
        "m7-09b phase 3: {} git processes over 7 a's",
        after.difference(&before).count()
    );
    for (argv, mut took) in by_argv {
        took.sort_by(f64::total_cmp);
        println!(
            "m7-09b phase 3: {}× {:.1} ms median (git's own clock)  {argv}",
            took.len(),
            took[took.len() / 2]
        );
    }
}

use std::collections::BTreeMap;

/// `path` with symlinks resolved (`/var` is `/private/var` on macOS).
fn canonical(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

fn median(took: &[std::time::Duration]) -> std::time::Duration {
    let mut sorted = took.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

/// `m7-09b <what>, round <n>: median <x> ms, spread <min>–<max> ms`.
fn report(what: &str, round: usize, took: &[std::time::Duration]) {
    let ms = |took: std::time::Duration| took.as_secs_f64() * 1e3;
    let (low, high) = (
        took.iter().min().copied().unwrap_or_default(),
        took.iter().max().copied().unwrap_or_default(),
    );
    println!(
        "m7-09b {what}, round {round}: median {:.1} ms, spread {:.1}\u{2013}{:.1} ms",
        ms(median(took)),
        ms(low),
        ms(high)
    );
}
