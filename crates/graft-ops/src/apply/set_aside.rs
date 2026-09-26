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
//! 6. [`close`] — `stash apply S` without `--index`, the index read
//!    **whatever the exit code** (ADR-0045 §7), then `S` dropped by id when
//!    nothing is unmerged; the entry's outcome decided from both.
//!
//! A rebase that stops at 5 leaves the entry `Pending` with `S` in the list;
//! `resume` runs step 6 when git is done with it, whichever way.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, PutBack, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::{GitArg, GitInvocation, RunError};

use crate::clash::Clash;
use crate::facts::FactsError;
use crate::operation::{Operation, RepoFacts};
use crate::snapshot::SnapshotError;

use super::{
    AfterState, Applied, ApplyError, Context, EditsAt, Pending, Refresh, finish, reason_for,
};

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
    let outcome = match put_back(ctx, stash, &mut commands) {
        Returned::Back => Outcome::Back,
        Returned::Clashed(paths) => Outcome::Clashed(paths),
        Returned::Kept(kept) => {
            // One read of the list serves the words and where the edits are.
            let listed = listed_at(ctx, stash);
            Outcome::Kept {
                words: kept.words(&listed, stash),
                edits: kept.edits(&listed),
                error: kept.into_error(),
            }
        }
    };
    let status = status_for(&outcome, ended.failure.as_ref(), ended.done);
    let mut unread = match finish(ctx, &mut pending, status, commands) {
        Ok(unread) => unread,
        Err(error) => {
            // The outcome line would have said where the edits are; the
            // error has to say it instead (73 review, m1).
            let kept = match &outcome {
                Outcome::Back => None,
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

/// The entry's outcome status, from the put-back and the operation's own
/// failure (see [`close`]).
fn status_for(outcome: &Outcome, failure: Option<&RunError>, done: EntryStatus) -> EntryStatus {
    let failed = failure.map(reason_for);
    match (outcome, failed) {
        (Outcome::Kept { words, .. }, Some(failed)) => EntryStatus::Aborted {
            reason: format!("{failed}; {words}"),
        },
        (Outcome::Kept { words, .. }, None) => EntryStatus::Aborted {
            reason: words.clone(),
        },
        (Outcome::Back | Outcome::Clashed(_), Some(failed)) => {
            EntryStatus::Aborted { reason: failed }
        }
        (Outcome::Back | Outcome::Clashed(_), None) => done,
    }
}

/// What [`close`] answers, once the outcome line is written.
fn answer(
    outcome: Outcome,
    entry: JournalEntry,
    stash: Oid,
    failure: Option<RunError>,
    unread: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    let recoverable = entry.worktree_before.is_some();
    match outcome {
        Outcome::Clashed(paths) => {
            let clash = Clash {
                stash,
                why: PutBackWhy::SetAside,
                paths,
            };
            if unread.is_empty() {
                return Err(ApplyError::Clashed {
                    entry: Box::new(entry),
                    clash,
                });
            }
            // The clash is re-found from the journal by `live_clash`; what
            // could not be read must not be lost behind it.
            Err(ApplyError::AfterState {
                applied: Box::new(applied(entry)),
                unread,
            })
        }
        Outcome::Kept {
            words,
            edits,
            error,
        } => Err(match (failure, error) {
            (Some(failed), _) | (None, Ok(failed)) => ApplyError::Git {
                entry: Box::new(entry),
                // Known place: git's own words, and `edits` for the caller
                // to word. Unknown: the words ride on git's line, so every
                // place that shows the error still says where they are.
                error: match edits {
                    Some(_) => failed,
                    None => naming(failed, &words),
                },
                recoverable,
                edits,
            },
            // No git error to carry the words: a read failed after the
            // operation went through. The entry is journaled, so it is an
            // after-state failure carrying the words (73 review, m2), which
            // the caller refreshes after.
            (None, Err(error)) => {
                let mut unread = unread;
                unread.insert(0, AfterState::SetAsideKept { error, words });
                ApplyError::AfterState {
                    applied: Box::new(applied(entry)),
                    unread,
                }
            }
        }),
        Outcome::Back => match failure {
            // Put back clean and dropped: the edits are simply back.
            Some(error) => Err(ApplyError::Git {
                entry: Box::new(entry),
                error,
                recoverable,
                edits: None,
            }),
            None if unread.is_empty() => Ok(applied(entry)),
            None => Err(ApplyError::AfterState {
                applied: Box::new(applied(entry)),
                unread,
            }),
        },
    }
}

/// Step 6's answer, with the words already read for a kept `S`.
enum Outcome {
    Back,
    Clashed(Vec<PathBuf>),
    Kept {
        /// What went wrong and where the edits are.
        words: String,
        /// Where the edits are, when it is known.
        edits: Option<EditsAt>,
        /// git's error, or the read that failed.
        error: Result<RunError, FactsError>,
    },
}

/// A set-aside operation that went through: a rebase or checkout, so the
/// graph is re-read.
fn applied(entry: JournalEntry) -> Applied {
    Applied {
        entry,
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: crate::undo::Replaced::default(),
    }
}

/// What step 6 came to.
enum Returned {
    /// Put back with nothing unmerged, and `S` dropped.
    Back,
    /// Put back with these paths unmerged; `S` kept.
    Clashed(Vec<PathBuf>),
    /// `S` kept for another reason.
    Kept(Kept),
}

/// Why `S` is still in the list after step 6.
enum Kept {
    /// git failed: the apply (nothing came back), or the drop after a clean
    /// apply (`back`: the edits are in the tree **and** in the list).
    Git { error: RunError, back: bool },
    /// The status could not be read after the apply, so whether it clashed
    /// is unknown; `S` is not dropped on a guess.
    Unread(FactsError),
    /// The apply was clean, but the stash list could not be read to find
    /// `S`'s position to drop it: the edits are back **and** `S` is still
    /// listed (73 review, m3).
    Unlisted(FactsError),
}

impl Kept {
    fn into_error(self) -> Result<RunError, FactsError> {
        match self {
            Kept::Git { error, .. } => Ok(error),
            Kept::Unread(why) | Kept::Unlisted(why) => Err(why),
        }
    }

    /// Where the edits are, when it is known (ADR-0045 §5 amended), from
    /// `listed`, the one read of the stash list [`close`] made.
    fn edits(&self, listed: &Result<Option<usize>, FactsError>) -> Option<EditsAt> {
        let at = listed.as_ref().ok().copied().flatten();
        match self {
            // Not back: the copy in the list is the only place they are. An
            // unfound or unreadable copy has no position to name.
            Kept::Git { back: false, .. } => at.map(|n| EditsAt::Stash { n }),
            Kept::Git { back: true, .. } => Some(EditsAt::Back { copy: at }),
            // Whether the put-back clashed is unknown: no place is claimed.
            Kept::Unread(_) => None,
            Kept::Unlisted(_) => Some(EditsAt::Back { copy: None }),
        }
    }

    /// The entry's reason: what went wrong, then where the edits are.
    fn words(&self, listed: &Result<Option<usize>, FactsError>, stash: Oid) -> String {
        match self {
            Kept::Git { error, back: false } => {
                format!("{} · {}", reason_for(error), where_listed(listed, stash))
            }
            Kept::Git { error, back: true } => format!(
                "your edits are back, and the set-aside copy could not be dropped: {} · {}",
                reason_for(error),
                match listed {
                    Ok(Some(at)) => format!("a copy stays in stash@{{{at}}}"),
                    _ => where_listed(listed, stash),
                }
            ),
            Kept::Unread(why) => format!(
                "the status after putting your edits back could not be read: {why} · {}",
                where_listed(listed, stash)
            ),
            // The list is what could not be read, so the copy is named by
            // its commit rather than by a second read of the same list.
            Kept::Unlisted(why) => format!(
                "your edits are back; the set-aside copy could not be found to drop ({why}) · \
                 it is stash commit {}",
                short(stash)
            ),
        }
    }
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

/// Step 6 itself: `stash apply S`, the status, and the drop.
fn put_back(ctx: &Context<'_>, stash: Oid, commands: &mut Vec<String>) -> Returned {
    let never = &();
    let apply = GitInvocation::new(ctx.repo).args([
        GitArg::Flag("stash"),
        GitArg::Flag("apply"),
        GitArg::Oid(stash),
    ]);
    commands.push(apply.command_line());
    let applied = apply.run(never);
    // Whatever the exit code: `stash apply` exits 1 on a clash and on a
    // refusal alike, and only the index tells them apart (ADR-0045 §7).
    let status = match graft_git::status::read(ctx.repo, never) {
        Ok(status) => status,
        Err(why) => return Returned::Kept(Kept::Unread(why.into())),
    };
    let unmerged: Vec<PathBuf> = status
        .entries
        .into_iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path)
        .collect();
    if !unmerged.is_empty() {
        return Returned::Clashed(unmerged);
    }
    if let Err(error) = applied {
        return Returned::Kept(Kept::Git { error, back: false });
    }
    match drop_by_id(ctx, stash, commands) {
        Ok(()) => Returned::Back,
        Err(Dropped::Git(error)) => Returned::Kept(Kept::Git { error, back: true }),
        Err(Dropped::Unlisted(why)) => Returned::Kept(Kept::Unlisted(why)),
    }
}

/// Why the drop did not happen.
enum Dropped {
    Git(RunError),
    /// The stash list could not be read to find `S`'s position.
    Unlisted(FactsError),
}

/// `stash drop stash@{n}`, `n` looked up by `S`'s commit id now — git will
/// not drop by a bare commit id (72's `git_refuses_a_stash_drop_by_bare_commit_id`).
/// `S` not listed at all means something else already took it out: there
/// is nothing left to drop.
fn drop_by_id(ctx: &Context<'_>, stash: Oid, commands: &mut Vec<String>) -> Result<(), Dropped> {
    let Some(at) = listed_at(ctx, stash).map_err(Dropped::Unlisted)? else {
        return Ok(());
    };
    let drop = GitInvocation::new(ctx.repo).args([
        GitArg::Flag("stash"),
        GitArg::Flag("drop"),
        GitArg::Word(format!("stash@{{{at}}}")),
    ]);
    commands.push(drop.command_line());
    drop.run(&()).map(|_output| ()).map_err(Dropped::Git)
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
                stderr_first_line: format!("{said} · {words}"),
                command,
            }
        }
        other => other,
    }
}
