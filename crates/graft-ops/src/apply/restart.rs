//! `d` after `⏎` (m6-conflicts/08 scope 5(c)): where a restart finds a
//! path's conflict stages once `git add` has dropped them from the index, the
//! gate asked of those stages before anything is written, and the write that
//! puts them back after the backup.

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, IndexState};
use graft_core::oid::Oid;
use graft_git::state::InProgress;

use crate::facts::FactsError;
use crate::operation::{Operation, Refusal, RepoFacts};
use crate::snapshot::{self, SnapshotError};

use super::{Applied, ApplyError, Context, Pending, as_apply_error, finish, resume};

/// Where the stages a restart needs are to be found.
///
/// `d` puts a conflicted file back the way git left it, and git's own
/// `checkout --merge` merges stages 1, 2 and 3 **out of the index**. After
/// `⏎` — which is `git add` — the path sits at stage 0 and the index has
/// none of them, so the stages have to come from somewhere before the plan
/// can run (m6-conflicts/08 scope 5(c)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageSource {
    /// The index still holds them: the plan alone does the job.
    Index,
    /// The index does not, but a recorded stage tree does. `tree` is an
    /// [`IndexState::Unmerged`]'s tree — a merged one has nothing to offer.
    Backup {
        /// The stage tree `{ 0/<path>, 1/<path>, 2/<path>, 3/<path> }`.
        tree: Oid,
    },
}

/// Where `d` can get `path`'s conflict stages from, or `None` when nobody
/// kept them.
///
/// Newest first, because the newest recorded conflict is the one the user
/// means by "start this file over":
///
/// 1. the index, when the status still calls the path unmerged;
/// 2. an operation being live, its entry's backup ref chain — the commit
///    [`snapshot::chain_backup`] took before each folded step, which is the
///    only place the stages survive once `git add` dropped them;
/// 3. otherwise the journal's in-force entries, `index_after` then
///    `index_before` — the conflicted stash pop, where each resolution is its
///    own entry (03's rule) and there is no live span to chain onto.
///
/// Only an [`IndexState::Unmerged`] holding the path at a stage **above 0**
/// counts. A merged state records the resolution, not the conflict.
///
/// # Why not REUC
///
/// git keeps its own resolve-undo extension, and `checkout --merge` on a
/// stage-0 path restores from it — measured, git 2.54.0 (Apple Git-157). It
/// is one slot per path, overwritten by the next resolution, and nothing
/// promises it across a relaunch or a pruned index: with REUC cleared the
/// same command exits 0 and silently does nothing. The recorded stage tree is
/// what makes `d` durable; REUC is what would make it look as though it were.
///
/// # Errors
/// [`FactsError`] when a `git` read will not run, or the caller stopped.
pub fn stage_source(
    ctx: &Context<'_>,
    facts: &RepoFacts,
    path: &Path,
) -> Result<Option<StageSource>, FactsError> {
    if facts
        .status
        .entry(path)
        .and_then(|entry| entry.unmerged_how)
        .is_some()
    {
        return Ok(Some(StageSource::Index));
    }
    let recorded = match facts.in_progress {
        Some(live) => chain_states(ctx, live)?,
        None => journal_states(ctx),
    };
    for state in recorded {
        if !state.is_unmerged() {
            continue;
        }
        let held = snapshot::stage_entries(ctx.repo, state, path, ctx.stop).map_err(as_facts)?;
        if held.iter().any(|entry| entry.stage >= 1) {
            return Ok(Some(StageSource::Backup { tree: state.tree() }));
        }
    }
    Ok(None)
}

/// The live entry's backup ref chain, newest first. Empty when the operation
/// is not one of Graft's, or was begun in the shell: there is no chain of
/// ours to read, which is an honest "nobody kept them".
fn chain_states(ctx: &Context<'_>, live: InProgress) -> Result<Vec<IndexState>, FactsError> {
    let span = resume::live_entry(ctx.repo, ctx.journal.entries(), live, ctx.stop).map_err(
        |why| match why {
            ApplyError::Facts(error) => error,
            // `live_entry`'s only other failure is the stop it checks first.
            _ => FactsError::Stopped,
        },
    )?;
    let Some(name) = span.and_then(|entry| entry.backup_ref.as_deref()) else {
        return Ok(Vec::new());
    };
    snapshot::backup_chain(ctx.repo, name, ctx.stop).map_err(as_facts)
}

/// The in-force entries' recorded index states, newest entry first and
/// `index_after` before `index_before` within one entry.
fn journal_states(ctx: &Context<'_>) -> Vec<IndexState> {
    let entries = ctx.journal.entries();
    let flags = crate::journal::in_force_all(entries);
    entries
        .iter()
        .zip(flags)
        .rev()
        .filter(|(_, in_force)| *in_force)
        .flat_map(|(entry, _)| [entry.index_after, entry.index_before])
        .flatten()
        .collect()
}

/// A snapshot failure as a facts failure, the **stop kept as a stop**.
fn as_facts(why: SnapshotError) -> FactsError {
    match why {
        SnapshotError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(other),
    }
}

/// What a `RestartConflict` needs doing before its plan can run.
#[derive(Clone, Debug)]
pub(crate) enum RestartPrep {
    /// Nothing: not a restart, or the index still holds the stages.
    Nothing,
    /// Put these stages back for this path first.
    Restore {
        /// The recorded stage tree they come out of.
        index: IndexState,
        /// The path.
        path: PathBuf,
    },
}

/// Decide what a restart needs — **before anything is written**.
///
/// A refusal's contract is that nothing was written and nothing journaled, so
/// the two reads and the shape gate happen here, ahead of the `Pending` line
/// and ahead of the chained backup. [`run_restart_prep`] does the writing,
/// after them.
///
/// This is where [`crate::operation::Operation::check`]'s half of the gate
/// lands: `check` is pure over [`RepoFacts`] and cannot spawn, so a path the
/// status no longer calls unmerged is admitted there and answered here,
/// against the stages that were actually kept. The test is the same one —
/// `checkout --merge` needs both sides — asked of whichever index holds the
/// path.
pub(crate) fn prepare_restart(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
) -> Result<RestartPrep, ApplyError> {
    let Operation::RestartConflict(params) = op else {
        return Ok(RestartPrep::Nothing);
    };
    let path = params.path.as_path();
    let refused = |refusal| Err(ApplyError::Refused(refusal));
    match stage_source(ctx, facts, path).map_err(as_apply_facts)? {
        None => refused(Refusal::NoStagesKept {
            path: path.to_path_buf(),
        }),
        Some(StageSource::Index) => Ok(RestartPrep::Nothing),
        Some(StageSource::Backup { tree }) => {
            let index = IndexState::Unmerged { unmerged: tree };
            // Read again rather than have `stage_source` hand the entries
            // back: one `ls-tree` on one path, against a public answer that
            // stays a plain "where from".
            let held =
                snapshot::stage_entries(ctx.repo, index, path, ctx.stop).map_err(as_apply_error)?;
            match crate::operation::shape_of_stages(&held) {
                Some(how) if how.has_markers() => Ok(RestartPrep::Restore {
                    index,
                    path: path.to_path_buf(),
                }),
                Some(how) => refused(Refusal::NotMarkerShape {
                    path: path.to_path_buf(),
                    how,
                }),
                // `stage_source` only returns `Backup` for a state holding a
                // stage above 0, so an empty set cannot arise — and saying
                // "the sides were not kept" is the truth if it ever does.
                None => refused(Refusal::NoStagesKept {
                    path: path.to_path_buf(),
                }),
            }
        }
    }
}

/// Put the stages back, once the caller's own backup is on disk.
///
/// Returns the command lines run, to go into the entry's Command Log ahead of
/// the plan's own.
pub(crate) fn run_restart_prep(
    ctx: &mut Context<'_>,
    prep: &RestartPrep,
) -> Result<Vec<String>, ApplyError> {
    match prep {
        RestartPrep::Nothing => Ok(Vec::new()),
        RestartPrep::Restore { index, path } => {
            crate::restore::restore_stages(ctx, *index, path).map_err(as_apply_error)
        }
    }
}

/// A facts failure as an apply failure, the **stop kept as a stop**.
fn as_apply_facts(why: FactsError) -> ApplyError {
    match why {
        FactsError::Stopped => ApplyError::Stopped,
        other => ApplyError::Facts(other),
    }
}

/// Close a `Pending` entry nothing will run under, and give the caller back
/// the failure that stopped it.
///
/// The `Pending` line is already on disk by the time the stages are put back,
/// and an entry with no outcome reads as an operation still in flight — which
/// this one is not. `finish` folds whatever after-state it could not read
/// into the `Aborted` reason, which is why its return is not carried on
/// separately here (the same reason `apply`'s own failure path drops it).
pub(super) fn abandon_pending(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    why: ApplyError,
) -> Result<Applied, ApplyError> {
    let status = EntryStatus::Aborted {
        reason: why.to_string(),
    };
    match finish(ctx, pending, status, Vec::new()) {
        Ok(_unread) => Err(why),
        Err(error) => Err(ApplyError::Journal { error, after: None }),
    }
}
