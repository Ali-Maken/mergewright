//! What folding by default puts on a row (ease-pass 06, mock G4): a
//! folding merge's tail, `▸ 4 commits from PR 16021`, and which opened
//! merge's side a row is on ([`InsideMerge`]), from the fold facts the
//! history worker sent with the window (`history::RowFold`).

use graft_core::oid::Oid;
use graft_graph::Row;
use graft_graph::recover_branch_name;
use graft_term::sanitize_line;
use graft_ui::graph_screen::{FoldCount, FoldMark};

use crate::history::History;
use crate::update::AppState;
use crate::update::fold_count::FoldCount as Asked;

/// The innermost opened merge whose merged side shows a row, and the branch
/// name recovered from that merge's subject — what the graph draws the
/// opened rows from (dim, the branch's last path segment; ease-pass 07).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ease-pass 07 (graph rows) draws the opened rows from it"
    )
)]
pub(crate) struct InsideMerge {
    /// The opened merge.
    pub(crate) merge: Oid,
    /// Its merged branch's name, when the subject names one.
    pub(crate) label: Option<Vec<u8>>,
}

/// The opened merge row `at` sits inside, if the fold says it does.
#[must_use]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "ease-pass 07 (graph rows) draws the opened rows from it"
    )
)]
pub(crate) fn inside_merge(history: &History, at: usize) -> Option<InsideMerge> {
    let inside = history.fold_at(at)?.inside.as_ref()?;
    Some(InsideMerge {
        merge: inside.merge,
        label: inside.label.as_deref().map(<[u8]>::to_vec),
    })
}

/// Row `at`'s tail when it is a merge with a merged branch folded behind it
/// (`▸`) or opened (`▾`): the count from `AppState::fold_counts` — none
/// until it lands, `?` if it failed — and the name the merge's subject
/// gives, in full; `PR <n>` for an Azure `Merged PR <n>: …`. No tail on a
/// merge that folds nothing (a sync merge), nor before the fold knows.
pub(in crate::view) fn fold_tail(
    state: &AppState,
    history: &History,
    row: &Row,
    at: usize,
) -> Option<FoldMark> {
    if history.fold_at(at)?.folds != Some(true) {
        return None;
    }
    let merge = row.commit.id;
    let open = history
        .view
        .inclusion
        .opened()
        .is_some_and(|opened| opened.contains(merge));
    let count = match state.fold_counts.get(&merge) {
        None | Some(Asked::Counting) => FoldCount::Pending,
        Some(Asked::Counted(count)) => FoldCount::Known(u32::try_from(*count).unwrap_or(u32::MAX)),
        Some(Asked::Failed) => FoldCount::Failed,
    };
    let from = recover_branch_name(row.commit.subject())
        .map(|name| sanitize_line(&String::from_utf8_lossy(&name)));
    Some(FoldMark { open, count, from })
}

#[cfg(test)]
mod tests;
