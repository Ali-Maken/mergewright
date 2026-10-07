//! `u` and `U`: plan the inverse, check §6.7's rails, run it.
//!
//! The planning is `undo`'s own and pure; this is the half that reads the
//! repository and writes to it. The rail between them is `drift`: before an
//! undo touches anything, the state the entry says the operation left behind
//! is compared against what is actually there — refs, the index and the
//! working tree, because a plain edit moves no ref. Anything that differs, or
//! anything the entry never recorded, stops the undo and is explained; only
//! the user's own `force` goes past it.

use graft_core::journal::{EntryId, JournalEntry};
use graft_git::state::InProgress;

use crate::apply::{Applied, Context};
use crate::facts::{self, FactsError};
use crate::keys::{KeyNames, OpKey};
use crate::restore::{self, RestoreError};
use crate::undo::{
    Drift, InversePlan, NotInvertible, RepoStateNow, drift, plan_redo, plan_revert_to, plan_undo,
    replaced,
};

/// Which way a reversal runs: `u` takes an operation back, `U` puts an
/// undo back. The drift line's "again" names the key that goes ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// `u`, and every step of the time machine.
    Undo,
    /// `U`.
    Redo,
}

/// Why an undo or redo did not happen.
#[derive(Debug, thiserror::Error)]
pub enum UndoError {
    /// There is nothing to undo, or the entry cannot be inverted.
    #[error("{}", nothing_to_say(.0))]
    Nothing(NotInvertible),
    /// The repository moved since the operation. The user decides.
    ///
    /// Boxed: with `Drift::replaced` (m6-conflicts/77) the drift is the
    /// largest thing any undo answer carries, and every `Result` in this
    /// module would pay its size on the happy path (`result_large_err`).
    #[error("{}", .drift.explain())]
    Drift {
        /// What moved.
        drift: Box<Drift>,
        /// Whether a `u` or a `U` met it, so the words name the key that
        /// goes ahead (`KNOWN_GAPS` :494).
        direction: Direction,
    },
    /// The restore itself failed.
    #[error("{0}")]
    Restore(RestoreError),
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// A [`revert_to`] stopped part-way. `done` is what it had already put
    /// back, newest first, each its own `Undo` entry — so the journal is
    /// true, `U` walks them back one at a time, and the caller can say how
    /// far it got instead of pretending nothing happened.
    #[error("{} of {} steps done, then: {error}", done.len(), done.len() + 1)]
    PartWay {
        /// The steps that ran, in the order they ran.
        done: Vec<Applied>,
        /// Why the next one did not.
        error: Box<UndoError>,
    },
}

impl UndoError {
    /// One line for the status line, its keys spelled through `keys` (an
    /// unbound one's clause left out). The `Display` is these words with
    /// [`NoKeys`](crate::keys::NoKeys), less the drift's `again` clause.
    #[must_use]
    pub fn explain(&self, keys: &dyn KeyNames) -> String {
        match self {
            UndoError::Drift { drift, direction } => {
                let key = match direction {
                    Direction::Undo => OpKey::Undo,
                    Direction::Redo => OpKey::Redo,
                };
                match keys.label(key) {
                    Some(key) => format!("{} — {key} again to go ahead", drift.explain()),
                    None => drift.explain(),
                }
            }
            UndoError::Restore(error) => error.explain(keys),
            UndoError::PartWay { done, error } => format!(
                "{} of {} steps done, then: {}",
                done.len(),
                done.len() + 1,
                error.explain(keys)
            ),
            other => other.to_string(),
        }
    }
}

/// Take back the newest operation still in force.
pub fn undo(ctx: &mut Context<'_>, force: bool) -> Result<Applied, UndoError> {
    reverse(ctx, force, Direction::Undo, plan_undo)
}

/// Put back the newest undo still in force.
pub fn redo(ctx: &mut Context<'_>, force: bool) -> Result<Applied, UndoError> {
    reverse(ctx, force, Direction::Redo, plan_redo)
}

/// The time machine (§6.6): put the repository back to just before `target`,
/// one journaled `Undo` entry per step, newest first.
///
/// Each step is a full [`restore::execute`] with its own backup and its own
/// entry, so `U` walks the revert back one operation at a time rather than
/// all at once — the journal stays a record of what happened, not a summary.
/// `force` is §6.7's rail, applied per step: the state each entry says it
/// left behind is compared with what is there now, and a difference stops
/// the revert unless the user has already said to go ahead.
///
/// The stop is checked between steps. Cancelling leaves the steps that ran
/// in place — they are journaled and undoable — rather than rolling them
/// back, which would be a second unasked-for write.
///
/// # Errors
/// [`UndoError::Nothing`] when there is nothing to revert; otherwise
/// [`UndoError::PartWay`] carrying what had already run and why the next
/// step did not.
pub fn revert_to(
    ctx: &mut Context<'_>,
    target: &EntryId,
    force: bool,
) -> Result<Vec<Applied>, UndoError> {
    let plans = plan_revert_to(ctx.journal.entries(), target).map_err(UndoError::Nothing)?;
    let mut done = Vec::new();
    for (step, plan) in plans.iter().enumerate() {
        if step > 0 {
            // One entry per step, so one id per step: a `Context` names one
            // entry until it is told to move on.
            ctx.advance();
        }
        if ctx.stop.should_stop() {
            return Err(UndoError::PartWay {
                done,
                error: Box::new(UndoError::Restore(RestoreError::CANCELLED_BEFORE_ANY_STEP)),
            });
        }
        match run_plan(ctx, plan, force, Direction::Undo) {
            Ok(applied) => done.push(applied),
            Err(error) => {
                return Err(UndoError::PartWay {
                    done,
                    error: Box::new(error),
                });
            }
        }
    }
    Ok(done)
}

fn reverse(
    ctx: &mut Context<'_>,
    force: bool,
    direction: Direction,
    plan_of: fn(&[JournalEntry]) -> Result<InversePlan, NotInvertible>,
) -> Result<Applied, UndoError> {
    let plan = plan_of(ctx.journal.entries()).map_err(UndoError::Nothing)?;
    run_plan(ctx, &plan, force, direction)
}

/// §6.7's rail, then the restore: the entry the plan reverses is compared
/// with the repository as it is now, and only the user's own `force` goes
/// past a difference.
fn run_plan(
    ctx: &mut Context<'_>,
    plan: &InversePlan,
    force: bool,
    direction: Direction,
) -> Result<Applied, UndoError> {
    let target = ctx
        .journal
        .entries()
        .iter()
        .find(|entry| entry.id == plan.reverses)
        .cloned()
        // `plan_of` built the plan from an entry of this journal, so it is
        // there; if it were not, undoing blind is the one thing never to do.
        .ok_or(UndoError::Nothing(NotInvertible::NothingToUndo))?;

    let mut names: Vec<String> = target.refs_before.keys().cloned().collect();
    for name in target.touches.refs.iter().chain(plan.touches.refs.iter()) {
        if !names.contains(name) {
            names.push(name.clone());
        }
    }
    // An unmerged index — what a stash pop that hit a conflict leaves — has
    // no tree, and `read_state_now` reports that as `index_tree: None` rather
    // than failing: `HEAD`, the refs and the working tree are all still
    // readable, and the user is owed all of it, not just "the index changed"
    // (m3/03 review, M5). The restore itself copes: its backup keeps the
    // working tree, conflict markers and all.
    let now = facts::read_state_now(ctx.repo, ctx.scratch(), &names, ctx.stop)
        .map_err(|why| facts_or_cancel(ctx.stop, why))?;

    // Before the rail: a live operation is not drift the user can force
    // past, it is a refusal. With it here, `drift`'s `MergeDrift::Begun`
    // is never reached from this path (m6-conflicts/57).
    refuse_while_live(ctx, &target, &now)?;

    let mut moved = drift(&target, &now);
    if moved.is_clean() {
        return restore::execute(ctx, plan).map_err(UndoError::Restore);
    }
    // Drift: name what the inverse would replace (m6-conflicts/77). Read
    // here and only here, so a clean undo pays nothing for it; asked again
    // on the forced run, because the tree may have moved since the box.
    let mut reads = Vec::new();
    moved.replaced = replaced::replaced(ctx, plan, &now, &mut reads)
        .map_err(|why| facts_or_cancel(ctx.stop, why))?;
    if !force {
        return Err(UndoError::Drift {
            drift: Box::new(moved),
            direction,
        });
    }
    restore::execute_over_drift(ctx, plan, reads, moved.replaced).map_err(UndoError::Restore)
}

/// The undo gate (m6-conflicts/57, decision 3): while git has an operation
/// in progress, an entry is reversed only when it was recorded **inside
/// that same merge** — its `merge_after` names the merge standing now (the
/// same `MERGE_HEAD` and the same other heads; the message and mode are
/// `drift`'s business, behind the force box). Anything else is
/// [`NotInvertible::OperationLive`]: nothing written, no drift box.
///
/// Every executor entry point comes through here — `undo`, `redo` and each
/// step of `revert_to` all run [`run_plan`] — so a redo needs no case of its
/// own: it reverses an `Undo` entry, whose `merge_after` is the merge that
/// undo put back.
///
/// A **bisect** is let through, as before this gate: it is not an operation
/// Graft ends (no banner, keymap §2.6a's "not covered by this rule"), and
/// the app sends `u` during one on purpose. Refusing it here would change a
/// behaviour nobody asked to change.
fn refuse_while_live(
    ctx: &Context<'_>,
    target: &JournalEntry,
    now: &RepoStateNow,
) -> Result<(), UndoError> {
    let live = graft_git::state::in_progress(ctx.repo)
        .map_err(|error| facts_or_cancel(ctx.stop, FactsError::from(error)))?;
    let kind = match live {
        None | Some(InProgress::Bisect) => return Ok(()),
        Some(kind) => kind,
    };
    let inside_this_merge = kind == InProgress::Merge
        && match (&target.merge_after, &now.merge) {
            (Some(then), Some(standing)) => {
                then.merge_head == standing.merge_head && then.other_heads == standing.other_heads
            }
            _ => false,
        };
    if inside_this_merge {
        Ok(())
    } else {
        Err(UndoError::Nothing(NotInvertible::OperationLive { kind }))
    }
}

fn nothing_to_say(why: &NotInvertible) -> String {
    match why {
        NotInvertible::Pending => {
            "that operation never finished; inspect it in the journal first".to_owned()
        }
        NotInvertible::NoBackup => {
            "no backup was taken for that operation, so it cannot be undone".to_owned()
        }
        NotInvertible::NotUndoable(kind) => {
            format!("{} cannot be undone", crate::apply::kind_name(*kind))
        }
        NotInvertible::NothingToUndo => "nothing to undo".to_owned(),
        NotInvertible::NothingToRedo => "nothing to redo".to_owned(),
        NotInvertible::Incomplete { field } => {
            format!(
                "the journal never recorded {field} after that operation, so it cannot be undone"
            )
        }
        // The app words this through `update::undo::live_refusal`, and a
        // graft test pins the two texts equal (`operation_live_words_match_ops`).
        NotInvertible::OperationLive { kind } => super::operation_live_words(*kind),
    }
}

/// A rail's read failure, a stop in it kept as what it is: the undo was
/// cancelled before any step ran, so nothing changed (`KNOWN_GAPS` :495,
/// planner ruling m7/01b). Nothing was journaled: the rails run before
/// `begin`.
///
/// The stop is asked, not only the error's shape: a stopped read surfaces
/// nested in many `FactsError` variants (`Snapshot(Stopped)`, a `RunError`
/// inside `History`, …), and a failure while the user has asked to stop is
/// the stop's doing — the stop latches, so asking again is exact.
fn facts_or_cancel(stop: &dyn graft_git::run::Stop, why: FactsError) -> UndoError {
    if matches!(why, FactsError::Stopped) || stop.should_stop() {
        UndoError::Restore(RestoreError::CANCELLED_BEFORE_ANY_STEP)
    } else {
        UndoError::Facts(why)
    }
}
