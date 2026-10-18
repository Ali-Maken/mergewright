//! The history walk's own share of [`super::update`]: the progress that
//! lands from the walk and the topological-sort promotion (ADR-0008).

use mergewright_core::job::JobId;

use crate::history::{HistoryUpdate, Phase, Promotion, Reveal};

use super::events::Command;
use super::place::short;
use super::state::{AppState, Status};

#[cfg(test)]
mod tests;

/// One update from history job `id`, taken in if it is the walk on screen.
pub(super) fn walk_progress(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    update: HistoryUpdate,
) {
    let height = next.graph_rows();
    // From a walk that was cancelled or superseded: dropped here, which is
    // the whole reason the id travels with it — no chunk lands after `Esc`.
    let Some(history) = next.history.as_mut().filter(|_| next.awaiting == Some(id)) else {
        return;
    };
    // ADR-0008: a provisional order is sorted properly off-thread.
    if let HistoryUpdate::Started(facts) = &update
        && facts.is_provisional()
    {
        commands.push(Command::PromoteHistory);
    }
    let counted = match &update {
        HistoryUpdate::FoldedHits {
            search,
            folded,
            unwalked,
        } => Some((*search, *folded, *unwalked)),
        _ => None,
    };
    let was_whole = walked_whole(history);
    history.apply(update, height);
    let completed = !was_whole && walked_whole(history);
    // Before any route looks at where its seek landed: a reveal seeks
    // again, so every route keeps waiting through it.
    if let Some(owed) = history.take_reveal() {
        reveal(next, commands, id, owed);
    }
    if let Some((asked, folded, unwalked)) = counted {
        super::search::folded_hits(next, asked, folded, unwalked);
    }
    super::search::seek_landed(next, commands, id);
    if completed {
        super::search::walk_completed(next, commands);
    }
}

/// Has the walk laid out to the end of history — by an answer that reached
/// it, or by idle work ([`HistoryUpdate::LaidOut`])? Every commit the view
/// can show has been walked then.
fn walked_whole(history: &crate::history::History) -> bool {
    history.phase == Phase::Complete || history.laid_out.is_some()
}

/// Carries out the reveal a find's answer left owed (ease-pass 06c): opens
/// the merge — one view change, as `space` opens it — and seeks the commit
/// again in the opened view. Not journaled: a view change, not an
/// operation. A merge already open means the walk answered for a view it
/// was not asked for: the seek ends on the merge, not in this view, and the
/// failure is said. A finished search on screen asks its fold count again,
/// on the same request: the merge opened may hold its hits.
fn reveal(next: &mut AppState, commands: &mut Vec<Command>, walk: JobId, owed: Reveal) {
    let height = next.graph_rows();
    let Some(history) = next.history.as_ref() else {
        return;
    };
    let Some(view) = super::keys::view_change::open_merge(&history.view, owed.merge) else {
        next.status = Status::Failed(format!(
            "the history walk put {} behind {}, which is open already",
            short(owed.target),
            short(owed.merge)
        ));
        return;
    };
    // Asked before either request below is built, so whichever is sent
    // carries it.
    super::search::ask_folded(next);
    let Some(history) = next.history.as_mut() else {
        return;
    };
    let changed = history.change_view(view, height);
    // The find replaces the view change's own request: it carries the same
    // view, and only the newest request is answered.
    if let Some(request) = history.seek(owed.target, height).or(changed) {
        commands.push(Command::ShowRows(walk, request));
    }
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
    sorted: super::events::SortedOrder,
) {
    if next.promoting != Some(id) {
        return;
    }
    next.promoting = None;
    let height = next.graph_rows();
    let mut promoted = false;
    if let Some(history) = next.history.as_mut() {
        match (next.awaiting, history.promote(sorted.order, height)) {
            (Some(walk), Some(request)) => {
                commands.push(Command::ShowRows(walk, request));
                promoted = true;
            }
            // The walk has gone; nobody would take the order up.
            _ => history.promotion = Some(Promotion::Kept),
        }
    }
    // Only a sort that was taken up counts: a kept order offers nothing.
    if promoted {
        super::commit_graph::sort_landed(next, sorted.took);
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
