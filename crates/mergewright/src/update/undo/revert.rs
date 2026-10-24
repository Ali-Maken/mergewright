//! The time machine's answers: a revert, forced past the drift guard, and
//! the drift boxes. Moved out of `undo.rs` (m8/21b-5, move-only).

use mergewright_core::job::JobId;
use mergewright_ops::apply::Refresh;
use mergewright_term::sanitize_line;

use super::super::confirm::{ConfirmThen, PendingConfirm};
use super::super::events::Command;
use super::super::state::{AppState, Status};
use super::consequence::{landing, with_consequence, wrapped};
use super::{journal_keys, refresh};
use crate::jobs::undo::RevertedTo;

/// The time machine again, past the drift guard: the user has read what
/// moved and chosen `[ Revert ]`.
///
/// Unstoppable for the same reason the first attempt was: a jump is several
/// restores, and `Esc` between two of them would leave the repository
/// somewhere nobody asked for.
pub(in crate::update) fn revert_to_forced(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    entry: mergewright_core::journal::EntryId,
    what: &str,
    next_id: &mut impl FnMut() -> JobId,
) {
    if super::super::operate::operation_in_flight(state) {
        return;
    }
    let id = next_id();
    let claimed = super::super::operate::claim_write(state, id);
    if claimed.is_err() {
        // Unreachable: `operation_in_flight` returned above.
        super::super::operate::say_if_refused(state, claimed);
        return;
    }
    // The entry's own description, not its id: `01J…` on the busy line is
    // a fact about the journal file, not about what the user asked for
    // (review 08 Minor 2).
    let description = format!("revert to before \u{201c}{what}\u{201d}");
    state.status = Status::Busy(description.clone());
    commands.extend(super::super::remote::launched(
        state,
        id,
        description,
        None,
        super::super::remote::Stoppable::No(super::super::preview::HISTORY_CANNOT_STOP),
        None,
    ));
    commands.push(Command::RevertTo(
        id,
        entry,
        true,
        std::sync::Arc::clone(&state.keymap),
    ));
}

/// The journal's time machine answered (DESIGN §6.6,
/// m5-history-and-ghost-preview/08).
///
/// A jump is several restores, and the three answers say so plainly: how
/// many steps it took, how far it got when it stopped, or what moved
/// underneath before it started. **`Everything` is re-read whenever a step
/// ran** — a jump moves refs, `HEAD`, the index and the working tree, and no
/// narrower scope covers all four.
pub(in crate::update) fn reverted_to(
    state: &mut AppState,
    id: JobId,
    outcome: RevertedTo,
    next_id: &mut impl FnMut() -> JobId,
) -> Vec<Command> {
    if state.operating != Some(id) {
        return Vec::new();
    }
    state.operating = None;
    let every_step_ran = matches!(outcome, RevertedTo::Done { .. });
    let commands = match outcome {
        RevertedTo::Done {
            description,
            total,
            steps,
            replaced,
            presses,
        } => {
            let what = format!(
                "reverted to before \u{201c}{description}\u{201d} ({total} {})",
                if total == 1 { "step" } else { "steps" }
            );
            state.status = landing(what, &replaced, false, presses);
            // `steps` is what ran, kept for the record rather than for the
            // refresh: `Everything` is wider than any of their own scopes.
            debug_assert_eq!(steps.len(), total, "every planned step journaled");
            refresh(state, Refresh::Everything, next_id)
        }
        RevertedTo::Drift {
            description,
            total,
            drift,
            target,
        } => {
            // As for `undone`'s drift box: the answer ends its own `Busy`.
            state.status = Status::Ready;
            state.confirm = Some(revert_drift_box(&description, &drift, total, target));
            Vec::new()
        }
        RevertedTo::Refused { description, error } => {
            // Nothing ran, so nothing is to be refreshed and nothing is
            // half-done: a refusal, worded as one (review 08 Minor 4).
            state.status = Status::sticky(format!(
                "cannot revert to before \u{201c}{description}\u{201d}: {error}"
            ));
            Vec::new()
        }
        RevertedTo::Failed {
            description: _,
            done,
            total,
            error,
        } => {
            // Sticky, and worded as a state: some steps really did run, and
            // each of them is its own journal entry the browser can show.
            state.status = Status::sticky_keys(
                format!(
                    "revert stopped after {done} of {total} {}: {error}",
                    if total == 1 { "step" } else { "steps" }
                ),
                journal_keys(&state.keymap),
            );
            if done > 0 {
                refresh(state, Refresh::Everything, next_id)
            } else {
                Vec::new()
            }
        }
    };
    // Last: a `q y` waiting on this jump quits only once the answer is in
    // place, and only when every step ran (m8/13b).
    super::super::quit::write_ended(state, id, every_step_ran);
    commands
}

/// The time machine's own drift box. Same rail, same words, but `[ Revert ]`
/// re-runs the jump rather than one `u`.
///
/// `total` is the jump's step count. The box only ever comes from the
/// jump's first step (a later step's drift stops it part-way), and `U`
/// walks the jump back from its last step, so bringing back what step 0
/// replaces takes all `total` presses.
fn revert_drift_box(
    description: &str,
    drift: &mergewright_ops::undo::Drift,
    total: usize,
    target: mergewright_core::journal::EntryId,
) -> PendingConfirm {
    PendingConfirm {
        view: mergewright_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("Revert to before \u{201c}{description}\u{201d}?")),
            with_consequence(
                wrapped(&format!("Since then, {}.", drift.explain())),
                drift,
                false,
                total,
            ),
            sanitize_line("revert restores the recorded state, step by step"),
        )
        .with_labels("Stop", "Revert"),
        then: ConfirmThen::RevertTo {
            entry: target,
            description: description.to_owned(),
        },
    }
}

/// The drift guard's box: what the operation was, what has moved since, and
/// the safe answer under the cursor.
///
/// The consequence line is `drift.explain()` verbatim — `ops` names which of
/// the three comparisons failed, and naming them again here would let the two
/// wordings drift apart.
pub(super) fn drift_box(
    description: &str,
    drift: &mergewright_ops::undo::Drift,
    redo: bool,
) -> PendingConfirm {
    let verb = if redo { "Redo" } else { "Undo" };
    PendingConfirm {
        view: mergewright_ui::confirm::ConfirmView::new(
            sanitize_line(&format!("{verb} \u{201c}{description}\u{201d}?")),
            with_consequence(
                wrapped(&format!("Since this {description}, {}.", drift.explain())),
                drift,
                redo,
                1,
            ),
            // No single git command to show: a restore is a plan of several,
            // and the journal browser's `Space` is where they are listed.
            sanitize_line(&format!(
                "{} restores the recorded state",
                verb.to_lowercase()
            )),
        )
        .with_labels("Stop", verb),
        then: ConfirmThen::Undo { redo },
    }
}
