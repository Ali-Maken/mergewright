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

use crate::apply::{Applied, Context};
use crate::facts::{self, FactsError};
use crate::restore::{self, RestoreError};
use crate::undo::{Drift, InversePlan, NotInvertible, drift, plan_redo, plan_revert_to, plan_undo};

/// Why an undo or redo did not happen.
#[derive(Debug, thiserror::Error)]
pub enum UndoError {
    /// There is nothing to undo, or the entry cannot be inverted.
    #[error("{}", nothing_to_say(.0))]
    Nothing(NotInvertible),
    /// The repository moved since the operation. The user decides.
    #[error("{}", .0.explain())]
    Drift(Drift),
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
    /// One line for the status line.
    pub fn explain(&self) -> String {
        match self {
            UndoError::Drift(drift) => format!("{} — u again to go ahead", drift.explain()),
            other => other.to_string(),
        }
    }
}

/// Take back the newest operation still in force.
pub fn undo(ctx: &mut Context<'_>, force: bool) -> Result<Applied, UndoError> {
    reverse(ctx, force, plan_undo)
}

/// Put back the newest undo still in force.
pub fn redo(ctx: &mut Context<'_>, force: bool) -> Result<Applied, UndoError> {
    reverse(ctx, force, plan_redo)
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
                error: Box::new(UndoError::Restore(RestoreError::Stopped)),
            });
        }
        match run_plan(ctx, plan, force) {
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
    plan_of: fn(&[JournalEntry]) -> Result<InversePlan, NotInvertible>,
) -> Result<Applied, UndoError> {
    let plan = plan_of(ctx.journal.entries()).map_err(UndoError::Nothing)?;
    run_plan(ctx, &plan, force)
}

/// §6.7's rail, then the restore: the entry the plan reverses is compared
/// with the repository as it is now, and only the user's own `force` goes
/// past a difference.
fn run_plan(ctx: &mut Context<'_>, plan: &InversePlan, force: bool) -> Result<Applied, UndoError> {
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
        .map_err(UndoError::Facts)?;

    let moved = drift(&target, &now);
    if !moved.is_clean() && !force {
        return Err(UndoError::Drift(moved));
    }
    restore::execute(ctx, plan).map_err(UndoError::Restore)
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
    }
}
