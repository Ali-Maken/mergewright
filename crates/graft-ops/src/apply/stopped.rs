//! Where a stop left git, read from what git left on disk
//! (m8-polish-and-ship/13c).
//!
//! A write's git is stopped hook-first (`GitInvocation::stop_hooks_first`):
//! its hook is ended, and git ends the way it ends when a hook fails. For a
//! merge that is a pause — `MERGE_HEAD`, `MERGE_MSG`, `MERGE_MODE` written,
//! exit 1 — so nothing is lost and `c`/`a` are offered. How git ended does
//! not decide the outcome; what it left does, because the fallback stops
//! (no helper running, the group unreadable) kill git outright:
//!
//! | git left | outcome |
//! |---|---|
//! | the operation's own state live, `HEAD` unmoved | [`StoppedAt::Paused`]: entry `Pending`, no outcome line |
//! | a merge: no `MERGE_HEAD`, `HEAD` moved (stopped in `post-merge`) | [`StoppedAt::Landed`]: `Committed` |
//! | anything else (git killed before it paused) | [`StoppedAt::BeforePause`]: `Aborted`, [`BEFORE_PAUSE`] |
//!
//! Every read here is never stopped (`&()` or no stop at all): the stop
//! flag is already set, and a read it cancelled would leave the entry
//! without the record of what git did.

use std::path::PathBuf;

use graft_core::journal::{HeadRef, JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::state::{self, InProgress};

use super::run::PlanFailure;
use super::{Applied, ApplyError, Context, resume};
use crate::facts;
use crate::operation::Operation;

/// The `Aborted` reason of a write git did not get to pause.
pub(crate) const BEFORE_PAUSE: &str = "stopped before git could pause it";

/// Where git was when a stop ended it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoppedAt {
    /// git paused the operation itself: the entry stays `Pending`, and the
    /// next launch offers continue or abort.
    Paused(InProgress),
    /// The merge was already committed; a hook after it was stopped.
    Landed,
    /// git ended before it could pause the operation. `changed`: the entry
    /// is in force, and `u` puts the files back.
    BeforePause {
        /// Whether anything changed, so `u` has something to put back.
        changed: bool,
    },
}

/// What a stop left, before the entry is closed.
pub(super) enum Left {
    /// git's state for `kind` is live: hold the entry.
    Paused {
        kind: InProgress,
        /// What is unmerged now (normally nothing: the hook stopped it, not
        /// a conflict).
        paths: Vec<PathBuf>,
    },
    /// A merge that committed before the stop reached its hook.
    Landed,
    /// The residual: git did not pause.
    BeforePause,
}

/// Reads where a stop left git. `opens` is the in-progress state the
/// operation opens (`opens_in_progress`); `kind` the entry's kind.
///
/// `HEAD` unmoved is part of "paused" for a merge, a cherry-pick and a
/// revert, whose `HEAD` stays put until git commits. A rebase moves `HEAD`
/// as it goes, so for it git's live state alone says paused — as
/// `stopped_on_conflict` read it before 13c.
pub(super) fn where_stopped(
    ctx: &Context<'_>,
    opens: Option<InProgress>,
    kind: OperationKind,
    entry: &JournalEntry,
) -> Result<Left, ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let unmoved = head_unmoved(entry, head_commit(ctx.repo)?);
    if let Some(opened) = opens
        && live == Some(opened)
        && (opened == InProgress::Rebase || unmoved)
    {
        return Ok(Left::Paused {
            kind: opened,
            paths: resume::unmerged_paths(ctx, &())?,
        });
    }
    if kind == OperationKind::Merge && live != Some(InProgress::Merge) && !unmoved {
        return Ok(Left::Landed);
    }
    Ok(Left::BeforePause)
}

/// The commit `HEAD` names now; `None` on an unborn branch.
fn head_commit(repo: &RepoHandle) -> Result<Option<Oid>, ApplyError> {
    Ok(match facts::head_ref(repo).map_err(ApplyError::Facts)? {
        HeadRef::Branch(name) => facts::read_refs(repo, std::slice::from_ref(&name))
            .map_err(ApplyError::Facts)?
            .get(&name)
            .copied()
            .flatten(),
        HeadRef::Detached(oid) => Some(oid),
        HeadRef::Unborn(_) => None,
    })
}

/// Whether `HEAD` names the commit it named when the entry opened — its
/// branch's recorded tip, or the detached commit. A branch tip the entry did
/// not record is no evidence `HEAD` moved: unmoved.
fn head_unmoved(entry: &JournalEntry, now: Option<Oid>) -> bool {
    match &entry.head_before {
        HeadRef::Branch(name) => entry
            .refs_before
            .get(name)
            .is_none_or(|before| *before == now),
        HeadRef::Detached(oid) => now == Some(*oid),
        HeadRef::Unborn(_) => now.is_none(),
    }
}

/// What `closed` goes on with once a stop is read.
pub(super) enum Read {
    /// git paused it: hold the entry ([`ApplyError::Paused`]).
    Held {
        kind: InProgress,
        paths: Vec<PathBuf>,
    },
    /// Close the entry on `failure` as any run closes; `stopped_at` goes on
    /// the answer ([`mark`]).
    Closes {
        failure: Option<PlanFailure>,
        stopped_at: Option<StoppedAt>,
    },
}

/// `apply`'s half: a local plan line the user stopped is read
/// ([`where_stopped`]); any other failure, or none, closes as before. A
/// remote operation's stop is not read here: its words are
/// `remote::short`'s, its local steps (a pull's merge) included.
pub(super) fn after_a_stop(
    ctx: &Context<'_>,
    op: &Operation,
    entry: &JournalEntry,
    failure: Option<PlanFailure>,
) -> Result<Read, ApplyError> {
    if !failure.as_ref().is_some_and(PlanFailure::stopped_locally) {
        return Ok(Read::Closes {
            failure,
            stopped_at: None,
        });
    }
    Ok(
        match where_stopped(ctx, op.starts_in_progress(), op.kind(), entry)? {
            Left::Paused { kind, paths } => Read::Held { kind, paths },
            Left::Landed => Read::Closes {
                failure: None,
                stopped_at: Some(StoppedAt::Landed),
            },
            Left::BeforePause => Read::Closes {
                failure: failure.map(PlanFailure::before_pause),
                stopped_at: None,
            },
        },
    )
}

/// The answer says where a stop left git ([`Applied::stopped_at`]), on
/// success and on an after-state that could not be read alike. Any other
/// error passes through as it is; `None` changes nothing.
pub(super) fn mark(
    stopped_at: Option<StoppedAt>,
    closed: Result<Applied, ApplyError>,
) -> Result<Applied, ApplyError> {
    if stopped_at.is_none() {
        return closed;
    }
    match closed {
        Ok(mut applied) => {
            applied.stopped_at = stopped_at;
            Ok(applied)
        }
        Err(ApplyError::AfterState {
            mut applied,
            unread,
        }) => {
            applied.stopped_at = stopped_at;
            Err(ApplyError::AfterState { applied, unread })
        }
        Err(other) => Err(other),
    }
}
