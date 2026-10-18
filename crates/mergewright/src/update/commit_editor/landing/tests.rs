//! `undo_landed`'s rule (08s m11): a commit's words come back only when an
//! undo moved a ref off it.

use std::collections::BTreeMap;

use mergewright_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches};

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; 20]).expect("20 bytes")
}

/// An undo entry with `main` at `before`, then `after`.
fn undo(before: Option<Oid>, after: Option<Oid>) -> JournalEntry {
    let main = |at| BTreeMap::from([("refs/heads/main".to_owned(), at)]);
    JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 1,
        kind: OperationKind::Undo,
        description: "undo".to_owned(),
        status: EntryStatus::Committed,
        touches: Touches::default(),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: main(before),
        refs_after: main(after),
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
        config_before: BTreeMap::new(),
        config_after: BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    }
}

fn with_last(commit: Oid) -> AppState {
    AppState {
        last_commit: Some(Box::new(LastCommit {
            commit,
            message: Draft {
                subject: "fix toc".to_owned(),
                ..Draft::default()
            },
        })),
        ..AppState::default()
    }
}

#[test]
fn undo_landed_needs_the_ref_moved_off_the_commit() {
    // A stage taken back: `main` on the commit before and after.
    let mut stayed = with_last(oid(2));
    undo_landed(&mut stayed, &undo(Some(oid(2)), Some(oid(2))));
    assert!(
        stayed.last_commit.is_some(),
        "still the last thing that happened"
    );
    assert_eq!(stayed.drafts.commit, None, "nothing given back");

    // The commit taken back: `main` moved off it.
    let mut moved = with_last(oid(2));
    undo_landed(&mut moved, &undo(Some(oid(2)), Some(oid(1))));
    assert!(moved.last_commit.is_none());
    assert_eq!(
        moved
            .drafts
            .commit
            .as_ref()
            .map(|draft| draft.subject.as_str()),
        Some("fix toc")
    );

    // A branch deleted off it counts too.
    let mut deleted = with_last(oid(2));
    undo_landed(&mut deleted, &undo(Some(oid(2)), None));
    assert!(deleted.drafts.commit.is_some());
}
