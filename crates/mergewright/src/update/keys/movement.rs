//! Graph motion: the cursor moving through the rows.

use crate::history::Motion;
use crate::update::events::Command;
use crate::update::state::AppState;

/// A graph motion key: the working-tree row's flag (`update::selection`)
/// sees it first, then whatever motion is left goes to `History`.
pub(super) fn move_cursor(next: &mut AppState, commands: &mut Vec<Command>, motion: Motion) {
    let Some(motion) = crate::update::selection::move_from_graph(next, motion) else {
        return;
    };
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.move_cursor(motion, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}
