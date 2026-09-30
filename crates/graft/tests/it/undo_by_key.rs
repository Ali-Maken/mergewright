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

/// The journal, read the way the app reads it. The old version of this
/// counted lines containing `"pending"` and rounded — a heuristic that
/// checked neither the entry count nor what each entry was, under an
/// assertion claiming both (m3/06 review, Major 7).
fn journal(repo: &std::path::Path) -> Vec<graft_core::journal::JournalEntry> {
    let path = journal_path(repo);
    graft_ops::journal::Journal::open(&path)
        .expect("the journal the writes just made is readable")
        .entries()
        .to_vec()
}

/// Where a repository's journal lives.
fn journal_path(repo: &std::path::Path) -> std::path::PathBuf {
    repo.join(".git/graft/undo-journal.jsonl")
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
    let entries = journal(repo);
    assert_eq!(
        entries.len(),
        2,
        "the discard and its undo, and nothing else: {entries:?}"
    );
    assert_eq!(entries[0].kind, graft_core::journal::OperationKind::Discard);
    assert_eq!(entries[1].kind, graft_core::journal::OperationKind::Undo);
    assert_eq!(
        entries[1].reverses.as_ref(),
        Some(&entries[0].id),
        "the undo names the entry it took back"
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
        graft::update::Status::Done { what: said, .. } => said.clone(),
        other => panic!("nothing to undo is an answer, not a failure: {other:?}"),
    };
    drop(state);
    app.close();
    assert!(
        said.contains("nothing to undo"),
        "it says which way round it is: {said}"
    );
}

/// A `Pending` line no outcome ever followed: what a process killed mid-write
/// leaves behind. Written straight into the journal, because the only other
/// way to make one is to kill Graft at the right microsecond.
fn tear_the_journal(repo: &std::path::Path, description: &str) {
    use graft_core::journal::{EntryId, HeadRef, JournalEntry, OperationKind, Touches};

    let path = journal_path(repo);
    std::fs::create_dir_all(path.parent().expect("the journal has a directory"))
        .expect("the scratch repo is writable");
    let mut journal = graft_ops::journal::Journal::open(&path).expect("a journal opens");
    let entry = JournalEntry {
        id: EntryId::new(1, [7; 10]),
        at_unix_ms: 1_700_000_000_000,
        kind: OperationKind::Commit,
        description: description.to_owned(),
        status: graft_core::journal::EntryStatus::Pending,
        touches: Touches {
            refs: Vec::new(),
            index: false,
            worktree: true,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: None,
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: None,
        pins: Vec::new(),
        commands: Vec::new(),
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
    };
    journal
        .append_pending(&entry)
        .expect("the pending line is written");
}

/// An operation that never finished is announced, and **keeps** being
/// announced until `J` is pressed — through the re-read every write does and
/// through `Ctrl+R` (m3/06 review, Major 1). The row for it reads
/// `interrupted`, not `failed: interrupted`, because recovery rewrites the
/// entry to `Aborted` before any view sees it (Major 2).
#[test]
fn an_interrupted_operation_is_announced_until_j_and_its_row_says_interrupted() {
    let scratch = repo_with_a_commit();
    let repo = scratch.path();
    tear_the_journal(repo, "commit \"half written\"");

    let (app, state) = App::open(repo);
    let state = app.pump(state, "the launch journal read", |state| {
        graft::update::undo::interrupted_notice(state).is_some()
    });

    let notice = graft::update::undo::interrupted_notice(&state).expect("the launch read found it");
    assert!(notice.contains("half written"), "{notice}");

    // The row the browser would draw, straight off the real read. Recovery
    // has already closed the entry as `Aborted { "interrupted" }` by now.
    let rows = graft::view::journal_view(&state, 40).rows;
    let row = rows.first().expect("one entry");
    assert_eq!(row.mark, graft_ui::journal_screen::RowMark::Interrupted);
    assert!(
        row.summary.as_str().starts_with("interrupted"),
        "a process that died did not `fail`: {}",
        row.summary.as_str()
    );

    // `Ctrl+R` re-reads, and that read closes nothing at all.
    let state = app.press(&state, KeyPress::control(KeyCode::Char('r')));
    let state = app.pump(state, "the re-read", |state| {
        matches!(
            state.journal.entries,
            graft::update::detail::Loading::Ready(_)
        ) && state.journal.reading.is_none()
    });
    assert!(
        graft::update::undo::interrupted_notice(&state).is_some(),
        "the notice is the only way the user learns an operation did not finish; \
         it must not vanish at the first refresh"
    );

    // `J` is what takes it up, and the only thing that ends it.
    let state = app.key(&state, KeyCode::Char('J'));
    assert_eq!(graft::update::undo::interrupted_notice(&state), None);
    drop(state);
    app.close();
}

/// The instant `the_journal_timestamp_comes_through_the_clock` pins the
/// run at: a round Unix second well clear of any fixture's dates.
const PINNED: i64 = 1_800_000_000;

/// **The clock seam** (m5-history-and-ghost-preview/25). `jobs::operate`
/// and `jobs::undo` stamped every journal entry from `SystemTime::now()`
/// directly, so a capture of a repository the app writes *during* the
/// capture drew an instant that changed on every run — and the journal's
/// own rows draw that instant.
///
/// The three captures this unblocks cannot be pinned render-side: the
/// journal row's age is `at_unix_ms` against the view's clock, so a render
/// clock pinned in the past would draw a real entry as a literal `future`.
/// Both ends have to come from the same `Clock`, and this is the end that
/// was missing.
///
/// Two entries, from the two modules: the discard is `jobs::operate`'s and
/// the undo is `jobs::undo`'s.
#[test]
fn the_journal_timestamp_comes_through_the_clock() {
    let scratch = repo_with_a_commit();
    let repo = scratch.path();
    std::fs::write(repo.join("scratch.bin"), AWKWARD).expect("the scratch repo is writable");

    let (app, state) = App::open_with_clock(repo, graft::session::Clock::Fixed(PINNED));
    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "scratch.bin");
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = app.settle_write(state, "the discard");
    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    drop(state);
    app.close();

    let entries = journal(repo);
    assert_eq!(entries.len(), 2, "the discard and its undo: {entries:?}");
    for entry in &entries {
        assert_eq!(
            entry.at_unix_ms,
            PINNED * 1_000,
            "`{}` was stamped from the wall clock, not the run's `Clock`",
            entry.description
        );
    }
}
