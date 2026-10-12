//! `d` after `⏎` (m6-conflicts/08 scope 5(c)): where a restart finds a
//! path's conflict stages once `git add` has dropped them from the index, the
//! gate asked of those stages before anything is written, and the write that
//! puts them back after the backup.

use std::io;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, IndexState, JournalEntry};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{RunError, Stop};
use graft_git::state::InProgress;

use crate::facts::FactsError;
use crate::journal::{JournalError, journal_path, parse};
use crate::operation::{OneSided, Operation, Refusal, RepoFacts, RestartKind};
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
///    `snapshot::chain_backup` took before each folded step, which is the
///    only place the stages survive once `git add` dropped them — read back
///    only as far as the current step began: a finished step's conflict is
///    not this step's to restore;
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
    find_source(ctx.repo, ctx.journal.entries(), facts, path, ctx.stop)
}

/// [`stage_source`] over any journal-entry slice: the lookup the act (through
/// its `Context`) and the gate (through a read-only parse) share.
fn find_source(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    facts: &RepoFacts,
    path: &Path,
    stop: &dyn Stop,
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
        Some(live) => chain_states(repo, entries, live, stop)?,
        None => journal_states(entries),
    };
    for state in recorded {
        if !state.is_unmerged() {
            continue;
        }
        let held = snapshot::stage_entries(repo, state, path, stop).map_err(as_facts)?;
        if held.iter().any(|entry| entry.stage >= 1) {
            return Ok(Some(StageSource::Backup { tree: state.tree() }));
        }
    }
    Ok(None)
}

/// What `d` would do to a path: where its stages come from, and what they
/// put back — markers, or a keep-or-delete choice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RestartPlan {
    /// Where the stages are.
    pub source: StageSource,
    /// What they spell. The app sends it back in
    /// [`crate::operation::RestartParams::kind`].
    pub kind: RestartKind,
}

/// The not-unmerged half of "can `d` start this path over": where the
/// stages are ([`stage_source`]'s lookup), then what a recovered stage set
/// spells. Shared by [`prepare_restart`] (the act) and [`restart_gate`] (the
/// offer), so the rule is written once.
///
/// A recovered set with both sides is [`RestartKind::Markers`]; one with a
/// single side (`UD`, `DU`, `AU`, `UA`) is [`RestartKind::WholeFile`] — the
/// keep-or-delete choice comes back (m6-conflicts/58). `DD` has no side to
/// write back and stays [`Refusal::NotMarkerShape`]. No binary check is made
/// here: a binary `UU` recovered from a backup answers `Markers`, as it did
/// before whole files were admitted.
///
/// Outer `Err`: a read failed or the caller stopped. Inner `Err`: the
/// refusal the act returns. `Index` is not shape-checked here — an unmerged
/// path is [`crate::operation::Operation::check`]'s (`check_restart`), which
/// has already refused a one-sided one, so it answers `Markers`.
pub(crate) fn restart_source(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    facts: &RepoFacts,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Result<RestartPlan, Refusal>, FactsError> {
    let no_stages = || Refusal::NoStagesKept {
        path: path.to_path_buf(),
    };
    let tree = match find_source(repo, entries, facts, path, stop)? {
        None => return Ok(Err(no_stages())),
        Some(StageSource::Index) => {
            return Ok(Ok(RestartPlan {
                source: StageSource::Index,
                kind: RestartKind::Markers,
            }));
        }
        Some(StageSource::Backup { tree }) => tree,
    };
    // Read again rather than have the lookup hand the entries back: one
    // `ls-tree` on one path, against a public answer that stays a plain
    // "where from".
    let index = IndexState::Unmerged { unmerged: tree };
    let held = snapshot::stage_entries(repo, index, path, stop).map_err(as_facts)?;
    let source = StageSource::Backup { tree };
    Ok(match crate::operation::shape_of_stages(&held) {
        Some(how) if how.has_markers() => Ok(RestartPlan {
            source,
            kind: RestartKind::Markers,
        }),
        Some(how) => match OneSided::from_how(how) {
            Some(shape) => Ok(RestartPlan {
                source,
                kind: RestartKind::WholeFile { shape },
            }),
            // `DD`: neither side has the file, so there is nothing to write
            // back and no choice to bring back.
            None => Err(Refusal::NotMarkerShape {
                path: path.to_path_buf(),
                how,
            }),
        },
        // The lookup only returns `Backup` for a state holding a stage above
        // 0, so an empty set cannot arise — and saying "the sides were not
        // kept" is the truth if it ever does.
        None => Err(no_stages()),
    })
}

/// Why [`restart_gate`] could not tell.
#[derive(Debug, thiserror::Error)]
pub enum RestartReadError {
    /// A `git` read would not run, or the chain could not be settled.
    #[error("{}", .0.explain())]
    Facts(FactsError),
    /// The journal's path could not be asked of git.
    #[error("{0}")]
    Run(#[from] RunError),
    /// The journal did not parse.
    #[error("the undo journal is unreadable: {0}")]
    Journal(#[from] JournalError),
    /// The journal exists and could not be read.
    #[error("could not read the undo journal: {0}")]
    Io(#[from] io::Error),
}

impl RestartReadError {
    /// The words the app shows.
    #[must_use]
    pub fn explain(&self) -> String {
        self.to_string()
    }
}

/// Can `d` start this path over? Read-only: no journal line, no ref moved, no
/// file written, and never `Journal::open` (which cuts a torn tail).
///
/// The same two rules the act runs, reached directly: `check_restart` (what
/// [`Operation::check`] asks of an unmerged path), then `restart_source`
/// (what `prepare_restart` asks of the rest). The answer can go stale if
/// the repository changes behind the caller; the act's own refusal is the
/// backstop — [`Refusal::RestartChanged`] when the kind the caller sends
/// back is no longer what the stages spell.
///
/// # Errors
/// Outer `Err`: could not tell. Inner `Err`: the refusal the act would
/// return, the same value.
pub fn restart_gate(
    repo: &RepoHandle,
    facts: &RepoFacts,
    path: &Path,
) -> Result<Result<RestartPlan, Refusal>, RestartReadError> {
    if let Err(refusal) = crate::operation::check_restart(path, facts) {
        return Ok(Err(refusal));
    }
    let entries = journal_entries(repo)?;
    // `&()` never stops: the gate is a short read nobody cancels.
    restart_source(repo, &entries, facts, path, &()).map_err(RestartReadError::Facts)
}

/// The journal's entries, read without opening it. No journal is no entries;
/// a torn last line is left out by `parse` and left on disk here.
fn journal_entries(repo: &RepoHandle) -> Result<Vec<JournalEntry>, RestartReadError> {
    let path = journal_path(repo)?;
    match std::fs::read(path) {
        Ok(bytes) => {
            let mut entries = parse(&bytes)?.entries;
            crate::rename::map_entries(&mut entries, repo.on_disk_names());
            Ok(entries)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// The live entry's backup ref chain **for the current step**, newest first
/// ([`snapshot::current_step_chain`]). Empty when the operation
/// is not one of Graft's, or was begun in the shell: there is no chain of
/// ours to read, which is an honest "nobody kept them".
fn chain_states(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    live: InProgress,
    stop: &dyn Stop,
) -> Result<Vec<IndexState>, FactsError> {
    // Already a `FactsError`, the stop included: no arm guessing what an
    // `ApplyError` was (m7-power-tools/01a, gap :601).
    let span = resume::live_entry_facts(repo, entries, live, stop)?;
    let Some(name) = span.and_then(|entry| entry.backup_ref.as_deref()) else {
        return Ok(Vec::new());
    };
    // The current step's links only: an earlier step's conflict is resolved
    // and committed, and restoring its stages here would put that step's
    // sides into this step's index (review m6-amend-10-22 M1).
    snapshot::current_step_chain(repo, name, stop).map_err(as_facts)
}

/// The in-force entries' recorded index states, newest entry first and
/// `index_after` before `index_before` within one entry.
fn journal_states(entries: &[JournalEntry]) -> Vec<IndexState> {
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
/// against the stages that were actually kept. The rule is the same one
/// ([`restart_source`]), asked of whichever index holds the path.
///
/// The plan was built from `params.kind` (the gate's answer when the box
/// opened). If the stages recovered now spell a different kind, that plan
/// would run the wrong `checkout` — so it is refused
/// [`Refusal::RestartChanged`] here, before anything is written.
pub(crate) fn prepare_restart(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
) -> Result<RestartPrep, ApplyError> {
    let Operation::RestartConflict(params) = op else {
        return Ok(RestartPrep::Nothing);
    };
    let path = params.path.as_path();
    let plan = match restart_source(ctx.repo, ctx.journal.entries(), facts, path, ctx.stop)
        .map_err(as_apply_facts)?
    {
        Err(refusal) => return Err(ApplyError::Refused(refusal)),
        Ok(plan) => plan,
    };
    if plan.kind != params.kind {
        return Err(ApplyError::Refused(Refusal::RestartChanged {
            path: path.to_path_buf(),
        }));
    }
    Ok(match plan.source {
        StageSource::Index => RestartPrep::Nothing,
        StageSource::Backup { tree } => RestartPrep::Restore {
            index: IndexState::Unmerged { unmerged: tree },
            path: path.to_path_buf(),
        },
    })
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
        Err(error) => Err(ApplyError::journal(error)),
    }
}
