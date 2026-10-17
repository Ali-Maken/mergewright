//! `u` under a one-millisecond tie (m8-polish-and-ship/19e §R3). Two
//! operations stamped in the same millisecond, the one journaled last with
//! the **smaller** id, so id order and the journal's own order disagree.
//! `u` takes the one journaled last: undo order is the order the journal
//! file holds its entries in, never the ids'. Built, not looped: the tie
//! and the disagreement are asserted before `u` runs.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_core::journal::{EntryStatus, JournalEntry};
use graft_ops::apply::{Context, apply};
use graft_ops::journal::next_undo;
use graft_ops::operation::{CommitParams, Operation};

use crate::support::{Letters, NOW, Never, TempRepo, git_version, journal_for, run_undo};

/// A commit of a new `file` through Graft, stamped at `NOW` with `random`
/// as its id's random half: two of these share a millisecond, and their
/// ids order by `random` alone.
fn commit_at_now(repo: &TempRepo, file: &str, random: [u8; 10]) -> JournalEntry {
    repo.write(file, b"new\n");
    repo.git(&["add", "--", file]);
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = Context::open(&handle, git_version(), &mut journal, NOW, random, &never)
        .expect("the context opens");
    let op = Operation::Commit(CommitParams {
        message: format!("add {file}"),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
        include: graft_ops::operation::Include::Staged,
    });
    apply(&mut ctx, &op)
        .unwrap_or_else(|why| panic!("{}", why.explain(&Letters)))
        .entry
}

#[test]
fn u_on_a_one_millisecond_tie_takes_the_last_appended() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("base");
    let first = commit_at_now(&repo, "first.txt", [0xff; 10]);
    let last = commit_at_now(&repo, "last.txt", [0x00; 10]);

    // The tie as built: one millisecond, both in force, and the entry
    // appended last carries the smaller id.
    assert_eq!(first.at_unix_ms, last.at_unix_ms, "one millisecond");
    assert_eq!(first.status, EntryStatus::Committed);
    assert_eq!(last.status, EntryStatus::Committed);
    assert!(last.id < first.id, "{:?} < {:?}", last.id, first.id);
    let handle = repo.open();
    let journal = journal_for(&handle);
    let entries = journal.entries();
    let order: Vec<_> = entries.iter().map(|entry| &entry.id).collect();
    assert_eq!(order, [&first.id, &last.id], "the file holds append order");

    // The newest-undoable read `u` plans from.
    assert_eq!(
        next_undo(entries).map(|entry| &entry.id),
        Some(&last.id),
        "next_undo takes the entry appended last"
    );

    // And `u` itself: the last commit taken back, the first still in.
    let undone =
        run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain(&Letters)));
    assert_eq!(undone.entry.reverses.as_ref(), Some(&last.id));
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]).trim(),
        "add first.txt"
    );
}
