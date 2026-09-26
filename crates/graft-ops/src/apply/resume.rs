//! Taking a stopped operation on or calling it off (`--continue`, `--skip`,
//! `--abort`), and the one question the steps, the resume and a relaunch all
//! ask: which `Pending` entry is *this* live operation of Graft's
//! ([`live_entry`]).
//!
//! The rule they share is M5 hard choice 2 and DESIGN.md §6.4: a stopped
//! operation is **one entry**, `Pending` from the stop to the finish.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, RunError, Stop};
use graft_git::state::{self, InProgress};

use crate::facts::{self, FactsError};
use crate::operation::{Operation, Refusal, opens_in_progress};
use crate::snapshot;

mod rewrite;
pub(super) mod shell;

use rewrite::{resume_invocation, started_on_its_line, stopped_on_edit};
use shell::{Target, refuse_unmerged, shell_merge};

use super::engine::{go_home, plan_of, remove};
use super::{
    AfterState, Applied, ApplyError, Context, Pending, Refresh, apply, as_apply_error, finish,
    reason_for, set_aside,
};

/// How [`resume`] takes a stopped history operation on (m5/07).
///
/// Not `Copy` since m6-conflicts/09: `ContinueWith` carries the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resume {
    /// `--continue`: every unmerged path must be staged first. A merge
    /// commits with git's own message, untouched.
    Continue,
    /// A merge finished with the message the user edited: `git commit
    /// --cleanup=whitespace -F -`, the message on stdin (m6-conflicts/09,
    /// Amendment 2 hard choice 3; `whitespace` since m6-conflicts/22). git's `MERGE_MSG` is never written — git
    /// removes it itself once the commit lands. Merge only: any other kind
    /// is [`Refusal::MessageNotHere`] before anything runs.
    ContinueWith {
        /// The message as the user left it. git trims trailing whitespace
        /// and blank lines but keeps every `#` line: git's own comment lines
        /// were removed once, when `finish_summary` read `MERGE_MSG`, so a
        /// `#` line here is the user's (`#123 fixes login`) and lands.
        message: Vec<u8>,
    },
    /// `--skip`: drop the commit being replayed (rebase, cherry-pick,
    /// revert). A merge has nothing to skip.
    Skip,
    /// `--abort`: back to where the operation started.
    Abort,
}

/// The newest `Pending` entry that is genuinely *this* live operation of
/// Graft's: the entry `resume` closes and recovery leaves alone (m5/07).
///
/// Kind alone was not enough (m5/07 review, M7). A Graft merge entry left
/// `Pending` by a crash *before* git ran, plus a `git merge` since started
/// in the shell, matched on kind — and `resume(Continue)` would have
/// finished someone else's merge and closed Graft's entry `Committed` with
/// a before-state from a different act. Two facts git leaves behind settle
/// it, both read here rather than assumed:
///
/// - **Where the operation started.** A rebase detaches `HEAD`, so its
///   branch is `rebase-merge/head-name`; a merge, cherry-pick or revert
///   leaves `HEAD` where it was. Either way the entry's `head_before` must
///   equal it. A `rebase-apply` directory with no `head-name` is a `git am`,
///   which matches no entry of Graft's at all (this also closes the group's
///   own Minor 11).
/// - **The branch has not moved.** A stopped merge, cherry-pick or revert
///   has written no commit, so the branch is still where the entry recorded
///   it. A rebase's own starting tip is in `rebase-merge/orig-head`, which
///   `graft_git::state` does not expose, so that leg rests on `head-name`
///   alone.
///
/// `ORIG_HEAD` is deliberately not used, although the review named it:
/// verified against git 2.54, a merge and a rebase set it but a conflicted
/// **cherry-pick and revert do not**, so requiring it would refuse two real
/// resumes to catch one impostor.
///
/// A fact the entry never recorded is not evidence against it: the
/// comparison is skipped rather than failed, which keeps a real conflict
/// resumable.
pub(crate) fn live_entry<'a>(
    repo: &RepoHandle,
    entries: &'a [JournalEntry],
    kind: InProgress,
    stop: &dyn Stop,
) -> Result<Option<&'a JournalEntry>, ApplyError> {
    if stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let detail = state::in_progress_detail(repo).map_err(|error| match error {
        state::StateError::Io(io) => ApplyError::Facts(FactsError::State(io)),
    })?;
    let started_on = match kind {
        InProgress::Rebase => match detail.as_ref().and_then(|found| found.head_name.as_deref()) {
            Some(name) => HeadRef::Branch(name.to_owned()),
            // A `git am`, or a rebase whose state file is unreadable:
            // nothing here can be shown to be Graft's.
            None => return Ok(None),
        },
        _ => facts::head_ref(repo).map_err(ApplyError::Facts)?,
    };
    let tip_now = match (&started_on, kind) {
        (
            HeadRef::Branch(name),
            InProgress::Merge | InProgress::CherryPick | InProgress::Revert,
        ) => facts::read_refs(repo, std::slice::from_ref(name))
            .map_err(ApplyError::Facts)?
            .get(name)
            .copied()
            .flatten()
            .map(|oid| (name.clone(), oid)),
        _ => None,
    };
    for entry in entries.iter().rev() {
        let candidate = entry.status == EntryStatus::Pending
            && opens_in_progress(entry.kind) == Some(kind)
            && match &tip_now {
                Some((name, now)) => match entry.refs_before.get(name) {
                    Some(Some(recorded)) => recorded == now,
                    // Not recorded, or recorded absent: nothing to compare.
                    _ => true,
                },
                None => true,
            };
        if candidate
            && (entry.head_before == started_on || started_on_its_line(repo, entry, &started_on)?)
        {
            return Ok(Some(entry));
        }
    }
    Ok(None)
}

/// The unmerged paths the status lists now.
///
/// `stop` is `ctx.stop` only before git has run (`refuse_unmerged`). After
/// it has run — git stopped, or was killed part-way by the stop itself —
/// pass `&()`: the read is the record of what git left, and a stop that
/// cancelled git would otherwise cancel it too, every time, turning a held
/// rebase into a `Facts` error with its commands never held (M7/02b; the
/// same rule as `record::read_after` and `set_aside::close`).
pub(super) fn unmerged_paths(
    ctx: &Context<'_>,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, ApplyError> {
    let status =
        graft_git::status::read(ctx.repo, stop).map_err(|error| ApplyError::Facts(error.into()))?;
    Ok(status
        .entries
        .into_iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path)
        .collect())
}

/// Take a stopped history operation on: `--continue`, `--skip` or `--abort`
/// on the newest `Pending` entry whose kind matches git's live state, with
/// `GIT_EDITOR=true` so the resolved commit's message is never edited
/// (m5/07, design note G5).
///
/// `Continue` with unmerged paths still in the index is refused before git
/// runs — git would refuse too, but with a line about `git add`. A rebase
/// may stop again on its next commit: that is [`ApplyError::Conflicted`]
/// again, on the same entry, and the entry stays `Pending`. On success the
/// after-state is read now and the outcome written `Committed`; `Abort`
/// writes `Aborted("aborted")` with the after-state read just as fresh — it
/// should equal the before-state, and when it does not (an edit made
/// mid-conflict) `in_force` treats it honestly and the backup keeps the
/// pre-operation tree. The resume command joins the entry's `commands`
/// after the plan's own (kept in memory since the stop; a relaunch loses
/// them, since only the outcome line records commands).
///
/// A **merge begun in the shell** has no entry here. `Continue` and
/// `ContinueWith` finish it all the same, as an operation of its own:
/// [`Operation::FinishMerge`], through [`apply`], so it gets its own entry
/// and backup and `u` puts the merge back (m6-conflicts/20). Unmerged paths
/// are refused first, in the same words as a merge Graft began. `Abort`
/// aborts it the same way, as [`Operation::AbortMerge`]: `git merge
/// --abort` behind a full backup, so `u` puts the merge back as it stood
/// (m6-conflicts/57).
///
/// # Errors
/// [`Refusal::NotInProgress`] when git has no operation in progress, or no
/// `Pending` entry of Graft's matches it and it is not a merge being
/// continued or aborted (a skip of one begun in the shell, a shell rebase,
/// cherry-pick or revert); [`Refusal::OctopusMerge`] for an octopus merge
/// begun in the shell, either end; [`Refusal::AutostashWaits`] while
/// `MERGE_AUTOSTASH` waits;
/// [`Refusal::CannotSkip`] for `Skip` on a merge;
/// [`Refusal::Unmerged`] for `Continue` or `ContinueWith` over unmerged
/// paths; [`Refusal::MessageNotHere`] for `ContinueWith` on anything but a
/// merge, before the entry is even looked up; the executor's own errors
/// otherwise — a hook refusing `ContinueWith`'s commit leaves the merge live,
/// so it is [`ApplyError::ResumeFailed`] like any other refusal.
pub fn resume(ctx: &mut Context<'_>, how: Resume) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let (kind, entry) = match resume_target(ctx, &how)? {
        Target::Live { kind, entry } => (kind, *entry),
        Target::ShellMerge(params) => return apply(ctx, &Operation::FinishMerge(params)),
        Target::ShellAbort(params) => return apply(ctx, &Operation::AbortMerge(params)),
    };

    // What git is about to write over. `--abort` throws the span's work away
    // and `--continue` commits it, so in both cases the last state the user
    // saw is only in the chain after this (m6-conflicts/08 Decision 2).
    snapshot::chain_backup(ctx, &entry, snapshot::LinkKind::Resume).map_err(as_apply_error)?;

    let aborting = how == Resume::Abort;
    let flag = |flag| vec![GitArg::Flag(kind.name()), GitArg::Flag(flag)];
    let (args, message) = match how {
        // `resume_target` has refused every kind but a merge.
        Resume::ContinueWith { message } => (
            vec![
                GitArg::Flag("commit"),
                GitArg::Flag("--cleanup=whitespace"),
                GitArg::Flag("-F"),
                GitArg::Flag("-"),
            ],
            Some(message),
        ),
        Resume::Continue => (flag("--continue"), None),
        Resume::Skip => (flag("--skip"), None),
        Resume::Abort => (flag("--abort"), None),
    };
    // A rewrite's saved plan (ADR-0047), if this is one.
    let plan = match kind {
        InProgress::Rebase => plan_of(ctx.repo, &entry.id).map_err(ApplyError::Facts)?,
        _ => None,
    };
    let mut commands = entry.commands.clone();
    commands.push(graft_git::run::command_line(&args));
    let plan_dir = plan.as_ref().map(|(dir, _)| dir.as_path());
    let ran = resume_invocation(ctx.repo, args, plan_dir, message).run(ctx.stop);

    let failure = match ran {
        Ok(_) => {
            // A rewrite's `edit` row: git stopped on purpose, exit 0, and
            // the entry stays `Pending` for the next resume.
            let live =
                state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
            if plan.is_some() && live == Some(kind) {
                return Err(stopped_on_edit(ctx, kind, entry, commands)?);
            }
            None
        }
        Err(RunError::Stopped) => return Err(ApplyError::Stopped),
        Err(error) => {
            // Still live: git stopped again (a rebase's next conflict) or
            // refused to go on. Either way the entry is not finished.
            let live =
                state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
            if live == Some(kind) {
                return Err(still_pending(ctx, kind, entry, commands, error)?);
            }
            Some(error)
        }
    };
    // git is done with the rewrite: `HEAD` back to his branch when git ran
    // on the top of the line, before the after-state is read, and the plan
    // removed. Neither fails the resume: each is an after-state note.
    let mut left = Vec::new();
    if let Some((dir, plan)) = &plan {
        left.extend(go_home(ctx.repo, plan, &mut commands));
        left.extend(remove(dir));
    }
    close_resume(ctx, entry, aborting, commands, failure, left)
}

/// Which entry a resume closes, and the checks that run before git does.
///
/// git's own state names the kind; the journal's newest `Pending` entry of
/// that kind is the one Graft opened. The order (m6-conflicts/20 A1):
///
/// 1. `ContinueWith` on anything but a merge → [`Refusal::MessageNotHere`].
/// 2. A live entry → that entry, checked as it always was.
/// 3. No live entry, `Continue`/`ContinueWith`, git merging with one
///    `MERGE_HEAD` → unmerged paths refused first, then
///    [`Target::ShellMerge`]. Before this, a shell merge reached
///    `NotInProgress` ahead of its unmerged check. `Abort` there →
///    [`Target::ShellAbort`], unmerged paths and all (m6-conflicts/57).
/// 4. Everything else → [`Refusal::NotInProgress`], in its old words, which
///    stay true for every case that still reaches them.
fn resume_target(ctx: &Context<'_>, how: &Resume) -> Result<Target, ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let Some(kind) = live else {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    };
    if matches!(how, Resume::ContinueWith { .. }) && kind != InProgress::Merge {
        return Err(ApplyError::Refused(Refusal::MessageNotHere { kind }));
    }
    let continuing = matches!(how, Resume::Continue | Resume::ContinueWith { .. });
    let Some(entry) = live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?.cloned() else {
        return match (kind, how) {
            (InProgress::Merge, Resume::Continue | Resume::ContinueWith { .. } | Resume::Abort) => {
                shell_merge(ctx, how)
            }
            _ => Err(ApplyError::Refused(Refusal::NotInProgress)),
        };
    };
    if *how == Resume::Skip && kind == InProgress::Merge {
        return Err(ApplyError::Refused(Refusal::CannotSkip(kind)));
    }
    if continuing {
        refuse_unmerged(ctx)?;
    }
    Ok(Target::Live {
        kind,
        entry: Box::new(entry),
    })
}

/// git would not finish and its state is still live: the entry stays
/// `Pending` and the banner stays up. Unmerged paths mean it stopped again
/// on a conflict; none mean it refused (a hook, an empty pick).
fn still_pending(
    ctx: &mut Context<'_>,
    kind: InProgress,
    mut entry: JournalEntry,
    commands: Vec<String>,
    error: RunError,
) -> Result<ApplyError, ApplyError> {
    ctx.journal.hold_commands(&entry.id, commands.clone());
    let paths = unmerged_paths(ctx, &())?;
    entry.commands = commands;
    Ok(if paths.is_empty() {
        ApplyError::ResumeFailed {
            entry: Box::new(entry),
            kind,
            error,
        }
    } else {
        ApplyError::Conflicted {
            entry: Box::new(entry),
            kind,
            paths,
        }
    })
}

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

/// git is done with the operation, one way or the other: write the outcome
/// line the stop deferred. `aborting` is `Resume::Abort`; every other way
/// on commits. `left` is what a rewrite's close could not tidy (its way
/// home, its saved plan), reported beside the outcome.
fn close_resume(
    ctx: &mut Context<'_>,
    entry: JournalEntry,
    aborting: bool,
    commands: Vec<String>,
    failure: Option<RunError>,
    left: Vec<AfterState>,
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
            failure,
            done: went_through(aborting),
            unread: noted,
        };
        return set_aside::close(ctx, pending, commands, stash, ended);
    }
    let status = match &failure {
        Some(error) => EntryStatus::Aborted {
            reason: reason_for(error),
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
        });
    }
    let applied = Applied {
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
