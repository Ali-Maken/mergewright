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

mod rollback;
mod steps;

use std::path::Path;

use graft_core::journal::{EntryStatus, HeadRef, IndexState};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::apply::{AfterState, Applied, Context, Refresh, begin, finish, reason_for};
use crate::facts::FactsError;
use crate::journal::JournalError;
use crate::snapshot::SnapshotError;
use crate::undo::{InversePlan, Replaced, Step};

use rollback::{Before, Cancelled, roll_back_after_stop};
use steps::run_step;

/// Why a restore could not be made, or could not finish.
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
    #[error("{}", journal_failed(.error, .after.as_deref()))]
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
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
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
    #[error("undo cancelled part-way · {} · U restores the backup", rollback_what(.rollback))]
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
    /// git's merge files could not be put back, or git does not see the
    /// merge they describe afterwards ([`Step::SetMergeState`],
    /// m6-conflicts/20).
    #[error("{0}")]
    MergeState(crate::merge_state::MergeStateError),
}

impl RestoreError {
    /// One line for the status line.
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

/// A roll-back failure in the words of a roll-back.
///
/// [`RestoreError::RefMoved`]'s own message ends "nothing was changed", which
/// is true of a plan that stopped at its first compare-and-swap and false
/// here: by the time a roll-back refuses, the steps it could not reverse are
/// in force.
fn rollback_what(why: &RestoreError) -> String {
    match why {
        RestoreError::RefMoved { name } => format!("{name} is not where the undo left it"),
        other => other.to_string(),
    }
}

/// The journal failing, with the plan's own failure when there was one.
fn journal_failed(error: &JournalError, after: Option<&RestoreError>) -> String {
    match after {
        Some(why) => format!("{error} (the restore had already failed: {why})"),
        None => error.to_string(),
    }
}

/// Every after-state failure on one line.
fn after_state(unread: &[AfterState]) -> String {
    unread
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

impl From<crate::apply::PrepareError> for RestoreError {
    fn from(why: crate::apply::PrepareError) -> Self {
        use crate::apply::PrepareError;
        match why {
            PrepareError::Facts(why) => RestoreError::Facts(why),
            PrepareError::Snapshot(why) => RestoreError::Snapshot(why),
            PrepareError::Journal(why) => RestoreError::Journal {
                error: why,
                after: None,
            },
            PrepareError::Stopped => RestoreError::Stopped,
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
        return Err(RestoreError::Stopped);
    }
    let mut pending = begin(
        ctx,
        ctx.next_id(),
        plan.kind,
        plan.description.clone(),
        plan.touches.clone(),
        plan.pins.clone(),
        Some(plan.reverses.clone()),
    )?;

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
        merge: pending.entry.merge_before.clone(),
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

/// How a plan's steps ended.
enum Ran {
    /// Every step ran.
    Done,
    /// A step failed, and nothing after it ran.
    Failed(RestoreError),
    /// The caller asked to stop, and the roll-back has already run.
    Cancelled(Cancelled),
}

/// Run the plan's steps, stopping at the first failure — or, on a stop,
/// putting back what already ran before returning.
fn run_plan(
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
            Err(RestoreError::Stopped) => {
                cancelled = true;
                stopped_in = Some(step);
                break;
            }
            Err(why) => return Ran::Failed(why),
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

/// The entry's outcome line, and the error the caller gets.
fn outcome_of(ran: Ran) -> (EntryStatus, Option<RestoreError>) {
    match ran {
        Ran::Done => (EntryStatus::Committed, None),
        Ran::Failed(why) => (
            EntryStatus::Aborted {
                reason: aborted_reason(&why),
            },
            Some(why),
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

fn aborted_reason(why: &RestoreError) -> String {
    match why {
        RestoreError::Run(run) => reason_for(run),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use graft_core::journal::HeadRef;

    use super::refresh_after;
    use crate::apply::Refresh;

    fn branch(name: &str) -> HeadRef {
        HeadRef::Branch(format!("refs/heads/{name}"))
    }

    /// The scope rule as a table (m6-conflicts/79): any ref, a `SetHead`
    /// step, a recorded `HEAD` move, or an unread `HEAD` re-reads the graph;
    /// only a restore that provably left refs and `HEAD` alone does not.
    #[test]
    fn refresh_covers_head_whenever_the_restore_moves_it() {
        let main = branch("main");
        let feature = branch("feature");
        let rows: [(bool, bool, Option<&HeadRef>, Refresh, &str); 6] = [
            (
                false,
                false,
                Some(&main),
                Refresh::WorkingTree,
                "nothing moved",
            ),
            (true, false, Some(&main), Refresh::Everything, "a ref moved"),
            (
                false,
                true,
                Some(&main),
                Refresh::Everything,
                "the plan sets HEAD",
            ),
            (
                false,
                false,
                Some(&feature),
                Refresh::Everything,
                "HEAD recorded moved",
            ),
            (false, false, None, Refresh::Everything, "HEAD after unread"),
            (true, true, Some(&feature), Refresh::Everything, "all of it"),
        ];
        for (refs, sets_head, after, want, why) in rows {
            assert_eq!(refresh_after(refs, sets_head, &main, after), want, "{why}");
        }
    }
}
