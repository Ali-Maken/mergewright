//! Running a plan: one `git` per invocation, in order, stopping at the first
//! failure, with a remote invocation's askpass and progress wired in.

use graft_core::journal::JournalEntry;
use graft_git::remote::RemoteFailure;
use graft_git::run::{GitInvocation, RunError};

use crate::operation::{Invocation, RemoteNames};

use super::error::joined;
use super::{AfterState, ApplyError, Context, reason_for, remote};

/// What running a plan came to.
pub(super) struct PlanRun {
    /// The invocation that failed, if one did.
    pub(super) failure: Option<PlanFailure>,
    /// Askpass servers that would not shut down cleanly. Never fatal.
    pub(super) cleanup: Vec<AfterState>,
}

/// The invocation that failed.
pub(super) struct PlanFailure {
    pub(super) error: RunError,
    /// For a remote operation: what went wrong in its terms.
    remote: Option<RemoteFailed>,
}

/// A remote operation's failure: what the whole of git's stderr said, its
/// last few segments, and the operation's names for the failure line.
struct RemoteFailed {
    failure: RemoteFailure,
    stderr_tail: Vec<String>,
    names: Box<RemoteNames>,
}

impl PlanFailure {
    /// The entry's `Aborted` reason.
    pub(super) fn reason(&self) -> String {
        match &self.remote {
            Some(failed) => remote::short(&failed.failure),
            None => reason_for(&self.error),
        }
    }

    /// The error `apply` returns, the entry journaled.
    pub(super) fn into_error(self, entry: JournalEntry) -> ApplyError {
        if let Some(RemoteFailed {
            failure,
            stderr_tail,
            names,
        }) = self.remote
        {
            return ApplyError::Remote {
                entry: Box::new(entry),
                failure,
                stderr_tail,
                names,
            };
        }
        let recoverable = entry.worktree_before.is_some();
        ApplyError::Git {
            entry: Box::new(entry),
            error: self.error,
            recoverable,
            // A set-aside's failure closes through `set_aside::close`.
            edits: None,
        }
    }
}

/// An `Aborted` reason with any askpass cleanup failure after it.
pub(super) fn with_cleanup(reason: String, cleanup: &[AfterState]) -> String {
    if cleanup.is_empty() {
        reason
    } else {
        format!("{reason}; {}", joined(cleanup))
    }
}

/// Run the plan's invocations in order, stopping at the first that fails.
///
/// The order is the operation's own: group 02 puts the step that can fail
/// first, so a failure leaves the least behind it. A remote operation's
/// invocations go through `remote::run`, which keeps their stderr; a local
/// operation's run exactly as before. `names` is `Some` exactly for a remote
/// operation (`Operation::remote_names`).
pub(super) fn run_plan(
    ctx: &Context<'_>,
    plan: &[Invocation],
    names: Option<&RemoteNames>,
) -> PlanRun {
    let mut cleanup = Vec::new();
    for invocation in plan {
        // No plan step ever wants an editor: every message travels on stdin
        // or in `-m`. A step that would still open one (`merge` writing its
        // message, `cherry-pick` without `--no-edit`) gets `true` and
        // carries on with git's own text (M5 design note, G5 plan).
        let mut git = GitInvocation::new(ctx.repo)
            .args(invocation.args.iter().cloned())
            .env("GIT_EDITOR", "true");
        if let Some(bytes) = &invocation.stdin {
            git = git.stdin(bytes.clone());
        }
        let failure = if let Some(names) = names {
            remote::run(ctx, git, invocation, &mut cleanup)
                .err()
                .map(|failed| PlanFailure {
                    remote: Some(RemoteFailed {
                        failure: remote::failure_of(&failed.error, &failed.segments),
                        stderr_tail: remote::tail(&failed.segments),
                        names: Box::new(names.clone()),
                    }),
                    error: failed.error,
                })
        } else {
            git.run(ctx.stop).err().map(|error| PlanFailure {
                error,
                remote: None,
            })
        };
        if failure.is_some() {
            return PlanRun { failure, cleanup };
        }
    }
    PlanRun {
        failure: None,
        cleanup,
    }
}
