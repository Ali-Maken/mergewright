//! Step 6 of `apply`: close the entry the way git left the repository —
//! held on a conflict or a pause, a put-back that clashed, done, or failed.
//! Moved out of `apply.rs` (m8/13e).

use std::path::PathBuf;

use mergewright_core::journal::{EntryStatus, JournalEntry};
use mergewright_core::oid::Oid;
use mergewright_git::state::{self, InProgress};

use crate::clash::Clash;
use crate::operation::Operation;

use super::run::{self, PlanFailure, PlanRun, with_cleanup};
use super::{
    AfterState, Applied, ApplyError, Context, Pending, Refresh, finish, lost_edit, resume,
    set_aside, stopped,
};

/// git stopped — on a conflict, or a rewrite's `edit` row: the entry stays
/// `Pending` with no outcome line and `resume` closes it (hard choice 2).
fn held(
    ctx: &mut Context<'_>,
    pending: Pending,
    commands: Vec<String>,
    kind: InProgress,
    paths: Vec<PathBuf>,
) -> ApplyError {
    ApplyError::Conflicted {
        entry: Box::new(hold(ctx, pending, commands)),
        kind,
        paths,
    }
}

/// The user stopped the write and git paused it itself: held exactly as
/// [`held`] holds a conflict (m8-polish-and-ship/13c).
fn paused(
    ctx: &mut Context<'_>,
    pending: Pending,
    commands: Vec<String>,
    kind: InProgress,
    paths: Vec<PathBuf>,
) -> ApplyError {
    ApplyError::Paused {
        entry: Box::new(hold(ctx, pending, commands)),
        kind,
        paths,
    }
}

/// The `Pending` entry keeps the commands run so far; `resume` adds its own.
fn hold(ctx: &mut Context<'_>, mut pending: Pending, commands: Vec<String>) -> JournalEntry {
    ctx.journal
        .hold_commands(&pending.entry.id, commands.clone());
    pending.entry.commands = commands;
    pending.entry
}

/// The plan has run: close the entry the way git left the repository —
/// stopped on a conflict (the entry stays `Pending`), a put-back that
/// clashed, done, or failed.
pub(super) fn closed(
    ctx: &mut Context<'_>,
    op: &Operation,
    mut pending: Pending,
    mut commands: Vec<String>,
    run: PlanRun,
) -> Result<Applied, ApplyError> {
    let PlanRun {
        failure,
        cleanup,
        edit_stop,
    } = run;
    // The user stopped git (m8-polish-and-ship/13c). How git ended does not
    // decide the outcome — the fallback stops kill it outright — what it
    // left on disk does (`stopped`): paused, landed, or ended before it
    // could pause.
    let (failure, stopped_at) = match stopped::after_a_stop(ctx, op, &pending.entry, failure)? {
        stopped::Read::Held { kind, paths } => {
            return Err(paused(ctx, pending, commands, kind, paths));
        }
        stopped::Read::Closes {
            failure,
            stopped_at,
        } => (failure, stopped_at),
    };
    // Stopped, not failed.
    //
    // `cleanup` is dropped on this path and nothing is lost by it: it is
    // filled by `remote::run`, which `run_plan` reaches only when
    // `remote_names()` is `Some`, while `starts_in_progress()` is `Some`
    // only for merge, rebase, cherry-pick and revert — all local; and by
    // `Engine::run` only when git did not stop. So here it is always empty.
    if let Some((kind, paths)) = stopped_on_conflict(ctx, op, failure.is_some() || edit_stop)? {
        return Err(held(ctx, pending, commands, kind, paths));
    }
    // A write that set edits aside: step 6, whether the operation went
    // through or git refused it (ADR-0045 §4). The stop's mark rides on its
    // answer as on any other close (19d-a2 D1): a set-aside merge or pick
    // that landed before its hook was stopped still says `Landed`.
    if let Some(stash) = set_aside::stash_of(&pending.entry) {
        let closed = closed_set_aside(ctx, op, pending, commands, stash, failure, cleanup);
        return stopped::mark(stopped_at, closed);
    }
    // A put-back that failed: the index says whether it clashed, never the
    // exit code (ADR-0045 §7). Read before the outcome, which it decides.
    // Only a failure is read: a put-back that went through left nothing
    // unmerged, and `close_unreadable_put_back` needs git's failure to name.
    let (failure, clash) = match failure {
        None => (None, None),
        Some(failed) => match clashed_put_back(ctx, &pending.entry) {
            Ok(clash) => (Some(failed), clash),
            Err(error) => {
                return Err(close_unreadable_put_back(
                    ctx,
                    &mut pending,
                    commands,
                    failed,
                    &error,
                ));
            }
        },
    };
    let status = outcome_status(failure.as_ref(), clash.is_some(), &cleanup);
    // A lost edit's put-back that went through (m7-power-tools/30w): his own
    // bytes over git's checkout, where the backup kept them. Before `finish`, so its spawns
    // are on this entry's log.
    let after_failed = lost_edit::after_put_back(ctx, op, failure.is_none(), &mut commands);
    let unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        // The outcome line is what makes the entry undoable, so this is the
        // headline — but git's own failure goes with it rather than under it,
        // and a failed raw step's words ride along, as 30s's do.
        Err(error) => {
            return Err(ApplyError::Journal {
                error,
                after: failure.map(|failed| failed.error),
                kept: after_failed.map(|after_failed| after_failed.to_string()),
            });
        }
    };

    let entry = pending.entry;
    if let Some(clash) = clash {
        return clashed(entry, clash, unread, cleanup);
    }
    match failure {
        // The content he asked for is on disk; a failed raw step is the first
        // thing the answer says went short (30s's `NotAsTheyWere`, unchanged).
        None => {
            let went_short = after_failed.into_iter().chain(unread).collect();
            stopped::mark(stopped_at, succeeded(entry, op, went_short, cleanup))
        }
        // `finish` has already put the after-state failures, if any, into the
        // entry's `Aborted` reason: git's own failure is the headline, but
        // nothing is lost behind it.
        Some(failed) => Err(failed.into_error(entry)),
    }
}

/// [`closed`] for a write that set edits aside: step 6 puts them back.
fn closed_set_aside(
    ctx: &mut Context<'_>,
    op: &Operation,
    pending: Pending,
    commands: Vec<String>,
    stash: Oid,
    failure: Option<PlanFailure>,
    cleanup: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    // Absorb's rebuilt stash only once it went through; otherwise `S`
    // itself, still with `--index` (`set_aside::put_back`).
    let index_base = match op {
        Operation::Rewrite(params) if failure.is_none() => params.index_base,
        _ => None,
    };
    // The reason goes with git's error, so a stop read as before-pause keeps
    // its words here too (19d-a2 D2).
    let failure = failure.map(|failed| set_aside::Failed {
        reason: failed.local_reason(),
        error: failed.error,
    });
    let ended = set_aside::Ended {
        failure,
        done: EntryStatus::Committed,
        unread: cleanup,
        index_base,
    };
    set_aside::close(ctx, pending, commands, stash, ended)
}

/// The outcome a run closes its entry with: `Aborted` with git's reason when
/// the plan failed, else `Committed`. A clash is an outcome, not a failure:
/// the stash's changes are in the tree with the conflicts, git kept the
/// stash, and the entry is in force so `u` is the way back (ADR-0045 §2, §6).
fn outcome_status(
    failure: Option<&PlanFailure>,
    clashed: bool,
    cleanup: &[AfterState],
) -> EntryStatus {
    match failure {
        Some(failed) if !clashed => EntryStatus::Aborted {
            reason: with_cleanup(failed.reason(), cleanup),
        },
        _ => EntryStatus::Committed,
    }
}

/// The plan ran through: what the caller re-reads, and what the record of it
/// could not keep.
fn succeeded(
    entry: JournalEntry,
    op: &Operation,
    mut unread: Vec<AfterState>,
    cleanup: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    unread.extend(cleanup);
    // A remote operation moved refs the entry does not record (a fetch's
    // tracking refs), so the graph is re-read regardless; a history
    // operation moved `HEAD` or a ref the graph draws, even when
    // `touches.refs` is empty (a checkout, m5/07).
    let refresh = if entry.touches.refs.is_empty() && !op.is_remote() && !op.is_history() {
        Refresh::WorkingTree
    } else {
        Refresh::Everything
    };
    let applied = Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry,
        refresh,
        step_backup: None,
        replaced: crate::undo::Replaced::default(),
    };
    if unread.is_empty() {
        Ok(applied)
    } else {
        // The operation itself succeeded; what failed is the record of it.
        // An `Err` the caller has to look at is the only way that reaches
        // the user, and `applied` is in it so the refresh still happens.
        Err(ApplyError::AfterState {
            applied: Box::new(applied),
            unread,
        })
    }
}

/// Did a failed put-back clash? Called only when git failed. `Some` when
/// the entry carries a `put_back` (a pop or apply) and the status lists
/// unmerged paths (ADR-0045 §1). Not [`stopped_on_conflict`]: a clash is
/// not an operation in progress — there is nothing to `c`ontinue or `a`bort.
///
/// Nothing unmerged is `None`: a plain failure (`apply --index` refusing,
/// an untracked file in the way), closed `Aborted` as before.
fn clashed_put_back(ctx: &Context<'_>, entry: &JournalEntry) -> Result<Option<Clash>, ApplyError> {
    let Some(put_back) = entry.put_back.as_ref() else {
        return Ok(None);
    };
    // Never stopped: the put-back has already run, and a stop that cut it
    // short must not also cut short the read that says what it left
    // (`record::read_after`, m3/03 review M1).
    let status =
        mergewright_git::status::read(ctx.repo, &()).map_err(|error| ApplyError::Facts(error.into()))?;
    let paths: Vec<PathBuf> = status
        .entries
        .into_iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path)
        .collect();
    if paths.is_empty() {
        return Ok(None);
    }
    Ok(Some(Clash {
        stash: put_back.stash,
        why: put_back.why,
        paths,
    }))
}

/// A put-back failed and the status could not be read to say whether it
/// clashed. The entry is closed `Aborted` all the same — an entry left open
/// reads as an operation still running — with both failures in its reason,
/// and git's failure is the error returned. The after-state `finish` reads
/// includes the index, clash or not, so `u` puts the tree back either way.
fn close_unreadable_put_back(
    ctx: &mut Context<'_>,
    pending: &mut Pending,
    commands: Vec<String>,
    failure: run::PlanFailure,
    unread: &ApplyError,
) -> ApplyError {
    let status = EntryStatus::Aborted {
        reason: format!(
            "{}; the status after could not be read: {unread}",
            failure.reason()
        ),
    };
    if let Err(error) = finish(ctx, pending, status, commands) {
        return ApplyError::Journal {
            error,
            after: Some(failure.error),
            kept: None,
        };
    }
    failure.into_error(pending.entry.clone())
}

/// A put-back that clashed, journaled `Committed`: the answer is
/// [`ApplyError::Clashed`], which carries the entry so the caller still
/// refreshes. An after-state field that could not be read is not lost
/// behind it: it goes to `AfterState` instead, whose `applied` holds the
/// same entry — and the clash is then re-found from the journal by
/// `live_clash`.
fn clashed(
    entry: JournalEntry,
    clash: Clash,
    mut unread: Vec<AfterState>,
    cleanup: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    unread.extend(cleanup);
    if unread.is_empty() {
        return Err(ApplyError::Clashed {
            entry: Box::new(entry),
            clash,
            stopped_at: None,
        });
    }
    Err(ApplyError::AfterState {
        applied: Box::new(Applied {
            came_back: Vec::new(),
            stopped_at: None,
            entry,
            refresh: Refresh::WorkingTree,
            step_backup: None,
            replaced: crate::undo::Replaced::default(),
        }),
        unread,
    })
}

/// Did git stop on a conflict rather than fail?
///
/// A non-zero exit from an operation that opens its own in-progress state,
/// with that state live afterwards, is a stop: `MERGE_HEAD` or
/// `rebase-merge/` is there and the work is half done. Anything else
/// non-zero is a failure and closes the entry (m5/07, hard choice 2). A
/// rewrite that exits 0 on an `edit` row is a stop too (`stopped`), with
/// nothing unmerged.
fn stopped_on_conflict(
    ctx: &Context<'_>,
    op: &Operation,
    stopped: bool,
) -> Result<Option<(InProgress, Vec<PathBuf>)>, ApplyError> {
    if !stopped {
        return Ok(None);
    }
    // A `FinishMerge` is `None` here on purpose (m8/13e, ruling R2): its
    // merge was begun before this operation, so a commit that fails without
    // a stop answers `Git` — entry `Aborted`, merge still live, banner still
    // `c`/`a`, nothing lost. Holding it would keep a `Pending` entry for a
    // merge Mergewright never began (13d Q4).
    let Some(started) = op.starts_in_progress() else {
        return Ok(None);
    };
    let live = state::in_progress(ctx.repo).map_err(|error| ApplyError::Facts(error.into()))?;
    if live != Some(started) {
        return Ok(None);
    }
    Ok(Some((started, resume::unmerged_paths(ctx, &())?)))
}
