//! The undo journal browser's view model (`J`,
//! m3-working-tree-undo-and-stash/06).
//!
//! Pure: [`JournalState`](crate::update::journal::JournalState) plus the
//! journal as it was last read, turned into rows. Which entry `u` would take
//! back is `graft_ops`' answer, never one worked out here — the row marks
//! come from `next_undo`/`next_redo`/`in_force_all` so the browser and the
//! key can never disagree.

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind};
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
            // Live first, and instead of every other mark: git is part-way
            // through this very entry. `interrupted` would be a lie about a
            // process that is alive, and `← u` would offer to undo an
            // operation that has not finished happening (m5/09 seam A).
            let is_live = state.live_entries.contains(&entry.id);
            let mark = if is_live {
                RowMark::None
            } else if Some(&entry.id) == next_undo {
                RowMark::NextUndo
            } else if Some(&entry.id) == next_redo {
                RowMark::NextRedo
            } else if entry.is_interrupted() {
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
                summary: sanitize(&summary(entry, is_live)),
                mark,
                commands: entry.commands.iter().map(|line| sanitize(line)).collect(),
                failure: failure(entry, is_live).map(|line| sanitize(&line)),
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
    // "future" and "now" are already whole answers; "now ago" is not English,
    // and the journal browser opened straight after a write is full of
    // entries less than a minute old.
    if relative == "future" || relative == "now" {
        relative
    } else {
        format!("{relative} ago")
    }
}

/// What a live entry's row says instead of a verdict: the operation has not
/// finished happening, and these are the two keys that end it either way.
const IN_PROGRESS: &str = "in progress \u{2014} c continue  a abort";

/// What the entry moved, and whether a backup was kept — the two facts that
/// decide whether `u` can put it back.
///
/// `live` is git standing part-way through this very entry
/// ([`crate::update::AppState::live_entries`]); it outranks every verdict,
/// because there is not one yet.
fn summary(entry: &JournalEntry, live: bool) -> String {
    let mut parts: Vec<String> = Vec::new();
    // A push or a fetch is journaled but `u` passes over it (design note,
    // choice 1): the row says what moved and that it stays moved, instead of
    // the ref count and backup that mean "undoable" everywhere else. The
    // verdict comes first and the tracking ref is not repeated (the
    // description names it): at 80 columns the summary has some 40 cells,
    // and it is the verdict that must survive (m4 acceptance review M2).
    match entry.kind {
        OperationKind::Push => {
            let tracking = crate::update::remote::pushed_to(entry);
            if let Some(name) = entry.touches.refs.first()
                && let Some(Some(after)) = entry.refs_after.get(name)
            {
                let before = entry
                    .refs_before
                    .get(name)
                    .copied()
                    .flatten()
                    .map_or_else(|| "new".to_owned(), short);
                parts.push(format!("{before} → {}", short(*after)));
            }
            let remote = tracking.split('/').next().unwrap_or("the remote");
            parts.insert(0, format!("not undoable \u{2014} {remote} has it"));
            return with_failure(entry, live, &parts);
        }
        OperationKind::Fetch => {
            parts.push("remote-tracking refs".to_owned());
            parts.push("not undoable".to_owned());
            return with_failure(entry, live, &parts);
        }
        _ => {}
    }
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
    // Interrupted first, and *instead of* `failed:` — recovery closes a torn
    // entry as `Aborted { reason: "interrupted" }`, so the two arms overlap
    // and "failed: interrupted" is the wrong word for a process that died
    // (m3/06 review, Major 2).
    if live {
        parts.insert(0, IN_PROGRESS.to_owned());
    } else if entry.is_interrupted() {
        parts.insert(0, "interrupted".to_owned());
    } else if let EntryStatus::Aborted { reason } = &entry.status {
        // The row carries the failure in short form; the whole reason is
        // under `Space`, which is where a multi-line stderr belongs.
        parts.insert(0, format!("failed: {}", first_line(reason)));
    }
    if entry.backup_ref.is_some() {
        parts.push("backup saved".to_owned());
    }
    parts.join(" · ")
}

/// A push or fetch row that failed says so first, and nothing else: an
/// aborted push sent nothing, so "origin has it" would be false.
fn with_failure(entry: &JournalEntry, live: bool, parts: &[String]) -> String {
    if live {
        return IN_PROGRESS.to_owned();
    }
    if entry.is_interrupted() {
        return "interrupted".to_owned();
    }
    match &entry.status {
        EntryStatus::Aborted { reason } => format!("failed: {}", first_line(reason)),
        EntryStatus::Pending | EntryStatus::Committed => parts.join(" · "),
    }
}

/// Seven hex digits, as every row writes a commit.
fn short(oid: graft_core::oid::Oid) -> String {
    let mut hex = oid.to_string();
    hex.truncate(7);
    hex
}

/// The whole reason an `Aborted` entry gives, for the expanded row.
///
/// A live entry has none: nothing failed, and the operation has not finished
/// happening. Saying "a previous run stopped between this entry's two
/// journal lines" about a merge running right now is the very lie this
/// ticket exists to stop.
fn failure(entry: &JournalEntry, live: bool) -> Option<String> {
    if live {
        return None;
    }
    if entry.is_interrupted() {
        return Some(
            "interrupted: a previous run stopped between this entry's two journal lines".to_owned(),
        );
    }
    match &entry.status {
        EntryStatus::Aborted { reason } => Some(format!("failed: {reason}")),
        // Unreachable: `is_interrupted` has taken every `Pending` entry
        // above. Kept as the honest arm of the match rather than a
        // catch-all that would silently call a torn entry a clean one.
        EntryStatus::Pending | EntryStatus::Committed => None,
    }
}

/// git's first stderr line, which is the one that says what went wrong.
fn first_line(reason: &str) -> &str {
    reason.lines().next().unwrap_or(reason)
}

#[cfg(test)]
mod tests {
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
            backup_ref: Some("refs/graft/backup/main/x".to_owned()),
            pins: Vec::new(),
            commands: vec!["git add -- src/a.rs".to_owned()],
            reverses: None,
            merge_before: None,
            merge_after: None,
            put_back: None,
        }
    }

    fn state_with(entries: Vec<JournalEntry>) -> AppState {
        let mut state = AppState::default();
        state.journal.entries = Loading::Ready(Arc::new(JournalRead {
            entries,
            unreadable: Vec::new(),
            live: Vec::new(),
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
    fn remote_entry(
        n: i64,
        kind: OperationKind,
        description: &str,
        refs: Vec<String>,
    ) -> JournalEntry {
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
}
