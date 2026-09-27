//! Setting edits aside around a rebase or checkout and putting them back,
//! inside the operation's one journal entry (ADR-0045 §4).
//!
//! [`apply`](super::apply) runs the order; this module holds the steps that
//! are not the operation's own:
//!
//! 1. [`create`] — `git stash create` → `S`. Objects only: no ref, index or
//!    file changes. Before `begin`, so `S` can go on the Pending line.
//! 2. `begin` — the backup and the Pending line, with `put_back =
//!    SetAside(S)`, `pins ∋ S` and `touches ∋ refs/stash`.
//! 3. and 4. [`put_aside`] — `stash store -m <words> S`, then `reset --hard`
//!    (tracked files only; untracked files stay where they are).
//! 5. The operation's own plan.
//! 6. [`close`] — `stash apply --index S`, the index read **whatever the
//!    exit code** (ADR-0045 §7), falling back to a plain `stash apply S`
//!    when git could not reinstate the index and wrote nothing (the
//!    2026-09-26 b amendment, [`put_back`]); then `S` dropped by id when
//!    nothing is unmerged; the entry's outcome decided from both. Absorb,
//!    gone through, applies `S` rebuilt on its base (06b,
//!    `rebuild::from_index_base`).
//!
//! A rebase that stops at 5 leaves the entry `Pending` with `S` in the list;
//! `resume` runs step 6 when git is done with it, whichever way.

use graft_core::journal::{EntryStatus, JournalEntry, PutBack, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError};

use crate::facts::FactsError;
use crate::operation::{Operation, RepoFacts};
use crate::snapshot::SnapshotError;

use super::error::staging_clause;
use super::{
    AfterState, Applied, ApplyError, Context, EditsAt, Pending, Refresh, finish, reason_for,
};

mod answer;
mod put_back;
mod rebuild;
mod returned;

use answer::{Outcome, answer, status_for};
use returned::{Returned, put_back};

pub(super) use put_back::tracked_changed;

/// The message `S` is created and listed under (ADR-0045 §4 step 3).
pub(super) fn words(description: &str) -> String {
    format!("graft: your edits, set aside for {description}")
}

/// The set-aside stash an entry carries, if it carries one.
pub(super) fn stash_of(entry: &JournalEntry) -> Option<Oid> {
    match entry.put_back {
        Some(PutBack {
            stash,
            why: PutBackWhy::SetAside,
        }) => Some(stash),
        _ => None,
    }
}

/// Step 1: `git stash create <message>` when the operation sets edits aside.
/// `None` when it does not, or when git finds nothing to save.
///
/// Nothing is journaled yet, so a failure here is reported the way a backup
/// that could not be taken is — the same kind of write (objects only), and
/// the same promise: the operation did not run.
pub(super) fn create(
    ctx: &Context<'_>,
    op: &Operation,
    facts: &RepoFacts,
    message: &str,
    commands: &mut Vec<String>,
) -> Result<Option<Oid>, ApplyError> {
    if op.sets_aside(facts).is_none() {
        return Ok(None);
    }
    let git = GitInvocation::new(ctx.repo).args([
        GitArg::Flag("stash"),
        GitArg::Flag("create"),
        GitArg::Word(message.to_owned()),
    ]);
    commands.push(git.command_line());
    let printed = git
        .run(ctx.stop)
        .map_err(|error| match error {
            RunError::Stopped => ApplyError::Stopped,
            other => ApplyError::Snapshot(SnapshotError::Run(other)),
        })?
        .stdout;
    let printed = String::from_utf8_lossy(&printed);
    let line = printed.trim();
    if line.is_empty() {
        return Ok(None);
    }
    Oid::from_hex(line).map(Some).map_err(|_| {
        ApplyError::Snapshot(SnapshotError::NotAnOid {
            what: "stash create",
            printed: line.lines().next().unwrap_or_default().to_owned(),
        })
    })
}

/// Steps 3 and 4: list `S`, then clear the tracked edits from the tree.
/// Each command goes on the entry before it runs, so a failure shows it.
///
/// The stop is asked before each step, not only inside it: the runner asks
/// only once git is spawned, so a stop already asked for would still start
/// the next step, and on a loaded machine that git can finish before the
/// kill lands — a stop "before 3" that stored `S` anyway (M7/02b). Between
/// two steps is the one place the outcome is exact, as in `restore`.
pub(super) fn put_aside(
    ctx: &Context<'_>,
    stash: Oid,
    message: &str,
    commands: &mut Vec<String>,
) -> Result<(), RunError> {
    let steps = [
        vec![
            GitArg::Flag("stash"),
            GitArg::Flag("store"),
            GitArg::Flag("-m"),
            GitArg::Word(message.to_owned()),
            GitArg::Oid(stash),
        ],
        vec![GitArg::Flag("reset"), GitArg::Flag("--hard")],
    ];
    for args in steps {
        if ctx.stop.should_stop() {
            return Err(RunError::Stopped);
        }
        let git = GitInvocation::new(ctx.repo).args(args);
        commands.push(git.command_line());
        git.run(ctx.stop)?;
    }
    Ok(())
}

/// Step 3 or 4 failed (or was stopped): the operation never ran. The entry
/// is closed `Aborted`, and when `S` made it into the list the words say
/// where the edits are (ADR-0045 §5: stopped at 3, `S` is objects only,
/// pinned by the backup; at 4, `S` is listed and `u` restores the backup).
///
/// `left` is what the caller could not clean up as the entry closes (a
/// rewrite's saved plan, `engine::remove`): it joins the `Aborted` reason,
/// as every after-state failure of an aborted entry does (`finish`).
pub(super) fn abandon(
    ctx: &mut Context<'_>,
    mut pending: Pending,
    commands: Vec<String>,
    stash: Oid,
    error: RunError,
    left: Option<AfterState>,
) -> Result<Applied, ApplyError> {
    let listed = listed_at(ctx, stash);
    // `error` stays git's own words: where the edits are is `edits`, and the
    // caller words it (ADR-0045 §5 amended).
    let (reason, edits) = match &listed {
        Ok(Some(at)) => (
            format!("{} · {}", reason_for(&error), in_stash(*at)),
            Some(EditsAt::Backup { copy: Some(*at) }),
        ),
        // Not listed: the tree was never touched, the edits are where they
        // were.
        Ok(None) => (reason_for(&error), None),
        // Unreadable: stored or not, the reset may have run, and the backup
        // holds the tree as it was either way.
        Err(why) => (
            format!(
                "{}; the stash list could not be read to find {}: {why}",
                reason_for(&error),
                short(stash)
            ),
            Some(EditsAt::Backup { copy: None }),
        ),
    };
    let reason = match left {
        Some(left) => format!("{reason}; {left}"),
        None => reason,
    };
    if let Err(journal) = finish(ctx, &mut pending, EntryStatus::Aborted { reason }, commands) {
        // No outcome line to say where the edits are: the error says it.
        return Err(ApplyError::Journal {
            error: journal,
            after: Some(error),
            kept: edits.map(|edits| edits.words()),
        });
    }
    let recoverable = pending.entry.worktree_before.is_some();
    Err(ApplyError::Git {
        entry: Box::new(pending.entry),
        error,
        recoverable,
        edits,
    })
}

/// How the operation (step 5, or its resume) ended, for [`close`].
pub(super) struct Ended {
    /// git's own failure of the operation, when it failed without stopping.
    pub(super) failure: Option<RunError>,
    /// The status when the operation went through: `Committed`, or
    /// `Aborted("aborted")` for `--abort`.
    pub(super) done: EntryStatus,
    /// After-state failures the caller already holds (askpass cleanup, a
    /// backup ref gone while stopped).
    pub(super) unread: Vec<AfterState>,
    /// Absorb's `RewriteParams::index_base`, when the rewrite went through
    /// (not failed, not aborted): the put-back applies `S` rebuilt on it
    /// ([`put_back`]). `None` for every other operation and ending, which
    /// apply `S` itself — with `--index` all the same.
    pub(super) index_base: Option<Oid>,
}

/// Step 6 and the outcome line: put `S` back, then close the entry the way
/// the operation and the put-back left the repository.
///
/// - Put back, nothing unmerged: `S` dropped by id; the entry closes as the
///   operation did (`done`, or `Aborted` with git's reason when it failed —
///   ADR-0045 §4, "fails at 5": the edits back onto the unchanged tip, then
///   the refusal).
/// - Unmerged: `S` kept, [`ApplyError::Clashed`] with `why: SetAside`. The
///   entry is `Committed` (in force, so `u` is the way back and `live_clash`
///   finds it) — unless the operation itself had failed, when it is
///   `Aborted` with git's reason, so that failure is not lost behind the
///   clash. Either way it is in force: its index changed.
/// - Anything else (git refused the put-back, the status could not be read,
///   the drop failed or could not find `S`): `S` kept, the entry `Aborted`
///   with words naming where the edits are, and [`ApplyError::Git`] saying
///   the same — or, when a read failed with no git failure to carry them,
///   [`ApplyError::AfterState`] with [`AfterState::SetAsideKept`].
/// - The outcome line cannot be written: [`ApplyError::Journal`], whose
///   `kept` carries those words for a kept or clashed `S`.
///
/// The put-back never stops: it is the second half of a write that has
/// already happened, and a stop that cut it short would leave the edits in
/// the list for no reason (the same rule as `record::read_after`).
pub(super) fn close(
    ctx: &mut Context<'_>,
    mut pending: Pending,
    mut commands: Vec<String>,
    stash: Oid,
    ended: Ended,
) -> Result<Applied, ApplyError> {
    let outcome = match put_back(ctx, stash, ended.index_base, &mut commands) {
        Returned::Back { unstaged } => Outcome::Back { unstaged },
        Returned::Clashed(paths) => Outcome::Clashed(paths),
        Returned::Kept(kept) => {
            // One read of the list serves the words and where the edits are.
            Outcome::kept(kept, &listed_at(ctx, stash), stash)
        }
    };
    let status = status_for(&outcome, ended.failure.as_ref(), ended.done);
    let mut unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            // The outcome line would have said where the edits are; the
            // error has to say it instead (73 review, m1).
            let kept = match &outcome {
                Outcome::Back { unstaged } if unstaged.is_empty() => None,
                Outcome::Back { unstaged } => Some(
                    AfterState::StagingNotKept {
                        paths: unstaged.clone(),
                    }
                    .to_string(),
                ),
                Outcome::Clashed(_) => Some(format!(
                    "your edits clashed when put back · {}",
                    where_kept(ctx, stash)
                )),
                Outcome::Kept { words, .. } => Some(words.clone()),
            };
            return Err(ApplyError::Journal {
                error,
                after: ended.failure,
                kept,
            });
        }
    };
    unread.extend(ended.unread);
    answer(outcome, pending.entry, stash, ended.failure, unread)
}

/// Where `S` is, for words: `stash@{n}` when listed, else its commit.
fn where_kept(ctx: &Context<'_>, stash: Oid) -> String {
    where_listed(&listed_at(ctx, stash), stash)
}

/// [`where_kept`] over a list already read.
fn where_listed(listed: &Result<Option<usize>, FactsError>, stash: Oid) -> String {
    match listed {
        Ok(Some(at)) => in_stash(*at),
        Ok(None) => format!("your edits are in stash commit {}", short(stash)),
        Err(why) => format!(
            "your edits are in stash commit {} (the stash list could not be read: {why})",
            short(stash)
        ),
    }
}

/// Where `S` is in the stash list now, read by commit id — never assumed.
fn listed_at(ctx: &Context<'_>, stash: Oid) -> Result<Option<usize>, FactsError> {
    let listed = graft_git::stash::commits(ctx.repo, &())?;
    Ok(listed
        .into_iter()
        .find(|(commit, _)| *commit == stash)
        .map(|(_, at)| at))
}

fn in_stash(at: usize) -> String {
    format!("your edits are in stash@{{{at}}}")
}

fn short(stash: Oid) -> String {
    stash.to_string().chars().take(7).collect()
}

/// git's error with the stash words on its first line, so every place that
/// shows the error (`ApplyError::explain`, the Display) names `stash@{n}`.
/// Only an `Exited` error has a line of git's to extend; any other (a stop,
/// git missing) is returned as it is, and the entry's reason carries the
/// words.
fn naming(error: RunError, words: &str) -> RunError {
    appended(error, &format!(" · {words}"))
}

/// git's error with `tail` added to its first line, as [`naming`] adds the
/// stash words: how the staging clause rides on a failure line
/// (review m7-30-06d Minor 1). An empty `tail` leaves it as it is.
fn appended(error: RunError, tail: &str) -> RunError {
    if tail.is_empty() {
        return error;
    }
    match error {
        RunError::Exited {
            status,
            stderr_first_line,
            command,
        } => {
            let said = if stderr_first_line.is_empty() {
                format!("`{command}` failed ({status})")
            } else {
                stderr_first_line
            };
            RunError::Exited {
                status,
                stderr_first_line: format!("{said}{tail}"),
                command,
            }
        }
        other => other,
    }
}
