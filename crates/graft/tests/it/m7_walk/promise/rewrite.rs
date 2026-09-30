//! Rewrite: his branch (`feature-a`, five commits since `main`) with a
//! teammate's (`feature-b`) built on it — R1's mock, `rewrite_editor`'s
//! fixture.
//!
//! 1. `R s s y` (`squash-last-three`, 4): three in one; `feature-b` follows.
//! 2. `u` (1): both branches back.
//! 3. An `edit` stop, made in the editor (`R e y`, 3 — reaching the stop
//!    lands with no key, `edit-stop-lands` 0), then his edit and `space a c`
//!    (`edit-stop-amend-continue`, 3): the graph, the stack gone.
//!
//! Total 4 + 1 + 3 + 0 + 3 = 11 keys.

use std::path::Path;

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress};

use super::super::rebase_line::{frame, git, open, within};
use super::super::rewrite_editor::{
    capital_r, every_ref, message_shown, mock_repo, place_on, read, replanned, subjects, undo,
};
use crate::app_harness::App;

/// The scenario's keys: 4 + 1 + 3 + 0 + 3.
const TOTAL: usize = 11;

fn press(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    key: &'static str,
) -> AppState {
    route.push(key);
    let code = if key == "space" {
        KeyCode::Char(' ')
    } else {
        KeyCode::Char(key.chars().next().unwrap_or_else(|| panic!("a key")))
    };
    app.key(state, code)
}

/// Step 1: `R s s y` on HEAD. Returns the message the editor showed.
fn squash_three(
    app: &App,
    state: AppState,
    repo: &Path,
    route: &mut Vec<&'static str>,
) -> AppState {
    let tree_a = read(repo, "feature-a^{tree}");
    let tree_b = read(repo, "feature-b^{tree}");
    let state = place_on(app, state, repo, "HEAD");
    let mut journey = Vec::new();
    let state = capital_r(app, &state, &mut journey, 187, 50);
    let s = KeyPress::plain(KeyCode::Char('s'));
    let state = replanned(app, &state, &mut journey, s, "s", 187, 50);
    let state = replanned(app, &state, &mut journey, s, "s", 187, 50);
    let shown = message_shown(&frame(&state, 187, 50));
    journey.push("y");
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the squash");
    within("squash-last-three", &journey);
    route.extend(journey);

    assert_eq!(
        subjects(repo, "main..feature-a"),
        ["Handle expired tokens", "Add OAuth callback"],
        "three in one; the fixup melded"
    );
    assert_eq!(
        subjects(repo, "main..feature-b"),
        ["b1", "Handle expired tokens", "Add OAuth callback"],
        "feature-b's own commit, on the new line"
    );
    assert_eq!(read(repo, "feature-b^"), read(repo, "feature-a"), "follows");
    assert_eq!(
        read(repo, "feature-a^{tree}"),
        tree_a,
        "feature-a's tree kept"
    );
    assert_eq!(
        read(repo, "feature-b^{tree}"),
        tree_b,
        "feature-b's tree kept"
    );
    let message = git(repo, &["log", "-1", "--format=%B", "feature-a"], 0);
    let written: Vec<&str> = message.trim_end().lines().map(str::trim).collect();
    assert_eq!(written, shown, "the message written is the one shown");
    assert_eq!(
        written.first().copied(),
        Some("Handle expired tokens"),
        "the message leads with the kept row: {written:?}"
    );
    state
}

/// Step 3's setup half: `R` on "Handle expired tokens" (HEAD~2), `e` on its
/// row, `y` — the rewrite stops there and lands on the graph (no key).
fn edit_stop(app: &App, state: AppState, repo: &Path, route: &mut Vec<&'static str>) -> AppState {
    // The undo's re-read of the graph: HEAD's row back in the window.
    let head = read(repo, "HEAD");
    let state = app.pump(state, "the graph re-read after u", |state| {
        state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| {
                window
                    .rows
                    .iter()
                    .any(|row| row.commit.id.to_string() == head)
            })
    });
    let state = place_on(app, state, repo, "HEAD~2");
    let mut journey = Vec::new();
    let state = capital_r(app, &state, &mut journey, 187, 50);
    route.extend(journey);
    let state = press(app, &state, route, "e");
    let state = app.pump(state, "the replan", |state| {
        !frame(state, 187, 50).contains("checking\u{2026}")
    });
    let state = press(app, &state, route, "y");
    let state = app.pump(state, "the stop", |state| {
        state.operating.is_none() && state.stopped.is_some()
    });
    assert_eq!(
        git(repo, &["log", "-1", "--format=%s"], 0).trim(),
        "Handle expired tokens",
        "stopped on the row marked e"
    );
    within("edit-stop-lands", &[]);
    // The stop's progress is read after the stop lands: a wait, no key.
    let state = app.pump(state, "the stack still to apply", |state| {
        graft::update::banner::pending_view(state).is_some()
    });
    assert!(!state.banner_focus, "the graph has the keyboard");
    state
}

/// Step 3's journey: his edit to `tokens.txt`, then `space a c`.
fn amend_and_continue(
    app: &App,
    state: &AppState,
    repo: &Path,
    route: &mut Vec<&'static str>,
) -> AppState {
    std::fs::write(repo.join("tokens.txt"), "Handle expired tokens, fixed\n")
        .unwrap_or_else(|why| panic!("{why}"));
    let state = app.focus_gained(state);
    let state = app.pump(state, "the edit, read", |state| {
        state.working_tree.as_ref().is_some_and(|tree| {
            tree.entries
                .iter()
                .any(|entry| entry.path == Path::new("tokens.txt") && entry.unstaged.is_some())
        })
    });
    let mut journey = Vec::new();
    let state = press(app, &state, &mut journey, "space");
    let state = app.settle_write(state, "the stage");
    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let state = press(app, &state, &mut journey, "a");
    let state = app.pump(state, "the amend queued", |state| state.operating.is_some());
    let state = app.settle_write(state, "the amend");
    let state = app.pump(state, "the files re-read", |state| {
        state.status_read.is_none()
    });
    let state = press(app, &state, &mut journey, "c");
    let state = app.pump(state, "the rewrite home", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    within("edit-stop-amend-continue", &journey);
    route.extend(journey);
    state
}

#[test]
fn promise_rewrite_without_leaving() {
    let dir = mock_repo();
    let repo = dir.path();
    let before = every_ref(repo);
    let (app, state) = open(repo, 187, 50);
    let mut route: Vec<&'static str> = Vec::new();

    let state = squash_three(&app, state, repo, &mut route);

    route.push("u");
    let state = undo(&app, &state);
    assert_eq!(every_ref(repo), before, "`u`: both branches back");

    let state = edit_stop(&app, state, repo, &mut route);
    let state = amend_and_continue(&app, &state, repo, &mut route);

    assert!(state.stopped.is_none(), "no rewrite stopped");
    assert!(
        graft::update::banner::pending_view(&state).is_none(),
        "the stack is gone"
    );
    assert_eq!(state.graph_overlay(), None, "the graph, no overlay");
    assert_eq!(
        git(repo, &["show", "feature-a~2:tokens.txt"], 0),
        "Handle expired tokens, fixed\n",
        "the stopped commit has the edit"
    );
    assert_eq!(read(repo, "feature-b^"), read(repo, "feature-a"), "follows");
    assert_eq!(route.len(), TOTAL, "the scenario's keys: {route:?}");
    drop(state);
    app.close();
}
