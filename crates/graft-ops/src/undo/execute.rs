//! `u` and `U`: plan the inverse, check §6.7's rails, run it.
//!
//! The planning is `undo`'s own and pure; this is the half that reads the
//! repository and writes to it. The rail between them is `drift`: before an
//! undo touches anything, the state the entry says the operation left behind
//! is compared against what is actually there — refs, the index and the
//! working tree, because a plain edit moves no ref. Anything that differs, or
//! anything the entry never recorded, stops the undo and is explained; only
//! the user's own `force` goes past it.

use graft_core::journal::JournalEntry;

use crate::apply::{Applied, Context};
use crate::facts::{self, FactsError};
use crate::restore::{self, RestoreError};
use crate::snapshot::SnapshotError;
use crate::undo::{Drift, InversePlan, NotInvertible, drift, plan_redo, plan_undo};

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

fn reverse(
    ctx: &mut Context<'_>,
    force: bool,
    plan_of: fn(&[JournalEntry]) -> Result<InversePlan, NotInvertible>,
) -> Result<Applied, UndoError> {
    let plan = plan_of(ctx.journal.entries()).map_err(UndoError::Nothing)?;
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
    let now = match facts::read_state_now(ctx.repo, &ctx.scratch.clone(), &names, ctx.stop) {
        Ok(now) => now,
        // An unmerged index — what a stash pop that hit a conflict leaves —
        // has no tree, so it cannot equal the one the entry recorded. That
        // is drift of the loudest kind, and the comparison has nothing to
        // add to it. The restore itself copes: its backup keeps the working
        // tree, conflict markers and all.
        Err(FactsError::Snapshot(SnapshotError::Unmerged)) => {
            if force {
                return restore::execute(ctx, &plan).map_err(UndoError::Restore);
            }
            return Err(UndoError::Drift(Drift {
                index: true,
                ..Drift::default()
            }));
        }
        Err(other) => return Err(UndoError::Facts(other)),
    };

    let moved = drift(&target, &now);
    if !moved.is_clean() && !force {
        return Err(UndoError::Drift(moved));
    }
    restore::execute(ctx, &plan).map_err(UndoError::Restore)
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
