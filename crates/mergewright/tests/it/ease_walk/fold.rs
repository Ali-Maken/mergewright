//! The fold checks (06, 06c, `fold_…`) and a shell-begun merge resolved with
//! picks (05, 05b, `picks_shell_merge`), on the mock-shaped repository.
//! Text and shape only; colour is unverified (rule 5).

use std::path::Path;
use std::process::Command;

use mergewright::jobs::Dispatch;
use mergewright::update::AppState;
use mergewright::update::detail::{DetailSpec, show_line};
use mergewright_core::event::KeyCode;
use mergewright_core::oid::Oid;

use super::fixture::{self, mock};
use super::flow::{committed, head_parents, pick, reviewed, unmerged};
use super::screen::{cursor_on, drawn, line_with, open, rows_side, selected_subject, shown};
use crate::app_harness::{App, git};

/// A merge of `main` into a feature branch is a sync, not a PR: it has
/// nothing folded behind it, so its row carries no `▸`.
#[test]
fn fold_sync_merge_has_no_mark() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let frame = drawn(&state);
    let sync = line_with(&frame, fixture::SYNC_MERGE);
    assert!(!sync.contains('▸') && !sync.contains('▾'), "{sync:?}");
    // The PR merge on the same screen is folded, so the mark is drawn when
    // it applies: the absence above is not the mark being undrawn.
    assert!(line_with(&frame, fixture::PR_MERGE).contains('▸'));
    app.close();
}

/// With the detail pane on a folded PR merge, following its second parent
/// opens the merge and lands the cursor on that parent.
#[test]
fn fold_reveal_parent() {
    let scratch = mock();
    let repo = scratch.path();
    let (app, state) = open(repo, 200);
    let state = cursor_on(&app, state, fixture::PR_MERGE);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.detail.shown, "the detail pane is on the merge");
    assert!(line_with(&drawn(&state), fixture::PR_MERGE).contains('▸'));

    let parent = git(
        repo,
        &["rev-parse", &format!(":/{}", fixture::PR_COMMITS[1])],
    );
    let parent = Oid::from_hex(parent.trim()).expect("a full hash");
    let mut next = state.clone();
    let commands = show_line(
        &mut next,
        DetailSpec::Commit(parent),
        Path::new("ledger.txt"),
        None,
        None,
    );
    for command in commands {
        app.jobs.dispatch(command);
    }
    let landed = app.pump(next, "the reveal and the landing", |state| {
        state.line_landing.is_none() && selected_subject(state).starts_with(fixture::PR_COMMITS[1])
    });
    let frame = drawn(&landed);
    assert!(
        line_with(&frame, fixture::PR_MERGE).contains('▾'),
        "{}",
        shown(&frame)
    );
    assert!(selected_subject(&landed).starts_with(fixture::PR_COMMITS[1]));
    app.close();
}

/// Without a commit-graph file the rows, once the view has promoted, read as
/// they do with one.
#[test]
fn fold_same_without_commit_graph() {
    let with = mock();
    git(with.path(), &["commit-graph", "write", "--reachable"]);
    let graph = with.path().join(".git/objects/info/commit-graph");
    assert!(graph.exists(), "the first run has a commit-graph");
    let (app, state) = open(with.path(), 200);
    let want = rows_side(&drawn(&state), 200);
    app.close();

    let without = mock();
    assert!(
        !without
            .path()
            .join(".git/objects/info/commit-graph")
            .exists(),
        "the second run has none"
    );
    let (app, state) = open(without.path(), 200);
    // `open` waits for the promotion: `walk_has_settled` holds
    // `history.promotion.is_none()`, never a sleep.
    assert!(
        state
            .history
            .as_ref()
            .is_some_and(|history| history.promotion.is_none())
    );
    assert_eq!(rows_side(&drawn(&state), 200), want);
    app.close();
}

/// `git merge` in the shell stops on the clash; launching Mergewright lands
/// on the banner; `⏎ o u o y y` merges. Nothing is staged until the last `y`.
#[test]
fn picks_shell_merge() {
    let scratch = mock();
    let repo = scratch.path();
    let stopped = Command::new("git")
        .args(["merge", "--no-edit", "--quiet", "integration"])
        .current_dir(repo)
        .output()
        .expect("git runs");
    assert!(!stopped.status.success(), "the merge stops on its clash");
    assert!(!unmerged(repo).is_empty());

    let (app, state) = App::open(repo);
    let state = app.pump(state, "the launch on the banner", |state| {
        state.in_progress.is_some() && state.banner_focus && state.working_tree.is_some()
    });
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the first open clash", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some() && open.job.is_none() && open.set_job.is_none())
    });
    let mut state: AppState = state;
    for key in ['o', 'u', 'o'] {
        state = pick(&app, &state, key);
        assert!(!unmerged(repo).is_empty(), "`{key}` stages nothing");
    }
    let state = reviewed(&app, &state);
    assert!(!unmerged(repo).is_empty(), "the review stages nothing");
    let _landed = committed(&app, &state);
    assert!(unmerged(repo).is_empty(), "the last `y` stages and commits");
    assert_eq!(head_parents(repo), 2, "a two-parent merge");
    app.close();
}
