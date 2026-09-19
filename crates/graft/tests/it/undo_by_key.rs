//! `u` and `U` over the real job pool and real `git`
//! (m3-working-tree-undo-and-stash/06).
//!
//! The milestone's promise is *you cannot lose work*, and `u` is the key that
//! keeps it. Everything else about it is pure — which `Command` the key
//! builds, what the status line says — and none of that is evidence. These
//! tests do the operation for real, press the key, and then read the
//! repository back and compare it, bytes included, with what was there
//! before.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft_core::event::{KeyCode, KeyPress};

use crate::app_harness::{App, changes_pane, cursor_to, git, git_as_graft};

/// Bytes no editor would write and no diff would survive: a NUL and a byte
/// that is not valid UTF-8. If undo reconstructs the file through any text
/// path, this is where it shows.
const AWKWARD: &[u8] = b"\x00\xff\n";

/// A scratch repository with one commit and one tracked file.
fn repo_with_a_commit() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\ntwo\nthree\n")
        .expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    git_as_graft(repo, &["commit", "-m", "add sample"]);
    scratch
}

/// How many entries the journal file holds. Two lines per entry — the
/// pending line and its outcome — so this is what the browser would list.
fn journal_entries(repo: &std::path::Path) -> usize {
    let path = repo.join(".git/graft/undo-journal.jsonl");
    let text = std::fs::read_to_string(&path).expect("the journal was written");
    text.lines()
        .filter(|line| line.contains("\"pending\"") || line.contains("\"Pending\""))
        .count()
        .max(text.lines().count().div_ceil(2))
}

/// Discard an untracked file, then `u`: the file comes back byte for byte,
/// awkward bytes and all. This is the promise at its narrowest, and the one
/// place it can be checked exactly.
#[test]
fn u_brings_a_discarded_untracked_file_back_byte_for_byte() {
    let scratch = repo_with_a_commit();
    let repo = scratch.path();
    std::fs::write(repo.join("scratch.bin"), AWKWARD).expect("the scratch repo is writable");
    let before = std::fs::read(repo.join("scratch.bin")).expect("the file was just written");

    let (app, state) = App::open(repo);
    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "scratch.bin");

    // `d` asks, `→` moves off the safe answer, `Enter` discards.
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the discard");
    assert!(
        !repo.join("scratch.bin").exists(),
        "the discard really deleted it"
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    drop(state);
    app.close();

    let after = std::fs::read(repo.join("scratch.bin")).expect("`u` put the file back");
    assert_eq!(after, before, "byte for byte, not merely the same length");
    assert_eq!(
        after, AWKWARD,
        "and the bytes are the awkward ones, not a re-encoding"
    );
    assert!(
        journal_entries(repo) >= 2,
        "the discard and its undo are both in the journal"
    );
}

/// Commit, `u`, `U`: `HEAD` goes back to where it was with the file staged
/// again, and then forward to the commit. Read out of git, not out of
/// `AppState`.
#[test]
fn u_takes_a_commit_back_and_capital_u_puts_it_again() {
    let scratch = repo_with_a_commit();
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\ntwo\nTHREE\n")
        .expect("the scratch repo is writable");
    let before_head = git(repo, &["rev-parse", "HEAD"]);

    let (app, state) = App::open(repo);
    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "sample.txt");

    // `Space` in the file list stages the whole file.
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.settle_write(state, "the stage");
    assert!(
        git(repo, &["diff", "--cached", "--name-only"]).contains("sample.txt"),
        "the file is staged before the commit"
    );
    // The editor refuses while `AppState` still says nothing is staged, so
    // wait for the write's own refresh to land before opening it.
    let state = app.pump(state, "the refreshed status", |state| {
        state
            .working_tree
            .as_ref()
            .is_some_and(|status| status.staged().next().is_some())
    });

    // `c` opens the editor; type a subject and commit with the button.
    let state = app.key(&state, KeyCode::Char('c'));
    let mut state = app.pump(state, "the commit editor", |state| {
        state.commit_editor.is_some()
    });
    for letter in "shout three".chars() {
        state = app.key(&state, KeyCode::Char(letter));
    }
    // `Ctrl+Enter` commits from wherever the focus is. The `[ Commit ]`
    // button is the binding that works in every terminal (ADR-0034) and has
    // its own tests in `commit_editor_writes`; what matters here is that a
    // commit really lands, so that `u` has one to take back.
    let state = app.press(&state, KeyPress::control(KeyCode::Enter));
    let state = app.settle_write(state, "the commit");
    let after_head = git(repo, &["rev-parse", "HEAD"]);
    assert_ne!(after_head, before_head, "the commit landed");

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]),
        before_head,
        "`u` put HEAD back"
    );
    assert!(
        git(repo, &["diff", "--cached", "--name-only"]).contains("sample.txt"),
        "and the change is staged again, not merely uncommitted"
    );

    let state = app.key(&state, KeyCode::Char('U'));
    let state = app.settle_write(state, "the redo");
    drop(state);
    app.close();

    assert_eq!(
        git(repo, &["rev-parse", "HEAD"]),
        after_head,
        "`U` put the very same commit back"
    );
}

/// `u` with nothing in force says so, and is not a failure: the status line
/// clears on the next key like any other acknowledgement.
#[test]
fn u_on_a_repository_nothing_has_written_says_there_is_nothing_to_undo() {
    let scratch = repo_with_a_commit();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    // A clean tree has no uncommitted-changes row, so the changes pane is
    // not the way in here; `u` is global and works from the graph.
    let state = app.pump(state, "the first rows", |state| {
        state
            .history
            .as_ref()
            .and_then(|history| history.window.as_ref())
            .is_some_and(|window| !window.rows.is_empty())
    });

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the undo to answer", |state| {
        state.operating.is_none()
    });
    let said = match &state.status {
        graft::update::Status::Done(said) => said.clone(),
        other => panic!("nothing to undo is an answer, not a failure: {other:?}"),
    };
    drop(state);
    app.close();
    assert!(
        said.contains("nothing to undo"),
        "it says which way round it is: {said}"
    );
}
