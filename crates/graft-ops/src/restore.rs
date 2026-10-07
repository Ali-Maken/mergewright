//! Running an [`InversePlan`]: the steps that put the repository back.
//!
//! A restore is itself an operation, so it goes through the same protocol
//! (§6.4, "undo snapshots before it restores"): its own entry, its own
//! backup taken *now*, its own outcome line. An edit made after the operation
//! and before `u` moved no ref, but it is still someone's work, and the fresh
//! snapshot is what makes "never lose work" hold for it (ADR-0009).
//!
//! Steps run in the order the planner put them in — refs, `HEAD`, index,
//! working tree, stash reflog. Refs first is deliberate: a compare-and-swap
//! that fails stops the plan before a single file has been written, so a
//! moved ref leaves the working tree exactly as it was.
//!
//! The stop is checked **before every step**, and a stop rolls back what
//! already ran (ADR-0037 §6). An operation that cannot be cancelled is the
//! opposite of what undo is for, and a half-applied undo is worse than no
//! undo at all — so cancelling is all-or-nothing, put back from the entry's
//! own before-fields and its own backup ref, both taken before the first
//! step. Where it cannot be, the entry says so and the backup ref still
//! holds everything. That half is the `rollback` module; the
//! `git` each step actually runs is the `steps` module.

mod outcome;
mod raw;
mod ref_names;
mod rollback;
mod steps;

use std::path::Path;

use graft_core::journal::{EntryStatus, HeadRef, IndexState};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::run::RunError;

use crate::apply::{AfterState, Applied, Context, Opening, Refresh, begin, finish, reason_for};
use crate::facts::FactsError;
use crate::journal::JournalError;
use crate::keys::{KeyNames, NoKeys, OpKey, clause, then};
use crate::snapshot::SnapshotError;
use crate::undo::{InversePlan, Replaced, Step};

pub(crate) use raw::{same_content_paths, write_raw};
use rollback::{Before, Cancelled, roll_back_after_failure, roll_back_after_stop};
use steps::run_step;

use outcome::{
    after_state, cancelled_partway_words, failed_partway_words, journal_failed, outcome_of,
    run_plan,
};
use ref_names::check_ref_names;

/// Why a restore could not be made, or could not finish.
///
/// A stop leaves a restore as **one shape** (`KNOWN_GAPS` :495, planner
/// ruling m7/01b): [`RestoreError::Cancelled`] — `rolled_back: 0` when it
/// came before any step ran — or [`RestoreError::CancelledPartway`]. Inside,
/// every stop (git's, the backup's, a facts read's) is folded into the
/// crate-private `STOPPED` where it enters, so the cancel path matches one
/// arm; it never leaves this module as that.
#[derive(Debug, thiserror::Error)]
pub enum RestoreError {
    /// A ref is not where the journal said it was, so the plan stopped
    /// before touching anything else.
    #[error("{name} is not where it was; nothing was changed")]
    RefMoved {
        /// The ref that moved.
        name: String,
    },
    /// The stash commit the plan wanted to put back no longer exists.
    #[error("the stashed commit {commit} is gone")]
    StashGone {
        /// The commit that could not be read.
        commit: Oid,
    },
    /// A `git` command failed.
    #[error("{0}")]
    Run(RunError),
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// The restore's own backup could not be taken, so it did not run.
    #[error("{0}")]
    Snapshot(SnapshotError),
    /// The journal could not be written. When the plan had already failed,
    /// `after` carries that failure too, so it is not lost behind this one
    /// (m3/03 review, Minor 1).
    #[error("{}", journal_failed(.error, .after.as_deref(), &NoKeys))]
    Journal {
        /// What the journal said.
        error: JournalError,
        /// What the plan's own failure was, when it had one.
        after: Option<Box<RestoreError>>,
    },
    /// The plan ran and is journaled, but part of what it left behind could
    /// not be read or kept reachable. `applied` carries the result, so the
    /// caller still refreshes.
    #[error("{}: {}", .applied.entry.description, after_state(.unread))]
    AfterState {
        /// What ran.
        applied: Box<Applied>,
        /// What could not be read or sealed.
        unread: Vec<AfterState>,
    },
    /// The caller asked to stop, and every step that had already run was put
    /// back: the repository stands exactly where it did before the restore
    /// began, and the entry is `Aborted` and not in force.
    #[error("undo cancelled · nothing changed")]
    Cancelled {
        /// How many steps were reversed.
        rolled_back: usize,
    },
    /// The caller asked to stop, and the roll-back could not finish. What
    /// `applied` names is still in force; the entry's own backup ref holds
    /// the state the restore started from, so `U` on it gets everything back.
    ///
    /// `U`, not `u`. This entry is an `Undo` entry and it *is* in force (it
    /// has a backup and its after-state differs from its before-state), so
    /// `journal::next_undo` steps over it — `u` would reach an older
    /// operation, or nothing, and this entry's target now counts as
    /// reversed. `journal::next_redo` is what reaches it, and its inverse is
    /// the `Redo` plan `undo::inverse` labels "(after a failed undo)": it
    /// puts back the state the cancelled undo started from. `ApplyError`'s
    /// forward-operation wording keeps `u`, correctly — there the aborted
    /// entry is an operation, so `next_undo` does reach it.
    #[error("{}", cancelled_partway_words(.rollback, &NoKeys))]
    CancelledPartway {
        /// What the roll-back could not put back, newest first.
        applied: Vec<String>,
        /// Why it stopped.
        rollback: Box<RestoreError>,
    },
    /// The roll-back had nothing to put a field back from: the entry's own
    /// before-state does not record it. It is a ref's case now: an unmerged
    /// index used to be the real one, and since m6-conflicts/08 it has a
    /// recorded state of its own (`IndexState::Unmerged`) rather than a gap.
    #[error("the restore's own backup does not hold {field}, so it could not be put back")]
    NoBefore {
        /// Which field: a ref name, `HEAD`, `the index`, `the working tree`.
        field: String,
    },
    /// A step failed, and the roll-back of what had already run could not
    /// finish. What `applied` names is still in force; the entry's own backup
    /// ref holds the state the restore started from, so `U` gets it back
    /// (`U`, not `u`, for the reason [`RestoreError::CancelledPartway`]
    /// gives).
    #[error("{}", failed_partway_words(.failure, .applied, .rollback, &NoKeys))]
    FailedPartway {
        /// The step's own failure: the reason the restore did not finish.
        failure: Box<RestoreError>,
        /// What the roll-back could not put back, newest first.
        applied: Vec<String>,
        /// Why the roll-back stopped.
        rollback: Box<RestoreError>,
    },
    /// A ref name in the plan is not one git's `check-ref-format` rules
    /// accept, or is not a full name under `refs/`. Checked before the
    /// restore writes anything — its journal entry and backup included.
    #[error("refused: \"{name}\" is not a ref name git accepts ({reason}) · nothing was changed")]
    BadRefName {
        /// The name, escaped for display.
        name: String,
        /// Which rule it broke, in words.
        reason: String,
    },
    /// git's merge files could not be put back, or git does not see the
    /// merge they describe afterwards ([`Step::SetMergeState`],
    /// m6-conflicts/20).
    #[error("{0}")]
    MergeState(crate::merge_state::MergeStateError),
    /// git wrote `path` back, but his own bytes (the snapshot's `raw/`,
    /// ADR-0052) could not be written over it: git's version is what is on
    /// disk there, and the backup still holds his.
    #[error("could not write {} back as it was: {source}", .path.display())]
    WriteRaw {
        /// The path, relative to the working tree's root.
        path: std::path::PathBuf,
        /// What the filesystem said.
        source: std::io::Error,
    },
}

impl RestoreError {
    /// One line for the status line, its keys spelled through `keys` (an
    /// unbound one's clause left out). The `Display` is these words with
    /// [`NoKeys`].
    #[must_use]
    pub fn explain(&self, keys: &dyn KeyNames) -> String {
        match self {
            RestoreError::Journal { error, after } => journal_failed(error, after.as_deref(), keys),
            RestoreError::CancelledPartway { rollback, .. } => {
                cancelled_partway_words(rollback, keys)
            }
            RestoreError::FailedPartway {
                failure,
                applied,
                rollback,
            } => failed_partway_words(failure, applied, rollback, keys),
            other => other.to_string(),
        }
    }
}

impl RestoreError {
    /// The one shape a stop takes: git's own stop, as every `git` run
    /// reports it.
    pub(crate) const STOPPED: RestoreError = RestoreError::Run(RunError::Stopped);

    /// A stop that came before any step ran: nothing changed.
    pub(crate) const CANCELLED_BEFORE_ANY_STEP: RestoreError =
        RestoreError::Cancelled { rolled_back: 0 };

    /// A facts read's failure, the stop kept as [`Self::STOPPED`].
    pub(crate) fn from_facts(why: FactsError) -> Self {
        match why {
            FactsError::Stopped => Self::STOPPED,
            other => Self::Facts(other),
        }
    }
}

impl From<crate::apply::PrepareError> for RestoreError {
    fn from(why: crate::apply::PrepareError) -> Self {
        use crate::apply::PrepareError;
        match why {
            PrepareError::Facts(why) => RestoreError::from_facts(why),
            PrepareError::Snapshot(SnapshotError::Stopped) | PrepareError::Stopped => {
                RestoreError::STOPPED
            }
            PrepareError::Snapshot(why) => RestoreError::Snapshot(why),
            PrepareError::Journal(why) => RestoreError::Journal {
                error: why,
                after: None,
            },
        }
    }
}

/// Put one path's recorded index stages back, and nothing else.
///
/// The narrow half of [`Step::RestoreIndex`], for the one caller that is not
/// undoing anything: `d` on a path `⏎` already staged
/// ([`crate::apply::stage_source`], m6-conflicts/08 scope 5(c)). The stages
/// have to be in the index before `git checkout --merge` has anything to
/// merge, and `plan()` returns [`crate::operation::Invocation`]s — a pure
/// description of what git will be asked to do, with nowhere to put a
/// recorded tree or the stdin it needs. Growing `Invocation` to carry stdin
/// would let every later operation smuggle state through its plan, so the one
/// operation that needs a write `plan()` cannot describe asks for it here
/// instead, and `plan()` stays pure.
///
/// No journal line, no entry, no backup: the caller is inside an operation of
/// its own and has already taken both.
///
/// Returns the command lines run, for the Command Log.
///
/// # Errors
/// [`SnapshotError`] — the stop, or a `git` that would not run. It is exactly
/// the error set of the two commands this runs.
pub fn restore_stages(
    ctx: &mut Context<'_>,
    index: IndexState,
    path: &Path,
) -> Result<Vec<String>, SnapshotError> {
    let mut commands = Vec::new();
    let paths = [path.to_path_buf()];
    steps::put_stages_back(ctx, index, &paths, &mut commands)?;
    Ok(commands)
}

/// Run a plan, journaled as its own entry.
pub fn execute(ctx: &mut Context<'_>, plan: &InversePlan) -> Result<Applied, RestoreError> {
    execute_over_drift(ctx, plan, Vec::new(), Replaced::default())
}

/// [`execute`], for an undo the user forced past drift (m6-conflicts/77).
///
/// `reads` are the command lines of the read that named what the run
/// replaces (`undo::replaced`); they head the entry's Command Log, because
/// they ran for this undo, as the restore's own scope reads after them do
/// (the index's too since m6-conflicts/80). `replaced`
/// rides on the [`Applied`] — the `AfterState` one too — so the landing can
/// name the key that brings it back.
pub(crate) fn execute_over_drift(
    ctx: &mut Context<'_>,
    plan: &InversePlan,
    reads: Vec<String>,
    replaced: Replaced,
) -> Result<Applied, RestoreError> {
    if ctx.stop.should_stop() {
        return Err(RestoreError::CANCELLED_BEFORE_ANY_STEP);
    }
    // Before `begin`: a refused name leaves no entry and no backup.
    check_ref_names(plan)?;
    let mut pending = begin(ctx, ctx.next_id(), opening_of(plan)).map_err(cancel_if_stopped)?;

    // The entry's own before-fields, read by `begin` before the first step
    // and backed by the backup ref it took at the same moment (§6.4). This,
    // not the plan, is what a roll-back puts things back to: the plan says
    // where the *operation* left the repository, this says where the
    // *restore* found it.
    let before = Before {
        refs: pending.entry.refs_before.clone(),
        head: pending.entry.head_before.clone(),
        index: pending.entry.index_before,
        worktree: pending.entry.worktree_before,
        worktree_raw: pending.entry.worktree_raw_before,
        merge: pending.entry.merge_before.clone(),
        config: pending.entry.config_before.clone(),
    };

    let mut commands = reads;
    let (status, outcome) = outcome_of(run_plan(ctx, plan, &before, &mut commands));
    let unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            return Err(RestoreError::Journal {
                error,
                after: outcome.map(Box::new),
            });
        }
    };

    match outcome {
        None => {
            let refresh = refresh_after(
                !pending.entry.touches.refs.is_empty(),
                plan.steps
                    .iter()
                    .any(|step| matches!(step, Step::SetHead(_))),
                &pending.entry.head_before,
                pending.entry.head_after.as_ref(),
            );
            let applied = Applied {
                came_back: Vec::new(),
                stopped_at: None,
                entry: pending.entry,
                refresh,
                step_backup: None,
                replaced,
            };
            if unread.is_empty() {
                Ok(applied)
            } else {
                Err(RestoreError::AfterState {
                    applied: Box::new(applied),
                    unread,
                })
            }
        }
        // `finish` has put the after-state failures into the entry's
        // `Aborted` reason already; the plan's own failure — or the cancel —
        // is the headline.
        Some(why) => Err(why),
    }
}

/// `begin`'s failure, a stop in it a cancel before any step: `begin`
/// journals what it wrote itself, and nothing here invents an entry
/// (`KNOWN_GAPS` :495, planner ruling m7/01b).
fn cancel_if_stopped(why: crate::apply::PrepareError) -> RestoreError {
    match RestoreError::from(why) {
        RestoreError::Run(RunError::Stopped) => RestoreError::CANCELLED_BEFORE_ANY_STEP,
        other => other,
    }
}

/// The entry a restore opens: the plan's own kind, words and reach.
fn opening_of(plan: &InversePlan) -> Opening {
    Opening {
        kind: plan.kind,
        description: plan.description.clone(),
        touches: plan.touches.clone(),
        pins: plan.pins.clone(),
        reverses: Some(plan.reverses.clone()),
        put_back: None,
    }
}

/// What the caller re-reads after a restore ran: the graph as well whenever a
/// ref moved **or `HEAD` did** (m6-conflicts/79).
///
/// `touches.refs` alone is not enough: a checkout's inverse moves only
/// `HEAD` — its `touches` names no ref, by design (`CheckoutParams`) — and a
/// working-tree re-read left the graph's HEAD badge on the branch the undo
/// had just left. `HEAD` counts as moved when the plan sets it, or when the
/// entry's recorded after-value differs from its before-value; an after-value
/// that could not be read is not known to be unmoved, so it re-reads too.
fn refresh_after(
    refs_touched: bool,
    sets_head: bool,
    head_before: &HeadRef,
    head_after: Option<&HeadRef>,
) -> Refresh {
    let head_moved = sets_head || head_after != Some(head_before);
    if refs_touched || head_moved {
        Refresh::Everything
    } else {
        Refresh::WorkingTree
    }
}

#[cfg(test)]
mod tests;
