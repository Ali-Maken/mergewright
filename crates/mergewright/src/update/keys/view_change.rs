//! The keys that change which history is shown (`Space`, `1`, `H`), split
//! out of `keys` (review m6-amend-16 M3).

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::oid::Oid;
use mergewright_graph::Follow;

use crate::config::MergedBranches;
use crate::history::{History, View};
use crate::update::{AppState, Command};

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
                if fold.folds == Some(true) {
                    // A folding merge opens, an opened one closes: the
                    // merge rule wins inside an opened merge too.
                    if let Some(opened) = open_merge(&view, commit) {
                        view = opened;
                    } else {
                        view.inclusion.opened_mut()?.close(commit);
                    }
                } else if let Some(inside) = &fold.inside {
                    view.inclusion.opened_mut()?.close(inside.merge);
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

/// Sends a view change's request: the rows `changed.view` shows, with the
/// cursor kept on its commit (or the merge that commit now folds behind,
/// ease-pass 06h) and a finished search's folded count asked again on it
/// (gap `ease/06c-suffix-stale`).
pub(super) fn send(next: &mut AppState, commands: &mut Vec<Command>, changed: Changed) {
    let height = next.graph_rows();
    if next
        .history
        .as_ref()
        .is_none_or(|history| history.view == changed.view)
    {
        return;
    }
    // Asked before the request is built, so it rides on it.
    crate::update::search::ask_folded(next);
    let Some(history) = next.history.as_mut() else {
        return;
    };
    let mut request = history.change_view(changed.view, height);
    // `space` inside an opened merge: the cursor goes to the merge it
    // closed. Rows above the merge keep their numbers when it closes, so a
    // merge row on hand is where it will be.
    if let Some(merge) = changed.land_on {
        history.reanchor = None;
        request = history.seek(merge, height).or(request);
    }
    if let Some(request) = request
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// Switch the graph mode for this run (ease-pass 06h; 06i calls it on ⏎).
/// Only the fold changes: `follow` and every filter stay, every opened
/// merge closes, and folding again keeps `HEAD`'s line as a fresh start
/// does. Through [`send`], so the cursor lands by `keep` and a live
/// search's folded count is asked again. Same mode → nothing. Not an
/// operation: no journal entry.
pub(crate) fn switch_graph_mode(state: &mut AppState, to: MergedBranches) -> Vec<Command> {
    state.graph_mode = to;
    let mut commands = Vec::new();
    let Some(history) = state.history.as_ref() else {
        return commands;
    };
    if history.merged_branches() == to {
        return commands;
    }
    let mut view = history.view.clone();
    view.inclusion.fold = View::starting(to).inclusion.fold;
    let head = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    let view = with_head_kept(&view, head).unwrap_or(view);
    send(
        state,
        &mut commands,
        Changed {
            view,
            land_on: None,
        },
    );
    commands
}

/// `view` with one folded `merge` opened — `space`'s open half, and a
/// reveal's (ease-pass 06c): one view change, one re-layout from the merge
/// down. `None` when it is open already, or the view does not fold.
pub(in crate::update) fn open_merge(view: &View, merge: Oid) -> Option<View> {
    let mut opened = view.clone();
    opened.inclusion.opened_mut()?.open(merge).then_some(opened)
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

/// A view key (order, filter, stash visibility): asks the history job for
/// the rows the changed view shows.
pub(super) fn apply_view_change(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    change: ViewChange,
) {
    let stashes: Vec<Oid> = next.refs.as_deref().map_or_else(Vec::new, |refs| {
        refs.stashes().iter().map(|&(commit, _)| commit).collect()
    });
    let changed = next
        .history
        .as_ref()
        .and_then(|history| change.apply(history, &stashes));
    if let Some(changed) = changed {
        send(next, commands, changed);
    }
}
