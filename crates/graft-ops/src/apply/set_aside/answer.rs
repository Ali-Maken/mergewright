//! The outcome of step 6 and what [`super::close`] answers once the outcome
//! line is written.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::clash::Clash;
use crate::facts::FactsError;

use super::{
    AfterState, Applied, ApplyError, EditsAt, Refresh, naming, reason_for, staging_clause,
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
        (Outcome::Back { unstaged }, Some(failed)) => EntryStatus::Aborted {
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
        Outcome::Back { unstaged } => match failure {
            // Put back clean and dropped: the edits are simply back (the
            // entry's reason says when their staging did not come back).
            Some(error) => Err(ApplyError::Git {
                entry: Box::new(entry),
                error,
                recoverable,
                edits: None,
            }),
            None => back(entry, unstaged, unread),
        },
    }
}

/// The operation went through and the edits are back, `S` dropped. Back
/// without its staging is a completed write the caller words as
/// incomplete: [`AfterState::StagingNotKept`], first among what it reports.
fn back(
    entry: JournalEntry,
    unstaged: Vec<PathBuf>,
    mut unread: Vec<AfterState>,
) -> Result<Applied, ApplyError> {
    if !unstaged.is_empty() {
        unread.insert(0, AfterState::StagingNotKept { paths: unstaged });
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
    /// As [`Returned::Back`].
    Back {
        unstaged: Vec<PathBuf>,
    },
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
