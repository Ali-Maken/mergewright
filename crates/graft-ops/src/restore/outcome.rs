//! How a restore's steps ended, and the words its failures and stops take.

#[allow(clippy::wildcard_imports)]
use super::*;

/// A roll-back failure in the words of a roll-back.
///
/// [`RestoreError::RefMoved`]'s own message ends "nothing was changed", which
/// is true of a plan that stopped at its first compare-and-swap and false
/// here: by the time a roll-back refuses, the steps it could not reverse are
/// in force.
pub(super) fn rollback_what(why: &RestoreError) -> String {
    match why {
        RestoreError::RefMoved { name } => format!("{name} is not where the undo left it"),
        other => other.to_string(),
    }
}

/// The journal failing, with the plan's own failure when there was one.
pub(super) fn journal_failed(
    error: &JournalError,
    after: Option<&RestoreError>,
    keys: &dyn KeyNames,
) -> String {
    match after {
        Some(why) => format!(
            "{error} (the restore had already failed: {})",
            why.explain(keys)
        ),
        None => error.to_string(),
    }
}

/// [`RestoreError::CancelledPartway`]'s words: `undo cancelled part-way ·
/// <what> · U restores the backup`.
pub(super) fn cancelled_partway_words(rollback: &RestoreError, keys: &dyn KeyNames) -> String {
    then(
        format!("undo cancelled part-way · {}", rollback_what(rollback)),
        [restores_the_backup(keys)],
    )
}

/// [`RestoreError::FailedPartway`]'s words: `<failure> · rolled back
/// part-way · <applied> still changed: <what> · U restores the backup`.
pub(super) fn failed_partway_words(
    failure: &RestoreError,
    applied: &[String],
    rollback: &RestoreError,
    keys: &dyn KeyNames,
) -> String {
    then(
        format!(
            "{} · rolled back part-way · {} still changed: {}",
            failure.explain(keys),
            applied.join(", "),
            rollback_what(rollback)
        ),
        [restores_the_backup(keys)],
    )
}

/// `U restores the backup` — `U`, not `u`, for the reason
/// [`RestoreError::CancelledPartway`] gives.
fn restores_the_backup(keys: &dyn KeyNames) -> Option<String> {
    clause(keys, OpKey::Redo, "restores the backup")
}

/// Every after-state failure on one line.
pub(super) fn after_state(unread: &[AfterState]) -> String {
    unread
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// How a plan's steps ended.
pub(super) enum Ran {
    /// Every step ran.
    Done,
    /// A step failed, nothing after it ran, and the roll-back of what had
    /// already run has too (gap :496). The error is the step's own.
    Failed(RestoreError, Cancelled),
    /// The caller asked to stop, and the roll-back has already run.
    Cancelled(Cancelled),
}

/// The entry's outcome line, and the error the caller gets.
pub(super) fn outcome_of(ran: Ran) -> (EntryStatus, Option<RestoreError>) {
    match ran {
        Ran::Done => (EntryStatus::Committed, None),
        // Put back whole: the after-state `finish` reads equals the
        // before-state, so the entry is not in force, and the step's own
        // failure is the whole story.
        Ran::Failed(why, Cancelled::Clean(_)) => (
            EntryStatus::Aborted {
                reason: aborted_reason(&why),
            },
            Some(why),
        ),
        // Half a restore is in force. Never silent: the entry and the error
        // both name what is left, and the backup ref keeps everything.
        Ran::Failed(
            why,
            Cancelled::Partway {
                applied,
                why: rollback,
            },
        ) => (
            EntryStatus::Aborted {
                reason: format!(
                    "{} — rollback failed: {}",
                    aborted_reason(&why),
                    rollback_what(&rollback)
                ),
            },
            Some(RestoreError::FailedPartway {
                failure: Box::new(why),
                applied,
                rollback: Box::new(rollback),
            }),
        ),
        // The repository is back where the restore found it, so the entry's
        // after-state — read fresh by `finish` — equals its before-state and
        // `journal::in_force` says this entry is not in force: it shadows
        // nothing, and `next_undo` still points at the entry `u` was about
        // to reverse.
        Ran::Cancelled(Cancelled::Clean(rolled_back)) => (
            EntryStatus::Aborted {
                reason: "cancelled".to_owned(),
            },
            Some(RestoreError::Cancelled { rolled_back }),
        ),
        // The after-state here is real and possibly half-way, which is the
        // point: `in_force` says so, `drift` explains it, and the backup ref
        // keeps everything.
        Ran::Cancelled(Cancelled::Partway { applied, why }) => (
            EntryStatus::Aborted {
                reason: format!("cancelled — rollback failed: {}", rollback_what(&why)),
            },
            Some(RestoreError::CancelledPartway {
                applied,
                rollback: Box::new(why),
            }),
        ),
    }
}

pub(super) fn aborted_reason(why: &RestoreError) -> String {
    match why {
        RestoreError::Run(run) => reason_for(run),
        other => other.to_string(),
    }
}
/// Run the plan's steps, stopping at the first failure or stop, and putting
/// back what already ran before returning.
pub(super) fn run_plan(
    ctx: &mut Context<'_>,
    plan: &InversePlan,
    before: &Before,
    commands: &mut Vec<String>,
) -> Ran {
    let mut cancelled = false;
    let mut stopped_in = None;
    let mut applied: Vec<&Step> = Vec::new();
    for step in &plan.steps {
        // Before every step, not only the first: between two steps is the
        // one place a plan can be stopped and still be put back exactly.
        if ctx.stop.should_stop() {
            cancelled = true;
            break;
        }
        match run_step(ctx, step, commands) {
            Ok(()) => applied.push(step),
            // The runner SIGTERMed git part-way through this step, so
            // whether it landed is not knowable from the error.
            Err(RestoreError::Run(RunError::Stopped)) => {
                cancelled = true;
                stopped_in = Some(step);
                break;
            }
            Err(why) => {
                let rolled = roll_back_after_failure(ctx, before, &mut applied, step, commands);
                return Ran::Failed(why, rolled);
            }
        }
    }
    if cancelled {
        Ran::Cancelled(roll_back_after_stop(
            ctx,
            before,
            &mut applied,
            stopped_in,
            commands,
        ))
    } else {
        Ran::Done
    }
}
