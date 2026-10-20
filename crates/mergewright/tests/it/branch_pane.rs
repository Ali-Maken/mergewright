//! The branch pane through the real app (ease-pass 09): repositories shaped
//! like his team's, opened over the real job pool, the cursor moved with
//! `j`/`k`, the pane read at rest and drawn right of the rows.
#![allow(
    clippy::expect_used,
    reason = "a fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright::update::branch_pane::{BranchNames, candidates, identity_of};
use mergewright_core::event::KeyCode;
use mergewright_core::ref_name::RefName;

use crate::app_harness::App;
use crate::captures::frame_at;

mod merged_label;
mod picker_on;
mod reads;
mod shots;
mod states;

const ROWS: u16 = 30;
/// After every fixture commit, so ages read the same.
const NOW: i64 = 1_700_100_000;
/// The first fixture commit's time; each step is a minute later.
const START: i64 = 1_700_000_000;

const README: &str = "Add the readme";
const START_RELEASE: &str = "Start the wallet release";
const TOC: &str = "toc: fix the anchors";
const ALI: &str = "x: try the other layout";
const MORE_RELEASE: &str = "Bump the wallet version";
const START_DEVELOP: &str = "Start develop";
const TOC_TWO: &str = "toc: number the headings";
const MORE_DEVELOP: &str = "develop: merge the payments work";

/// Runs `steps` as git commands in a fresh repository, a minute apart.
fn repo_of(steps: &[&[&str]]) -> tempfile::TempDir {
    let scratch = tempfile::tempdir().expect("a temporary directory");
    let repo = scratch.path();
    mergewright_fixtures::git::init_repo(repo).expect("git init");
    let mut at = START;
    for args in steps {
        at += 60;
        mergewright_fixtures::git::run_at(repo, args, &format!("@{at} +0000"))
            .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    }
    scratch
}

fn commit(subject: &str) -> Vec<&str> {
    vec!["commit", "--quiet", "--allow-empty", "-m", subject]
}

/// `main ← wallet-release (HEAD) ← sara/fix-toc`: `wallet-release` has no
/// commit after `sara/fix-toc` was cut from it, so it is sara's base (01r R1:
/// an ordinary branch all of which sits under hers). Sara's commit is hers.
fn sara_on_wallet() -> tempfile::TempDir {
    repo_of(&[
        &commit(README),
        &["switch", "--quiet", "-c", "wallet-release"],
        &commit(START_RELEASE),
        &["switch", "--quiet", "-c", "sara/fix-toc"],
        &[
            "-c",
            "user.name=Sara Khan",
            "-c",
            "user.email=sara@example.invalid",
            "commit",
            "--quiet",
            "--allow-empty",
            "-m",
            TOC,
        ],
        &["switch", "--quiet", "wallet-release"],
    ])
}

/// [`sara_on_wallet`], then `ali/x` cut from `wallet-release` where sara
/// cut hers, with a commit, then a commit on `wallet-release`.
fn with_sibling() -> tempfile::TempDir {
    repo_of(&[
        &commit(README),
        &["switch", "--quiet", "-c", "wallet-release"],
        &commit(START_RELEASE),
        &["switch", "--quiet", "-c", "sara/fix-toc"],
        &commit(TOC),
        &["switch", "--quiet", "-c", "ali/x", "wallet-release"],
        &commit(ALI),
        &["switch", "--quiet", "wallet-release"],
        &commit(MORE_RELEASE),
    ])
}

/// `main ← develop ← wallet-release ← sara/fix-toc` (2 commits), then a
/// commit on `develop` (HEAD) dated after sara's newest: develop is a moved
/// integration base, on sara's route and drawn as its own lane beside her
/// rows (09c check 2; an `Other` base cannot be both, 01r R1 and Q15 a).
fn with_moved_integration() -> tempfile::TempDir {
    repo_of(&[
        &commit(README),
        &["switch", "--quiet", "-c", "develop"],
        &commit(START_DEVELOP),
        &["switch", "--quiet", "-c", "wallet-release"],
        &commit(START_RELEASE),
        &["switch", "--quiet", "-c", "sara/fix-toc"],
        &commit(TOC),
        &commit(TOC_TWO),
        &["switch", "--quiet", "develop"],
        &commit(MORE_DEVELOP),
    ])
}

/// Pumped to rest with the pane's read altered: `None` withholds every
/// `BranchStatusRead` (the pane stays `reading…`); `Some(reason)` answers
/// each with `Err(reason)`. Everything else at rest as `captures::at_rest`.
/// `App::pump`'s loop with the answer intercepted before `update`; the same
/// per-event patience and whole-pump deadline, no sleep.
fn rest_with(app: &App, state: AppState, fail: Option<&str>) -> AppState {
    use crate::app_harness::{DEADLINE, Deadline, PATIENCE};
    use mergewright::jobs::Dispatch;
    use mergewright::update::{AppEvent, Product, update};
    let what = "rest with the branch pane's read altered";
    let mut deadline = Deadline::of(DEADLINE);
    let mut state = state;
    let mut withheld = false;
    loop {
        let rest = crate::captures::walk_has_settled(&state)
            && crate::captures::radar_has_answered(&state)
            && mergewright::update::fold_count::settled(&state);
        let pane = match fail {
            None => withheld,
            Some(_) => mergewright::update::branch_pane::settled(&state),
        };
        if rest && pane {
            return state;
        }
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for {what}: {why}"));
        deadline
            .note(what, &event)
            .unwrap_or_else(|why| panic!("{why}"));
        let event = match (event, fail) {
            // Withheld: `done` is asked again before the next wait.
            (AppEvent::Produced(_, Product::BranchStatusRead { .. }), None) => {
                withheld = true;
                continue;
            }
            (AppEvent::Produced(id, Product::BranchStatusRead { key, .. }), Some(reason)) => {
                AppEvent::Produced(
                    id,
                    Product::BranchStatusRead {
                        key,
                        result: Err(reason.to_owned()),
                    },
                )
            }
            (event, _) => event,
        };
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            app.jobs.dispatch(command);
        }
    }
}

fn drawn(state: &AppState, columns: u16) -> Vec<String> {
    frame_at(state, columns, ROWS, Some(Clock::Fixed(NOW)))
}

/// Opened at `columns`, up to its first rows.
fn opened(repo: &Path, columns: u16) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, columns, ROWS);
    let state = app.pump(state, "the first rows", |state| {
        state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
            && state.refs.is_some()
    });
    (app, state)
}

fn selected_subject(state: &AppState) -> String {
    state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| String::from_utf8_lossy(&row.commit.message).into_owned())
        .unwrap_or_default()
}

/// `j` until the cursor is on the row whose subject starts with `subject`.
fn cursor_on(app: &App, state: AppState, subject: &str) -> AppState {
    let mut state = state;
    for _ in 0..40 {
        if selected_subject(&state).starts_with(subject) {
            return state;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("the cursor never reached {subject:?}");
}

/// Pumped until the pane's read has landed: no line says `reading…`.
fn read(app: &App, state: AppState, columns: u16) -> AppState {
    app.pump(state, "the branch pane's read", |state| {
        let frame = drawn(state, columns);
        frame.iter().any(|line| line.contains(" built on"))
            && frame.iter().all(|line| !line.contains("reading…"))
    })
}

/// The pane's text: every line's part right of the pane's `│` rule. The
/// rule's column is the frame's width less the slot, not the line's: a
/// line is trimmed at its last drawn cell.
fn pane(frame: &[String], columns: u16) -> Vec<String> {
    let slot = usize::from(mergewright_ui::branch_pane::pane_width(columns).expect("a pane") + 2);
    let at = usize::from(columns) - slot;
    frame
        .iter()
        .filter_map(|line| {
            let cells: Vec<char> = line.chars().collect();
            (cells.get(at) == Some(&'│'))
                .then(|| cells[at + 1..].iter().collect::<String>().trim().to_owned())
        })
        .collect()
}

#[test]
fn the_trunk_and_integration_names_are_held_after_open() {
    let repo = repo_of(&[
        &commit(README),
        &["switch", "--quiet", "-c", "develop"],
        &commit(START_RELEASE),
        &["switch", "--quiet", "main"],
    ]);
    let (app, state) = opened(repo.path(), 120);
    let walk = state
        .history
        .as_ref()
        .and_then(|history| history.walk.as_ref())
        .expect("the walk's facts are held on History.walk");
    let names: Vec<&[u8]> = walk.integration.iter().map(RefName::as_bytes).collect();
    assert_eq!(
        walk.default_branch.as_ref().map(RefName::as_bytes),
        Some(&b"refs/heads/main"[..])
    );
    assert_eq!(names, [&b"refs/heads/develop"[..]]);
    app.close();
}

#[test]
fn the_identity_of_every_candidate_is_its_lanes() {
    let repo = sara_on_wallet();
    let (app, state) = opened(repo.path(), 120);
    let history = state.history.as_ref().expect("a walk");
    let walk = history.walk.as_ref().expect("walk facts");
    let refs = state.refs.as_deref().expect("refs read");
    let window = history.window.as_ref().expect("a window");
    let all = candidates(refs, &BranchNames::of(walk));
    assert_eq!(all.len(), 3, "{all:?}");
    let lane_of = |name: &str| {
        let candidate = all
            .iter()
            .find(|candidate| candidate.name == name)
            .unwrap_or_else(|| panic!("{name} is a candidate"));
        let row = window
            .rows
            .iter()
            .find(|row| row.commit.id == candidate.tip)
            .unwrap_or_else(|| panic!("{name} has its tip on screen"));
        (identity_of(candidate), mergewright_graph::made_on(row))
    };
    // A branch's tip row is made on that branch, wherever its lane came from
    // (ADR-0064): wallet-release sits under sara's lane and takes its name
    // there, so its identity is its tip row's lane too.
    for name in [
        "refs/heads/main",
        "refs/heads/sara/fix-toc",
        "refs/heads/wallet-release",
    ] {
        let (identity, lane) = lane_of(name);
        assert_eq!(Some(identity), lane, "{name}");
    }
    app.close();
}
