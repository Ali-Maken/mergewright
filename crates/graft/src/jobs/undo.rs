//! `u`, `U` and the journal read: the other place `app` opens the undo
//! journal and hands it to `ops` (`jobs::operate` is the first).
//!
//! The seam rule (ARCHITECTURE.md, `crates/graft/tests/it/seam_rule.rs`):
//! `ops` plans the inverse, checks §6.7's rails, journals and restores;
//! `app` only wires the key to the call and carries the answer back.

use std::sync::Arc;

use graft_core::journal::JournalEntry;
use graft_git::version::system_git;
use graft_ops::apply::{Applied, Context, Refresh};
use graft_ops::journal::{Journal, journal_path};
use graft_ops::restore::RestoreError;
use graft_ops::undo::{self, Drift, UndoError};

use super::Cancel;
use crate::repos::RepoStore;

/// What one undo or redo came back with.
///
/// Mirrors [`crate::jobs::operate::Operated`], and for the same reason: a
/// plain `Result<Applied, _>` cannot carry [`RestoreError::AfterState`],
/// where the restore *did* run and only part of its after-state could not be
/// read back. Two of the answers here are not failures at all — "nothing to
/// undo" is a fact about the journal, and drift is a question for the user —
/// so neither is worded as one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Undone {
    /// Ran and journaled cleanly.
    Done(Applied),
    /// Ran and journaled, but part of the after-state could not be read or
    /// kept reachable. It happened: refresh, and show `error`.
    Incomplete {
        /// What ran.
        applied: Applied,
        /// What could not be read, as one line.
        error: String,
    },
    /// There was nothing to undo or redo, or the entry cannot be inverted.
    /// An ordinary answer; the reason is already a sentence.
    Nothing(String),
    /// The repository moved since the operation, so nothing was touched.
    /// The user decides whether to go ahead (§6.7's rail).
    Drift {
        /// The entry the undo was aimed at, in the journal's own words.
        description: String,
        /// What differs from what the entry recorded.
        drift: Drift,
        /// Whether this was `U` rather than `u`, so the confirm box's
        /// answer re-runs the right one.
        redo: bool,
    },
    /// Nothing was put back, and this is why.
    Failed {
        /// What went wrong, as one line.
        error: String,
        /// What the caller must re-read anyway, or `None` where nothing can
        /// have been written. An undo restores refs before files, so a
        /// failure part-way through is `Everything`, never a narrower guess.
        refresh: Option<Refresh>,
    },
}

/// The journal as the browser shows it, plus what reading it had to close.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRead {
    /// Every entry, oldest first — the order the file has them in.
    pub entries: Vec<JournalEntry>,
    /// Interrupted entries whose after-state could not be read, and why.
    /// Their `u` restores the whole working tree after asking, and the
    /// reason for that belongs on screen rather than on the floor
    /// (m3/03 review, M1).
    pub unreadable: Vec<String>,
}

/// What a failed restore leaves stale on screen.
///
/// `None` only before `ops` began: planning, or a rail that stopped it. Once
/// [`graft_ops::restore::execute`] has been entered the answer is always
/// `Everything`, because `execute` writes the entry's `Pending` line first
/// and then runs its steps in order, breaking at the first failure and
/// journalling `Aborted`. So even a failure that touched no file has changed
/// the journal — and with two `UpdateRef` steps, a `RefMoved` on the second
/// leaves the first ref already moved and the graph showing a tip that is no
/// longer there.
///
/// This used to answer `None` for `RefMoved`, `Facts` and `Snapshot` on the
/// grounds that "the compare-and-swap refused before any file was touched".
/// True of the first step only, and never true of the journal
/// (m3/06 review, Major 3). The honest narrower answer needs `ops` to say how
/// many steps ran; until it does, a re-read of everything is the answer that
/// cannot be wrong, and it is what `Ctrl+R` does anyway.
fn refresh_after_failure(error: &UndoError) -> Option<Refresh> {
    match error {
        // Planning failed, or the rails stopped it: `execute` was never
        // entered, so not even a journal line exists.
        UndoError::Nothing(_) | UndoError::Drift(_) | UndoError::Facts(_) => None,
        UndoError::Restore(_) => Some(Refresh::Everything),
    }
}

/// `run_undo`'s answer for one `Err` from `ops`, with the `AfterState` case
/// lifted back out into [`Undone::Incomplete`] rather than collapsed into a
/// string that drops the [`Applied`] the caller refreshes from.
fn undone_from(error: UndoError, redo: bool, description: impl FnOnce() -> String) -> Undone {
    match error {
        UndoError::Nothing(_) => Undone::Nothing(error_line(&error)),
        UndoError::Drift(drift) => Undone::Drift {
            description: description(),
            drift,
            redo,
        },
        UndoError::Restore(RestoreError::AfterState { applied, unread }) => Undone::Incomplete {
            applied: *applied,
            error: unread
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; "),
        },
        other => Undone::Failed {
            refresh: refresh_after_failure(&other),
            error: error_line(&other),
        },
    }
}

/// One line for the status line, in `ops`' own words.
fn error_line(error: &UndoError) -> String {
    error.explain()
}

/// A failure before anything could run: nothing on disk to re-read.
fn nothing_ran(error: &dyn std::fmt::Display) -> Undone {
    Undone::Failed {
        error: error.to_string(),
        refresh: None,
    }
}

/// Everything an operation on this repository needs, opened fresh.
///
/// The journal is opened here and handed out by reference, because
/// [`Context`] borrows it for as long as the operation runs.
fn with_context<T>(
    repos: &RepoStore,
    cancel: &Cancel,
    then: impl FnOnce(&mut Context<'_>) -> T,
    failed: impl FnOnce(&dyn std::fmt::Display) -> T,
) -> T {
    let repo = match repos.shared() {
        Ok(repo) => repo,
        Err(error) => return failed(&error),
    };
    let handle = repo.handle();
    let git = match system_git() {
        Ok(found) => found.version(),
        Err(error) => return failed(&error),
    };
    let path = match journal_path(&handle) {
        Ok(path) => path,
        Err(error) => return failed(&error),
    };
    let mut journal = match Journal::open(&path) {
        Ok(journal) => journal,
        Err(error) => return failed(&error),
    };
    let now_unix_ms = i64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |duration| duration.as_millis()),
    )
    .unwrap_or(i64::MAX);
    let mut random = [0u8; 10];
    if let Err(error) = getrandom::fill(&mut random) {
        return failed(&format!("could not read random bytes: {error}"));
    }
    let mut ctx = match Context::open(&handle, git, &mut journal, now_unix_ms, random, cancel) {
        Ok(ctx) => ctx,
        Err(error) => return failed(&error),
    };
    then(&mut ctx)
}

/// Take back the newest operation still in force, or put back the newest
/// undo. `force` goes past §6.7's drift rail — the user has been shown what
/// moved and said go ahead.
pub(super) fn run_undo(repos: &RepoStore, redo: bool, force: bool, cancel: &Cancel) -> Undone {
    with_context(
        repos,
        cancel,
        |ctx| {
            // The entry the inverse is aimed at, read before running: a
            // drift refusal has to name it, and afterwards the journal has
            // a newer entry of its own on top.
            let aimed_at = aimed_at(ctx, redo);
            let ran = if redo {
                undo::redo(ctx, force)
            } else {
                undo::undo(ctx, force)
            };
            match ran {
                Ok(applied) => Undone::Done(applied),
                Err(error) => undone_from(error, redo, || aimed_at),
            }
        },
        nothing_ran,
    )
}

/// What `u` (or `U`) is about to take back, in the journal's own words.
fn aimed_at(ctx: &Context<'_>, redo: bool) -> String {
    let entries = ctx.journal.entries();
    let entry = if redo {
        graft_ops::journal::next_redo(entries)
    } else {
        graft_ops::journal::next_undo(entries)
    };
    entry.map_or_else(|| "operation".to_owned(), |entry| entry.description.clone())
}

/// Read the journal for the browser, closing whatever a previous run left
/// open first (DESIGN.md §6.2 step 3).
///
/// Recovery is `ops`' [`graft_ops::apply::recover_interrupted`], not a bare
/// `mark_interrupted`: it also seals the recovered entry's trees behind its
/// backup ref, without which `git gc` prunes exactly what that entry's `u`
/// would restore (group 03's review fix).
pub(super) fn read_journal(repos: &RepoStore, cancel: &Cancel) -> Result<Arc<JournalRead>, String> {
    with_context(
        repos,
        cancel,
        |ctx| {
            let recovered =
                graft_ops::apply::recover_interrupted(ctx).map_err(|error| error.to_string())?;
            let entries = ctx.journal.entries();
            let described = |id: &graft_core::journal::EntryId| {
                entries
                    .iter()
                    .find(|entry| &entry.id == id)
                    .map_or_else(|| id.as_str().to_owned(), |entry| entry.description.clone())
            };
            // `recovered.closed` is deliberately not carried: the notice is
            // derived from the entries themselves, so that it survives the
            // next read, which closes nothing (m3/06 review, Major 1).
            Ok(Arc::new(JournalRead {
                unreadable: recovered
                    .unreadable
                    .iter()
                    .map(|(id, why)| format!("{}: {why}", described(id)))
                    .collect(),
                entries: entries.to_vec(),
            }))
        },
        |error| Err(error.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A restore that stopped at a compare-and-swap still wrote a `Pending`
    /// line and journalled its `Aborted` outcome, and with more than one ref
    /// step it may have moved a ref before refusing. Answering "nothing to
    /// re-read" left the browser and possibly the graph stale
    /// (m3/06 review, Major 3).
    #[test]
    fn a_restore_that_refused_still_asks_for_a_refresh() {
        let refused = UndoError::Restore(RestoreError::RefMoved {
            name: "refs/heads/main".to_owned(),
        });
        assert_eq!(
            refresh_after_failure(&refused),
            Some(Refresh::Everything),
            "the journal changed even where no file did"
        );
    }

    /// The other half of the same rule: `ops` never began, so there is
    /// nothing new on disk to read.
    #[test]
    fn a_failure_before_ops_began_asks_for_nothing() {
        assert_eq!(
            refresh_after_failure(&UndoError::Nothing(
                graft_ops::undo::NotInvertible::NothingToUndo
            )),
            None
        );
    }
}
