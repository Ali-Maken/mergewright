//! Everything about an operation git already has in progress.
//!
//! Three things live here, and they are one subject: the steps a user takes
//! *while* a merge, rebase, cherry-pick or revert is stopped
//! ([`inside_live_operation`]), the way that operation is taken on or called
//! off ([`resume`]), and what a relaunch makes of an entry a previous run
//! left `Pending` ([`recover_interrupted`]).
//!
//! The rule they share is M5 hard choice 2 and DESIGN.md §6.4: a stopped
//! operation is **one entry**, `Pending` from the stop to the finish. Every
//! step inside the span folds into that entry and writes no line of its own,
//! so `u` after the finish takes the whole span back.

use std::collections::BTreeMap;
use std::path::PathBuf;

use graft_core::journal::{EntryId, EntryStatus, HeadRef, JournalEntry};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};
use graft_git::state::{self, InProgress};

use crate::facts::{self, FactsError};
use crate::operation::{Invocation, Operation, Refusal, RepoFacts, opens_in_progress};
use crate::snapshot;
use crate::undo::RepoStateNow;

use super::{
    AfterState, Applied, ApplyError, Context, Pending, PlanRun, Refresh, RestartPrep, anchors,
    as_apply_error, finish, reason_for, run_plan, run_restart_prep,
};

/// How [`resume`] takes a stopped history operation on (m5/07).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Resume {
    /// `--continue`: every unmerged path must be staged first.
    Continue,
    /// `--skip`: drop the commit being replayed (rebase, cherry-pick,
    /// revert). A merge has nothing to skip.
    Skip,
    /// `--abort`: back to where the operation started.
    Abort,
}

/// Run a step that belongs to an operation git already has in progress.
///
/// `check` admits exactly `Stage` and `Unstage` while a merge, rebase,
/// cherry-pick or revert is open — the resolver's "mark resolved". Such a
/// step is **part of that operation, not an operation of its own**: DESIGN
/// §6.4 ("a mid-flight operation … is not a journal entry yet … the undo
/// journal picks up once there's a finished operation to reverse") and M5
/// hard choice 2 (one entry, `Pending` from the stop to the finish). So it
/// runs, its command line joins the live entry, and it writes **no journal
/// line of its own**. `u` after the finish takes the whole span back.
///
/// Journaling it as an ordinary entry is what killed `u` outright (m5/07
/// review, C1): `write-tree` has no tree for an unmerged index, so the
/// entry's `index_before` was `None` and its inverse `NoBackup` — and undo
/// is linear, so the merge behind it became unreachable. The rule is "an
/// operation is in progress", not "the index cannot be snapshotted": the
/// last file a user resolves leaves the index clean, and that step must fold
/// into the span like every one before it.
///
/// **Writing no line is not the same as keeping no backup.** This used to say
/// nothing was at risk here, on the grounds that every admitted step wrote
/// only the index. That stopped being true when the resolver landed: a
/// whole-file take is `checkout --ours|--theirs` + `add`, or `rm --quiet`,
/// and a restart is `checkout --merge` — all of them write the working tree
/// (`touches.worktree`), and all of them are admitted here. A hand edit made
/// in `$EDITOR` and then resolved the other way was overwritten with no
/// confirmation, no line and no snapshot, and the span's own entry predates
/// the edit (m6 g03 review, M2).
///
/// So the span's entry stays one entry, and every step whose `touches` names
/// the index or the working tree takes a **chained backup commit** first
/// ([`snapshot::chain_backup`], Decision 2 of m6-conflicts/08): the
/// repository as it stands, hung off the entry's own backup ref, no journal
/// line. It is what `d` reads back after `⏎`, and it is the recovery point
/// for that hand edit.
pub(super) fn inside_live_operation(
    ctx: &mut Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
    live: InProgress,
    prep: &RestartPrep,
) -> Result<Applied, ApplyError> {
    // Before anything runs: the entry the step folds into, and the snapshot
    // that keeps what the step is about to overwrite.
    let span = live_entry(ctx.repo, ctx.journal.entries(), live, ctx.stop)?.cloned();
    let touches = op.touches(facts);
    let step_backup = match &span {
        Some(entry) if touches.index || touches.worktree => {
            snapshot::chain_backup(ctx, entry).map_err(as_apply_error)?
        }
        // No entry of Graft's to chain onto (an operation begun in the
        // shell), or a step that writes neither: nothing to keep.
        _ => None,
    };

    // A restart of a path `⏎` already staged needs its stages back in the
    // index before `checkout --merge` has anything to merge (scope 5(c)).
    // After the chained backup, so what it is about to overwrite is kept.
    // `prepare_restart` decided this before a byte was written; this is the
    // write half.
    let mut commands = run_restart_prep(ctx, prep)?;

    let plan = op.plan(facts);
    commands.extend(plan.iter().map(Invocation::command_line));
    let PlanRun { failure, .. } = run_plan(ctx, &plan, op.remote_names().as_ref());
    fold_commands(ctx, span.as_ref(), commands.clone());

    let mut record = JournalEntry {
        id: ctx.next_id(),
        at_unix_ms: ctx.now_unix_ms,
        kind: op.kind(),
        description: op.describe(),
        status: EntryStatus::Committed,
        touches,
        head_before: facts.head.clone(),
        head_after: Some(facts.head.clone()),
        refs_before: BTreeMap::default(),
        refs_after: BTreeMap::default(),
        index_before: None,
        index_after: None,
        worktree_before: None,
        worktree_after: None,
        backup_ref: None,
        pins: Vec::new(),
        commands,
        reverses: None,
    };
    match failure {
        None => Ok(Applied {
            entry: record,
            refresh: Refresh::WorkingTree,
            step_backup,
        }),
        Some(failed) => {
            record.status = EntryStatus::Aborted {
                reason: failed.reason(),
            };
            Err(failed.into_error(record))
        }
    }
}

/// Record a folded step on the live entry: its command line after the ones
/// already held, so the Command Log shows the span in the order it happened.
///
/// A `Pending` entry's commands live in memory until its outcome line is
/// written, so a caller that opens a fresh journal per job sees only the
/// step it just ran (the same limitation the stop itself has — the plan's
/// own lines do not reach disk either until `resume` closes the entry).
///
/// `None` is an operation begun outside Graft: there is no entry of Graft's
/// to record the step on. Nothing is dropped that was ever ours — git's own
/// state is the whole record of that operation, and the step is reported to
/// the caller on the returned `Applied`.
fn fold_commands(ctx: &mut Context<'_>, span: Option<&JournalEntry>, commands: Vec<String>) {
    let Some(entry) = span else {
        return;
    };
    let mut held = entry.commands.clone();
    held.extend(commands);
    ctx.journal.hold_commands(&entry.id, held);
}

/// Close every entry left `Pending` by a previous run as `interrupted`,
/// recording the repository as it stands now (DESIGN.md §6.2 step 3).
///
/// Run this at launch, before the user can touch anything: the state read
/// here is recorded as the interrupted operation's after-state, so the later
/// it runs, the more of the user's own work it attributes to that operation.
/// When the repository cannot be read at all, the entry is closed with no
/// after-state — `u` then offers to restore the whole tree, and says so.
///
/// Returns what it closed **and** what it could not read, so the caller can
/// surface both: an entry closed with no after-state is one whose `u` will
/// offer to overwrite the whole working tree, and the reason for that belongs
/// on screen, not on the floor (m3/03 review, M1).
pub fn recover_interrupted(ctx: &mut Context<'_>) -> Result<Recovered, ApplyError> {
    // A conflict that survived a relaunch: git's own state says the
    // operation is still live, so its entry stays `Pending` and the banner
    // takes over (m5/07, hard choice 2). Only the newest matching entry is
    // live — an older one of the same kind was left behind by something
    // else and closes as interrupted like any other.
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let live_id = match live {
        Some(kind) => live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?
            .map(|entry| entry.id.clone()),
        None => None,
    };
    let open: Vec<(EntryId, Vec<String>, Option<String>)> = ctx
        .journal
        .unresolved()
        .into_iter()
        .filter(|entry| Some(&entry.id) != live_id.as_ref())
        .map(|entry| {
            (
                entry.id.clone(),
                entry.refs_before.keys().cloned().collect(),
                entry.backup_ref.clone(),
            )
        })
        .collect();
    let mut recovered = Recovered {
        live: live_id.into_iter().collect(),
        ..Recovered::default()
    };
    for (id, refs, backup_ref) in open {
        let read = facts::read_state_now(ctx.repo, ctx.scratch(), &refs, ctx.stop);
        // The repository being unreadable must not leave the entry `Pending`
        // for ever, which blocks `u` outright. `None` is the honest record
        // and the widest restore, and the user is asked before it runs
        // (`Drift::unknown`) — and told why, here.
        let now = match read {
            Ok(now) => match seal_recovered(ctx, &id, backup_ref.as_deref(), &now) {
                Ok(()) => Some(now),
                // The trees were read but nothing keeps them alive, so
                // recording them would name objects `git gc` will prune.
                Err(why) => {
                    recovered.unreadable.push((id.clone(), why));
                    None
                }
            },
            Err(FactsError::Stopped) => return Err(ApplyError::Stopped),
            Err(unreadable) => {
                recovered.unreadable.push((id.clone(), unreadable));
                None
            }
        };
        ctx.journal
            .mark_interrupted(&id, now.as_ref())
            .map_err(|error| ApplyError::Journal { error, after: None })?;
        recovered.closed.push(id);
    }
    Ok(recovered)
}

/// What [`recover_interrupted`] did at launch.
#[derive(Debug, Default)]
pub struct Recovered {
    /// The entries closed as `interrupted`, in journal order.
    pub closed: Vec<EntryId>,
    /// The ones whose after-state could not be read or kept reachable, and
    /// why. Each is closed with no after-state, so its `u` restores the whole
    /// tree after asking.
    pub unreadable: Vec<(EntryId, FactsError)>,
    /// The entry left `Pending` on purpose: a history operation still
    /// stopped on a conflict by git's own account (`MERGE_HEAD`,
    /// `rebase-merge/` …). The banner offers `resume`; nothing was closed
    /// (m5/07). At most one.
    pub live: Vec<EntryId>,
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
pub(super) fn live_entry<'a>(
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
    Ok(entries.iter().rev().find(|entry| {
        entry.status == EntryStatus::Pending
            && opens_in_progress(entry.kind) == Some(kind)
            && entry.head_before == started_on
            && match &tip_now {
                Some((name, now)) => match entry.refs_before.get(name) {
                    Some(Some(recorded)) => recorded == now,
                    // Not recorded, or recorded absent: nothing to compare.
                    _ => true,
                },
                None => true,
            }
    }))
}

/// The unmerged paths the status lists now.
pub(super) fn unmerged_paths(ctx: &Context<'_>) -> Result<Vec<PathBuf>, ApplyError> {
    let status = graft_git::status::read(ctx.repo, ctx.stop)
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
/// # Errors
/// [`Refusal::NotInProgress`] when git has no operation in progress or no
/// `Pending` entry of Graft's matches it (one begun in the shell finishes
/// there); [`Refusal::CannotSkip`] for `Skip` on a merge;
/// [`Refusal::Unmerged`] for `Continue` over unmerged paths; the executor's
/// own errors otherwise.
pub fn resume(ctx: &mut Context<'_>, how: Resume) -> Result<Applied, ApplyError> {
    if ctx.stop.should_stop() {
        return Err(ApplyError::Stopped);
    }
    let (kind, entry) = resume_target(ctx, how)?;

    // What git is about to write over. `--abort` throws the span's work away
    // and `--continue` commits it, so in both cases the last state the user
    // saw is only in the chain after this (m6-conflicts/08 Decision 2).
    snapshot::chain_backup(ctx, &entry).map_err(as_apply_error)?;

    let args = [
        GitArg::Flag(kind.name()),
        GitArg::Flag(match how {
            Resume::Continue => "--continue",
            Resume::Skip => "--skip",
            Resume::Abort => "--abort",
        }),
    ];
    let mut commands = entry.commands.clone();
    commands.push(graft_git::run::command_line(&args));
    let ran = GitInvocation::new(ctx.repo)
        .args(args)
        .env("GIT_EDITOR", "true")
        .run(ctx.stop);

    let failure = match ran {
        Ok(_) => None,
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
    close_resume(ctx, entry, how, commands, failure)
}

/// Which entry a resume closes, and the checks that run before git does.
///
/// git's own state names the kind; the journal's newest `Pending` entry of
/// that kind is the one Graft opened. An operation begun in the shell has no
/// entry here and is finished there ([`Refusal::NotInProgress`]).
fn resume_target(ctx: &Context<'_>, how: Resume) -> Result<(InProgress, JournalEntry), ApplyError> {
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    let Some(kind) = live else {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    };
    let Some(entry) = live_entry(ctx.repo, ctx.journal.entries(), kind, ctx.stop)?.cloned() else {
        return Err(ApplyError::Refused(Refusal::NotInProgress));
    };
    if how == Resume::Skip && kind == InProgress::Merge {
        return Err(ApplyError::Refused(Refusal::CannotSkip(kind)));
    }
    if how == Resume::Continue {
        let paths = unmerged_paths(ctx)?;
        if !paths.is_empty() {
            return Err(ApplyError::Refused(Refusal::Unmerged { paths }));
        }
    }
    Ok((kind, entry))
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
    let paths = unmerged_paths(ctx)?;
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

/// git is done with the operation, one way or the other: write the outcome
/// line the stop deferred.
fn close_resume(
    ctx: &mut Context<'_>,
    entry: JournalEntry,
    how: Resume,
    commands: Vec<String>,
    failure: Option<RunError>,
) -> Result<Applied, ApplyError> {
    // The backup ref still points at the before-commit: `seal_after` moves
    // it only when the outcome is written, which is now.
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
    let mut pending = Pending {
        entry,
        before_commit,
    };
    let status = match (how, &failure) {
        (_, Some(error)) => EntryStatus::Aborted {
            reason: reason_for(error),
        },
        (Resume::Abort, None) => EntryStatus::Aborted {
            reason: "aborted".to_owned(),
        },
        (Resume::Continue | Resume::Skip, None) => EntryStatus::Committed,
    };
    let mut unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            return Err(ApplyError::Journal {
                error,
                after: failure,
            });
        }
    };
    unread.extend(backup_gone);
    let entry = pending.entry;
    if let Some(error) = failure {
        let recoverable = entry.worktree_before.is_some();
        return Err(ApplyError::Git {
            entry: Box::new(entry),
            error,
            recoverable,
        });
    }
    let applied = Applied {
        entry,
        refresh: Refresh::Everything,
        step_backup: None,
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

/// Hang a recovered entry's after-trees off its backup ref before they are
/// recorded, so `git gc` cannot prune what the entry is about to name.
///
/// Nothing to do when the entry took no backup: such an entry records no tree
/// (`mark_interrupted` copies its before-fields, which are `None`), and the
/// refs it names are live refs.
fn seal_recovered(
    ctx: &Context<'_>,
    id: &EntryId,
    backup_ref: Option<&str>,
    now: &RepoStateNow,
) -> Result<(), FactsError> {
    let Some(name) = backup_ref else {
        return Ok(());
    };
    let at = facts::read_refs(ctx.repo, std::slice::from_ref(&name.to_owned()))?
        .get(name)
        .copied()
        .flatten();
    let mut parents: Vec<Oid> = at.into_iter().collect();
    for anchor in anchors(Some(&now.head), &now.refs) {
        if !parents.contains(&anchor) {
            parents.push(anchor);
        }
    }
    let label = format!("interrupted, recovered: {id}");
    let commit = snapshot::commit_trees(
        ctx.repo,
        Some(now.index),
        Some(now.worktree_tree),
        id,
        &label,
        &parents,
        &(),
    )?;
    snapshot::move_backup_ref(ctx.repo, name, commit, &())?;
    Ok(())
}
