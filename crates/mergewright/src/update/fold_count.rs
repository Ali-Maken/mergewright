//! A folding merge's count, behind its tail `▸ 4 commits from …` (ease-pass
//! 06; m8-polish-and-ship/17b decision 1): how many commits the merge
//! brought in, counted by `jobs::fold_count` and kept for the session in
//! [`AppState::fold_counts`].
//!
//! The count is a fact of the merge commit — it never changes — so it is
//! never recounted: not on refresh, not on opening and closing. Asked at
//! rest ([`at_rest`], from `update::settle` whenever the cursor is not
//! mid-burst) for the folding merges drawn on screen, all in one read.

use mergewright_core::job::JobId;
use mergewright_core::oid::Oid;

use super::events::{Command, RestRead};
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

/// At rest: asks for every folding merge drawn on screen that has no count
/// yet, in one command, top row first (ADR-0065 D2). The answer is one event
/// with every count, so there is one wake and one frame — and no chain: an
/// answer asks nothing. Each merge is marked `Counting` and asked once per
/// session. Asks nothing when none is uncounted.
pub(super) fn at_rest(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut dyn FnMut() -> JobId,
) {
    let merges = uncounted(next);
    if merges.is_empty() {
        return;
    }
    for merge in &merges {
        next.fold_counts.insert(*merge, FoldCount::Counting);
    }
    commands.push(Command::AtRest(next_id(), RestRead::CountFolded { merges }));
}

/// Is a count owed to rest — a folding merge on screen not yet asked? A
/// burst defers the ask ([`at_rest`] runs only at rest), so the loop must
/// wake for rest while this holds, as it does for an owed follow
/// (`update::rest_owed`).
pub(crate) fn owed(state: &AppState) -> bool {
    !uncounted(state).is_empty()
}

/// Has every folding merge on screen its answer — none owed, none still
/// counting? What a capture or a walk waits on before it reads a tail: at
/// a width where the tail gives way to the subject (ease-pass 09 ruling
/// W), the drawn words cannot say whether the count landed.
#[must_use]
pub fn settled(state: &AppState) -> bool {
    !owed(state)
        && !state
            .fold_counts
            .values()
            .any(|count| *count == FoldCount::Counting)
}

/// The folding merges drawn on screen with no count asked yet, top row first.
fn uncounted(state: &AppState) -> Vec<Oid> {
    let height = state.graph_rows();
    let Some(history) = state.history.as_ref() else {
        return Vec::new();
    };
    (history.top..history.top.saturating_add(height))
        .filter_map(|at| {
            let folds = history.fold_at(at)?.folds == Some(true);
            let merge = history.row_at(at)?.commit.id;
            (folds && !state.fold_counts.contains_key(&merge)).then_some(merge)
        })
        .collect()
}

/// `Product::FoldedCounted`: keep every answer; a failure is said once, with
/// the first failure's words (rule 3), the others marked `?`.
pub(super) fn counted(next: &mut AppState, counts: Vec<(Oid, Result<usize, String>)>) {
    let mut said = false;
    for (merge, count) in counts {
        let entry = match count {
            Ok(count) => FoldCount::Counted(count),
            Err(reason) => {
                if !said {
                    let hex = merge.to_string();
                    let sha7 = hex.get(..7).unwrap_or(&hex);
                    next.status =
                        Status::Failed(format!("couldn't count {sha7}'s commits: {reason}"));
                    said = true;
                }
                FoldCount::Failed
            }
        };
        next.fold_counts.insert(merge, entry);
    }
}

#[cfg(test)]
mod tests;
