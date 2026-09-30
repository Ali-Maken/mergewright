use std::sync::Arc;

use graft_core::journal::{EntryId, HeadRef, IndexState, OperationKind, Touches};

use super::*;
use crate::jobs::undo::JournalRead;

/// A distinct, valid oid: the journal only ever compares them.
fn oid(byte: u8) -> graft_core::oid::Oid {
    graft_core::oid::Oid::from_bytes(&[byte; 20]).expect("20 bytes is a sha-1 oid")
}

fn entry(id: i64, kind: OperationKind, description: &str) -> JournalEntry {
    JournalEntry {
        id: EntryId::new(id, [0; 10]),
        at_unix_ms: id,
        kind,
        description: description.to_owned(),
        status: EntryStatus::Committed,
        touches: Touches {
            refs: Vec::new(),
            index: true,
            worktree: false,
        },
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        head_after: Some(HeadRef::Branch("refs/heads/main".to_owned())),
        refs_before: std::collections::BTreeMap::new(),
        refs_after: std::collections::BTreeMap::new(),
        index_before: Some(IndexState::Merged(oid(1))),
        index_after: Some(IndexState::Merged(oid(2))),
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: vec!["git add -- src/a.rs".to_owned()],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
    }
}

fn state_with(entries: Vec<JournalEntry>) -> AppState {
    let mut state = AppState::default();
    state.journal.entries = Loading::Ready(Arc::new(JournalRead {
        entries,
        unreadable: Vec::new(),
        live: Vec::new(),
        interrupted_edits: Vec::new(),
    }));
    crate::update::journal::rebuild(&mut state);
    state
}

#[test]
fn rows_are_newest_first_and_the_next_undo_is_marked() {
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        entry(2, OperationKind::Commit, "commit \"x\""),
    ]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows.len(), 2);
    assert_eq!(view.rows[0].description.as_str(), "commit \"x\"");
    assert_eq!(view.rows[0].mark, RowMark::NextUndo);
    assert_eq!(view.rows[1].mark, RowMark::None);
}

#[test]
fn an_undo_entry_is_the_redo_target_and_what_it_reversed_reads_undone() {
    let mut undo = entry(3, OperationKind::Undo, "undo: commit \"x\"");
    undo.reverses = Some(EntryId::new(2, [0; 10]));
    let state = state_with(vec![
        entry(1, OperationKind::Stage, "stage src/a.rs"),
        entry(2, OperationKind::Commit, "commit \"x\""),
        undo,
    ]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::NextRedo, "the undo itself");
    assert_eq!(view.rows[1].mark, RowMark::Undone, "what it took back");
    assert_eq!(view.rows[2].mark, RowMark::NextUndo, "what `u` reaches now");
}

#[test]
fn an_interrupted_entry_says_so_in_the_row_and_in_full_when_expanded() {
    let mut interrupted = entry(2, OperationKind::Commit, "commit \"x\"");
    interrupted.status = EntryStatus::Pending;
    let state = state_with(vec![interrupted]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::Interrupted);
    assert!(view.rows[0].summary.as_str().starts_with("interrupted"));
    assert!(
        view.rows[0]
            .failure
            .as_ref()
            .is_some_and(|line| line.as_str().starts_with("interrupted:")),
    );
}

/// `J` after a stash clash (m6-conflicts/72, 72b): the pop that clashed
/// is `Committed` — git kept the stash and the tree is unmerged, which
/// is the pop's own result, not a failure — so its row reads as any
/// other, never `failed:`. The drop the last file done launched is a
/// `StashDrop` row named by its position, as every stash is, and it is what `u`
/// reaches next.
#[test]
fn a_clashed_pop_reads_as_a_normal_row_and_its_drop_as_a_stash_drop() {
    let mut pop = entry(1, OperationKind::StashPop, "pop stash@{0}");
    pop.touches.worktree = true;
    pop.put_back = Some(graft_core::journal::PutBack {
        stash: oid(7),
        why: graft_core::journal::PutBackWhy::Pop,
    });
    let stage = entry(2, OperationKind::Stage, "stage notes.md");
    let mut drop = entry(3, OperationKind::StashDrop, "drop stash@{0}");
    drop.touches = Touches {
        refs: vec!["refs/stash".to_owned()],
        index: false,
        worktree: false,
    };
    let app = state_with(vec![pop, stage, drop]);
    let view = journal_view(&app, 10);

    let drop_row = &view.rows[0];
    assert_eq!(drop_row.description.as_str(), "drop stash@{0}");
    assert_eq!(
        drop_row.mark,
        RowMark::NextUndo,
        "`u` brings the stash back"
    );
    assert!(drop_row.failure.is_none());

    let pop_row = &view.rows[2];
    assert_eq!(pop_row.description.as_str(), "pop stash@{0}");
    assert_eq!(pop_row.mark, RowMark::None);
    assert_eq!(
        pop_row.summary.as_str(),
        "the index · the working tree · backup saved"
    );
    assert!(pop_row.failure.is_none(), "a clash is not a failure");
}

/// A remote entry: `refs` touched, no index, no tree, no backup.
fn remote_entry(n: i64, kind: OperationKind, description: &str, refs: Vec<String>) -> JournalEntry {
    let mut entry = entry(n, kind, description);
    entry.touches = Touches {
        refs,
        index: false,
        worktree: false,
    };
    entry.backup_ref = None;
    entry
}

/// Push, Fetch and Pull rows in the design note's words (New UI,
/// "Journal browser rows"). `← u` falls on the pull — the newest
/// undoable entry — never on the push or fetch newer than it; that mark
/// is `graft_ops::journal::next_undo`'s, asserted here, not restated.
#[test]
fn remote_rows() {
    let tracking = "refs/remotes/origin/main".to_owned();
    let mut pull = entry(1, OperationKind::Pull, "pull origin/main");
    pull.touches = Touches {
        refs: vec!["refs/heads/main".to_owned()],
        index: true,
        worktree: true,
    };
    let mut push = remote_entry(
        2,
        OperationKind::Push,
        "push main → origin/main",
        vec![tracking.clone()],
    );
    let a1 = graft_core::oid::Oid::from_hex("a1b2c3d4e5f60718293a4b5c6d7e8f9012345678")
        .expect("40 hex digits");
    let nine = graft_core::oid::Oid::from_hex("9f8e7d6c5b4a39281706f5e4d3c2b1a098765432")
        .expect("40 hex digits");
    push.refs_before.insert(tracking.clone(), Some(a1));
    push.refs_after.insert(tracking, Some(nine));
    let unrecorded = remote_entry(
        3,
        OperationKind::Push,
        "push topic → origin/topic (set upstream)",
        Vec::new(),
    );
    let fetch = remote_entry(4, OperationKind::Fetch, "fetch all remotes", Vec::new());
    let state = state_with(vec![pull, push, unrecorded, fetch]);
    let view = journal_view(&state, 10);
    let rows: Vec<(&str, &str, RowMark)> = view
        .rows
        .iter()
        .map(|row| (row.description.as_str(), row.summary.as_str(), row.mark))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "fetch all remotes",
                "remote-tracking refs · not undoable",
                RowMark::None
            ),
            (
                "push topic → origin/topic (set upstream)",
                "not undoable — origin has it",
                RowMark::None
            ),
            (
                "push main → origin/main",
                "not undoable — origin has it · a1b2c3d → 9f8e7d6",
                RowMark::None
            ),
            (
                "pull origin/main",
                "1 ref · the index · the working tree · backup saved",
                RowMark::NextUndo
            ),
        ]
    );
}

/// What recovery actually leaves behind, and therefore the only shape
/// the browser ever really sees: `Aborted { reason: "interrupted" }`.
/// Keyed on `Pending` alone this row was unreachable and read
/// `failed: interrupted` (m3/06 review, Major 2).
#[test]
fn an_entry_recovery_closed_is_marked_interrupted_not_failed() {
    let mut recovered = entry(2, OperationKind::Commit, "commit \"x\"");
    // No before-tree: the process died before one was taken, so `u`
    // cannot reach it and the row is not the `← u` target. That is the
    // shape the warning mark exists for.
    recovered.worktree_before = None;
    recovered.status = EntryStatus::Aborted {
        reason: graft_core::journal::INTERRUPTED.to_owned(),
    };
    let state = state_with(vec![recovered]);
    let view = journal_view(&state, 10);
    assert_eq!(view.rows[0].mark, RowMark::Interrupted);
    let summary = view.rows[0].summary.as_str();
    assert!(summary.starts_with("interrupted"), "{summary}");
    assert!(
        !summary.contains("failed"),
        "a process that died: {summary}"
    );
    assert!(
        view.rows[0]
            .failure
            .as_ref()
            .is_some_and(|line| line.as_str().starts_with("interrupted:")),
    );
}

#[test]
fn an_aborted_entry_leads_with_gits_first_line_and_keeps_the_rest_for_space() {
    let mut aborted = entry(2, OperationKind::Commit, "commit \"x\"");
    // `in_force` counts an Aborted entry only with a backup of the tree
    // it changed; without one there is nothing for `u` to restore.
    aborted.worktree_before = Some(oid(3));
    aborted.status = EntryStatus::Aborted {
        reason: "hook rejected the commit\nsee .git/hooks/pre-commit".to_owned(),
    };
    let state = state_with(vec![aborted]);
    let view = journal_view(&state, 10);
    let summary = view.rows[0].summary.as_str();
    assert!(
        summary.starts_with("failed: hook rejected the commit"),
        "{summary}"
    );
    assert!(!summary.contains("pre-commit"), "{summary}");
    let failure = view.rows[0]
        .failure
        .as_ref()
        .expect("an aborted entry has a reason");
    assert!(failure.as_str().contains("pre-commit"), "{failure:?}");
    // It kept its backup and changed the index, so it is still what `u`
    // reaches: a write that failed part-way is undoable, and the row
    // must say so rather than look inert.
    assert!(view.rows[0].summary.as_str().ends_with("backup saved"));
    assert_eq!(view.rows[0].mark, RowMark::NextUndo);
}

#[test]
fn an_empty_journal_is_empty_not_ready_and_a_failed_read_carries_its_reason() {
    let state = state_with(Vec::new());
    assert_eq!(journal_view(&state, 10).status, JournalStatus::Empty);

    let mut state = AppState::default();
    state.journal.entries = Loading::Failed("line 3: torn".to_owned());
    match journal_view(&state, 10).status {
        JournalStatus::Failed(why) => assert_eq!(why.as_str(), "line 3: torn"),
        other => panic!("a failed read is not {other:?}"),
    }

    let mut state = AppState::default();
    state.journal.entries = Loading::Idle;
    assert_eq!(journal_view(&state, 10).status, JournalStatus::Loading);
}
/// M6/68: an entry the user aborted (`--abort`, `Aborted("aborted")`)
/// is not a failure: the row says who stopped it and what it left.
#[test]
fn an_aborted_entry_reads_aborted_by_you() {
    let mut aborted = entry(2, OperationKind::Merge, "merge feature");
    aborted.status = EntryStatus::Aborted {
        reason: "aborted".to_owned(),
    };
    let state = state_with(vec![aborted]);
    let view = journal_view(&state, 10);
    let summary = view.rows[0].summary.as_str();
    assert_eq!(summary, "aborted by you · nothing changed");
    assert!(!summary.contains("failed"), "{summary}");
}
