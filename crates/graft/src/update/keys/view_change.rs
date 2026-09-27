//! The keys that change which history is shown (`Space`, `1`, `H`), split
//! out of `keys` (review m6-amend-16 M3).

use graft_core::event::{KeyCode, KeyPress};
use graft_core::oid::Oid;
use graft_graph::Follow;

use crate::history::{History, View};

/// A key that changes which history is shown (PROMPT.md §1.3). DESIGN.md
/// §2.3 assigns none of these; they are recorded there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ViewChange {
    /// `Space` on a merge: fold its merged branch into it, or unfold it.
    Fold,
    /// `1`: the first-parent view, or back to every parent.
    FirstParent,
    /// `H`: hide the stash, or show it again.
    Stashes,
}

impl ViewChange {
    /// The view after this change, or `None` where it does nothing: `Space`
    /// off a merge, `H` with no stash.
    pub(super) fn apply(self, history: &History, stashes: &[Oid]) -> Option<View> {
        let mut view = history.view.clone();
        match self {
            ViewChange::Fold => {
                let commit = &history.selected()?.commit;
                // A folded merge is drawn with one parent, so it is known
                // by the fold, not by its parents.
                if !view.inclusion.folded.remove(&commit.id) {
                    if commit.parents.len() < 2 {
                        return None;
                    }
                    view.inclusion.folded.insert(commit.id);
                }
            }
            ViewChange::FirstParent => {
                view.follow = match view.follow {
                    Follow::AllParents => Follow::FirstParent,
                    Follow::FirstParent => Follow::AllParents,
                };
            }
            ViewChange::Stashes => {
                if view.inclusion.hidden_stashes.is_empty() {
                    // Every entry, not just the newest: `H` reads as "the
                    // stash", and hiding one of three would leave two
                    // behind (m3-working-tree-undo-and-stash/06).
                    if stashes.is_empty() {
                        return None;
                    }
                    view.inclusion
                        .hidden_stashes
                        .extend(stashes.iter().copied());
                } else {
                    view.inclusion.hidden_stashes.clear();
                }
            }
        }
        Some(view)
    }
}

/// `view` knowing `stashes` as the stash entries' own commits
/// (`Inclusion::with_stashes`, m7-power-tools/16), or `None` when it already
/// does. Each entry then keeps only its first parent, so its index and
/// untracked commits leave the graph. Set beside `H`'s `hidden_stashes`
/// because both are "which stash is drawn how"; called whenever the refs
/// (and so the stash list) are read, and when a walk starts.
pub(in crate::update) fn with_stash_entries(view: &View, stashes: &[Oid]) -> Option<View> {
    let entries: std::collections::BTreeSet<Oid> = stashes.iter().copied().collect();
    if view.inclusion.stashes() == &entries {
        return None;
    }
    let mut changed = view.clone();
    changed.inclusion = changed.inclusion.with_stashes(entries);
    Some(changed)
}

/// The view change a key asks for, if it is one.
pub(super) fn view_change(key: KeyPress) -> Option<ViewChange> {
    if key.modifiers.control || key.modifiers.alt {
        return None;
    }
    match key.code {
        KeyCode::Char(' ') => Some(ViewChange::Fold),
        KeyCode::Char('1') => Some(ViewChange::FirstParent),
        KeyCode::Char('H') => Some(ViewChange::Stashes),
        _ => None,
    }
}
