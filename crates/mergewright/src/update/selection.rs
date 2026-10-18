//! The uncommitted-changes row's one-bit selection.
//!
//! The row is synthesised by `view` (m1-the-graph/12b), not a position of
//! `History`'s cursor — making it one would shift every row index (windows,
//! requests, search hits, promotion anchors). So the selection lives beside
//! `History`'s cursor instead: `AppState::on_working_tree`. This module holds
//! the pure rules for setting, clearing and following it
//! (`docs/design-notes/m2-details-and-diffs.md`, "Spec fix (2026-09-17,
//! working-tree row)").

use mergewright_core::job::JobId;

use crate::history::Motion;

use super::detail::DetailSpec;
use super::events::Command;
use super::state::AppState;

/// A graph motion key, before `History::move_cursor` sees it: the motion
/// `History` should still apply, or `None` when the flag alone answered it
/// and nothing more happens this key.
///
/// Not on the row: `Up(1)` from the viewport's top row (`History::cursor ==
/// History::top`; in a one-row pane, where it is not pinned, from row 0),
/// while the row is shown, lands on it instead of scrolling
/// — `History` is asked for nothing. Any other motion passes through
/// unchanged (a `g`/`PageUp` that lands on row 0 stops there; one more `k`
/// reaches the virtual row).
///
/// On the row: `Down(n)` leaves it (`Down(0)` needs nothing more — `j`'s
/// `Down(1)` lands exactly on `HEAD`'s row, where the history cursor already
/// sits); `PageDown`/`Bottom` leave it and apply as usual; `Up`/`PageUp`/`Top`
/// do nothing at the very top of everything (`History::top == 0`), else
/// leave the row (the viewport scrolls up, the row goes with `HEAD`).
pub fn move_from_graph(state: &mut AppState, motion: Motion) -> Option<Motion> {
    if !state.on_working_tree {
        // Pinned, the row is drawn above the viewport's top row, so `k` from
        // that row reaches it. In a one-row pane it is not pinned and sits
        // above history's first row only: from any other row, `k` there
        // would flip between it and history (m7-power-tools/48, Decided 4).
        let pinned = state.uncommitted_row_pinned();
        let at_top = state
            .history
            .as_ref()
            .is_some_and(|history| history.cursor == if pinned { history.top } else { 0 });
        if matches!(motion, Motion::Up(1)) && at_top && state.uncommitted_row_shown() {
            state.on_working_tree = true;
            return None;
        }
        return Some(motion);
    }
    match motion {
        Motion::Down(n) => {
            state.on_working_tree = false;
            // `Down(1)` (`j`) lands exactly on `HEAD`'s row, where the
            // history cursor already sits — `Down(0)`'s "nothing more"
            // reading, so it collapses to `None` too.
            (n > 1).then(|| Motion::Down(n - 1))
        }
        Motion::PageDown | Motion::Bottom => {
            state.on_working_tree = false;
            Some(motion)
        }
        Motion::Up(_) | Motion::PageUp | Motion::Top => {
            let top = state.history.as_ref().map_or(0, |history| history.top);
            if top == 0 {
                None
            } else {
                state.on_working_tree = false;
                Some(motion)
            }
        }
    }
}

/// Called once at the end of every event, with `History::cursor`'s value
/// from the start of it. Clears the flag when the row is no longer shown
/// (the tree went clean on `Ctrl+R`, refs moved `HEAD`, the wheel scrolled
/// `HEAD` off the top, a resize) or the history cursor moved under it (a
/// search hit, a click, a `G` landing). Returns whether it cleared.
pub fn reconcile(state: &mut AppState, cursor_before: usize) -> bool {
    if !state.on_working_tree {
        return false;
    }
    let cursor_now = state.history.as_ref().map_or(0, |history| history.cursor);
    if !state.uncommitted_row_shown() || cursor_now != cursor_before {
        state.on_working_tree = false;
        true
    } else {
        false
    }
}

/// What the pane and the evil probe follow, by identity (G-settle,
/// m8-polish-and-ship/03): the loop compares it before and after each wake
/// to tell a motion, so it needs no keymap knowledge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FollowKey {
    /// A commit row.
    Commit(mergewright_core::oid::Oid),
    /// The uncommitted-changes row.
    WorkingTree,
}

/// The selection [`follow_selection`] reads; `None` before the cursor's row
/// has arrived.
pub(crate) fn follow_key(state: &AppState) -> Option<FollowKey> {
    state.selected_spec().and_then(|spec| match spec {
        DetailSpec::Commit(oid) => Some(FollowKey::Commit(oid)),
        DetailSpec::WorkingTree => Some(FollowKey::WorkingTree),
        DetailSpec::Compare { .. } => None,
    })
}

/// The cursor has moved off what was last followed, and the follow has not
/// run yet.
pub(crate) fn follow_owed(state: &AppState) -> bool {
    follow_key(state) != state.followed
}

/// The cursor has left what was last followed, and the follow has not run:
/// what the loop waits on for rest and the pane's `reading…` shows. Not
/// before anything was followed — the first follow is never held.
pub(crate) fn follow_behind(state: &AppState) -> bool {
    state.followed.is_some() && follow_owed(state)
}

/// The pane follows the cursor: it is shown, and it is not a compare opened
/// from the menu (no base pinned), which stays put. The one answer to "does
/// a moved cursor change what the pane reads", for [`followed`], the
/// burst's hold ([`follow_waits`]) and the title's `reading…`
/// (`view::detail`) alike — review M1 (m8-03): the hold once asked only
/// `shown`, cancelled a menu compare's read, and nothing asked again.
pub(crate) fn pane_follows(state: &AppState) -> bool {
    let fixed = state.compare_base.is_none()
        && matches!(state.detail.spec, Some(DetailSpec::Compare { .. }));
    state.detail.shown && !fixed
}

/// Does this update hold the follow back? Mid-burst (`defer_follow`, set by
/// the loop) with the cursor off what was followed, and following would
/// start work: the pane follows the cursor ([`pane_follows`]), or the
/// cursor is on a merge to probe.
///
/// Not when nothing was followed before (a `G` whose rows have just
/// arrived, the first window at launch): there is nothing in flight to
/// save, and the pane would sit empty until rest. Not with the pane hidden
/// (or fixed) off a merge: that follow starts nothing — it only cancels a
/// probe still running — so it runs now and leaves nothing to wait for.
pub(crate) fn follow_waits(state: &AppState) -> bool {
    state.defer_follow
        && follow_behind(state)
        && (pane_follows(state) || selected_merge(state).is_some())
}

/// After any event: a shown pane follows the selection, the working-tree row
/// included. A no-op while hidden, or when the selection did not change
/// (`DetailState::follow`'s own check).
///
/// While a compare base is pinned (`AppState::compare_base`,
/// m2-details-and-diffs/11), the cursor moving onto another commit opens a
/// fresh `DetailSpec::Compare` against it instead of `Commit`; onto the
/// working-tree row it follows to `WorkingTree` as usual, the base kept
/// (`x` is a no-op there — `update::keys`); back onto the base's own row it
/// does nothing (comparing a commit with itself is not a spec). With no base
/// pinned, a pane already showing a `Compare` (e.g. from the context menu,
/// `context-actions-menu`) does not follow the cursor at all.
pub fn follow_selection(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    let mut commands = followed(state, next_id);
    // `update` calls this at the tail of every event, so this is the one
    // place that sees whatever the cursor ended up on -- a moved cursor, a
    // fresh `Enter`, a context-menu open or a close. `probe_evil` is
    // idempotent, so calling it every time asks for nothing new.
    //
    // The probe follows the *selection*, not the pane (M2 review M3):
    // PROMPT.md §1.4 asks for the selected merge, and a merge the cursor is
    // on has to carry its marker whether or not the pane is open. Only a
    // merge is probed: the verdict is about what a merge's tree did to its
    // parents, and asking about an ordinary commit would spawn `git` for an
    // answer that is always "clean".
    let selected = selected_merge(state);
    commands.extend(
        state
            .detail
            .probe_evil(selected, &state.radar.policy, next_id),
    );
    commands
}

/// The commit under the cursor, when it is a merge: what the evil-merge
/// probe is asked about. `None` on the working-tree row, on an ordinary
/// commit, or before any history has landed.
///
/// Also `None` on a stash commit (m6/66): its tree differs from its parents
/// by construction, so the probe always called it evil. And `None` while
/// the refs are unread — until they land a stash cannot be told from a
/// merge, and unknown is not "not a stash".
fn selected_merge(state: &AppState) -> Option<mergewright_core::oid::Oid> {
    if state.on_working_tree {
        return None;
    }
    let row = state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)?;
    let id = row.commit.id;
    let refs = state.refs.as_deref()?;
    crate::history::parents::is_merge(row, Some(refs)).then_some(id)
}

/// `follow_selection` without the evil-merge probe: what the pane itself
/// does about a moved cursor.
fn followed(state: &mut AppState, next_id: &mut impl FnMut() -> JobId) -> Vec<Command> {
    if !pane_follows(state) {
        return Vec::new();
    }
    if let Some(base) = state.compare_base {
        let spec = match state.selected_spec() {
            Some(DetailSpec::Commit(oid)) => {
                // Unmoved, with a compare against this base on show: it was
                // picked on purpose (a click, `x` on another row), not by
                // the cursor, so it stays.
                let picked = matches!(
                    state.detail.spec,
                    Some(DetailSpec::Compare { base: shown, .. }) if shown == base.oid
                );
                if picked && state.detail.compare_followed == Some(oid) {
                    return Vec::new();
                }
                state.detail.compare_followed = Some(oid);
                // A line landing (`show_line`) that opened the base as a
                // plain commit keeps it: that was asked for by name.
                if oid == base.oid && state.detail.spec == Some(DetailSpec::Commit(oid)) {
                    return Vec::new();
                }
                // m7-14: on the base row itself this is `Compare { base,
                // head: base }`, whose header says `x` clears it.
                DetailSpec::Compare {
                    base: base.oid,
                    head: oid,
                }
            }
            Some(spec @ DetailSpec::WorkingTree) => {
                state.detail.compare_followed = None;
                spec
            }
            _ => return Vec::new(),
        };
        return state.detail.follow(spec, next_id);
    }
    let Some(spec) = state.selected_spec() else {
        return Vec::new();
    };
    state.detail.follow(spec, next_id)
}

#[cfg(test)]
mod tests;
