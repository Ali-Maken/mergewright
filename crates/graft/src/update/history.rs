//! The history walk's own share of [`super::update`]: the progress that
//! lands from the walk and the topological-sort promotion (ADR-0008).

use graft_core::job::JobId;

use crate::history::{HistoryUpdate, Phase, Promotion};

use super::events::Command;
use super::state::AppState;

/// One update from history job `id`, taken in if it is the walk on screen.
pub(super) fn walk_progress(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    update: HistoryUpdate,
) {
    let height = next.graph_rows();
    if next.awaiting == Some(id)
        && let Some(history) = next.history.as_mut()
    {
        // ADR-0008: a provisional order is sorted properly off-thread.
        if let HistoryUpdate::Started(facts) = &update
            && facts.is_provisional()
        {
            commands.push(Command::PromoteHistory);
        }
        history.apply(update, height);
        super::search::seek_landed(next, commands, id);
    }
    // Otherwise it is from a walk that was cancelled or superseded. Dropped
    // here, which is the whole reason the id travels with it: no chunk lands
    // after `Esc`.
}

/// A topological sort has started (ADR-0008). Only the newest is wanted: an
/// older one still running is superseded and cancelled.
pub(super) fn sort_started(next: &mut AppState, commands: &mut Vec<Command>, id: JobId) {
    if let Some(older) = next.promoting.replace(id) {
        commands.push(Command::CancelJob(older));
    }
    if let Some(history) = next.history.as_mut() {
        history.promotion = Some(Promotion::Sorting);
    }
}

/// A topological sort has produced its `order`: handed to the walk on screen
/// if it is from the sort still wanted, dropped if it was cancelled or
/// superseded.
pub(super) fn sorted(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    order: std::sync::Arc<[graft_core::commit_handle::CommitHandle]>,
) {
    if next.promoting != Some(id) {
        return;
    }
    next.promoting = None;
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut() {
        match (next.awaiting, history.promote(order, height)) {
            (Some(walk), Some(request)) => commands.push(Command::ShowRows(walk, request)),
            // The walk has gone; nobody would take the order up.
            _ => history.promotion = Some(Promotion::Kept),
        }
    }
}

/// Stops promoting to topological order: cancels the sort if it is still
/// running, and leaves the order on screen marked provisional. A promotion
/// already swapping in is left alone unless the walk itself has ended — the
/// walk may have taken the order up, and only its answer can say.
pub(in crate::update) fn give_up_promotion(next: &mut AppState, commands: &mut Vec<Command>) {
    if let Some(id) = next.promoting.take() {
        commands.push(Command::CancelJob(id));
    }
    let walk_gone = next.awaiting.is_none();
    if let Some(history) = next.history.as_mut() {
        match history.promotion {
            Some(Promotion::Sorting) => history.promotion = Some(Promotion::Kept),
            Some(Promotion::Swapping { .. }) if walk_gone => {
                history.promotion = Some(Promotion::Kept);
            }
            _ => {}
        }
    }
}

/// **The one reading of "the history walk is reading"**: the walk the screen
/// waits on is working on an ask — opening, rows asked and not yet laid out,
/// or a `G`/find under way (`History::is_reading`). `Esc` with nothing else
/// to back out of cancels the walk only then, and the header draws
/// `Opening`/`Reading`/`Jumping` only then (`view::rows::walk_view`), so the
/// header says reading exactly when `Esc` would stop something. A walk
/// resting between asks, or complete, is not reading: `Esc` leaves it be
/// (m6-conflicts/85 A1).
pub(crate) fn walk_is_reading(state: &AppState) -> bool {
    state.awaiting.is_some()
        && state
            .history
            .as_ref()
            .is_none_or(crate::history::History::is_reading)
}

/// Moves the walk on screen to `phase`. Called only for the walk the screen
/// was waiting for, which always has a [`crate::history::History`].
pub(in crate::update) fn set_phase(next: &mut AppState, phase: Phase) {
    if let Some(history) = next.history.as_mut() {
        history.phase = phase;
    }
}
