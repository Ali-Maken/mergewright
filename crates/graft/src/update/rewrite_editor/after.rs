//! The AFTER plan (ruling 4) and `y` (ruling 6).
//!
//! The editor owns its plan job. Every edit that changes the params cancels
//! the plan still out (as `close_preview` does), asks a new one through the
//! preview's own job (`Command::PlanPreview`), and keeps only that one's
//! answer: the last edit wins. The editor never sets `state.preview`.

use std::collections::HashSet;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_ops::operation::Operation;

use super::{After, RewriteEditor};
use crate::update::events::Command;
use crate::update::preview::{HISTORY_CANNOT_STOP, PreviewRequest, PreviewSubject};
use crate::update::state::AppState;

/// The key that opened the editor: a `Moved` refusal after `y` names it
/// (`R again`).
const OPENED_BY: &str = "R";

/// Ask for the AFTER plan of the editor's params as they now stand, unless
/// the plan in hand (or in flight) is already of exactly those. An invalid
/// todo plans nothing: the AFTER side and `y` give the problem's words.
pub(super) fn replan(
    state: &mut AppState,
    editor: &mut RewriteEditor,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let params = editor.params();
    let current = match &editor.after {
        After::Pending { asked, .. } | After::Ready { asked, .. } => Some(asked),
        After::NotAsked | After::Failed(_) | After::Refused(_) | After::Invalid(_) => None,
    };
    if let (Ok(params), Some(asked)) = (&params, current)
        && params == asked
    {
        return;
    }
    if let After::Pending { job, .. } = &editor.after {
        commands.push(Command::CancelJob(*job));
        state.rewrite.stale.insert(*job);
    }
    editor.after = match params {
        Ok(params) => {
            let job = next_id();
            let request = PreviewRequest {
                subject: PreviewSubject::Operation {
                    op: Box::new(Operation::Rewrite(params.clone())),
                    opened_by: OPENED_BY,
                },
                resident: resident(state),
            };
            commands.push(Command::PlanPreview(job, Box::new(request)));
            After::Pending { job, asked: params }
        }
        Err(problem) => After::Invalid(problem),
    };
}

/// The commits the graph holds in `[top − height, top + 2·height)`: the set
/// `open_preview` snapshots, so the job reads only the anchors it lacks.
fn resident(state: &AppState) -> HashSet<Oid> {
    let Some(history) = state.history.as_ref() else {
        return HashSet::new();
    };
    let height = state.graph_rows();
    let first = history.top.saturating_sub(height);
    let last = history.top.saturating_add(2 * height);
    (first..last)
        .filter_map(|row| history.row_at(row))
        .map(|row| row.commit.id)
        .collect()
}

/// `y`: run the planned rewrite. Inert while the plan is out, failed or
/// refused; an invalid todo says its problem. The op run is the op
/// pictured (hard choice 3): the plan's `runs` when it has one, else the
/// params it was asked for.
pub(super) fn start(
    state: &mut AppState,
    mut editor: Box<RewriteEditor>,
    commands: &mut Vec<Command>,
    next_id: &mut impl FnMut() -> JobId,
) {
    let op = match &editor.after {
        After::Ready { planned, asked } => planned
            .runs
            .as_deref()
            .cloned()
            .unwrap_or_else(|| Operation::Rewrite(asked.clone())),
        After::Invalid(problem) => {
            editor.notice = Some(super::words::invalid(&editor, problem));
            state.rewrite.editor = Some(editor);
            return;
        }
        After::NotAsked | After::Pending { .. } | After::Failed(_) | After::Refused(_) => {
            state.rewrite.editor = Some(editor);
            return;
        }
    };
    // A rewrite is a `rebase -i`: `Esc` must not kill it part-way, the
    // same as every history write the preview confirms.
    match crate::update::operate::launch_unstoppable(
        state,
        commands,
        op,
        HISTORY_CANNOT_STOP,
        Some(OPENED_BY),
        next_id,
    ) {
        Ok(()) => state.rewrite.launched = Some(editor),
        Err(busy) => {
            editor.notice = Some(busy.words());
            state.rewrite.editor = Some(editor);
        }
    }
}
