//! The eight tasks and how each ticks: from the practice repo's journal
//! only, never from what was read on the card (ticket 19 §A1-A3). Only
//! `Committed` entries count; `Aborted` and `Pending` never tick.

use std::collections::BTreeSet;

use graft_core::journal::{EntryId, EntryStatus, JournalEntry, OperationKind};

use crate::journal::root_operation;

/// One task of the tour, in the order the card teaches them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TourTask {
    /// Any `Stage` entry (the card teaches one hunk; the journal records no
    /// hunk count, §A1).
    StageHunk,
    /// A `Commit` (not `Amend`).
    Commit,
    /// An `Undo` whose root operation is a `Commit`.
    Undo,
    /// A `StashPush`, then a later `StashApply` or `StashPop`.
    StashAndBack,
    /// A `BranchCreate`.
    Branch,
    /// Any `Merge`.
    MergeWithPreview,
    /// A committed `Merge` whose result git could not have made alone: its
    /// id is in `conflicted` (m8/19c R1). The resolver writes no journal
    /// entry of its own, so this reads the result, not the steps.
    ResolveConflict,
    /// A `Push`.
    Push,
}

impl TourTask {
    /// Every task, in card order; `tasks_done`'s flags are in this order.
    pub const ALL: [TourTask; 8] = [
        TourTask::StageHunk,
        TourTask::Commit,
        TourTask::Undo,
        TourTask::StashAndBack,
        TourTask::Branch,
        TourTask::MergeWithPreview,
        TourTask::ResolveConflict,
        TourTask::Push,
    ];
}

/// One flag per [`TourTask::ALL`]: done or not, read from `entries`. Pure.
/// `conflicted` names the `Merge` entries whose result git could not have
/// made alone; [`super::tour_progress`] works that out from the repo.
/// A task done early ticks early; order between tasks is not required.
#[must_use]
pub fn tasks_done(entries: &[JournalEntry], conflicted: &BTreeSet<EntryId>) -> [bool; 8] {
    TourTask::ALL.map(|task| is_done(task, entries, conflicted))
}

fn committed(entry: &JournalEntry) -> bool {
    entry.status == EntryStatus::Committed
}

fn any_committed(entries: &[JournalEntry], kind: OperationKind) -> bool {
    entries
        .iter()
        .any(|entry| entry.kind == kind && committed(entry))
}

fn is_done(task: TourTask, entries: &[JournalEntry], conflicted: &BTreeSet<EntryId>) -> bool {
    match task {
        TourTask::StageHunk => any_committed(entries, OperationKind::Stage),
        TourTask::Commit => any_committed(entries, OperationKind::Commit),
        TourTask::Undo => entries.iter().any(|entry| {
            entry.kind == OperationKind::Undo
                && committed(entry)
                && root_operation(entries, entry)
                    .is_some_and(|root| root.kind == OperationKind::Commit)
        }),
        TourTask::StashAndBack => entries.iter().any(|push| {
            push.kind == OperationKind::StashPush
                && committed(push)
                && entries.iter().any(|back| {
                    matches!(
                        back.kind,
                        OperationKind::StashApply | OperationKind::StashPop
                    ) && committed(back)
                        && back.id > push.id
                })
        }),
        TourTask::Branch => any_committed(entries, OperationKind::BranchCreate),
        TourTask::MergeWithPreview => any_committed(entries, OperationKind::Merge),
        TourTask::ResolveConflict => entries.iter().any(|merge| {
            merge.kind == OperationKind::Merge && committed(merge) && conflicted.contains(&merge.id)
        }),
        TourTask::Push => any_committed(entries, OperationKind::Push),
    }
}
