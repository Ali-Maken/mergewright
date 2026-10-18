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
//! | the merge a finish or abort of a shell merge runs in, live, `HEAD` unmoved | [`StoppedAt::Paused`]: entry closed `Aborted` as a failed finish closes (13d Q4) |
//! | a merge, pick or revert: its state gone, `HEAD` moved (a hook after the commit stopped) | [`StoppedAt::Landed`]: `Committed` |
//! | anything else, for an operation git can pause (git killed before it paused) | [`StoppedAt::BeforePause`]: `Aborted`, [`BEFORE_PAUSE`] |
//! | anything else, for one it cannot (a discard …) | [`StoppedAt::Stopped`]: `Aborted`, `git was stopped` |
//!
//! Which state is read is [`runs_in_progress`]: what the operation opens, or
//! the merge a finish or abort of a shell merge runs inside.
//!
//! Every read here is never stopped (`&()` or no stop at all): the stop
//! flag is already set, and a read it cancelled would leave the entry
//! without the record of what git did.

use std::path::PathBuf;

use mergewright_core::journal::{HeadRef, JournalEntry, OperationKind};
use mergewright_core::oid::Oid;
use mergewright_git::repo::RepoHandle;
use mergewright_git::state::{self, InProgress};

use super::run::PlanFailure;
use super::{Applied, ApplyError, Context, resume};
use crate::facts;
use crate::operation::{Operation, runs_in_progress};

/// The `Aborted` reason of a write git did not get to pause.
pub(crate) const BEFORE_PAUSE: &str = "stopped before git could pause it";

/// Where git was when a stop ended it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoppedAt {
    /// git paused the operation itself: the entry stays `Pending`, and the
    /// next launch offers continue or abort.
    Paused(InProgress),
    /// A merge, cherry-pick or revert had already committed; a hook after
    /// the commit was stopped.
    Landed,
    /// git ended before it could pause an operation that pauses (one that
    /// opens an in-progress state). `changed`: the entry is in force, and
    /// `u` puts the files back.
    BeforePause {
        /// Whether anything changed, so `u` has something to put back.
        changed: bool,
    },
    /// git was stopped in an operation that never pauses (a discard, a
    /// checkout …): there is no paused state to offer. `changed` as for
    /// `BeforePause` (13d Q3).
    Stopped {
        /// Whether anything changed, so `u` has something to put back.
        changed: bool,
    },
}

impl StoppedAt {
    /// The residual's answer for an entry of `kind`: `BeforePause` when
    /// the kind could have been paused, `Stopped` when it never can.
    pub(super) fn residual(kind: OperationKind, changed: bool) -> StoppedAt {
        if crate::operation::opens_in_progress(kind).is_some() {
            StoppedAt::BeforePause { changed }
        } else {
            StoppedAt::Stopped { changed }
        }
    }
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
    /// A merge, pick or revert that committed before the stop reached a
    /// hook after the commit.
    Landed,
    /// The residual: git did not pause.
    Residual,
}

/// Reads where a stop left git. `reads` is the in-progress state the stop
/// is read against ([`runs_in_progress`], or the live kind a resume runs).
///
/// `HEAD` unmoved is part of "paused" for a merge, a cherry-pick and a
/// revert, whose `HEAD` stays put until git commits. A rebase moves `HEAD`
/// as it goes, so for it git's live state alone says paused — as
/// `stopped_on_conflict` read it before 13c.
///
/// Landed (13d Q2) is every kind that makes one commit: a merge, a pick or
/// a revert whose state is gone and whose `HEAD` moved. A rebase never
/// lands here: with its state gone after a stop it cannot be told from one
/// git ended half-way, and the residual's `u` loses nothing.
pub(super) fn where_stopped(
    ctx: &Context<'_>,
    reads: Option<InProgress>,
    entry: &JournalEntry,
) -> Result<Left, ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let unmoved = head_unmoved(entry, head_commit(ctx.repo)?);
    let Some(reads) = reads else {
        return Ok(Left::Residual);
    };
    if live == Some(reads) && (reads == InProgress::Rebase || unmoved) {
        return Ok(Left::Paused {
            kind: reads,
            paths: resume::unmerged_paths(ctx, &())?,
        });
    }
    let one_commit = matches!(
        reads,
        InProgress::Merge | InProgress::CherryPick | InProgress::Revert
    );
    if one_commit && live != Some(reads) && !unmoved {
        return Ok(Left::Landed);
    }
    Ok(Left::Residual)
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
    let Some(failure) = failure else {
        return Ok(Read::Closes {
            failure: None,
            stopped_at: None,
        });
    };
    if !failure.stopped_locally() {
        return Ok(Read::Closes {
            failure: Some(failure),
            stopped_at: None,
        });
    }
    let opens = op.starts_in_progress();
    Ok(
        match where_stopped(ctx, runs_in_progress(op.kind()), entry)? {
            // Its own state: held, as a conflict is.
            Left::Paused { kind, paths } if opens == Some(kind) => Read::Held { kind, paths },
            // The shell merge a finish or abort runs in (13d Q4): the entry
            // closes as a failed finish closes — `Aborted`, `git was
            // stopped` — never held `Pending` for a merge Mergewright did not
            // begin; the answer says the merge is still paused.
            Left::Paused { kind, .. } => Read::Closes {
                failure: Some(failure.left_paused(kind)),
                stopped_at: None,
            },
            Left::Landed => Read::Closes {
                failure: None,
                stopped_at: Some(StoppedAt::Landed),
            },
            // 13d Q3: "before git could pause it" only where git could have.
            Left::Residual if opens.is_some() => Read::Closes {
                failure: Some(failure.before_pause()),
                stopped_at: None,
            },
            Left::Residual => Read::Closes {
                failure: Some(failure),
                stopped_at: None,
            },
        },
    )
}

/// The answer says where a stop left git ([`Applied::stopped_at`]), on
/// success, on an after-state that could not be read, and on a set-aside
/// put-back that clashed after the operation landed (its own `stopped_at`,
/// 19d-a3) alike. Any other error passes through as it is; `None` changes
/// nothing.
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
        Err(ApplyError::Clashed { entry, clash, .. }) => Err(ApplyError::Clashed {
            entry,
            clash,
            stopped_at,
        }),
        Err(other) => Err(other),
    }
}
