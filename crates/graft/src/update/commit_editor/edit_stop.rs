//! Amend at a rewrite's `edit` stop: the editor opened on the stopped commit (07b).

use super::*;

/// `a` at a rewrite's `edit` stop (m7-power-tools/07b): amend the stopped
/// commit with what is staged, **keeping its message, no editor** — `git
/// commit --amend --no-edit`, through `Operation::Amend` and `apply` like
/// every amend. `false` when this is not such a stop, or the cursor is on
/// an older row: `a` then answers as it always has (P14's words while live).
///
/// Asks for `HEAD`'s message first, as the editor's `a` does; the amend
/// launches on the answer ([`head_message_arrived`]).
pub(crate) fn amend_at_edit_stop(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if !at_edit_stop(state) || !on_head_or_the_work(state) {
        return false;
    }
    if state.head_message.is_some() || state.commit_editor.is_some() {
        return true;
    }
    let id = next_id();
    state.head_message = Some(id);
    state.edit_amend = true;
    commands.push(Command::ReadHeadMessage(id));
    true
}

/// A rebase is live and stopped on an `edit` row (07a's fact, carried on
/// the stop): P14's one exception.
pub(crate) fn at_edit_stop(state: &AppState) -> bool {
    matches!(live_operation(state), Some((InProgress::Rebase, _)))
        && state
            .stopped
            .as_ref()
            .filter(|stop| stop.kind == InProgress::Rebase)
            .and_then(crate::update::banner::edit_stop_of)
            .is_some()
}

/// The cursor is on `HEAD`'s row, the uncommitted row, or the changes pane.
pub(super) fn on_head_or_the_work(state: &AppState) -> bool {
    if state.on_working_tree || state.on_changes_pane() {
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
