//! The outcome of step 6 and what [`super::close`] answers once the outcome
//! line is written.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::apply::RawStepOf;
use crate::apply::error::not_as_they_were_words;
use crate::clash::Clash;
use crate::facts::FactsError;

use super::came_back::came_back_words;
use super::put_back::NotAsTheyWere;
use super::returned::Kept;
use super::{
    AfterState, Applied, ApplyError, Context, EditsAt, Refresh, appended, naming, staging_clause,
};

/// The entry's outcome status, from the put-back and the reason the
/// operation itself failed, if it did (see [`close`]).
pub(super) fn status_for(
    outcome: &Outcome,
    failed: Option<&str>,
    done: EntryStatus,
) -> EntryStatus {
    let failed = failed.map(str::to_owned);
    match (outcome, failed) {
        (Outcome::Kept { words, .. }, Some(failed)) => EntryStatus::Aborted {
            reason: format!("{failed}; {words}"),
        },
        (Outcome::Kept { words, .. }, None) => EntryStatus::Aborted {
            reason: words.clone(),
        },
        // The operation failed and the edits came back onto the unchanged
        // tip; the reason also says when their staging did not.
        (Outcome::Back { unstaged, .. }, Some(failed)) => EntryStatus::Aborted {
            reason: format!("{failed}{}", staging_clause(unstaged)),
        },
        (Outcome::Clashed(_), Some(failed)) => EntryStatus::Aborted { reason: failed },
        (Outcome::Back { .. } | Outcome::Clashed(_), None) => done,
    }
}

/// What [`close`] answers, once the outcome line is written.
pub(super) fn answer(
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
                    // A stop's mark goes on in `stopped::mark`, after the
                    // close (19d-a3).
                    stopped_at: None,
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
            unstaged,
            error,
            not_as_they_were,
        } => Err(match (failure, error) {
            (Some(failed), _) | (None, Ok(failed)) => ApplyError::Git {
                entry: Box::new(entry),
                // Known place: git's own words, and `edits` for the caller
                // to word, with the staging clause when the fallback ran.
                // Unknown: the words (clause included) ride on git's line,
                // so every place that shows the error still says where they
                // are. A failed raw step rides after either.
                error: with_not_as_they_were(
                    match edits {
                        Some(_) => appended(failed, &staging_clause(&unstaged)),
                        None => naming(failed, &words),
                    },
                    not_as_they_were,
                ),
                recoverable,
                edits,
                // Set-aside writes are a rebase's or a checkout's: none runs
                // inside a shell merge (`runs_in_progress`).
                paused: None,
            },
            // No git error to carry the words: a read failed after the
            // operation went through. The entry is journaled, so it is an
            // after-state failure carrying the words (73 review, m2), which
            // the caller refreshes after. A failed raw step comes first.
            (None, Err(error)) => kept_but_unread(entry, error, words, not_as_they_were, unread),
        }),
        Outcome::Back {
            unstaged,
            not_as_they_were,
            came_back,
        } => back(
            entry,
            failure,
            unstaged,
            not_as_they_were,
            came_back,
            unread,
        ),
    }
}

/// Kept, git did not fail, but a read did: the words ride as the first
/// unread after-state, behind a failed raw step if there was one.
fn kept_but_unread(
    entry: JournalEntry,
    error: FactsError,
    words: String,
    not_as_they_were: Option<NotAsTheyWere>,
    mut unread: Vec<AfterState>,
) -> ApplyError {
    unread.insert(0, AfterState::SetAsideKept { error, words });
    if let Some(not_as_they_were) = not_as_they_were {
        unread.insert(0, answer_not_as_they_were(not_as_they_were));
    }
    ApplyError::AfterState {
        applied: Box::new(applied(entry)),
        unread,
    }
}

/// Put back clean and dropped, but git failed after. When their staging did
/// not come back, git's line says so, as the entry's reason does (review
/// m7-30-06d Minor 1); likewise a failed raw step.
fn back_but_git_failed(
    entry: JournalEntry,
    error: RunError,
    unstaged: &[PathBuf],
    came_back: &[PathBuf],
    not_as_they_were: Option<NotAsTheyWere>,
) -> ApplyError {
    // A file back from the index rides on git's line as the staging clause
    // does: this answer has no `Applied` to carry it (review m8-14 Major 3).
    let came_back = came_back_words(came_back)
        .map(|words| format!(" \u{b7} {words}"))
        .unwrap_or_default();
    let error = appended(error, &format!("{}{came_back}", staging_clause(unstaged)));
    ApplyError::Git {
        recoverable: entry.worktree_before.is_some(),
        entry: Box::new(entry),
        error: with_not_as_they_were(error, not_as_they_were),
        edits: None,
        paused: None,
    }
}

/// The operation went through and the edits are back, `S` dropped. Back
/// without its staging, or not as he left them, is a completed write the
/// caller words as incomplete: [`AfterState::NotAsTheyWere`] first among
/// what it reports, then [`AfterState::StagingNotKept`] (Ruling 2026-10-01
/// §1). `failure`: git failed after all ([`back_but_git_failed`]).
/// `came_back` goes on the `Applied` either way, for the landing line.
fn back(
    entry: JournalEntry,
    failure: Option<RunError>,
    unstaged: Vec<PathBuf>,
    not_as_they_were: Option<NotAsTheyWere>,
    came_back: Vec<PathBuf>,
    mut unread: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    if let Some(error) = failure {
        return Err(back_but_git_failed(
            entry,
            error,
            &unstaged,
            &came_back,
            not_as_they_were,
        ));
    }
    if !unstaged.is_empty() {
        unread.insert(0, AfterState::StagingNotKept { paths: unstaged });
    }
    if let Some(not_as_they_were) = not_as_they_were {
        unread.insert(0, answer_not_as_they_were(not_as_they_were));
    }
    let applied = Applied {
        came_back,
        ..applied(entry)
    };
    if unread.is_empty() {
        return Ok(applied);
    }
    Err(ApplyError::AfterState {
        applied: Box::new(applied),
        unread,
    })
}

/// Step 6's answer, with the words already read for a kept `S`.
pub(super) enum Outcome {
    /// As [`super::Returned::Back`].
    Back {
        unstaged: Vec<PathBuf>,
        not_as_they_were: Option<NotAsTheyWere>,
        /// Staged-new files he had deleted from disk that git checked out
        /// again (`super::came_back`): named on every line that answers
        /// this outcome — clean, after-state, git failure, or no outcome
        /// line (review m8-14 Major 3).
        came_back: Vec<PathBuf>,
    },
    Clashed(Vec<PathBuf>),
    Kept {
        /// What went wrong and where the edits are.
        words: String,
        /// Where the edits are, when it is known.
        edits: Option<EditsAt>,
        /// The staged paths whose staging did not come back, when the edits
        /// are back through the fallback (already in `words`).
        unstaged: Vec<PathBuf>,
        /// git's error, or the read that failed.
        error: Result<RunError, FactsError>,
        /// The raw step's failure, when the edits are back (not in `words`).
        not_as_they_were: Option<NotAsTheyWere>,
    },
}

impl Outcome {
    /// A kept `S`'s outcome, from `listed`, the one read of the stash list
    /// [`super::close`] made.
    pub(super) fn kept(kept: Kept, listed: &Result<Option<usize>, FactsError>, stash: Oid) -> Self {
        let words = kept.words(listed, stash);
        let edits = kept.edits(listed);
        let unstaged = kept.unstaged().to_vec();
        let (error, not_as_they_were) = kept.into_error();
        Outcome::Kept {
            words,
            edits,
            unstaged,
            error,
            not_as_they_were,
        }
    }

    /// A failed raw step's words, for a line with no `unread` to carry them
    /// (the outcome line not written, [`super::close`]).
    pub(super) fn failed_words(&self) -> Option<String> {
        let (Outcome::Back {
            not_as_they_were: Some(not_as_they_were),
            ..
        }
        | Outcome::Kept {
            not_as_they_were: Some(not_as_they_were),
            ..
        }) = self
        else {
            return None;
        };
        Some(not_as_they_were_words(
            &not_as_they_were.error,
            not_as_they_were.files,
        ))
    }
}

/// The raw step's failure as the after-state it is.
pub(super) fn answer_not_as_they_were(not_as_they_were: NotAsTheyWere) -> AfterState {
    AfterState::NotAsTheyWere {
        error: not_as_they_were.error,
        files: not_as_they_were.files,
        of: RawStepOf::SetAside,
    }
}

/// git's error with a failed raw step's words after it, `; `-joined as
/// [`ApplyError::AfterState`] joins what it reports. Rides only on an
/// `Exited` line, as [`appended`] does.
fn with_not_as_they_were(error: RunError, not_as_they_were: Option<NotAsTheyWere>) -> RunError {
    match not_as_they_were {
        Some(not_as_they_were) => appended(
            error,
            &format!("; {}", answer_not_as_they_were(not_as_they_were)),
        ),
        None => error,
    }
}

/// A set-aside operation that went through: a rebase or checkout, so the
/// graph is re-read.
fn applied(entry: JournalEntry) -> Applied {
    Applied {
        came_back: Vec::new(),
        stopped_at: None,
        entry,
        refresh: Refresh::Everything,
        step_backup: None,
        replaced: crate::undo::Replaced::default(),
    }
}

/// The outcome line could not be written: what it would have said about
/// the edits, for the error to say instead (73 review, m1) — where a kept
/// or clashed `S` is, a staging not kept, a file back from the index
/// (review m8-14 Major 3). `None` when the edits are back with nothing to
/// add.
pub(super) fn unwritten_words(ctx: &Context<'_>, outcome: &Outcome, stash: Oid) -> Option<String> {
    let kept = match outcome {
        Outcome::Back {
            unstaged,
            came_back,
            ..
        } => {
            let staging = (!unstaged.is_empty()).then(|| {
                AfterState::StagingNotKept {
                    paths: unstaged.clone(),
                }
                .to_string()
            });
            match (staging, came_back_words(came_back)) {
                (Some(staging), Some(came_back)) => Some(format!("{staging}; {came_back}")),
                (staging, came_back) => staging.or(came_back),
            }
        }
        Outcome::Clashed(_) => Some(format!(
            "your edits clashed when put back · {}",
            super::where_kept(ctx, stash)
        )),
        Outcome::Kept { words, .. } => Some(words.clone()),
    };
    // A failed raw step is in no outcome's words: first, as in `unread`
    // (Ruling 2026-10-01 §1).
    match (outcome.failed_words(), kept) {
        (Some(failed_words), Some(kept)) => Some(format!("{failed_words}; {kept}")),
        (failed_words, kept) => failed_words.or(kept),
    }
}

#[cfg(test)]
mod tests;
