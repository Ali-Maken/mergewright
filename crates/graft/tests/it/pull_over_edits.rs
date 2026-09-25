//! m6-conflicts/85 D: a fast-forward pull over an unsaved edit to a file the
//! pull changes is refused in his words, and the keys the refusal names work
//! from where he is — the changes pane (review `m6-usability-2` D, m2).
//!
//! The pull itself still does not set the edits aside (`KNOWN_GAPS`, M7): this
//! walks the words and the keys, not a new journey, so it counts keys
//! without a note row to count against.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::detail::Loading;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;
use graft_fixtures::remote::LocalRemote;
use graft_ui::detail::DetailFocus;

use crate::app_harness::App;
use crate::remote_by_key::{finish, local, ready, rev, run};
use crate::stash_clash::stash_list;

const SHARED: &str = "shared.txt";
const EDIT: &str = "mine, unsaved\n";

/// Pushed `base`, then `theirs` on top (pushed from here and stepped back,
/// as a teammate's would be): origin is one ahead, touching the file he then
/// edits without saving. Returns origin's tip and HEAD, 40 hex each.
fn origin_ahead_over_an_edit(remote: &LocalRemote) -> (String, String) {
    let work = remote.work();
    std::fs::write(work.join(SHARED), "base\n").expect("the working clone is writable");
    run(remote, work, &["add", "--", SHARED]);
    run(remote, work, &["commit", "-q", "-m", "add shared"]);
    std::fs::write(work.join(SHARED), "theirs\n").expect("the working clone is writable");
    run(remote, work, &["commit", "-q", "-am", "their change"]);
    run(remote, work, &["push", "-q", "origin", "HEAD"]);
    let theirs = rev(work, "HEAD");
    run(remote, work, &["reset", "-q", "--hard", "HEAD~1"]);
    std::fs::write(work.join(SHARED), EDIT).expect("the working clone is writable");
    (theirs, rev(work, "HEAD"))
}

/// The refusal `p` answered: his words, the keys, HEAD and the tree as they
/// were.
fn refused(state: &AppState, work: &Path, before: &str) {
    let Status::Sticky { reason, keys } = &state.status else {
        panic!("a refused pull is a sticky line: {:?}", state.status);
    };
    assert_eq!(
        reason,
        &format!("pull refused: your edits to {SHARED} would be overwritten")
    );
    assert_eq!(keys.as_deref(), Some("S s stashes them first"));
    assert_eq!(rev(work, "HEAD"), before, "HEAD unchanged");
    assert_eq!(
        std::fs::read_to_string(work.join(SHARED)).expect("the file"),
        EDIT,
        "the tree unchanged"
    );
}

/// `p` refused, then from the changes pane `S s ⏎` (stash his edit), then
/// `esc p` — HEAD at origin's tip, one stash listed.
#[test]
fn a_ff_pull_over_overlapping_edits_names_keys_that_work() {
    let remote = local();
    let work = remote.work();
    let (theirs, before) = origin_ahead_over_an_edit(&remote);
    let (app, state) = App::open(work);
    let state = app.resize(&state, 120, 40);
    let mut count = 0;
    let mut press = |state: &AppState, code: KeyCode| {
        count += 1;
        app.key(state, code)
    };

    // `p`: refused, his words and the keys.
    let state = ready(&app, state);
    let state = press(&state, KeyCode::Char('p'));
    let state = finish(&app, state, "the refused pull");
    refused(&state, work, &before);

    // Into the changes pane: `⏎` on the uncommitted row.
    let state = press(&state, KeyCode::Enter);
    let state = app.pump(state, "the changes pane", |state| {
        state.detail.shown && matches!(state.detail.files, Loading::Ready(_))
    });
    assert_eq!(
        state.detail.focus,
        DetailFocus::Files,
        "on the changes pane"
    );

    // `S s ⏎` from the pane: the list, the save prompt, saved.
    let state = press(&state, KeyCode::Char('S'));
    assert!(state.stash.shown, "`S` opens the list from the pane");
    let state = app.pump(state, "the stash list", |state| {
        !matches!(state.stash.entries, Loading::Idle | Loading::Pending(_))
    });
    let state = press(&state, KeyCode::Char('s'));
    let state = press(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the stash");
    assert_eq!(stash_list(work).len(), 1, "his edit is stashed");

    // `esc p`: the list closes, the pull runs.
    let state = press(&state, KeyCode::Escape);
    assert!(!state.stash.shown, "esc closes the list");
    let state = ready(&app, state);
    let state = press(&state, KeyCode::Char('p'));
    let state = finish(&app, state, "the pull");
    assert!(
        matches!(&state.status, Status::Done { what, .. } if what.contains("pulled origin/")),
        "{:?}",
        state.status
    );
    assert_eq!(rev(work, "HEAD"), theirs, "HEAD at origin's tip");
    assert_eq!(stash_list(work).len(), 1, "one stash listed");
    assert_eq!(count, 7, "p, ⏎ into the pane, S s ⏎, esc p");
    drop(state);
    app.close();
}
