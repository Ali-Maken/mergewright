//! G-settle over a walked repository that folds (ease-pass 06 box 8, built
//! by 06e). The parent module's held-`j` test runs over a hand-laid
//! history whose window carries no fold facts; this one opens a real
//! repository through the job pool, so the window it scrolls carries the
//! fold's own facts — every row a merge whose PR branch is folded behind
//! it — and the loop's settle reads them on every key.

use std::path::Path;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use super::*;
use crate::jobs::{JobRunner, Shutdown};
use crate::repos::RepoStore;

/// Merges on `main`'s first-parent chain, each bringing in one commit from
/// a PR branch that no longer exists. More than one screen, so held `j`
/// scrolls folding merges onto it whose counts nobody has asked yet.
const MERGES: usize = 60;
/// Keys held: enough to scroll past the first screen.
const HELD: u32 = 50;
/// A hang guard for opening, not a budget.
const PATIENCE: Duration = Duration::from_secs(60);

/// `main` = a root commit, then [`MERGES`] PR merges in a row, each
/// `Merged PR <n>: change <n>` over one side commit; built with
/// `commit-tree`, so no branch of the PRs survives. Dated a minute apart.
fn folded_prs(repo: &Path) {
    graft_fixtures::git::init_repo(repo).expect("git init");
    let date = |minute: usize| format!("@{} +0000", 1_700_000_000 + minute * 60);
    let run = |args: &[&str], minute: usize| -> String {
        let out = graft_fixtures::git::run_at(repo, args, &date(minute)).expect("git");
        String::from_utf8(out).expect("utf-8").trim().to_owned()
    };
    run(&["commit", "--quiet", "--allow-empty", "-m", "root"], 0);
    let tree = run(&["rev-parse", "HEAD^{tree}"], 0);
    let mut tip = run(&["rev-parse", "HEAD"], 0);
    for n in 1..=MERGES {
        let side = run(
            &[
                "commit-tree",
                &tree,
                "-p",
                &tip,
                "-m",
                &format!("pr {n}: the change"),
            ],
            2 * n - 1,
        );
        tip = run(
            &[
                "commit-tree",
                &tree,
                "-p",
                &tip,
                "-p",
                &side,
                "-m",
                &format!("Merged PR {n}: change {n}"),
            ],
            2 * n,
        );
    }
    run(&["update-ref", "refs/heads/main", &tip], 2 * MERGES);
}

/// `repo` opened as the binary opens it — `update::opening` over the real
/// job pool — and pumped until the walk is laid out to the end, the refs
/// are read and the pool has gone quiet (the first screen's counts
/// answered).
fn opened(repo: &Path) -> AppState {
    let (sender, events) = mpsc::channel();
    let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
    let jobs = JobRunner::start(sender, 2, store).expect("the job pool starts");
    let (mut state, commands) = crate::update::opening(
        repo.to_path_buf(),
        Size {
            columns: COLUMNS,
            rows: ROWS,
        },
    );
    for command in commands {
        jobs.dispatch(command);
    }
    let deadline = Instant::now() + PATIENCE;
    let ready = |state: &AppState| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| window.done && !window.rows.is_empty())
    };
    loop {
        assert!(Instant::now() < deadline, "opening never settled");
        let event = match events.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => event,
            // Quiet for half a second once ready: the opening is done.
            Err(mpsc::RecvTimeoutError::Timeout) if ready(&state) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => panic!("the pool hung up"),
        };
        let (next, commands) = update(&state, event);
        for command in commands {
            jobs.dispatch(command);
        }
        state = next;
    }
    let stopped = jobs.shutdown(Duration::from_secs(10), crate::jobs::EXIT_WRITE_WAIT, &());
    assert_eq!(stopped, Shutdown::CLEAN, "the pool stops cleanly");
    state.harness.clock = crate::session::Clock::Fixed(graft_fixtures::PINNED_NOW);
    state.detail.shown = true;
    state
}

fn counts_asked(commands: &[Command]) -> usize {
    commands
        .iter()
        .filter(|command| matches!(command, Command::CountFolded { .. }))
        .count()
}

/// Not vacuous: the window is the fold's — the merges and the root, no PR
/// commit, a fold fact per row and every merge folding.
fn every_merge_folds(state: &AppState) {
    let window = state
        .history
        .as_ref()
        .and_then(|history| history.window.as_ref())
        .expect("a window");
    assert_eq!(
        window.rows.len(),
        MERGES + 1,
        "the merges and the root, no PR commit"
    );
    assert_eq!(window.fold.len(), window.rows.len(), "a fold fact per row");
    assert_eq!(
        window
            .fold
            .iter()
            .filter(|fact| fact.folds == Some(true))
            .count(),
        MERGES,
        "every merge folds its PR"
    );
}

/// Held `j` over the walked, folded window: the follow starts for the
/// first row and the row it rests on and nothing between, and no folded
/// merge's count is read mid-burst — the first one the scroll brought on
/// screen is asked once, at rest.
#[test]
fn a_held_j_over_a_folded_walk_follows_and_counts_only_at_rest() {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    folded_prs(scratch.path());
    let state = opened(scratch.path());
    every_merge_folds(&state);
    assert_eq!(cursor(&state), 0, "on HEAD");

    let jobs = Recorded::default();
    let mut wakes = Timed::new(&jobs, held_j(HELD, Duration::from_millis(16)));
    let (state, _) = run(state, &mut wakes);
    let commands = jobs.commands();
    let handed = wakes.handed.clone();
    let held = usize::try_from(HELD).expect("small");
    assert_eq!(handed.len(), held);
    assert_eq!(cursor(&state), held);
    let top = state.history.as_ref().map_or(0, |history| history.top);
    assert!(top > 0, "the held key scrolled the screen");

    // Key 1: one file list, for row 1.
    let first = &commands[handed[0]..handed[1]];
    assert_eq!(
        first
            .iter()
            .filter(|c| matches!(c, Command::ListFiles(..)))
            .count(),
        1,
        "{first:?}"
    );
    // Keys 3 to the last: nothing for the pane, the probe or a count.
    let burst = &commands[handed[2]..handed[held - 1]];
    assert!(
        !burst.iter().any(starts_follow),
        "a follow mid-burst: {burst:?}"
    );
    assert_eq!(counts_asked(burst), 0, "a count read mid-burst: {burst:?}");
    // At rest: one set for the row it rests on, and the first uncounted
    // folding merge on screen counted — one ask per rest.
    let rest = &commands[handed[held - 1]..];
    assert_eq!(
        rest.iter()
            .filter(|c| matches!(c, Command::ListFiles(..)))
            .count(),
        1,
        "{rest:?}"
    );
    let rested_on = state
        .history
        .as_ref()
        .and_then(|history| history.row_at(held))
        .map(|row| row.commit.id)
        .expect("the row it rests on");
    assert_eq!(state.detail.spec, Some(DetailSpec::Commit(rested_on)));
    assert_eq!(counts_asked(rest), 1, "{rest:?}");
    assert!(!follow_behind(&state));
}
