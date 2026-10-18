//! Taking a stopped operation on or calling it off (`--continue`, `--skip`,
//! `--abort`), and the one question the steps, the resume and a relaunch all
//! ask: which `Pending` entry is *this* live operation of Mergewright's
//! ([`live_entry`]).
//!
//! The rule they share is M5 hard choice 2 and DESIGN.md §6.4: a stopped
//! operation is **one entry**, `Pending` from the stop to the finish.

use std::path::PathBuf;

use mergewright_core::journal::JournalEntry;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};
use mergewright_git::state::{self, InProgress};

use crate::operation::{Operation, Refusal, stage_solved_args};
use crate::snapshot;

mod close;
mod live;
mod rewrite;
pub(super) mod shell;

use close::close_resume;
pub(crate) use live::{live_entry, live_entry_facts};
// `apply`'s tests read its stop mark (m8-polish-and-ship/13c).
pub(super) use rewrite::resume_invocation;
use rewrite::stopped_on_edit;
use shell::{Target, shell_merge, solved_of, unmerged_to_stage};

use super::engine::{go_home, plan_of, remove};
use super::stopped::{self, Left, StoppedAt, where_stopped};
use super::{Applied, ApplyError, Context, apply, as_apply_error};

/// How [`resume`] takes a stopped history operation on (m5/07).
///
/// Not `Copy` since m6-conflicts/09: `ContinueWith` carries the message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resume {
    /// `--continue`: every unmerged path must be staged first, or named in
    /// `solved`. A merge commits with git's own message, untouched.
    Continue {
        /// A merge's files the resolver calls solved (ease-pass/05b): still
        /// unmerged, every clash picked. Staged here, inside the finishing
        /// operation, just before git commits — never earlier. Empty for
        /// every other kind and every other caller.
        solved: Vec<PathBuf>,
    },
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
        /// As [`Resume::Continue`]'s.
        solved: Vec<PathBuf>,
    },
    /// `--skip`: drop the commit being replayed (rebase, cherry-pick,
    /// revert). A merge has nothing to skip.
    Skip,
    /// `--abort`: back to where the operation started.
    Abort,
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
    let status = mergewright_git::status::read(ctx.repo, stop)
        .map_err(|error| ApplyError::Facts(error.into()))?;
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
/// are refused first, in the same words as a merge Mergewright began. `Abort`
/// aborts it the same way, as [`Operation::AbortMerge`]: `git merge
/// --abort` behind a full backup, so `u` puts the merge back as it stood
/// (m6-conflicts/57).
///
/// # Errors
/// [`Refusal::NotInProgress`] when git has no operation in progress, or no
/// `Pending` entry of Mergewright's matches it and it is not a merge being
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
    // The entry resumed is `Pending` and this run is about to finish it:
    // hold the writer's lock from before git moves, so a journal read in
    // another job or window does not close it as interrupted the moment git
    // is done (m8-polish-and-ship/58). It re-reads the journal, so the
    // entry looked up next is the file's, not a stale copy's.
    ctx.journal.claim().map_err(ApplyError::journal)?;
    let (kind, entry, stage) = match resume_target(ctx, &how)? {
        Target::Live { kind, entry, stage } => (kind, *entry, stage),
        Target::ShellMerge(params) => return apply(ctx, &Operation::FinishMerge(params)),
        Target::ShellAbort(params) => return apply(ctx, &Operation::AbortMerge(params)),
    };

    // What git is about to write over. `--abort` throws the span's work away
    // and `--continue` commits it, so in both cases the last state the user
    // saw is only in the chain after this (m6-conflicts/08 Decision 2).
    snapshot::chain_backup(ctx, &entry, snapshot::LinkKind::Resume).map_err(as_apply_error)?;

    let aborting = how == Resume::Abort;
    let (args, message) = resume_args(kind, how);
    // A rewrite's saved plan (ADR-0047), if this is one.
    let plan = match kind {
        InProgress::Rebase => plan_of(ctx.repo, &entry.id).map_err(ApplyError::Facts)?,
        _ => None,
    };
    let mut commands = entry.commands.clone();
    let entry = stage_inside(ctx, entry, kind, &stage, &mut commands)?;
    commands.push(mergewright_git::run::command_line(&args));
    let plan_dir = plan.as_ref().map(|(dir, _)| dir.as_path());
    let ran = resume_invocation(ctx.repo, kind, args, plan_dir, message).run(ctx.stop);

    let mut stopped_at = None;
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
        Err(RunError::Stopped) => {
            let (failure, at) = stopped_resume(ctx, kind, &entry, &commands)?;
            stopped_at = at;
            failure
        }
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
    let index_base = plan.as_ref().and_then(|(_, plan)| plan.index_base);
    let closed = close_resume(ctx, entry, aborting, commands, failure, left, index_base);
    stopped::mark(stopped_at, closed)
}

/// A merge's solved files, staged inside the span, after the backup and
/// just before git commits (ease-pass/05b, hard choice 3): `u` after the
/// finish puts the stage back with the commit. A refused `git add` is the
/// resume's refusal, the command held on the entry.
fn stage_inside(
    ctx: &mut Context<'_>,
    mut entry: JournalEntry,
    kind: InProgress,
    stage: &[PathBuf],
    commands: &mut Vec<String>,
) -> Result<JournalEntry, ApplyError> {
    let Some(add) = stage_solved_args(stage) else {
        return Ok(entry);
    };
    commands.push(mergewright_git::run::command_line(&add));
    match GitInvocation::new(ctx.repo).args(add).run(ctx.stop) {
        Ok(_) => Ok(entry),
        Err(error) => {
            ctx.journal.hold_commands(&entry.id, commands.clone());
            entry.commands.clone_from(commands);
            Err(ApplyError::ResumeFailed {
                entry: Box::new(entry),
                kind,
                error,
            })
        }
    }
}

/// The resume's git arguments, and its message on stdin for `ContinueWith`.
fn resume_args(kind: InProgress, how: Resume) -> (Vec<GitArg>, Option<Vec<u8>>) {
    let flag = |flag| vec![GitArg::Flag(kind.name()), GitArg::Flag(flag)];
    match how {
        // `resume_target` has refused every kind but a merge.
        Resume::ContinueWith { message, .. } => (
            vec![
                GitArg::Flag("commit"),
                GitArg::Flag("--cleanup=whitespace"),
                GitArg::Flag("-F"),
                GitArg::Flag("-"),
            ],
            Some(message),
        ),
        Resume::Continue { .. } => (flag("--continue"), None),
        Resume::Skip => (flag("--skip"), None),
        Resume::Abort => (flag("--abort"), None),
    }
}

/// The user stopped the resume's git (m8-polish-and-ship/13c): what git
/// left decides, read as `apply` reads it. Still live → held again, the
/// resume's line with it, and [`ApplyError::Paused`] is the answer; a merge,
/// pick or revert that committed → no failure, `Landed` (13d Q2); otherwise
/// the stop is the failure `close_resume` closes the entry on.
fn stopped_resume(
    ctx: &mut Context<'_>,
    kind: InProgress,
    entry: &JournalEntry,
    commands: &[String],
) -> Result<(Option<RunError>, Option<StoppedAt>), ApplyError> {
    match where_stopped(ctx, Some(kind), entry)? {
        Left::Paused { kind, paths } => {
            ctx.journal.hold_commands(&entry.id, commands.to_vec());
            let mut entry = entry.clone();
            entry.commands = commands.to_vec();
            Err(ApplyError::Paused {
                entry: Box::new(entry),
                kind,
                paths,
            })
        }
        Left::Landed => Ok((None, Some(StoppedAt::Landed))),
        Left::Residual => Ok((Some(RunError::Stopped), None)),
    }
}

/// Which entry a resume closes, and the checks that run before git does.
///
/// git's own state names the kind; the journal's newest `Pending` entry of
/// that kind is the one Mergewright opened. The order (m6-conflicts/20 A1):
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
    let continuing = matches!(how, Resume::Continue { .. } | Resume::ContinueWith { .. });
    let Some(entry) = live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?.cloned() else {
        return match (kind, how) {
            (
                InProgress::Merge,
                Resume::Continue { .. } | Resume::ContinueWith { .. } | Resume::Abort,
            ) => shell_merge(ctx, how),
            _ => Err(ApplyError::Refused(Refusal::NotInProgress)),
        };
    };
    if *how == Resume::Skip && kind == InProgress::Merge {
        return Err(ApplyError::Refused(Refusal::CannotSkip(kind)));
    }
    // Only a merge stages solved files: any other kind's `solved` is empty
    // by contract, and is ignored rather than staged if it is not.
    let solved = if kind == InProgress::Merge {
        solved_of(how)
    } else {
        &[]
    };
    let stage = if continuing {
        unmerged_to_stage(ctx, solved)?
    } else {
        Vec::new()
    };
    Ok(Target::Live {
        kind,
        entry: Box::new(entry),
        stage,
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
