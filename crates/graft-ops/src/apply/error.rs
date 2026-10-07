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
use crate::journal::{JournalError, in_force};
use crate::operation::{Refusal, RemoteNames};
use crate::restore::RestoreError;
use crate::snapshot::SnapshotError;

use super::{Applied, PrepareError, StoppedAt, remote};

mod words;

pub(super) use words::{joined, not_as_they_were_words, staging_clause};
use words::{journal_failed, staging_not_kept_words};

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
        /// A stop that left the merge this operation runs in still paused:
        /// finishing or aborting a merge begun in the shell, stopped with
        /// `MERGE_HEAD` still there and `HEAD` unmoved (13d Q4). The entry
        /// is closed all the same — Graft did not begin that merge — and
        /// [`ApplyError::stopped_at`] says `Paused`. `None` otherwise.
        paused: Option<InProgress>,
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
    /// design note hard choice 2 — and [`resume`](super::resume) closes it. No outcome line
    /// is written here. `paths` are the unmerged paths the status lists
    /// (m5/07).
    #[error("{}", conflicted_words(entry, *kind, paths))]
    Conflicted {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state git left: what the banner shows and what `resume` runs.
        kind: InProgress,
        /// The unmerged paths.
        paths: Vec<PathBuf>,
    },
    /// A write stopped by the user (the quit box's stop, or the signal path)
    /// that git paused itself: its hook was ended first, and git wrote its
    /// in-progress state as it does when a hook fails (m8-polish-and-ship/13c).
    /// Closes like [`ApplyError::Conflicted`]: the entry stays `Pending`
    /// with its commands held, no outcome line, and
    /// [`resume`](super::resume) continues or aborts it — found by
    /// `live_entry`, listed `live` by `recover_interrupted`. `paths` are
    /// what is unmerged now, normally none.
    #[error("{}: stopped · the {} is paused · c continues, a aborts", entry.description, kind.name())]
    Paused {
        /// The entry, still `Pending`.
        entry: Box<JournalEntry>,
        /// The state git left.
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
    /// [`resume`](super::resume) ran `--continue`, `--skip` or `--abort`, git refused, and
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
    /// A touched branch's `branch.<name>.*` config could not be read after
    /// the operation, so the entry records none: `u` then puts back only the
    /// keys that were there before, and cannot remove one the operation
    /// added (review m7-16b..21b, Major 1).
    #[error("the branch's config after the operation: {0}")]
    BranchConfig(FactsError),
    /// A rewrite's saved plan (`<git dir>/graft/rewrite/<entry-id>/`) could
    /// not be removed when its entry closed (ADR-0047 §1). The entry is
    /// closed all the same; the files are Graft's, not the user's, and a
    /// banner may name the wrong branch for a later rebase on the same top
    /// until they are gone.
    #[error("the rewrite's saved plan at {} could not be removed: {error}", dir.display())]
    PlanLeft {
        /// The plan's directory.
        dir: PathBuf,
        /// Why.
        error: std::io::Error,
    },
    /// A rewrite ran on the top of the line and `HEAD` could not be switched
    /// back to his branch after it (ADR-0047 §3): `HEAD` is on the top
    /// branch. Every branch is where the entry records it.
    #[error("HEAD is still on {top}: switching back to {home} failed: {error}")]
    NotHome {
        /// His branch, short name.
        home: String,
        /// The top of the line, short name.
        top: String,
        /// git's failure.
        error: RunError,
    },
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
    /// The put-back brought the edits back, but git could not reinstate the
    /// index on the landed tree (`stash apply --index` refused and wrote
    /// nothing), so it ran without `--index`. `paths`: the staged paths
    /// whose staging did not come back. `u` restores the split exactly
    /// (ADR-0045 §6, amended 2026-09-26 b).
    #[error("{}", staging_not_kept_words(paths))]
    StagingNotKept {
        /// The staged paths whose staging did not come back.
        paths: Vec<PathBuf>,
    },
    /// A set-aside put-back brought his edits back in git's encoding, and
    /// writing his own bytes over them failed (m7-power-tools/30s). The entry
    /// is `Committed` and `S` dropped; `u` does not rewrite these bytes, and
    /// the words carry no `u` clause for that reason. The exact bytes stay in
    /// the before-snapshot's `raw/` tree, pinned by the backup (ADR-0052).
    #[error("{}", not_as_they_were_words(error, *files))]
    NotAsTheyWere {
        /// The failure: [`RestoreError::WriteRaw`] names the first path that
        /// failed; anything else is the check that finds which paths to write.
        error: RestoreError,
        /// How many paths the raw step was to write (1 → the singular words;
        /// 0 when the check itself failed).
        files: usize,
    },
    /// A set-aside put-back brought his edits back, and the read that names
    /// the files that came back from the index failed
    /// ([`came_back_from_index`](super::came_back_from_index),
    /// m8-polish-and-ship/14). His edits are back; only the naming is
    /// missing, so a staged-new file he had deleted may be on disk again
    /// unannounced.
    #[error("your edits are back · which files came back from the index could not be read: {0}")]
    CameBackUnread(RunError),
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

    /// Where a stop left git, for an error a stop produced
    /// (m8-polish-and-ship/13c, 13d): [`ApplyError::Paused`] → `Paused`; a
    /// stopped local run closed `Aborted` → `Paused` when it left the shell
    /// merge it ran in paused (`paused`), else `BeforePause` for a kind git
    /// can pause and `Stopped` for one it never can, `changed` when the
    /// entry is in force (`u` has something to put back). `None` for every
    /// other error, a stop before git ran included.
    #[must_use]
    pub fn stopped_at(&self) -> Option<StoppedAt> {
        match self {
            ApplyError::Paused { kind, .. }
            | ApplyError::Git {
                error: RunError::Stopped,
                paused: Some(kind),
                ..
            } => Some(StoppedAt::Paused(*kind)),
            ApplyError::Git {
                entry,
                error: RunError::Stopped,
                paused: None,
                ..
            } => Some(StoppedAt::residual(
                entry.kind,
                // The entry is the journal's newest, so nothing can have
                // reversed it yet: in force is "took effect".
                in_force(std::slice::from_ref(&**entry), &entry.id),
            )),
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
                paused: _,
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

/// [`ApplyError::Conflicted`]'s words. A merge stopped with nothing
/// unmerged was stopped by a hook that said no (`pre-merge-commit`,
/// `prepare-commit-msg`, `commit-msg`) or by signing: there is nothing to
/// resolve, so the words do not say there is (13d Q4b). `c` then runs
/// `git commit`, which runs `pre-commit`, not `pre-merge-commit` — git's
/// design, not Graft's. Outside a merge, nothing unmerged is not a hook (a
/// rewrite's `edit` row, an empty pick): those keep the conflict's words.
fn conflicted_words(entry: &JournalEntry, kind: InProgress, paths: &[PathBuf]) -> String {
    if paths.is_empty() && kind == InProgress::Merge {
        return format!(
            "{}: a git hook stopped it · c continues, a aborts",
            entry.description
        );
    }
    format!(
        "{}: {} unmerged · resolve, then c continues, a aborts",
        entry.description,
        paths.len()
    )
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

#[cfg(test)]
mod tests;
