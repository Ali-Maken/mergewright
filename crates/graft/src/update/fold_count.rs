//! A folding merge's count, behind its tail `▸ 4 commits from …` (ease-pass
//! 06; m8-polish-and-ship/17b decision 1): how many commits the merge
//! brought in, counted by `jobs::fold_count` and kept for the session in
//! [`AppState::fold_counts`].
//!
//! The count is a fact of the merge commit — it never changes — so it is
//! never recounted: not on refresh, not on opening and closing. Asked at
//! rest ([`at_rest`], from `update::settle` whenever the cursor is not
//! mid-burst) for the folding merges drawn on screen.

use graft_core::job::JobId;
use graft_core::oid::Oid;

use super::events::Command;
use super::state::{AppState, Status};

/// Where one folded merge's count stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FoldCount {
    /// Asked; the tail has no number yet.
    Counting,
    /// The commits the merge brought in.
    Counted(usize),
    /// Git could not count; the tail says `?` and the status line said why.
    Failed,
}

/// At rest: asks for the count of the first folding merge drawn on screen
/// that has none yet — one command per rest, top row first. Every answer is
/// an event whose settle asks for the next, so every folding merge on screen
/// is counted without a burst of reads at first paint. Once per merge per
/// session.
pub(super) fn at_rest(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    let Some(merge) = uncounted(next) else {
        return;
    };
    next.fold_counts.insert(merge, FoldCount::Counting);
    commands.push(Command::CountFolded {
        id: next_id(),
        merge,
    });
}

/// Is a count owed to rest — a folding merge on screen not yet asked? A
/// burst defers the ask ([`at_rest`] runs only at rest), so the loop must
/// wake for rest while this holds, as it does for an owed follow
/// (`update::rest_owed`).
pub(crate) fn owed(state: &AppState) -> bool {
    uncounted(state).is_some()
}

/// The first folding merge drawn on screen, top row first, with no count
/// asked yet.
fn uncounted(state: &AppState) -> Option<Oid> {
    let height = state.graph_rows();
    let history = state.history.as_ref()?;
    (history.top..history.top.saturating_add(height)).find_map(|at| {
        let folds = history.fold_at(at)?.folds == Some(true);
        let merge = history.row_at(at)?.commit.id;
        (folds && !state.fold_counts.contains_key(&merge)).then_some(merge)
    })
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
