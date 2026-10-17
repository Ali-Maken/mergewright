//! Running a plan: one `git` per invocation, in order, stopping at the first
//! failure, with a remote invocation's askpass and progress wired in.

use std::path::Path;

use graft_core::journal::JournalEntry;
use graft_git::remote::RemoteFailure;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitInvocation, RunError};
use graft_git::state::InProgress;

use crate::keys::NoKeys;
use crate::operation::{IndexFile, Invocation, RemoteNames};
use crate::snapshot::{SnapshotError, TempIndex};

use super::error::joined;
use super::stopped::BEFORE_PAUSE;
use super::{AfterState, ApplyError, Context, reason_for, remote};

/// What running a plan came to.
pub(super) struct PlanRun {
    /// The invocation that failed, if one did.
    pub(super) failure: Option<PlanFailure>,
    /// Askpass servers that would not shut down cleanly, a rewrite that
    /// could not go home or remove its plan (`engine`). Never fatal.
    pub(super) cleanup: Vec<AfterState>,
    /// git exited 0 and is still in the middle of a rewrite: it stopped on
    /// an `edit` row (`engine`). A stop, not a finish.
    pub(super) edit_stop: bool,
}

/// The invocation that failed.
pub(super) struct PlanFailure {
    pub(super) error: RunError,
    /// For a remote operation: what went wrong in its terms.
    remote: Option<RemoteFailed>,
    /// A stop that git did not get to pause, as read from disk
    /// (`stopped::where_stopped`, m8-polish-and-ship/13c): the entry's
    /// reason says so. Set only by [`PlanFailure::before_pause`].
    before_pause: bool,
    /// A stop that left the merge a finish or abort of a shell merge runs
    /// in still paused (13d Q4): the error says so. Set only by
    /// [`PlanFailure::left_paused`].
    paused: Option<InProgress>,
}

/// A remote operation's failure: what the whole of git's stderr said, its
/// last few segments, and the operation's names for the failure line.
struct RemoteFailed {
    failure: RemoteFailure,
    stderr_tail: Vec<String>,
    names: Box<RemoteNames>,
    /// The refs a fetch moved could not be listed after it (ADR-0062).
    moves_unread: Option<RunError>,
}

impl PlanFailure {
    /// A local invocation's failure.
    pub(super) fn local(error: RunError) -> Self {
        PlanFailure {
            error,
            remote: None,
            before_pause: false,
            paused: None,
        }
    }

    /// The user stopped a local invocation: where git was left decides the
    /// outcome, not this failure (`stopped::where_stopped`). A remote
    /// operation's stop keeps its own words (`remote::short`), its local
    /// steps (a pull's merge) included.
    pub(super) fn stopped_locally(&self) -> bool {
        self.remote.is_none() && matches!(self.error, RunError::Stopped)
    }

    /// This stop ended git before it could pause the operation.
    pub(super) fn before_pause(self) -> Self {
        PlanFailure {
            before_pause: true,
            ..self
        }
    }

    /// This stop left the shell merge it ran in paused, as it was.
    pub(super) fn left_paused(self, kind: InProgress) -> Self {
        PlanFailure {
            paused: Some(kind),
            ..self
        }
    }

    /// A remote failure carries why the refs its fetch moved could not be
    /// listed, so its status line says so (ADR-0062). A local failure has
    /// no such line: the error is handed back.
    pub(super) fn carry_moves_unread(&mut self, error: RunError) -> Option<RunError> {
        match self.remote.as_mut() {
            Some(failed) => {
                failed.moves_unread = Some(error);
                None
            }
            None => Some(error),
        }
    }

    /// The entry's `Aborted` reason.
    pub(super) fn reason(&self) -> String {
        match &self.remote {
            Some(failed) => remote::short(&failed.failure),
            None => self.local_reason(),
        }
    }

    /// The reason in git's own words, with a stop read as before-pause
    /// saying so. The set-aside close gives this one for every failure,
    /// a remote operation's local step included (a pull's merge, ADR-0045
    /// amendment 03b), as it did before 19d-a2.
    pub(super) fn local_reason(&self) -> String {
        if self.before_pause {
            BEFORE_PAUSE.to_owned()
        } else {
            reason_for(&self.error)
        }
    }

    /// The error `apply` returns, the entry journaled.
    pub(super) fn into_error(self, entry: JournalEntry) -> ApplyError {
        if let Some(RemoteFailed {
            failure,
            stderr_tail,
            names,
            moves_unread,
        }) = self.remote
        {
            return ApplyError::Remote {
                entry: Box::new(entry),
                failure,
                stderr_tail,
                names,
                moves_unread: moves_unread.map(Box::new),
            };
        }
        let recoverable = entry.worktree_before.is_some();
        ApplyError::Git {
            entry: Box::new(entry),
            error: self.error,
            recoverable,
            // A set-aside's failure closes through `set_aside::close`.
            edits: None,
            paused: self.paused,
        }
    }
}

/// An `Aborted` reason with any askpass cleanup failure after it.
pub(super) fn with_cleanup(reason: String, cleanup: &[AfterState]) -> String {
    if cleanup.is_empty() {
        reason
    } else {
        // A journal record names no key: keyless words (m8/10e).
        format!("{reason}; {}", joined(cleanup, &NoKeys))
    }
}

/// Run the plan's invocations in order, stopping at the first that fails.
///
/// The order is the operation's own: group 02 puts the step that can fail
/// first, so a failure leaves the least behind it. A remote operation's
/// invocations go through `remote::run`, which keeps their stderr; a local
/// operation's run exactly as before. `names` is `Some` exactly for a remote
/// operation (`Operation::remote_names`). A plan with `Scratch` lines runs
/// them on this context's own copy of the index, removed after, failure or
/// not.
pub(super) fn run_plan(
    ctx: &Context<'_>,
    plan: &[Invocation],
    names: Option<&RemoteNames>,
) -> PlanRun {
    let mut scratch = ScratchIndex::new(ctx);
    let mut cleanup = Vec::new();
    let failure = plan
        .iter()
        .find_map(|invocation| run_line(ctx, invocation, names, &mut scratch, &mut cleanup).err())
        .map(|failed| *failed);
    cleanup.extend(scratch.remove());
    PlanRun {
        failure,
        cleanup,
        edit_stop: false,
    }
}

/// The prefix of a scratch index's file name in the names dir:
/// `commit-index-<pid>-<n>`, one per [`Context`] (review ease-pass-04,
/// Major 1). Recovery sweeps what a dead run left (`recover`).
pub(super) const SCRATCH_INDEX: &str = "commit-index";

/// A plan line as the Command Log records it: a `Scratch` line with the
/// scratch index's real path, so it runs as shown.
pub(super) fn logged_line(ctx: &Context<'_>, invocation: &Invocation) -> String {
    invocation.command_line_in(ctx.scratch_index())
}

/// One plan line, run; a `Scratch` line on the scratch index.
fn run_line(
    ctx: &Context<'_>,
    invocation: &Invocation,
    names: Option<&RemoteNames>,
    scratch: &mut ScratchIndex<'_>,
    cleanup: &mut Vec<AfterState>,
) -> Result<(), Box<PlanFailure>> {
    let mut git = plan_git(ctx.repo, invocation);
    if invocation.index == IndexFile::Scratch {
        git = git.env("GIT_INDEX_FILE", scratch.ready(ctx)?);
    }
    let Some(names) = names else {
        return git
            .run(ctx.stop)
            .map(drop)
            .map_err(|why| Box::new(PlanFailure::local(why)));
    };
    remote::run(ctx.remote_run(), git, invocation, cleanup).map_err(|failed| {
        Box::new(PlanFailure {
            remote: Some(RemoteFailed {
                failure: remote::failure_of(&failed.error, &failed.segments),
                stderr_tail: remote::tail(&failed.segments),
                names: Box::new(names.clone()),
                moves_unread: None,
            }),
            error: failed.error,
            before_pause: false,
            paused: None,
        })
    })
}

/// The copy of the index a plan's `Scratch` lines run on: made before the
/// first of them, checked before each later one, removed by
/// [`ScratchIndex::remove`].
struct ScratchIndex<'a> {
    path: &'a Path,
    copy: Option<TempIndex>,
}

impl<'a> ScratchIndex<'a> {
    fn new(ctx: &'a Context<'_>) -> Self {
        ScratchIndex {
            path: ctx.scratch_index(),
            copy: None,
        }
    }

    /// The path to hand git as `GIT_INDEX_FILE`, the copy made on first
    /// asking. A copy gone by a later line is a failure, never the empty
    /// index git would read in its place: a commit on that deletes every
    /// file he did not choose.
    ///
    /// The run holds the journal's writer's lock while the copy exists, so
    /// recovery's sweep of dead runs' copies (`recover`) never takes a live
    /// one.
    fn ready(&mut self, ctx: &Context<'_>) -> Result<&'a Path, Box<PlanFailure>> {
        if !ctx.journal.claimed() {
            return Err(not_run(
                "the commit's index copy needs the journal held",
                "no entry is open",
            ));
        }
        if self.copy.is_some() {
            still_there(self.path)?;
        } else {
            let copy = TempIndex::copied_as(ctx.repo, self.path.to_path_buf(), ctx.stop).map_err(
                |why| match why {
                    SnapshotError::Stopped => Box::new(PlanFailure::local(RunError::Stopped)),
                    SnapshotError::Io(why) => not_run(COPY_FAILED, why),
                    other => not_run(COPY_FAILED, other),
                },
            )?;
            self.copy = Some(copy);
        }
        Ok(self.path)
    }

    /// Remove the copy, if one was made; what could not go, said.
    fn remove(self) -> Option<AfterState> {
        let copy = self.copy?;
        copy.remove()
            .err()
            .map(|error| AfterState::ScratchIndexLeft {
                path: self.path.to_path_buf(),
                error,
            })
    }
}

/// A failed copy's words: git never ran, so not "could not run git".
const COPY_FAILED: &str = "could not copy the index for the commit";

/// A failure before git ran, in the step's own words.
fn not_run(what: &'static str, why: impl std::fmt::Display) -> Box<PlanFailure> {
    Box::new(PlanFailure::local(RunError::NotRun {
        what,
        why: why.to_string(),
    }))
}

/// The scratch index is still where the run copied it.
fn still_there(path: &Path) -> Result<(), Box<PlanFailure>> {
    const GONE: &str = "the commit's index copy is gone";
    match path.try_exists() {
        Ok(true) => Ok(()),
        Ok(false) => Err(not_run(GONE, path.display())),
        Err(why) => Err(not_run(GONE, why)),
    }
}

/// The git one plan line runs.
///
/// No plan step ever wants an editor: every message travels on stdin or in
/// `-m`. A step that would still open one (`merge` writing its message,
/// `cherry-pick` without `--no-edit`) gets `true` and carries on with git's
/// own text (M5 design note, G5 plan).
///
/// A local line is stopped hook-first (m8-polish-and-ship/13c): a stuck hook
/// is ended and git pauses the operation itself, as it does when a hook
/// fails. A remote line (fetch, push) keeps the group stop: its helpers are
/// ssh and the credential helper, and there is nothing for git to pause.
pub(super) fn plan_git(repo: &RepoHandle, invocation: &Invocation) -> GitInvocation {
    let mut git = GitInvocation::new(repo)
        .args(invocation.args.iter().cloned())
        .env("GIT_EDITOR", "true");
    if let Some(bytes) = &invocation.stdin {
        git = git.stdin(bytes.clone());
    }
    if invocation.remote {
        git
    } else {
        git.stop_hooks_first()
    }
}

#[cfg(test)]
mod tests;
