//! The writer's lock (m8-polish-and-ship/58): two `Journal`s on one file
//! contend as two processes do.

use std::collections::BTreeMap;

use mergewright_core::journal::{EntryStatus, JournalEntry, OperationKind};

use super::Journal;
use crate::journal::JournalError;
use crate::journal::tests::{committed, id};

fn pending(at: i64) -> JournalEntry {
    JournalEntry {
        status: EntryStatus::Pending,
        head_after: None,
        refs_after: BTreeMap::new(),
        index_after: None,
        worktree_after: None,
        ..committed(at, OperationKind::Commit, 1, 2)
    }
}

/// A writer part-way through an entry keeps recovery out; once it drops,
/// recovery gets the journal alone and sees its outcome, so the entry is
/// never closed twice.
#[test]
fn a_writer_keeps_recovery_out_until_it_drops() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undo-journal.jsonl");
    let mut writer = Journal::open(&path).unwrap();
    let mut reader = Journal::open(&path).unwrap();

    writer.append_pending(&pending(10)).unwrap();
    assert!(
        reader.alone().unwrap().is_none(),
        "a running writer's entry is not recovery's to close"
    );

    writer
        .append_outcome(&committed(10, OperationKind::Commit, 1, 2))
        .unwrap();
    drop(writer);

    let alone = reader.alone().unwrap();
    assert!(alone.is_some(), "the writer has gone");
    assert!(
        reader.unresolved().is_empty(),
        "the copy was read again under the lock: the outcome is seen"
    );
    assert!(
        matches!(
            reader.mark_interrupted(&id(10), None),
            Err(JournalError::DuplicateOutcome(_))
        ),
        "nothing left to close"
    );
}

/// A writer waits out recovery, then sees what it closed: its copy is read
/// again under the lock, so it cannot write a second outcome from a stale
/// `Pending`.
#[test]
fn a_claim_reads_what_recovery_closed() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undo-journal.jsonl");
    let mut dead = Journal::open(&path).unwrap();
    dead.append_pending(&pending(10)).unwrap();
    drop(dead);

    let mut resumer = Journal::open(&path).unwrap();
    let mut reader = Journal::open(&path).unwrap();
    let alone = reader.alone().unwrap().expect("no writer is alive");
    reader.mark_interrupted(&id(10), None).unwrap();
    drop(alone);

    assert_eq!(resumer.unresolved().len(), 1, "stale before the claim");
    resumer.claim().unwrap();
    assert!(
        resumer.unresolved().is_empty(),
        "read again at the claim: recovery's outcome is seen"
    );
}

/// 58r (review 58, Minor 1): a `Journal` holding the writer's lock asks for
/// the journal alone only by mistake — recovery run on a writer's own
/// journal. It is refused, and the lock is kept: the entry it may be
/// writing stays out of recovery's reach.
#[test]
fn a_writer_asking_for_the_journal_alone_is_refused_and_keeps_its_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undo-journal.jsonl");
    let mut writer = Journal::open(&path).unwrap();
    writer.append_pending(&pending(10)).unwrap();

    let asked = writer.alone().map(|alone| alone.is_some());
    assert!(
        matches!(asked, Err(JournalError::StillWriting)),
        "refused, not done: {asked:?}"
    );
    let mut reader = Journal::open(&path).unwrap();
    assert!(
        reader.alone().unwrap().is_none(),
        "the writer's lock was kept"
    );
}

/// 58r: `open` cuts a torn last line only with the journal alone. While a
/// writer holds it, that line may be the writer's own append, not finished
/// yet — cutting it would lose the writer's `Pending` line, and its outcome
/// would later have none. It is left as `reread` leaves it, and not reported
/// as cut; once the writers have gone, the next `open` cuts it.
#[test]
fn open_leaves_a_torn_tail_alone_while_a_writer_holds_the_journal() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undo-journal.jsonl");
    let mut writer = Journal::open(&path).unwrap();
    writer.append_pending(&pending(10)).unwrap();
    let mut unfinished = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut unfinished, br#"{"id":"01","stat"#).unwrap();
    drop(unfinished);
    let text = std::fs::read_to_string(&path).unwrap();

    let reader = Journal::open(&path).unwrap();
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        text,
        "a writer holds the journal: its unfinished line is not cut"
    );
    assert_eq!(reader.torn_tail(), None, "nothing was cut");
    assert_eq!(reader.entries().len(), 1, "the finished line reads");

    drop(writer);
    let alone = Journal::open(&path).unwrap();
    assert_eq!(
        alone.torn_tail(),
        Some(2),
        "no writer left: a crash's wreckage"
    );
    assert!(
        std::fs::read(&path).unwrap().ends_with(b"\n"),
        "cut back to the last whole line"
    );
}

/// The wreckage a busy `open` left is cut by recovery, which holds the
/// journal alone: closing the dead entry must not splice its outcome onto
/// the torn bytes, or every later read refuses the file (58r review).
#[test]
fn recovery_alone_cuts_the_torn_tail_a_busy_open_left() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("undo-journal.jsonl");
    let mut dead = Journal::open(&path).unwrap();
    dead.append_pending(&pending(10)).unwrap();
    let mut torn = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .unwrap();
    std::io::Write::write_all(&mut torn, br#"{"id":"01","stat"#).unwrap();
    drop(torn);
    let mut recovery = Journal::open(&path).unwrap();
    drop(dead);

    let _alone = recovery.alone().unwrap().expect("no writer left");
    recovery.mark_interrupted(&id(10), None).unwrap();

    let reopened = Journal::open(&path).expect("the outcome is a line of its own");
    assert!(reopened.entries()[0].is_interrupted());
}
