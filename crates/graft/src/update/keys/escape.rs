//! `Esc` on the graph: one key's whole ladder, top to bottom.

use super::super::events::Command;
use super::super::state::AppState;

/// `Esc` (once the detail pane is closed): backs a `G` still walking out first, then leaves Focus
/// mode, then cancels the walk while it is reading, else nothing (m1-the-graph/17, ADR-0023,
/// DESIGN.md §2.2, m6-conflicts/85).
pub(super) fn escape_key(next: &mut AppState, commands: &mut Vec<Command>) {
    let height = next.graph_rows();
    // A search still running: stop it and put the selection back.
    if next
        .search
        .as_ref()
        .is_some_and(|search| matches!(search.state, crate::search::SearchState::Running { .. }))
    {
        super::super::search::cancel_search(next, commands);
        return;
    }
    // A `G` still walking is what `Esc` backs out of first; the
    // walk itself goes on serving the rows already on screen.
    if let Some(id) = next.awaiting
        && let Some(history) = next.history.as_mut()
        && let Some(request) = history.withdraw_jump(height)
    {
        commands.push(Command::ShowRows(id, request));
        return;
    }
    // Then a finished search's line.
    if next.search.take().is_some() {
        return;
    }
    // Then Focus mode: leaving it is not a reason to stop the walk.
    if next.focus {
        next.focus = false;
        return;
    }
    // Then a topological sort still running: the rows stay in the order they
    // are in, and the screen says that order is provisional. Once the order
    // has been handed to the walk it is no longer the sort's to cancel.
    if next.promoting.is_some() {
        super::super::history::give_up_promotion(next, commands);
        return;
    }
    // Then the walk — **only while it is reading** (m6-conflicts/85 A1). A
    // walk resting between asks, or complete, stays `awaiting` for as long
    // as it is on screen (it serves rows on request), and cancelling it
    // then stopped a history nothing was loading: `stopped at 6 rows` on a
    // 6-commit repo, and a long one left unscrollable. With nothing
    // running, `Esc` does nothing.
    //
    // Stop waiting first, then ask the job to stop. The order
    // matters: a job that finishes between the two lines still
    // sends its result, and it has to find nobody waiting for it.
    if super::super::walk_is_reading(next)
        && let Some(id) = next.awaiting.take()
    {
        commands.push(Command::CancelJob(id));
        super::super::history::set_phase(next, crate::history::Phase::Cancelled);
        return;
    }
    // Going back to a hidden trail is `detail::keys::place_key`'s: `detail_key` runs for every focus before this ladder.
    debug_assert!(
        !super::super::place::esc_goes_back(next),
        "Esc with a hidden trail reached escape_key"
    );
    // Last: in the practice tour, with nothing else left to back out of,
    // `Esc` leaves it (`tour.leave`, m8/19b). Outside the tour, nothing.
    super::super::tour::esc_leaves(next);
}
