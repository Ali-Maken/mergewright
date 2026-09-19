//! The undo journal browser's view model (`J`,
//! m3-working-tree-undo-and-stash/06).
//!
//! Pure: [`JournalState`](crate::update::journal::JournalState) plus the
//! journal as it was last read, turned into rows. Which entry `u` would take
//! back is `graft_ops`' answer, never one worked out here — the row marks
//! come from `next_undo`/`next_redo`/`in_force_all` so the browser and the
//! key can never disagree.

use graft_core::journal::{EntryStatus, JournalEntry};
use graft_term::sanitize::sanitize;
use graft_ui::graph_screen::columns::relative_date;
use graft_ui::journal_screen::{JournalRowView, JournalStatus, JournalView, RowMark};

use crate::update::AppState;
use crate::update::detail::Loading;

/// Milliseconds in a second, for the entries' `at_unix_ms`.
const MILLIS: i64 = 1_000;

/// [`JournalView`] for whatever the journal read has brought back.
#[must_use]
pub fn journal_view(state: &AppState, now_seconds: i64) -> JournalView {
    let location = sanitize(&state.repo_path.display().to_string());
    let (rows, status) = match &state.journal.entries {
        Loading::Idle | Loading::Pending(_) => (Vec::new(), JournalStatus::Loading),
        Loading::Failed(why) => (Vec::new(), JournalStatus::Failed(sanitize(why))),
        Loading::Ready(read) if read.entries.is_empty() => (Vec::new(), JournalStatus::Empty),
        Loading::Ready(read) => (
            rows_for(&read.entries, state, now_seconds),
            JournalStatus::Ready,
        ),
    };
    let selected = state.journal.selected.min(rows_end(&rows));
    JournalView {
        location,
        rows,
        selected,
        status,
    }
}

/// The last row's index, or 0 for an empty list — the cursor has nowhere
/// else to be.
fn rows_end(rows: &[JournalRowView]) -> usize {
    rows.len().saturating_sub(1)
}

/// Newest first, which is the order the browser lists them in and the order
/// `JournalState::selected` indexes.
fn rows_for(entries: &[JournalEntry], state: &AppState, now_seconds: i64) -> Vec<JournalRowView> {
    let next_undo = graft_ops::journal::next_undo(entries).map(|entry| &entry.id);
    let next_redo = graft_ops::journal::next_redo(entries).map(|entry| &entry.id);
    let in_force = graft_ops::journal::in_force_all(entries);
    entries
        .iter()
        .enumerate()
        .rev()
        .map(|(index, entry)| {
            let mark = if Some(&entry.id) == next_undo {
                RowMark::NextUndo
            } else if Some(&entry.id) == next_redo {
                RowMark::NextRedo
            } else if entry.status == EntryStatus::Pending {
                RowMark::Interrupted
            } else if in_force.get(index) == Some(&false) {
                RowMark::Undone
            } else {
                RowMark::None
            };
            let newest_first = entries.len() - 1 - index;
            JournalRowView {
                age: sanitize(&age(entry.at_unix_ms, now_seconds)),
                description: sanitize(&entry.description),
                summary: sanitize(&summary(entry)),
                mark,
                commands: entry.commands.iter().map(|line| sanitize(line)).collect(),
                failure: failure(entry).map(|line| sanitize(&line)),
                expanded: state.journal.expanded == Some(newest_first),
            }
        })
        .collect()
}

/// `2m ago`, from the same cascade the graph's date column uses, so two
/// views of the same moment never disagree.
fn age(at_unix_ms: i64, now_seconds: i64) -> String {
    let then = at_unix_ms.div_euclid(MILLIS);
    let relative = relative_date(then, now_seconds);
    if relative == "future" {
        relative
    } else {
        format!("{relative} ago")
    }
}

/// What the entry moved, and whether a backup was kept — the two facts that
/// decide whether `u` can put it back.
fn summary(entry: &JournalEntry) -> String {
    let mut parts: Vec<String> = Vec::new();
    match entry.touches.refs.len() {
        0 => {}
        1 => parts.push("1 ref".to_owned()),
        many => parts.push(format!("{many} refs")),
    }
    if entry.touches.index {
        parts.push("the index".to_owned());
    }
    if entry.touches.worktree {
        parts.push("the working tree".to_owned());
    }
    if let EntryStatus::Aborted { reason } = &entry.status {
        // The row carries the failure in short form; the whole reason is
        // under `Space`, which is where a multi-line stderr belongs.
        parts.insert(0, format!("failed: {}", first_line(reason)));
    }
    if entry.status == EntryStatus::Pending {
        parts.insert(0, "interrupted".to_owned());
    }
    if entry.backup_ref.is_some() {
        parts.push("backup saved".to_owned());
    }
    parts.join(" · ")
}

/// The whole reason an `Aborted` entry gives, for the expanded row.
fn failure(entry: &JournalEntry) -> Option<String> {
    match &entry.status {
        EntryStatus::Aborted { reason } => Some(format!("failed: {reason}")),
        EntryStatus::Pending => Some(
            "interrupted: a previous run stopped between this entry's two journal lines".to_owned(),
        ),
        EntryStatus::Committed => None,
    }
}

/// git's first stderr line, which is the one that says what went wrong.
fn first_line(reason: &str) -> &str {
    reason.lines().next().unwrap_or(reason)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use graft_core::journal::{EntryId, HeadRef, OperationKind, Touches};

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
            index_before: Some(oid(1)),
            index_after: Some(oid(2)),
            worktree_before: None,
            worktree_after: None,
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: vec!["git add -- src/a.rs".to_owned()],
            reverses: None,
        }
    }

    fn state_with(entries: Vec<JournalEntry>) -> AppState {
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries,
            interrupted: Vec::new(),
            unreadable: Vec::new(),
        }));
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
}
