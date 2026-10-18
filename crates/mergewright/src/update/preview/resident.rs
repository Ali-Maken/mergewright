//! The rows a Ghost Preview is laid out from, snapshotted when it opens.

use std::collections::HashSet;

use mergewright_core::commit::Commit;
use mergewright_core::oid::Oid;

use crate::history::parents;
use crate::update::AppState;

/// What the window held around the screen when the preview opened.
pub(super) struct Resident {
    /// The absolute history row number of `commits[0]`.
    pub(super) top: usize,
    /// The rows in drawing order, each a **drawing commit**: its parents are
    /// the drawn ones (`Row::drawn_parents`), or a stash entry's index
    /// commit would read as a lane leaving it. The one exception to 06 D1's
    /// "`Row.commit` is git's commit": no write reads it — the plan job gets
    /// ids (`open_preview`'s `PreviewRequest`).
    pub(super) commits: Vec<Commit>,
    /// Which of them the app calls merges ([`parents::is_merge`]), read off
    /// their source rows: a folded merge's drawing commit has one parent
    /// and cannot say (ease-pass 06r, review m1).
    pub(super) merges: HashSet<Oid>,
}

/// Everything the window holds in `[top − height, top + 2·height)`.
pub(super) fn slice(state: &AppState, height: usize) -> Resident {
    let mut resident = Resident {
        top: 0,
        commits: Vec::new(),
        merges: HashSet::new(),
    };
    let Some(history) = state.history.as_ref() else {
        return resident;
    };
    let refs = state.refs.as_deref();
    let first = history.top.saturating_sub(height);
    let last = history.top.saturating_add(2 * height);
    resident.top = first;
    for row in first..last {
        match history.row_at(row) {
            Some(found) => {
                if resident.commits.is_empty() {
                    resident.top = row;
                }
                if parents::is_merge(found, refs) {
                    resident.merges.insert(found.commit.id);
                }
                resident.commits.push(Commit {
                    parents: found.drawn_parents.clone(),
                    ..found.commit.clone()
                });
            }
            // A gap in the window: the rows on hand are contiguous, so this
            // only happens before the first one or after the last.
            None if resident.commits.is_empty() => {}
            None => break,
        }
    }
    resident
}

#[cfg(test)]
mod tests;
