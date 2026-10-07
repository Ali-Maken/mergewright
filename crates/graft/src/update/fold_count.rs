//! A folded merge's `+N` (m8-polish-and-ship/17b decision 1): how many
//! commits the merge brought in, counted once by `jobs::fold_count` and kept
//! for the session in [`AppState::fold_counts`].
//!
//! The count is a fact of the merge commit — it never changes — so it is
//! never recounted: not on refresh, not on unfold and fold again. Asked from
//! [`request`], which every place that adds to `Inclusion::folded` calls
//! (today one: `keys::view_change`'s `Space`, through
//! `keys::apply_view_change`).

use std::collections::BTreeSet;

use graft_core::oid::Oid;

use super::events::Command;
use super::state::{AppState, Status};

/// Where one folded merge's count stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FoldCount {
    /// Asked; the row ends ` +`.
    Counting,
    /// The commits the merge brought in; the row ends ` +N` (nothing at 0).
    Counted(usize),
    /// Git could not count; the row ends ` +?` and the status line said why.
    Failed,
}

/// Asks for the count of every merge in `folded` that has none yet.
pub(super) fn request(next: &mut AppState, commands: &mut Vec<Command>, folded: &BTreeSet<Oid>) {
    for &merge in folded {
        if next.fold_counts.contains_key(&merge) {
            continue;
        }
        let id = next.local_jobs.next_id();
        next.fold_counts.insert(merge, FoldCount::Counting);
        commands.push(Command::CountFolded { id, merge });
    }
}

/// `Product::FoldedCounted`: keep the answer; a failure is said once, here,
/// with git's reason (rule 3).
pub(super) fn counted(next: &mut AppState, merge: Oid, count: Result<usize, String>) {
    let entry = match count {
        Ok(count) => FoldCount::Counted(count),
        Err(reason) => {
            let hex = merge.to_string();
            let sha7 = hex.get(..7).unwrap_or(&hex);
            next.status = Status::Failed(format!("couldn't count {sha7}'s commits: {reason}"));
            FoldCount::Failed
        }
    };
    next.fold_counts.insert(merge, entry);
}

#[cfg(test)]
mod tests;
