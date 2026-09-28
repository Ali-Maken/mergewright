//! Journeys at a rewrite's stop (m7-power-tools/07c; note R4, journeys 4
//! and 5): a conflict stop lands on the resolver, an `edit` stop on the
//! graph with the still-to-apply block drawn, `space a c` amends and goes
//! home, and `⇥ a y` aborts a chain stop with every branch back.

use std::path::Path;

use graft::update::AppState;
use graft_core::event::KeyCode;
use graft_ui::still_to_apply::TITLE;

use super::rebase_line::{commit, cursor_to_main, frame, git, open, r, within};
use crate::app_harness::App;
use crate::rewrite_stop::{rebase_merge, relaunched, stopped_on_a1};

/// `feature-a` (a1 a2) ← `feature-b` (b1 b2) ← `feature-c` (c1), one line;
/// `main` moved on by m2, which writes `shared.txt` as b1 does: rebasing
/// the line onto main stops on b1 — step 3 of 5, branch 2 of 3.
fn chain_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    let shared = |text: &str, subject: &str, hour: u8| {
        std::fs::write(repo.join("shared.txt"), text).unwrap_or_else(|why| panic!("{why}"));
        git(repo, &["add", "-A"], hour);
        git(repo, &["commit", "-q", "-m", subject], hour);
    };
    git(repo, &["init", "-q", "-b", "main"], 0);
    shared("base\n", "m1", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(repo, "a1", 1);
    commit(repo, "a2", 2);
    git(repo, &["switch", "-q", "-c", "feature-b"], 3);
    shared("theirs\n", "b1", 3);
    commit(repo, "b2", 4);
    git(repo, &["switch", "-q", "-c", "feature-c"], 6);
    commit(repo, "c1", 6);
    git(repo, &["switch", "-q", "main"], 5);
    shared("ours\n", "m2", 5);
    git(repo, &["switch", "-q", "feature-a"], 9);
    git(repo, &["commit-graph", "write", "--reachable"], 9);
    dir
}

fn tip(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name], 0).trim().to_owned()
}

/// `y` on the preview; the rewrite stops (not a success).
fn y(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("y");
    let state = app.key(state, KeyCode::Char('y'));
    assert!(state.operating.is_some(), "y queued the rewrite");
    app.pump(state, "the stop", |state| {
        state.operating.is_none() && state.stopped.is_some()
    })
}

/// The chain stop through the app: cursor to main, `r`, `y`.
fn chain_stop(columns: u16, rows: u16) -> (tempfile::TempDir, App, AppState) {
    let dir = chain_repo();
    let (app, state) = open(dir.path(), columns, rows);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let state = y(&app, &state, &mut route);
    (dir, app, state)
}

/// `rewrite-stop-to-first-file`: after `y`, no key — the resolver is open on
/// the first conflicted file.
#[test]
fn rewrite_stop_to_first_file() {
    let (dir, app, state) = chain_stop(187, 50);
    assert!(rebase_merge(dir.path()), "git stopped");
    let route: Vec<&str> = Vec::new();
    let state = app.pump(state, "the resolver", |state| {
        state.in_progress.is_some()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some())
    });
    within("rewrite-stop-to-first-file", &route);
    let resolver = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(resolver.path, std::path::PathBuf::from("shared.txt"));
    let banner = graft::update::banner::banner_view(&state).expect("a banner");
    let words = graft_ui::graph_screen::banner_line(&banner, u16::MAX);
    assert!(
        words.contains("step 3 of 5") && words.contains("feature-b, branch 2 of 3"),
        "{words}"
    );
    crate::captures::check_shot("m7/rewrite-chain-stop-187x50.txt", &frame(&state, 187, 50));
    drop(state);
    app.close();
}

/// `edit-stop-lands`: no key — the graph has the keyboard, the cursor on
/// the stopped commit, the still-to-apply block drawn.
#[test]
fn edit_stop_lands() {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    let route: Vec<&str> = Vec::new();
    within("edit-stop-lands", &route);
    assert!(!state.banner_focus, "the graph has the keyboard");
    let pending = graft::update::banner::pending_view(&state).expect("the block");
    assert_eq!(
        pending.rows.len(),
        3,
        "a2 b1 c1 still to apply: {pending:?}"
    );
    assert_eq!(pending.rows[0].subject, "c1", "newest first");
    let shot = frame(&state, 120, 40);
    assert!(shot.contains(TITLE), "{shot}");
    let cursor_row = shot
        .lines()
        .find(|line| line.contains('▏'))
        .unwrap_or_else(|| panic!("no cursor row: {shot}"));
    assert!(
        cursor_row.contains("stopped here"),
        "the cursor is on the stopped commit: {cursor_row}"
    );
    drop(state);
    app.close();
}

/// Captures of the `edit` stop at 187×50 and 80×24.
#[test]
fn capture_edit_stop_sizes() {
    let dir = stopped_on_a1();
    let (app, state) = relaunched(dir.path());
    let wide = app.resize(&state, 187, 50);
    crate::captures::check_shot("m7/rewrite-edit-stop-187x50.txt", &frame(&wide, 187, 50));
    let narrow = app.resize(&wide, 80, 24);
    let shot = frame(&narrow, 80, 24);
    assert!(shot.contains("REBASING  step"), "{shot}");
    crate::captures::check_shot("m7/rewrite-edit-stop-80x24.txt", &shot);
    drop(narrow);
    app.close();
}

// `edit-stop-amend-continue` (`space a c`, fence 3) is not walked here.
// `space` in the graph is the fold, never a stage: `graph_key`
// (`src/update/keys.rs:314`) routes it to `view_change`, which maps it to
// `ViewChange::Fold` (`src/update/keys/view_change.rs:87`); no branch before
// it stages, on the uncommitted row or any other. (`k` does reach the
// uncommitted row — probed 2026-09-28 — but `space` there stages nothing.)
// Stopped and reported per the ticket, not rebound. The amend-then-`c` half
// is walked by 07b's
// `rewrite_stop::a_twice_then_c_amends_the_stopped_commit_and_the_rewrite_goes_home`.

/// `abort-from-a-chain-stop`: `⇥ a y` — every branch back where it was,
/// HEAD on his branch. The M7 fence has no count for this journey yet
/// (ticket 07c NOTES): the route is printed, not fenced. From the stop's
/// landing (the resolver) it is `esc a y`; the ticket's `⇥ a y` assumed
/// the graph had the keyboard.
#[test]
fn abort_from_a_chain_stop() {
    let dir = chain_repo();
    let repo = dir.path();
    let before: Vec<String> = ["feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|name| tip(repo, name))
        .collect();
    let (app, state) = open(repo, 187, 50);
    let mut setup = Vec::new();
    let state = cursor_to_main(&app, state, &mut setup);
    let state = r(&app, &state, &mut setup);
    let state = y(&app, &state, &mut setup);
    let state = app.pump(state, "the resolver", |state| {
        state.resolver.is_some() && state.in_progress.is_some()
    });
    // The resolver has no `a` of its own (keymap §2.6): `Esc` closes it to
    // the banner, then the banner's `a y`.
    let mut route: Vec<&str> = vec!["esc"];
    let mut state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "Esc closed the resolver");
    while !state.banner_focus {
        route.push("⇥");
        state = app.key(&state, KeyCode::Tab);
        assert!(route.len() < 6, "Tab never came round to the banner");
    }
    route.push("a");
    let state = app.key(&state, KeyCode::Char('a'));
    assert!(
        state.confirm.is_some(),
        "`a` asks first: {:?} {route:?}",
        state.status
    );
    route.push("y");
    let state = app.key(&state, KeyCode::Char('y'));
    assert!(
        state.operating.is_some(),
        "y sent the abort: {:?}",
        state.status
    );
    let state = app.pump(state, "the abort", |state| {
        state.operating.is_none() && state.status_read.is_none() && state.in_progress.is_none()
    });
    eprintln!("abort-from-a-chain-stop route: {route:?}");
    assert!(!rebase_merge(repo), "aborted");
    let after: Vec<String> = ["feature-a", "feature-b", "feature-c"]
        .iter()
        .map(|name| tip(repo, name))
        .collect();
    assert_eq!(after, before, "every branch back");
    assert_eq!(
        git(repo, &["symbolic-ref", "--short", "HEAD"], 0).trim(),
        "feature-a",
        "HEAD on his branch"
    );
    drop(state);
    app.close();
}
