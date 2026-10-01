//! The outcome of step 6 and what [`super::close`] answers once the outcome
//! line is written.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::apply::error::not_as_they_were_words;
use crate::clash::Clash;
use crate::facts::FactsError;

use super::put_back::NotAsTheyWere;
use super::returned::Kept;
use super::{
    AfterState, Applied, ApplyError, EditsAt, Refresh, appended, naming, reason_for, staging_clause,
};

/// The entry's outcome status, from the put-back and the operation's own
/// failure (see [`close`]).
pub(super) fn status_for(
    outcome: &Outcome,
    failure: Option<&RunError>,
    done: EntryStatus,
) -> EntryStatus {
    let failed = failure.map(reason_for);
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
            raw,
        } => Err(match (failure, error) {
            (Some(failed), _) | (None, Ok(failed)) => ApplyError::Git {
                entry: Box::new(entry),
                // Known place: git's own words, and `edits` for the caller
                // to word, with the staging clause when the fallback ran.
                // Unknown: the words (clause included) ride on git's line,
                // so every place that shows the error still says where they
                // are. A failed raw step rides after either.
                error: with_raw(
                    match edits {
                        Some(_) => appended(failed, &staging_clause(&unstaged)),
                        None => naming(failed, &words),
                    },
                    raw,
                ),
                recoverable,
                edits,
            },
            // No git error to carry the words: a read failed after the
            // operation went through. The entry is journaled, so it is an
            // after-state failure carrying the words (73 review, m2), which
            // the caller refreshes after. A failed raw step comes first.
            (None, Err(error)) => {
                let mut unread = unread;
                unread.insert(0, AfterState::SetAsideKept { error, words });
                if let Some(raw) = raw {
                    unread.insert(0, not_as_they_were(raw));
                }
                ApplyError::AfterState {
                    applied: Box::new(applied(entry)),
                    unread,
                }
            }
        }),
        Outcome::Back { unstaged, raw } => match failure {
            // Put back clean and dropped: the edits are simply back. When
            // their staging did not come back, git's line says so, as the
            // entry's reason does (review m7-30-06d Minor 1); likewise a
            // failed raw step.
            Some(error) => Err(ApplyError::Git {
                entry: Box::new(entry),
                error: with_raw(appended(error, &staging_clause(&unstaged)), raw),
                recoverable,
                edits: None,
            }),
            None => back(entry, unstaged, raw, unread),
        },
    }
}

/// The operation went through and the edits are back, `S` dropped. Back
/// without its staging, or not as he left them, is a completed write the
/// caller words as incomplete: [`AfterState::NotAsTheyWere`] first among
/// what it reports, then [`AfterState::StagingNotKept`] (Ruling 2026-10-01
/// §1).
fn back(
    entry: JournalEntry,
    unstaged: Vec<PathBuf>,
    raw: Option<NotAsTheyWere>,
    mut unread: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    if !unstaged.is_empty() {
        unread.insert(0, AfterState::StagingNotKept { paths: unstaged });
    }
    if let Some(raw) = raw {
        unread.insert(0, not_as_they_were(raw));
    }
    if unread.is_empty() {
        return Ok(applied(entry));
    }
    Err(ApplyError::AfterState {
        applied: Box::new(applied(entry)),
        unread,
    })
}

/// Step 6's answer, with the words already read for a kept `S`.
pub(super) enum Outcome {
    /// As [`super::Returned::Back`].
    Back {
        unstaged: Vec<PathBuf>,
        raw: Option<NotAsTheyWere>,
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
        raw: Option<NotAsTheyWere>,
    },
}

impl Outcome {
    /// A kept `S`'s outcome, from `listed`, the one read of the stash list
    /// [`super::close`] made.
    pub(super) fn kept(kept: Kept, listed: &Result<Option<usize>, FactsError>, stash: Oid) -> Self {
        let words = kept.words(listed, stash);
        let edits = kept.edits(listed);
        let unstaged = kept.unstaged().to_vec();
        let (error, raw) = kept.into_error();
        Outcome::Kept {
            words,
            edits,
            unstaged,
            error,
            raw,
        }
    }

    /// A failed raw step's words, for a line with no `unread` to carry them
    /// (the outcome line not written, [`super::close`]).
    pub(super) fn raw_words(&self) -> Option<String> {
        let (Outcome::Back { raw: Some(raw), .. } | Outcome::Kept { raw: Some(raw), .. }) = self
        else {
            return None;
        };
        Some(not_as_they_were_words(&raw.error, raw.files))
    }
}

/// The raw step's failure as the after-state it is.
pub(super) fn not_as_they_were(raw: NotAsTheyWere) -> AfterState {
    AfterState::NotAsTheyWere {
        error: raw.error,
        files: raw.files,
    }
}

/// git's error with a failed raw step's words after it, `; `-joined as
/// [`ApplyError::AfterState`] joins what it reports. Rides only on an
/// `Exited` line, as [`appended`] does.
fn with_raw(error: RunError, raw: Option<NotAsTheyWere>) -> RunError {
    match raw {
        Some(raw) => appended(error, &format!("; {}", not_as_they_were(raw))),
        None => error,
    }
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

#[cfg(test)]
mod tests;
