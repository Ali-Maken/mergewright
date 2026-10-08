//! A resume git is done with: the outcome line the stop deferred. Moved out
//! of `resume.rs` (m8/13e).

use graft_core::journal::{EntryStatus, JournalEntry};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::facts;

use super::super::stopped::BEFORE_PAUSE;
use super::super::{
    AfterState, Applied, ApplyError, Context, Pending, Refresh, finish, reason_for, set_aside,
};

/// The entry's status when git went through with the resume: `--abort`
/// closes it `Aborted("aborted")`; `Continue`, `ContinueWith` and `Skip` commit.
fn went_through(aborting: bool) -> EntryStatus {
    if aborting {
        EntryStatus::Aborted {
            reason: "aborted".to_owned(),
        }
    } else {
        EntryStatus::Committed
    }
}

/// The entry's before-commit, read from its backup ref — which still points
/// there: `seal_after` moves it only when the outcome is written, which is
/// now. A backup ref that is gone comes back as `BackupGone`.
fn before_commit_of(
    ctx: &Context<'_>,
    entry: &JournalEntry,
) -> Result<(Option<Oid>, Option<AfterState>), ApplyError> {
    let before_commit = match &entry.backup_ref {
        Some(name) => facts::read_refs(ctx.repo, std::slice::from_ref(name))
            .map_err(ApplyError::Facts)?
            .get(name)
            .copied()
            .flatten(),
        None => None,
    };
    let backup_gone = entry
        .backup_ref
        .clone()
        .filter(|_| before_commit.is_none())
        .map(|name| AfterState::BackupGone { name });
    Ok((before_commit, backup_gone))
}

/// A failed resume's `Aborted` reason. A stop reaches `close_resume` only
/// when git did not pause the operation (`stopped_resume`'s residual), and
/// the reason says so (m8-polish-and-ship/13c).
fn resume_reason(error: &RunError) -> String {
    match error {
        RunError::Stopped => BEFORE_PAUSE.to_owned(),
        other => reason_for(other),
    }
}

/// git is done with the operation, one way or the other: write the outcome
/// line the stop deferred. `aborting` is `Resume::Abort`; every other way
/// on commits. `left` is what a rewrite's close could not tidy (its way
/// home, its saved plan), reported beside the outcome. `index_base` is an
/// absorb's, from its saved plan: the put-back uses it only when the
/// rewrite went through.
pub(super) fn close_resume(
    ctx: &mut Context<'_>,
    entry: JournalEntry,
    aborting: bool,
    commands: Vec<String>,
    failure: Option<RunError>,
    left: Vec<AfterState>,
    index_base: Option<Oid>,
) -> Result<Applied, ApplyError> {
    let (before_commit, backup_gone) = before_commit_of(ctx, &entry)?;
    let noted: Vec<AfterState> = backup_gone.into_iter().chain(left).collect();
    let mut pending = Pending {
        entry,
        before_commit,
    };
    // A rebase that set edits aside and stopped: git is done with it now,
    // whichever way, so step 6 runs — onto the rebased tip, or onto the
    // original one after `--abort` (ADR-0045 §4). `put_back` comes from the
    // Pending line, so this holds across a relaunch.
    if let Some(stash) = set_aside::stash_of(&pending.entry) {
        let ended = set_aside::Ended {
            index_base: index_base.filter(|_| failure.is_none() && !aborting),
            // git's own words, as before 19d-a2: a stop that reaches here
            // (`stopped_resume`'s residual) is not reworded — no resume with
            // edits aside has been seen to reach it (NOTES 19d-a2).
            failure: failure.map(set_aside::Failed::git),
            done: went_through(aborting),
            unread: noted,
        };
        return set_aside::close(ctx, pending, commands, stash, ended);
    }
    let status = match &failure {
        Some(error) => EntryStatus::Aborted {
            reason: resume_reason(error),
        },
        None => went_through(aborting),
    };
    let mut unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            return Err(ApplyError::Journal {
                error,
                after: failure,
                kept: None,
            });
        }
    };
    unread.extend(noted);
    let entry = pending.entry;
    if let Some(error) = failure {
        let recoverable = entry.worktree_before.is_some();
        return Err(ApplyError::Git {
            entry: Box::new(entry),
            error,
            recoverable,
            // A set-aside's resume closes through `set_aside::close`.
            edits: None,
            // A resume runs on its own entry: a stop it left paused is held
            // (`stopped_resume`), never closed.
            paused: None,
        });
    }
    let applied = Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry,
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: crate::undo::Replaced::default(),
    };
    if unread.is_empty() {
        Ok(applied)
    } else {
        Err(ApplyError::AfterState {
            applied: Box::new(applied),
            unread,
        })
    }
}
