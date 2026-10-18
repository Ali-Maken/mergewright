//! `update` tests: sweep entries. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

/// One `git status` entry for `path`, modified in the tree, conflicted when
/// `unmerged`.
pub(super) fn status_entry(path: &str, unmerged: bool) -> mergewright_git::status::StatusEntry {
    mergewright_git::status::StatusEntry {
        path: path.into(),
        original_path: None,
        staged: None,
        unstaged: Some(mergewright_git::status::Change::Modified),
        untracked: false,
        unmerged,
        unmerged_how: None,
        submodule: None,
    }
}

/// A branch badge picked up from row 1 and held over row 0, which is where
/// the gesture the bar is hinting actually is.
pub(super) fn badge_in_the_air(walk: JobId) -> AppState {
    let (mut state, _) = merge_on_screen(walk, 1);
    let at = state
        .history
        .as_ref()
        .and_then(History::selected)
        .expect("the cursor's row has arrived")
        .commit
        .id;
    state.drag = Some(crate::update::drag::DragState {
        source: crate::update::drag::DragSource {
            name: "refs/heads/feature".to_owned(),
            short: "feature".to_owned(),
            at,
            current: false,
        },
        from_row: 1,
        over: Some(0),
    });
    state
}

/// One journal entry, so `⏎ revert` has something to undo.
pub(super) fn journal_entry() -> mergewright_core::journal::JournalEntry {
    use mergewright_core::journal::{
        EntryId, EntryStatus, HeadRef, JournalEntry, OperationKind, Touches,
    };

    JournalEntry {
        id: EntryId::new(1, [0; 10]),
        at_unix_ms: 1,
        kind: OperationKind::Commit,
        description: "commit \"x\"".to_owned(),
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
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        worktree_raw_before: None,
        backup_ref: Some("refs/graft/backup/main/x".to_owned()),
        pins: Vec::new(),
        commands: vec!["git commit".to_owned()],
        reverses: None,
        merge_before: None,
        merge_after: None,
        config_before: std::collections::BTreeMap::new(),
        config_after: std::collections::BTreeMap::new(),
        put_back: None,
        fetched_refs: None,
    }
}

/// One stash, so the overlay's keys have an entry under the cursor.
pub(super) fn stash_entry() -> mergewright_git::stash::StashEntry {
    mergewright_git::stash::StashEntry {
        index: 0,
        commit: Oid::from_bytes(&[9; Oid::SHA1_BYTES]).expect("20 bytes is an oid"),
        message: "WIP on main".to_owned(),
        at: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
        paths: vec![PathBuf::from("a.rs")],
    }
}
