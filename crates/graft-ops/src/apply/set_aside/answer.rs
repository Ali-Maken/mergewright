//! The outcome of step 6 and what [`super::close`] answers once the outcome
//! line is written.

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::RunError;

use crate::clash::Clash;
use crate::facts::FactsError;

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
            unstaged,
            error,
        } => Err(match (failure, error) {
            (Some(failed), _) | (None, Ok(failed)) => ApplyError::Git {
                entry: Box::new(entry),
                // Known place: git's own words, and `edits` for the caller
                // to word, with the staging clause when the fallback ran.
                // Unknown: the words (clause included) ride on git's line,
                // so every place that shows the error still says where they
                // are.
                error: match edits {
                    Some(_) => appended(failed, &staging_clause(&unstaged)),
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
            // Put back clean and dropped: the edits are simply back. When
            // their staging did not come back, git's line says so, as the
            // entry's reason does (review m7-30-06d Minor 1).
            Some(error) => Err(ApplyError::Git {
                entry: Box::new(entry),
                error: appended(error, &staging_clause(&unstaged)),
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
        /// The staged paths whose staging did not come back, when the edits
        /// are back through the fallback (already in `words`).
        unstaged: Vec<PathBuf>,
        /// git's error, or the read that failed.
        error: Result<RunError, FactsError>,
    },
}

impl Outcome {
    /// A kept `S`'s outcome, from `listed`, the one read of the stash list
    /// [`super::close`] made.
    pub(super) fn kept(kept: Kept, listed: &Result<Option<usize>, FactsError>, stash: Oid) -> Self {
        Outcome::Kept {
            words: kept.words(listed, stash),
            edits: kept.edits(listed),
            unstaged: kept.unstaged().to_vec(),
            error: kept.into_error(),
        }
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
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used)]

    use graft_core::journal::OperationKind;

    use super::*;
    use crate::journal::tests::committed;

    fn exited(said: &str) -> RunError {
        RunError::Exited {
            status: 1,
            stderr_first_line: said.to_owned(),
            command: "git something".to_owned(),
        }
    }

    fn stash() -> Oid {
        Oid::from_hex("acf15d58a3b48e5878fe281a624fb5c688dcaee1").unwrap()
    }

    fn f_txt() -> Vec<PathBuf> {
        vec![PathBuf::from("f.txt")]
    }

    /// B1 (review m7-30-06d Minor 1): the operation failed, the edits came
    /// back through the fallback. The failure line says the staging went.
    #[test]
    fn a_failed_operation_after_the_fallback_names_the_staging() {
        let entry = committed(1, OperationKind::Checkout, 1, 2);
        let error = answer(
            Outcome::Back { unstaged: f_txt() },
            entry,
            stash(),
            Some(exited("error: your local changes would be overwritten")),
            Vec::new(),
        )
        .expect_err("the operation failed");
        assert!(matches!(error, ApplyError::Git { .. }), "{error:?}");
        let line = error.explain();
        assert!(line.contains("staging not kept: f.txt"), "{line}");
        assert!(line.contains("would be overwritten"), "{line}");
    }

    /// B2: the drop failed after the fallback. The line says where the
    /// edits are **and** that their staging went.
    #[test]
    fn a_failed_drop_after_the_fallback_names_the_staging() {
        let entry = committed(1, OperationKind::Checkout, 1, 2);
        let kept = Kept::Git {
            error: exited("error: could not drop"),
            back: true,
            unstaged: f_txt(),
        };
        let outcome = Outcome::kept(kept, &Ok(Some(0)), stash());
        let error = answer(outcome, entry, stash(), None, Vec::new()).expect_err("S kept");
        let line = error.explain();
        assert!(line.contains("staging not kept: f.txt"), "{line}");
        assert!(
            line.contains("your edits are back · a copy stays in stash@{0}"),
            "{line}"
        );
    }
}
