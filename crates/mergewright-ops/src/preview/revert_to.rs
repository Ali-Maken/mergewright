//! The time machine: what reverting to just before an entry would do (§6.6).

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// What reverting to just before `target` would do (§6.6).
///
/// Drawn from the very plans [`crate::undo::revert_to`] would run, so the
/// picture and the act cannot disagree: each ref's `from` is the value the
/// newest step will compare against, and its `to` is the value the oldest
/// step restores. No ghosts — nothing is created, only put back — and the
/// working tree comes from the journal's backups rather than from a diff.
///
/// # Errors
/// [`PreviewError::Refused`] when there is nothing to revert, or an entry in
/// the range cannot be inverted; the reason is [`NotInvertible`]'s own.
pub fn plan_revert_to(
    entries: &[JournalEntry],
    target: &EntryId,
) -> Result<PreviewPlan, PreviewError> {
    let plans = inverse_plans(entries, target).map_err(|why| not_invertible(&why))?;
    let description = match entries.iter().find(|entry| entry.id == *target) {
        Some(entry) => format!(
            "revert to before \"{}\" ({} steps)",
            entry.description,
            plans.len()
        ),
        None => format!("revert {} steps", plans.len()),
    };
    Ok(PreviewPlan {
        description,
        ref_moves: ref_moves_of(&plans),
        head: head_of(&plans),
        worktree: WorktreeEffect::RestoredFromBackup,
        ..blank()
    })
}

/// One [`RefMove`] per ref the steps touch: `from` is what the newest step
/// expects to find, `to` what the oldest step puts back. Steps run newest
/// first, so the first mention of a ref gives `from` and the last gives
/// `to`.
pub(super) fn ref_moves_of(plans: &[InversePlan]) -> Vec<RefMove> {
    let mut moves: Vec<RefMove> = Vec::new();
    for step in plans.iter().flat_map(|plan| plan.steps.iter()) {
        let Step::UpdateRef { name, expect, to } = step else {
            continue;
        };
        match moves.iter_mut().find(|held| held.name == *name) {
            Some(held) => held.to = to.map(Target::Real),
            None => moves.push(RefMove {
                name: name.clone(),
                from: *expect,
                to: to.map(Target::Real),
            }),
        }
    }
    moves
}

/// Where `HEAD` ends up: the oldest step that moves it wins, since the
/// steps run newest first.
pub(super) fn head_of(plans: &[InversePlan]) -> Option<HeadMove> {
    plans
        .iter()
        .flat_map(|plan| plan.steps.iter())
        .filter_map(|step| match step {
            Step::SetHead(HeadRef::Branch(name) | HeadRef::Unborn(name)) => {
                Some(HeadMove::ToBranch(name.clone()))
            }
            Step::SetHead(HeadRef::Detached(oid)) => Some(HeadMove::Detached(Target::Real(*oid))),
            _ => None,
        })
        .next_back()
}

pub(super) fn not_invertible(why: &NotInvertible) -> PreviewError {
    PreviewError::Refused(Refusal::CannotRevertTo(match why {
        NotInvertible::Pending => "an operation in the range never finished".to_owned(),
        NotInvertible::NoBackup => "an operation in the range has no backup".to_owned(),
        NotInvertible::NotUndoable(kind) => {
            format!("{} cannot be undone", crate::apply::kind_name(*kind))
        }
        NotInvertible::NothingToUndo | NotInvertible::NothingToRedo => {
            "there is nothing to revert".to_owned()
        }
        NotInvertible::Incomplete { field } => {
            format!("the journal never recorded {field}")
        }
        // The planner never answers this — only the executor's gate does,
        // reading git's state (m6-conflicts/57) — but the words stay true if
        // that changes.
        NotInvertible::OperationLive { kind } => crate::undo::operation_live_words(*kind),
    }))
}
