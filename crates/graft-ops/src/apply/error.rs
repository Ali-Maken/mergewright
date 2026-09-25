//! How an operation fails: [`ApplyError`], the after-state fields that could
//! not be read ([`AfterState`]), and the one-line reason an `Aborted` entry
//! carries.

use std::path::PathBuf;

use graft_core::journal::JournalEntry;
use graft_git::remote::RemoteFailure;
use graft_git::run::RunError;
use graft_git::state::InProgress;

use crate::clash::Clash;
use crate::facts::FactsError;
use crate::journal::JournalError;
use crate::operation::{Refusal, RemoteNames};
use crate::snapshot::SnapshotError;

use super::{Applied, PrepareError, remote};

/// Why an operation did not run, or did not finish.
#[derive(Debug, thiserror::Error)]
pub enum ApplyError {
    /// A precondition said no. Nothing was written and nothing journaled.
    #[error("{}", .0.explain())]
    Refused(Refusal),
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// The backup could not be taken. The operation did not run.
    #[error("{0}")]
    Snapshot(SnapshotError),
    /// The journal could not be written. Before the operation ran, that means
    /// it did not run: a write Graft cannot record is a write it must not
    /// make. After it ran, the outcome line is missing — and `after` carries
    /// git's own failure, which would otherwise be lost behind this one
    /// (m3/03 review, Minor 1). `kept` says where a set-aside's edits are
    /// when the put-back left them in the stash list (kept, or clashed), so
    /// the missing outcome line does not take that with it (m6-conflicts/83).
    #[error("{}", journal_failed(.error, .after.as_ref(), .kept.as_deref()))]
    Journal {
        /// What the journal said.
        error: JournalError,
        /// What git said, when the operation had already failed.
        after: Option<RunError>,
        /// Where the set-aside edits are, when not simply back in the tree.
        kept: Option<String>,
    },
    /// The operation ran and is journaled, but part of what it left behind
    /// could not be read, or could not be kept reachable. The write happened,
    /// so the caller still refreshes — `applied` carries it — and `u` still
    /// works, more widely than it should.
    #[error("{}: {}", .applied.entry.description, joined(.unread))]
    AfterState {
        /// What ran. Boxed for the same reason as `Git`'s entry.
        applied: Box<Applied>,
        /// What could not be read or sealed, in the order it was tried.
        unread: Vec<AfterState>,
    },
    /// `git` failed part-way. The entry is journaled `Aborted`.
    #[error("{}: {error}", .entry.description)]
    Git {
        /// The entry, with whatever after-state could be read. Boxed: an
        /// entry is much larger than every other variant.
        entry: Box<JournalEntry>,
        /// What `git` said. When `edits` is `Some`, only git's own failure:
        /// where the edits are is `edits`, worded by the caller.
        error: RunError,
        /// A backup exists, so `u` can put the repository back.
        recoverable: bool,
        /// Where a set-aside's edits are, when the entry set edits aside and
        /// did not end in a clean put-back or a clash (ADR-0045 §5 amended).
        edits: Option<EditsAt>,
    },
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
    /// A fetch, pull or push failed. The entry is journaled `Aborted` with
    /// the failure's short text; `failure` is what the whole of git's stderr
    /// said (a stop is `Cancelled`).
    #[error("{}", remote::explain(.entry, .failure, .stderr_tail, .names))]
    Remote {
        /// The entry, with whatever after-state could be read.
        entry: Box<JournalEntry>,
        /// Why, as far as git said.
        failure: RemoteFailure,
        /// git's last few stderr segments, for a detail view.
        stderr_tail: Vec<String>,
        /// The branch and remote the failure line names, from the
        /// operation's params (`Operation::remote_names`).
        names: Box<RemoteNames>,
    },
    /// A history operation stopped on a conflict: git exited non-zero and
    /// left its own in-progress state behind (`MERGE_HEAD`, `rebase-merge/`
    /// …). The entry stays `Pending` — one entry from start to finish, M5
    /// design note hard choice 2 — and [`resume`] closes it. No outcome line
    /// is written here. `paths` are the unmerged paths the status lists
    /// (m5/07).
    #[error("{}: {} unmerged · resolve, then c continues, a aborts", entry.description, paths.len())]
    Conflicted {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state git left: what the banner shows and what `resume` runs.
        kind: InProgress,
        /// The unmerged paths.
        paths: Vec<PathBuf>,
    },
    /// A stash put back (a pop or apply) clashed: git failed and left
    /// unmerged paths. The entry is journaled `Committed` — the stash's
    /// changes and the conflicts are the outcome, the index-after is the
    /// stage tree, and `u` is the way back — and git kept the stash
    /// (ADR-0045 §1–2). Not retryable, like `Conflicted`; unlike it there is
    /// no operation in progress, so no `c` and no `a`.
    #[error("{} left conflicts in {} {}", entry.description, clash.paths.len(), if clash.paths.len() == 1 { "file" } else { "files" })]
    Clashed {
        /// The entry, `Committed`.
        entry: Box<JournalEntry>,
        /// The stash, why it was put back, and the unmerged paths.
        clash: Clash,
    },
    /// [`resume`] ran `--continue`, `--skip` or `--abort`, git refused, and
    /// its in-progress state is still there with no unmerged path to explain
    /// it (a hook said no, an empty pick). The entry stays `Pending`, the
    /// banner stays up; git's line says what to do (m5/07).
    #[error("{}: {} · the {} is still in progress", entry.description, reason_for(error), kind.name())]
    ResumeFailed {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state still live.
        kind: InProgress,
        /// What git said.
        error: RunError,
    },
}

/// Where a set-aside's edits are when the entry did not end in a clean
/// put-back or a clash (ADR-0045 §5 amended, m6-conflicts/84). Decided here,
/// where it is known whether the put-back ran; the app words it and reads no
/// stash list to guess.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditsAt {
    /// Not in the tree; the copy is listed at `n`. `S p` puts them back.
    Stash {
        /// The copy's position in the stash list, read by commit id.
        n: usize,
    },
    /// Back in the tree; a copy stays listed at `n`, or was not found to
    /// drop (`None`).
    Back {
        /// The copy's position, when it was found.
        copy: Option<usize>,
    },
    /// Stored, then the reset (ADR-0045 §4 step 4) failed or stopped: the
    /// entry's backup has them.
    Backup {
        /// The copy's position, when the list could be read.
        copy: Option<usize>,
    },
}

impl EditsAt {
    /// Where the edits are, in `graft-ops`' own words (no keys): the tail
    /// [`ApplyError::explain`] adds.
    #[must_use]
    pub fn words(&self) -> String {
        match self {
            EditsAt::Stash { n } => format!("your edits are in stash@{{{n}}}"),
            EditsAt::Back { copy: Some(n) } => {
                format!("your edits are back · a copy stays in stash@{{{n}}}")
            }
            EditsAt::Back { copy: None } => {
                "your edits are back · a copy may still be in the stash list".to_owned()
            }
            EditsAt::Backup { copy: Some(n) } => {
                format!("the backup has your edits · a copy is in stash@{{{n}}}")
            }
            EditsAt::Backup { copy: None } => "the backup has your edits".to_owned(),
        }
    }
}

/// A field of an entry's after-state that could not be read, or could not be
/// kept reachable.
///
/// The outcome line is written either way — an entry with no outcome reads as
/// an operation still in flight, which this one is not — so the failure has
/// nowhere else to go but back to the caller. It is never dropped: what it
/// costs the user is a wider `u` (an unreadable tree) or an `u` that refuses
/// (unreadable refs, which `generic_steps` reads as `Incomplete`), and they
/// are owed the reason at the moment it happens rather than a fortnight
/// later.
#[derive(Debug, thiserror::Error)]
pub enum AfterState {
    /// `HEAD` could not be read after the operation.
    #[error("HEAD after the operation: {0}")]
    Head(FactsError),
    /// The refs the operation touched could not be read after it.
    #[error("the refs after the operation: {0}")]
    Refs(FactsError),
    /// The index could not be written as a tree after the operation.
    #[error("the index after the operation: {0}")]
    Index(SnapshotError),
    /// The working tree could not be written as a tree after the operation.
    #[error("the working tree after the operation: {0}")]
    Worktree(SnapshotError),
    /// The after-state was read but could not be hung off the backup ref, so
    /// it is not recorded: `git gc` would prune it and leave the entry naming
    /// objects that are gone.
    #[error("the after-state could not be kept reachable: {0}")]
    Unsealed(SnapshotError),
    /// The askpass server a remote invocation ran with could not be shut
    /// down cleanly: its socket or directory may be left in the temp dir.
    /// The operation itself is unaffected.
    #[error("the credential prompt's socket could not be cleaned up: {0}")]
    AskPass(std::io::Error),
    /// The entry names a backup ref that no longer exists, so the after-state
    /// read by `resume` has nothing to hang off and is not sealed: something
    /// deleted `refs/graft/backup/…` while the operation was stopped. `u`
    /// of this entry has no backup to restore from either (m5/07).
    #[error("the backup ref {name} is gone; the after-state is not kept reachable")]
    BackupGone {
        /// The ref the entry named.
        name: String,
    },
    /// git's merge state could not be read after the operation, so the entry
    /// records none — which `u` reads as "not merging" (m6-conflicts/20).
    #[error("the merge in progress after the operation: {0}")]
    MergeState(crate::merge_state::MergeStateError),
    /// A set-aside stash was kept after the put-back because a read failed —
    /// the status after it, or the stash list to drop it by — with no git
    /// failure to carry the words. `words` say what failed (`error`'s text
    /// included) and where the edits are (m6-conflicts/83, m2 and m3).
    #[error("{words}")]
    SetAsideKept {
        /// The read that failed.
        error: FactsError,
        /// What went wrong and where the edits are, as the entry's reason.
        words: String,
    },
}

/// The journal failing, with git's own failure when there was one, and
/// where a set-aside's edits are when they are not back in the tree.
fn journal_failed(error: &JournalError, after: Option<&RunError>, kept: Option<&str>) -> String {
    let mut line = match after {
        Some(ran) => format!("{error} (git had already failed: {})", reason_for(ran)),
        None => error.to_string(),
    };
    if let Some(kept) = kept {
        line.push_str(" · ");
        line.push_str(kept);
    }
    line
}

/// Every after-state failure on one line.
pub(super) fn joined(unread: &[AfterState]) -> String {
    unread
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

impl ApplyError {
    /// Where a set-aside's edits are, `Some` only for a set-aside that ended
    /// other than clean or clashed (ADR-0045 §5 amended). The one place the
    /// app asks.
    #[must_use]
    pub fn edits_at(&self) -> Option<EditsAt> {
        match self {
            ApplyError::Git { edits, .. } => *edits,
            _ => None,
        }
    }

    /// One line for the status line.
    pub fn explain(&self) -> String {
        match self {
            ApplyError::Git {
                entry,
                error,
                recoverable,
                edits,
            } => {
                let mut line = format!("{}: {}", entry.description, reason_for(error));
                if let Some(edits) = edits {
                    line.push_str(" · ");
                    line.push_str(&edits.words());
                }
                if *recoverable {
                    line.push_str(" · u restores the backup");
                }
                line
            }
            ApplyError::Remote {
                entry,
                failure,
                stderr_tail,
                names,
            } => remote::explain(entry, failure, stderr_tail, names),
            other => other.to_string(),
        }
    }
}

impl From<PrepareError> for ApplyError {
    fn from(why: PrepareError) -> Self {
        match why {
            PrepareError::Facts(why) => ApplyError::Facts(why),
            PrepareError::Snapshot(why) => ApplyError::Snapshot(why),
            PrepareError::Journal(why) => ApplyError::Journal {
                error: why,
                after: None,
                kept: None,
            },
            PrepareError::Stopped => ApplyError::Stopped,
        }
    }
}

/// One line for an `Aborted` entry's reason.
pub(crate) fn reason_for(why: &RunError) -> String {
    match why {
        RunError::Exited {
            stderr_first_line, ..
        } if !stderr_first_line.is_empty() => stderr_first_line.clone(),
        other => other.to_string(),
    }
}

/// A snapshot failure as an apply failure, the **stop kept as a stop**, so a
/// caller can tell "you cancelled" from "git would not run".
pub(crate) fn as_apply_error(why: SnapshotError) -> ApplyError {
    match why {
        SnapshotError::Stopped => ApplyError::Stopped,
        other => ApplyError::Snapshot(other),
    }
}
