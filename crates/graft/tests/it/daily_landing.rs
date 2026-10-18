//! Where he lands after the daily keys (m8-polish-and-ship/13; UX §3.1;
//! design note J18, J20): a pull that brought nothing leaves the cursor
//! where it was; `u` and `U` land on what they brought back. Real
//! repositories, real job pool, real keys; the facts read back with `git`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::{App, git, git_as_graft};
use crate::remote_by_key::{local, plain, ready, rev};

fn selected(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// Runs the queued write (or undo) to its answer, then pumps until any
/// landing it armed is spent and the cursor is on a row.
fn settled(app: &App, state: AppState, what: &str) -> AppState {
    let state = app.settle_write(state, what);
    app.pump(state, "the landing", |state| {
        !state.land_on_head && selected(state).is_some()
    })
}

fn assert_on(state: &AppState, repo: &Path, rev_name: &str, what: &str) {
    assert_eq!(
        selected(state).as_deref(),
        Some(rev(repo, rev_name).as_str()),
        "{what}: the cursor is on {rev_name}"
    );
    assert!(
        state.graph_has_focus(),
        "{what}: the graph has the keyboard"
    );
}

/// J18 `pull-nothing-new` = 1 key: `p` on an up-to-date branch, cursor on a
/// row that is not `HEAD` → it stays there; the line says `✓ up to date`.
#[test]
fn j18_a_pull_with_nothing_new_leaves_the_cursor() {
    let remote = local();
    let work = remote.work();
    let (app, state) = App::open_with_clock(
        work,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));
    let before = selected(&state);
    assert_ne!(
        before.as_deref(),
        Some(rev(work, "HEAD").as_str()),
        "the cursor starts off HEAD"
    );
    let head = rev(work, "HEAD");
    // J18's one key.
    let state = app.press(&state, plain('p'));
    crate::m8_walk::within_the_note("pull-nothing-new", &["p"]);
    let state = app.settle_write(state, "the pull");
    assert!(!state.land_on_head, "nothing new owes no landing");
    assert_eq!(
        state.status,
        Status::Done {
            what: "up to date".to_owned(),
            also: None,
            keys: None,
        }
    );
    let state = ready(&app, state);
    assert_eq!(rev(work, "HEAD"), head, "nothing moved");
    assert_eq!(selected(&state), before, "the cursor stayed");
    drop(state);
    app.close();
}

/// A repo with two commits on `main`; the cursor starts on the older one.
fn two_commits() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\n").expect("writable");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "add sample"]);
    scratch
}

/// `c`, a message, `⏎` from the subject: a commit made by key, so the journal
/// has it to undo.
fn commit_by_key(app: &App, state: &AppState, message: &str) -> AppState {
    let mut state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    for character in message.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    app.key(&state, KeyCode::Enter)
}

/// J20 `undo-see-result` = 1 key each: `u` after a commit lands on the new
/// `HEAD` (its parent); `U` lands on the redone commit — wherever the
/// cursor was moved in between.
#[test]
fn j20_u_and_capital_u_land_on_the_result() {
    let scratch = two_commits();
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\ntwo\n").expect("writable");
    git(repo, &["add", "sample.txt"]);
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = ready(&app, state);
    let parent = rev(repo, "HEAD");
    let state = commit_by_key(&app, &state, "land here");
    let state = settled(&app, state, "the commit");
    let made = rev(repo, "HEAD");
    assert_ne!(made, parent, "a commit was made");

    // Off HEAD, then `u`.
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('u'));
    crate::m8_walk::within_the_note("undo-see-result", &["u"]);
    let state = settled(&app, state, "the undo");
    assert_eq!(rev(repo, "HEAD"), parent, "the commit was taken back");
    assert_on(&state, repo, "HEAD", "after u");

    // Off HEAD again, then `U`.
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('U'));
    let state = settled(&app, state, "the redo");
    assert_eq!(rev(repo, "HEAD"), made, "the same commit is back");
    assert_on(&state, repo, "HEAD", "after U");
    drop(state);
    app.close();
}

/// `u` after a stash: the edits are back on `HEAD`'s working tree, and the
/// cursor is on `HEAD` — not left where the stash row was.
#[test]
fn u_after_a_stash_lands_on_head() {
    let scratch = two_commits();
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\nedited\n").expect("writable");
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('S'));
    assert!(state.stash.shown, "`S` opens the overlay");
    let mut state = app.key(&state, KeyCode::Char('s'));
    for character in "wip".chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the stash push");
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.stash.shown, "`Esc` closes the overlay");
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char('u'));
    let state = settled(&app, state, "undoing the stash");
    assert_eq!(
        git(repo, &["stash", "list"]).lines().count(),
        0,
        "the stash was taken back"
    );
    assert_on(&state, repo, "HEAD", "after u of a stash");
    drop(state);
    app.close();
}

/// What right looks like (`m8-quit-while-running-80x24.txt`, ruling 4):
/// `f` then `q` before the fetch answers — a box titled `Quit?` reading
/// `fetch all remotes is still running · quit stops it`, `[ Cancel ]
/// [ Quit ]` with the cursor on Cancel, the bottom row the box's keys led by
/// `y quit`. `y` quits.
/// Colour unverified.
#[test]
fn capture_quit_while_a_fetch_runs() {
    let remote = local();
    let work = remote.work();
    let (app, state) = App::open_with_clock(
        work,
        graft::session::Clock::Fixed(graft_fixtures::PINNED_NOW),
    );
    let state = crate::captures::settled_frame(&app, ready(&app, state));
    // Pressed without pumping: the fetch's answer cannot arrive between
    // the two keys, so the write is still running when `q` lands.
    let state = app.press(&state, plain('f'));
    assert!(state.operating.is_some(), "the fetch runs");
    let state = app.press(&state, plain('q'));
    assert!(state.quit.is_none(), "the box asks first");
    let shown = crate::captures::frame(&state, 80, 24);
    assert!(
        shown
            .iter()
            .any(|row| row.contains("fetch all remotes is still running")),
        "{}",
        shown.join("\n")
    );
    assert!(
        shown.last().is_some_and(|bar| bar.contains("y quit")),
        "{}",
        shown.join("\n")
    );
    let state = app.press(&state, plain('y'));
    assert_eq!(state.quit, Some(graft::update::Quit::Requested));
    crate::captures::check_shot(
        "m8-quit-while-running-80x24.txt",
        &format!(
            "=== `f` then `q` while the fetch runs ===\n{}\n",
            shown.join("\n")
        ),
    );
    drop(state);
    app.close();
}
