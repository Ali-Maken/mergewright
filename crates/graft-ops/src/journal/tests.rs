//! Round trip, cursor and point-in-time tests. The entry builders are shared
//! with `undo`'s tests.

use std::collections::BTreeMap;

use graft_core::journal::{
    EntryId, EntryStatus, HeadRef, IndexState, JournalEntry, OperationKind, Touches,
};
use graft_core::oid::Oid;

use super::{
    JournalError, PointInTime, in_force, in_force_all, next_redo, next_undo, parse, render_outcome,
    render_pending, root_operation, state_at,
};

pub(crate) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).unwrap()
}

pub(crate) fn id(n: i64) -> EntryId {
    EntryId::new(n, [u8::try_from(n % 251).unwrap(); 10])
}

pub(crate) const MAIN: &str = "refs/heads/main";

/// A `Committed` entry opened at `at` that moved `main` from `before` to
/// `after`, with index and worktree trees recorded on both sides.
pub(crate) fn committed(at: i64, kind: OperationKind, before: u8, after: u8) -> JournalEntry {
    JournalEntry {
        id: id(at),
        at_unix_ms: at,
        kind,
        description: format!("{kind:?} at {at}"),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: vec![MAIN.to_owned()],
            index: true,
            worktree: true,
        },
        head_before: HeadRef::Branch(MAIN.to_owned()),
        head_after: Some(HeadRef::Branch(MAIN.to_owned())),
        refs_before: BTreeMap::from([(MAIN.to_owned(), Some(oid(before)))]),
        refs_after: BTreeMap::from([(MAIN.to_owned(), Some(oid(after)))]),
        index_before: Some(IndexState::Merged(oid(0xa0 + before))),
        index_after: Some(IndexState::Merged(oid(0xa0 + after))),
        worktree_before: Some(oid(0xb0 + before)),
        worktree_after: Some(oid(0xb0 + after)),
        backup_ref: Some(format!("refs/graft/backup/main/{}", id(at))),
        pins: vec![],
        commands: vec![format!("git {kind:?}")],
        reverses: None,
        merge_before: None,
        merge_after: None,
    }
}

/// An `Undo` (or `Redo`) entry at `at` reversing `target`, whose own
/// before/after are the target's after/before.
pub(crate) fn reversal(at: i64, kind: OperationKind, target: &JournalEntry) -> JournalEntry {
    JournalEntry {
        id: id(at),
        at_unix_ms: at,
        kind,
        description: format!("{kind:?} of {}", target.description),
        status: EntryStatus::Committed,
        touches: target.touches.clone(),
        head_before: target.head_after.clone().unwrap(),
        head_after: Some(target.head_before.clone()),
        refs_before: target.refs_after.clone(),
        refs_after: target.refs_before.clone(),
        index_before: target.index_after,
        index_after: target.index_before,
        worktree_before: target.worktree_after,
        worktree_after: target.worktree_before,
        backup_ref: Some(format!("refs/graft/backup/main/{}", id(at))),
        pins: vec![],
        commands: vec![],
        reverses: Some(target.id.clone()),
        merge_before: target.merge_after.clone(),
        merge_after: target.merge_before.clone(),
    }
}

/// An `Aborted` stash pop that got half way: the working tree moved before
/// git failed (a conflict), and the ref and index did not.
fn aborted(at: i64, with_backup: bool) -> JournalEntry {
    JournalEntry {
        status: EntryStatus::Aborted {
            reason: "git exited 1".to_owned(),
        },
        worktree_before: with_backup.then(|| oid(0xb1)),
        worktree_after: Some(oid(0xb2)),
        index_before: with_backup.then(|| IndexState::Merged(oid(0xa1))),
        ..committed(at, OperationKind::StashPop, 1, 1)
    }
}

/// An `Aborted` entry with a backup whose recorded after-state equals its
/// before-state: git failed having changed nothing.
fn aborted_no_op(at: i64, kind: OperationKind) -> JournalEntry {
    JournalEntry {
        status: EntryStatus::Aborted {
            reason: "hook rejected it".to_owned(),
        },
        ..committed(at, kind, 1, 1)
    }
}

fn pending(at: i64) -> JournalEntry {
    JournalEntry {
        status: EntryStatus::Pending,
        head_after: None,
        refs_after: BTreeMap::new(),
        index_after: None,
        worktree_after: None,
        commands: vec![],
        ..committed(at, OperationKind::Commit, 1, 2)
    }
}

fn file(entries: &[JournalEntry]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for entry in entries {
        bytes.extend(render_pending(entry).unwrap());
        if entry.status != EntryStatus::Pending {
            bytes.extend(render_outcome(entry).unwrap());
        }
    }
    bytes
}

// ---- lines ----------------------------------------------------------------

#[test]
fn three_entries_round_trip_through_two_lines_each() {
    let entries = vec![
        committed(10, OperationKind::Commit, 1, 2),
        aborted(20, true),
        pending(30),
    ];
    let bytes = file(&entries);
    assert_eq!(
        bytes.split(|&b| b == b'\n').count(),
        6,
        "five lines and a trailing newline"
    );
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries, entries);
    assert_eq!(parsed.torn_tail, None);
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("\"status\":\"pending\""), "{text}");
    assert!(text.contains("\"kind\":\"stash_pop\""), "{text}");
    assert!(text.contains(&format!("\"{}\"", oid(1))), "{text}");
}

#[test]
fn a_torn_last_line_is_reported_and_the_rest_parsed() {
    let entries = vec![committed(10, OperationKind::Stage, 1, 2), pending(20)];
    let mut bytes = file(&entries);
    bytes.extend(&render_pending(&committed(30, OperationKind::Stage, 2, 3)).unwrap()[..40]);
    let parsed = parse(&bytes).unwrap();
    assert_eq!(parsed.entries, entries);
    assert_eq!(parsed.torn_tail, Some(4));
}

#[test]
fn a_torn_line_that_is_not_last_is_an_error() {
    let mut bytes = render_pending(&committed(10, OperationKind::Stage, 1, 2)).unwrap();
    bytes.truncate(30);
    bytes.push(b'\n');
    bytes.extend(render_pending(&committed(20, OperationKind::Stage, 1, 2)).unwrap());
    assert!(matches!(
        parse(&bytes),
        Err(JournalError::TornTail { line: 1 })
    ));
}

#[test]
fn malformed_and_mismatched_lines_are_errors_with_line_numbers() {
    let good = committed(10, OperationKind::Stage, 1, 2);
    let mut bytes = render_pending(&good).unwrap();
    bytes.extend(b"{\"id\":\"nonsense\",\"status\":\"committed\"}\n");
    assert!(matches!(
        parse(&bytes),
        Err(JournalError::Json { line: 2, .. })
    ));

    let orphan = render_outcome(&committed(20, OperationKind::Stage, 1, 2)).unwrap();
    assert!(matches!(
        parse(&orphan),
        Err(JournalError::OutcomeWithoutPending(id)) if id == self::id(20)
    ));

    let mut twice = render_pending(&good).unwrap();
    twice.extend(render_pending(&good).unwrap());
    assert!(matches!(
        parse(&twice),
        Err(JournalError::DuplicatePending(_))
    ));

    let mut two_outcomes = file(std::slice::from_ref(&good));
    two_outcomes.extend(render_outcome(&good).unwrap());
    assert!(matches!(
        parse(&two_outcomes),
        Err(JournalError::DuplicateOutcome(_))
    ));

    assert!(matches!(
        render_outcome(&pending(1)),
        Err(JournalError::StillPending(_))
    ));
    assert_eq!(parse(b"").unwrap(), super::Parsed::default());
}

// ---- cursor ---------------------------------------------------------------

#[test]
fn cursor_after_an_undo() {
    let a = committed(10, OperationKind::Commit, 1, 2);
    let b = committed(20, OperationKind::Stage, 2, 3);
    let undo_b = reversal(30, OperationKind::Undo, &b);
    let entries = vec![a.clone(), b.clone(), undo_b.clone()];
    assert_eq!(next_undo(&entries), Some(&a));
    assert_eq!(next_redo(&entries), Some(&undo_b));
    assert!(in_force(&entries, &a.id));
    assert!(!in_force(&entries, &b.id));
    assert!(in_force(&entries, &undo_b.id));
}

#[test]
fn a_new_operation_after_an_undo_empties_redo() {
    let a = committed(10, OperationKind::Commit, 1, 2);
    let b = committed(20, OperationKind::Stage, 2, 3);
    let undo_b = reversal(30, OperationKind::Undo, &b);
    let c = committed(40, OperationKind::Discard, 2, 4);
    let entries = vec![a, b, undo_b, c.clone()];
    assert_eq!(next_undo(&entries), Some(&c));
    assert_eq!(next_redo(&entries), None);
}

#[test]
fn a_redo_puts_the_operation_back_in_force() {
    let a = committed(10, OperationKind::Commit, 1, 2);
    let b = committed(20, OperationKind::Stage, 2, 3);
    let undo_b = reversal(30, OperationKind::Undo, &b);
    let redo_b = reversal(40, OperationKind::Redo, &undo_b);
    let entries = vec![a, b.clone(), undo_b.clone(), redo_b.clone()];
    assert_eq!(next_undo(&entries), Some(&b));
    assert_eq!(next_redo(&entries), None);
    assert!(!in_force(&entries, &undo_b.id));
    assert_eq!(root_operation(&entries, &redo_b), Some(&b));
    assert_eq!(root_operation(&entries, &undo_b), Some(&b));
    assert_eq!(root_operation(&entries, &b), None);
}

#[test]
fn two_undos_redo_in_reverse_order() {
    let a = committed(10, OperationKind::Commit, 1, 2);
    let b = committed(20, OperationKind::Stage, 2, 3);
    let undo_b = reversal(30, OperationKind::Undo, &b);
    let undo_a = reversal(40, OperationKind::Undo, &a);
    let redo_a = reversal(50, OperationKind::Redo, &undo_a);
    let entries = vec![a.clone(), b, undo_b.clone(), undo_a.clone(), redo_a];
    assert_eq!(next_undo(&entries), Some(&a));
    assert_eq!(next_redo(&entries), Some(&undo_b));
}

#[test]
fn aborted_entries_are_undoable_only_with_a_backup() {
    let no_backup = vec![aborted(10, false)];
    assert_eq!(next_undo(&no_backup), None);
    let with_backup = vec![aborted(10, true)];
    assert_eq!(next_undo(&with_backup), Some(&with_backup[0]));
    let pending_only = vec![pending(10)];
    assert_eq!(next_undo(&pending_only), None);
    assert_eq!(next_redo(&pending_only), None);
}

#[test]
fn an_aborted_operation_that_changed_nothing_is_not_in_force() {
    // A commit whose hook rejected it: backed up, recorded, no different.
    let stage = committed(10, OperationKind::Stage, 1, 2);
    let rejected = aborted_no_op(20, OperationKind::Commit);
    let entries = vec![stage.clone(), rejected.clone()];
    assert!(!in_force(&entries, &rejected.id));
    assert_eq!(next_undo(&entries), Some(&stage));
}

#[test]
fn an_aborted_undo_that_moved_nothing_leaves_its_target_in_force() {
    // The undo's compare-and-swap refused: `main` had moved, nothing ran.
    let commit = committed(10, OperationKind::Commit, 1, 2);
    let mut undo = reversal(20, OperationKind::Undo, &commit);
    undo.status = EntryStatus::Aborted {
        reason: "update-ref: refs/heads/main is not at the expected value".to_owned(),
    };
    undo.head_after = Some(undo.head_before.clone());
    undo.refs_after = undo.refs_before.clone();
    undo.index_after = undo.index_before;
    undo.worktree_after = undo.worktree_before;
    let entries = vec![commit.clone(), undo.clone()];
    assert!(in_force(&entries, &commit.id));
    assert!(!in_force(&entries, &undo.id));
    assert_eq!(next_undo(&entries), Some(&commit));
    assert_eq!(next_redo(&entries), None);
}

#[test]
fn an_aborted_operation_is_in_force_when_any_recorded_field_moved_or_is_unknown() {
    let base = aborted_no_op(10, OperationKind::StashPop);
    let moved: Vec<(&str, JournalEntry)> = vec![
        ("ref", {
            let mut e = base.clone();
            e.refs_after.insert(MAIN.to_owned(), Some(oid(2)));
            e
        }),
        ("HEAD", {
            let mut e = base.clone();
            e.head_after = Some(HeadRef::Detached(oid(1)));
            e
        }),
        ("index", {
            let mut e = base.clone();
            e.index_after = Some(IndexState::Merged(oid(0xa2)));
            e
        }),
        ("worktree", {
            let mut e = base.clone();
            e.worktree_after = Some(oid(0xb2));
            e
        }),
        ("ref never recorded after", {
            let mut e = base.clone();
            e.refs_after.clear();
            e
        }),
        ("index never recorded after", {
            let mut e = base.clone();
            e.index_after = None;
            e
        }),
        ("worktree never recorded after", {
            let mut e = base.clone();
            e.worktree_after = None;
            e
        }),
    ];
    for (what, entry) in moved {
        let entries = vec![entry.clone()];
        assert!(in_force(&entries, &entry.id), "{what}");
        assert_eq!(next_undo(&entries), Some(&entries[0]), "{what}");
    }
    // A field the operation does not touch is not its doing.
    let mut untouched = base.clone();
    untouched.touches.index = false;
    untouched.index_after = None;
    assert!(!in_force(std::slice::from_ref(&untouched), &untouched.id));
}

// ---- state_at -------------------------------------------------------------

#[test]
fn state_at_reads_the_newest_after_state_or_the_oldest_before_state() {
    let other = "refs/heads/feature";
    let mut third = committed(30, OperationKind::Commit, 3, 4);
    third.refs_before.insert(other.to_owned(), Some(oid(0x50)));
    third.refs_after.insert(other.to_owned(), Some(oid(0x51)));
    let entries = vec![
        committed(10, OperationKind::Commit, 1, 2),
        committed(20, OperationKind::Commit, 2, 3),
        third,
    ];
    let at_25 = state_at(&entries, 25).unwrap();
    assert_eq!(
        at_25,
        PointInTime {
            head: Some(HeadRef::Branch(MAIN.to_owned())),
            refs: BTreeMap::from([
                (MAIN.to_owned(), Some(oid(3))),
                (other.to_owned(), Some(oid(0x50))),
            ]),
            index: Some(IndexState::Merged(oid(0xa3))),
            worktree: Some(oid(0xb3)),
        }
    );
    assert_eq!(state_at(&entries, 5), None);
    assert_eq!(state_at(&entries, 30).unwrap().refs[MAIN], Some(oid(4)));
    assert_eq!(state_at(&entries, 99).unwrap().refs[other], Some(oid(0x51)));
}

#[test]
fn state_at_skips_a_pending_entry_that_recorded_no_after_state() {
    let entries = vec![committed(10, OperationKind::Commit, 1, 2), pending(20)];
    assert_eq!(state_at(&entries, 25).unwrap().refs[MAIN], Some(oid(2)));
}

#[test]
fn fetch_and_push_are_not_undoable_kinds() {
    use super::undoable_kind;
    assert!(!undoable_kind(OperationKind::Fetch));
    assert!(!undoable_kind(OperationKind::Push));
    for kind in [
        OperationKind::Pull,
        OperationKind::Commit,
        OperationKind::StashPush,
        OperationKind::Undo,
        OperationKind::Redo,
    ] {
        assert!(undoable_kind(kind), "{kind:?}");
    }
}

#[test]
fn next_undo_steps_past_a_push_and_a_fetch() {
    let commit = committed(10, OperationKind::Commit, 1, 2);
    let push = committed(20, OperationKind::Push, 5, 6);
    let fetch = committed(30, OperationKind::Fetch, 6, 7);
    let entries = vec![commit.clone(), push, fetch];
    assert_eq!(next_undo(&entries), Some(&commit));
}

#[test]
fn a_pull_is_undone_like_any_operation() {
    let commit = committed(10, OperationKind::Commit, 1, 2);
    let pull = committed(20, OperationKind::Pull, 2, 3);
    let entries = vec![commit, pull.clone()];
    assert_eq!(next_undo(&entries), Some(&pull));
}

/// A `Committed` pull whose after equals its before on every field (git's
/// "Already up to date.") is passed over by `u` and does not end a redo
/// branch; the inverse refuses it (m4/04 review, M3).
#[test]
fn a_pull_that_moved_nothing_is_transparent() {
    let commit = committed(10, OperationKind::Commit, 1, 2);
    let still = committed(20, OperationKind::Pull, 2, 2);
    assert!(!super::undoable(&still));
    let entries = vec![commit.clone(), still.clone()];
    assert_eq!(next_undo(&entries), Some(&commit));
    assert_eq!(
        crate::undo::inverse(&still, None),
        Err(crate::undo::NotInvertible::NotUndoable(OperationKind::Pull))
    );

    let undo_commit = reversal(30, OperationKind::Undo, &commit);
    let after_undo = committed(40, OperationKind::Pull, 1, 1);
    let entries = vec![commit, undo_commit.clone(), after_undo];
    assert_eq!(next_redo(&entries), Some(&undo_commit));
}

#[test]
fn a_fetch_or_push_after_an_undo_keeps_redo() {
    let a = committed(10, OperationKind::Commit, 1, 2);
    let undo_a = reversal(20, OperationKind::Undo, &a);
    let fetch = committed(30, OperationKind::Fetch, 5, 6);
    let push = committed(40, OperationKind::Push, 6, 7);
    let entries = vec![a, undo_a.clone(), fetch, push];
    assert_eq!(next_redo(&entries), Some(&undo_a));
    assert_eq!(next_undo(&entries), None);
}

#[test]
fn pending_is_not_in_force() {
    // A merge stopped on a conflict: the entry stays `Pending` until
    // `resume` closes it, and a `Pending` entry is not in force whatever its
    // fields say — the operation is half done, so neither `u` nor the
    // browser's row mark may treat it as having taken effect (m5/07, M5
    // design note hard choice 2). The entry's before-fields are a commit's,
    // and its backup exists; only the missing outcome keeps it out of force.
    let done = committed(10, OperationKind::Stage, 1, 2);
    let stopped = JournalEntry {
        kind: OperationKind::Merge,
        ..pending(20)
    };
    let entries = vec![done.clone(), stopped.clone()];
    assert!(!in_force(&entries, &stopped.id));
    assert_eq!(in_force_all(&entries), vec![true, false]);
    // And it does not shadow the operation before it.
    assert_eq!(next_undo(&entries), Some(&done));
}
