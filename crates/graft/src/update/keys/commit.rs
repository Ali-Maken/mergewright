//! `c` / `a`: the commit editor, or the banner's `c` while an operation is live.

use super::super::events::Command;
use super::super::state::AppState;

/// `c` or `a` in the graph or the changes pane.
///
/// While an operation is live, `c` is the banner's `c` wherever that acts
/// (`commit_editor::pane_c_acts`, m6-conflicts/52): the finish summary for
/// a merge, `--continue` for a rebase, cherry-pick or revert Graft began —
/// with the banner's own refusals, and `Esc` from the summary back on the
/// banner, the operation's home. Everywhere else, and for `a` always,
/// `commit_editor::open` answers: the editor, or the words refusing it
/// while an operation is live (m6-conflicts/48).
pub(super) fn commit_key(next: &mut AppState, commands: &mut Vec<Command>, amend: bool) {
    use super::super::commit_editor::{live_operation, pane_c_acts};
    let mut local_jobs = next.local_jobs;
    let mut next_id = || local_jobs.next_id();
    if !amend && live_operation(next).is_some_and(pane_c_acts) {
        super::super::banner::continue_pressed(
            next,
            commands,
            super::super::finish::ReturnTo::Banner,
            &mut next_id,
        );
    } else if amend && super::super::commit_editor::amend_at_edit_stop(next, commands, &mut next_id)
    {
        // `a` at a rewrite's `edit` stop, on `HEAD` or the uncommitted
        // row: amends the stopped commit, message kept (07b).
    } else if amend {
        // `a` reads the cursor (m7-power-tools/09): an older commit gets
        // the put-in preview; `HEAD` gets today's amend unless something
        // is built on it.
        super::super::amend_into::a_key(next, commands, &mut next_id);
    } else {
        commands.extend(super::super::commit_editor::open(next, amend, &mut next_id));
    }
    next.local_jobs = local_jobs;
}
