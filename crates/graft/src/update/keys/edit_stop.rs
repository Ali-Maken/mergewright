//! `space` at a rewrite's `edit` stop (m7-power-tools/07d D1): on the rows
//! where `a` amends — the stopped commit (`HEAD · stopped here`) and the
//! Uncommitted-changes row — `space` stages every change, exactly as the
//! changes pane's `Ctrl+A` does, so `space a c` works from where the stop
//! lands. Every other row, and every other time, keeps `space` = fold.
//!
//! One predicate, two callers: `graph_key`'s `space` and the hint bar
//! (`view::rows`), so the bar says `space stage` exactly where it stages.

use graft_core::event::{KeyCode, KeyPress, Modifiers};

use super::super::commit_editor::at_edit_stop;
use super::super::events::Command;
use super::super::state::AppState;

/// True iff the graph has the keyboard, the stop is an edit stop, and the
/// cursor is on the stopped commit's row (`HEAD`) or the Uncommitted-changes
/// row. Callers: `graph_key`'s `space` arm; the edit-stop hint bar.
pub(crate) fn space_stages_here(state: &AppState) -> bool {
    graph_has_the_keyboard(state) && at_edit_stop(state) && on_a_staging_row(state)
}

/// `space` at an edit stop on a staging row: `Ctrl+A`'s stage-all, the same
/// `Operation`, command and journal entry (`operate::stage_all`). `true`
/// when the key was this one's, whether or not anything was stageable —
/// with nothing staged it says why, in `Ctrl+A`'s words (30n m6), and never
/// falls through to the fold.
pub(super) fn space_key(next: &mut AppState, commands: &mut Vec<Command>, key: KeyPress) -> bool {
    if key.code != KeyCode::Char(' ') || key.modifiers != Modifiers::NONE {
        return false;
    }
    if !space_stages_here(next) {
        return false;
    }
    let mut local_jobs = next.local_jobs;
    let staged = super::super::operate::stage_all(next, commands, &mut || local_jobs.next_id());
    next.local_jobs = local_jobs;
    if let Err(why) = staged {
        next.status = why.status();
    }
    true
}

/// The graph, not a pane, a modal or an overlay, has the keyboard.
fn graph_has_the_keyboard(state: &AppState) -> bool {
    state.graph_has_focus() && no_box_is_up(state)
}

/// No modal owns the keys and no overlay stands over the graph. Also asked
/// by the rewrite editor's open read when it answers: the editor never
/// opens over another box (`rewrite_editor::opened`, 30n Q2).
pub(crate) fn no_box_is_up(state: &AppState) -> bool {
    !state.modal_owns_keys() && state.graph_overlay().is_none()
}

/// The cursor is on `HEAD`'s row — the stopped commit's at an edit stop
/// (`view::entries` tags it `stopped here`) — or the Uncommitted row.
fn on_a_staging_row(state: &AppState) -> bool {
    if state.on_working_tree {
        return true;
    }
    let head = state.refs.as_deref().and_then(crate::refs::RefBadges::head);
    let selected = state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| row.commit.id);
    selected.is_some() && selected == head
}
