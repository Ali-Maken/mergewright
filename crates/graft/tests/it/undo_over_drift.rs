//! Usability review Q4 (`docs/reviews/m6-usability-final.md`), pinned for
//! m6-conflicts/69: after a checkout and an edit, `u y` over the drift box
//! takes the checkout back — and the edit? Then `u` again. Read back from
//! git and the files, never from `AppState`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::m6_stash_put_back::{cursor_to, main_and_fix, planned, rev};
use crate::preview::opened;
use crate::repo_truth::blob_of;

/// The edit made after the checkout: the drift the box names.
pub(crate) const EDIT: &str = "edited after the checkout\n";

/// On [`main_and_fix`]: check out `fix` through the menu (`. ⏎ y`), edit
/// `fix.txt`, press `u` and wait for the drift box.
pub(crate) fn drift_box_after_a_checkout(app: &App, path: &Path, state: AppState) -> AppState {
    let fix = rev(path, "fix");
    let state = cursor_to(app, opened(app, state), &fix);
    let state = app.key(&state, KeyCode::Char('.'));
    let state = app.key(&state, KeyCode::Enter);
    let state = planned(app, state);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the checkout");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/fix"
    );
    std::fs::write(path.join("fix.txt"), EDIT).expect("the scratch repo is writable");
    let state = app.key(&state, KeyCode::Char('u'));
    app.pump(state, "the drift box", |state| state.confirm.is_some())
}

#[test]
fn undo_over_drift_then_undo_again() {
    let scratch = main_and_fix();
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = drift_box_after_a_checkout(&app, path, state);
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the undo over drift");
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main",
        "`u y` took the checkout back"
    );
    assert!(
        !path.join("fix.txt").exists(),
        "observed: the edit left the working tree with the checkout (fix.txt is not on main)"
    );
    assert_eq!(
        state.status,
        Status::done("undo: checkout fix"),
        "observed: the line names the checkout, not the edit"
    );

    // Where the edit went: the undo entry's own backup holds its blob.
    let entries = graft_ops::journal::Journal::open(&path.join(".git/graft/undo-journal.jsonl"))
        .expect("the journal the writes just made is readable")
        .entries()
        .to_vec();
    let backup = entries
        .last()
        .and_then(|entry| entry.backup_ref.clone())
        .expect("the undo over drift took a backup");
    let edited = blob_of(path, EDIT.as_bytes()).expect("git hashes a blob");
    let objects = git(path, &["rev-list", "--objects", &backup]);
    assert!(
        objects.contains(&format!("{edited} worktree/fix.txt")),
        "the undo's backup {backup} keeps the edit's blob {edited}:\n{objects}"
    );

    // `u` again does not bring it back: nothing is left to undo.
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the second u's answer", |state| {
        state.operating.is_none() && state.confirm.is_none()
    });
    assert_eq!(
        state.status,
        Status::done("nothing to undo"),
        "observed: the second `u` has nothing to take back"
    );
    assert!(
        !path.join("fix.txt").exists(),
        "observed: the edit does not come back with a second `u`"
    );
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main"
    );
    drop(state);
    app.close();
}
