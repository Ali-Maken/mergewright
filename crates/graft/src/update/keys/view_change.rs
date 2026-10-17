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
    /// `Space`: open a folding merge or close an opened one; on a row
    /// inside an opened merge, close that merge (ease-pass 06).
    Fold,
    /// `1`: the first-parent view, or back to every parent.
    FirstParent,
    /// `H`: hide the stash, or show it again.
    Stashes,
}

/// A view change's outcome: the view, and the commit the cursor lands on
/// when it is not the one it was on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Changed {
    /// The view after the change.
    pub(super) view: View,
    /// `Space` inside an opened merge closes it and lands on the merge.
    pub(super) land_on: Option<Oid>,
}

impl ViewChange {
    /// The view after this change, or `None` where it does nothing: `Space`
    /// on a row that neither folds a branch nor sits inside an opened
    /// merge, `H` with no stash.
    pub(super) fn apply(self, history: &History, stashes: &[Oid]) -> Option<Changed> {
        let mut view = history.view.clone();
        let mut land_on = None;
        match self {
            ViewChange::Fold => {
                let commit = history.selected()?.commit.id;
                // A view that does not fold (first-parent) has no facts.
                let fold = history.fold_at(history.cursor)?;
                let opened = view.inclusion.opened_mut()?;
                if fold.folds == Some(true) {
                    // A folding merge opens, an opened one closes: the
                    // merge rule wins inside an opened merge too.
                    if !opened.close(commit) {
                        opened.open(commit);
                    }
                } else if let Some(inside) = &fold.inside {
                    opened.close(inside.merge);
                    land_on = Some(inside.merge);
                } else {
                    return None;
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
        Some(Changed { view, land_on })
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

/// `view` keeping `head`'s first-parent line out of the fold (ease-pass
/// 06: the trunk, integration branches and `HEAD` are never folded), or
/// `None` when it already does, or does not fold. Called wherever
/// [`with_stash_entries`] is: whenever the refs are read, and when a walk
/// starts.
pub(in crate::update) fn with_head_kept(view: &View, head: Option<Oid>) -> Option<View> {
    let keep: std::collections::BTreeSet<Oid> = head.into_iter().collect();
    if view.inclusion.keep()? == &keep {
        return None;
    }
    let mut changed = view.clone();
    *changed.inclusion.keep_mut()? = keep;
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
