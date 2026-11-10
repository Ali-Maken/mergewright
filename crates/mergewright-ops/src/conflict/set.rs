//! Which files the operation in progress conflicted on — the ones still
//! unmerged **and** the ones already resolved — read from git and Mergewright's
//! backup chain, never from this run's memory, so a relaunch and an
//! operation begun in the shell list the same files (m6-conflicts/09,
//! Amendment 2 hard choice 2).

use std::collections::HashSet;
use std::io;
use std::path::PathBuf;

use mergewright_core::journal::JournalEntry;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::RunError;
use mergewright_git::state::InProgress;
use mergewright_git::status::WorkingTreeStatus;

use crate::apply::live_entry_facts;
use crate::facts::FactsError;
use crate::journal::{self, JournalError};
use crate::operation::RepoFacts;
use crate::snapshot::{self, SnapshotError};

use super::merge_msg;

/// The files an operation conflicted on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConflictSet {
    /// The paths unmerged now, in git's status order; then the ones git's
    /// `Conflicts:` list names, in its order; then the ones Mergewright's backup
    /// chain recorded unmerged. Each once.
    pub paths: Vec<PathBuf>,
    /// A record of the conflicts was found — git's list, or a chained backup
    /// holding one. When false, `paths` is only what is unmerged now, and
    /// which of the staged changes were conflicts is not known.
    pub recorded: bool,
    /// The unmerged paths the resolver's rule calls solved on disk
    /// ([`file_state`](super::file_state), ease-pass/05b): a marker
    /// document with no clash open, not staged yet. So a relaunch reads
    /// what was solved before it (05c). Filled by [`operation_conflicts`];
    /// the finish's own read of the set never looks at it and leaves it
    /// empty.
    pub solved: Vec<PathBuf>,
}

/// Why the conflict set could not be read.
#[derive(Debug, thiserror::Error)]
pub enum ConflictSetError {
    /// `MERGE_MSG` or the journal exists but could not be read.
    #[error("could not read git's merge message or the journal: {0}")]
    Io(#[from] io::Error),
    /// A `git` read would not run.
    #[error("{0}")]
    Run(#[from] RunError),
    /// Which entry of Mergewright's is the live operation could not be settled.
    #[error("{}", .0.explain())]
    Facts(FactsError),
    /// The journal did not parse.
    #[error("the undo journal is unreadable: {0}")]
    Journal(#[from] JournalError),
    /// The backup chain could not be walked.
    #[error("the backup chain is unreadable: {0}")]
    Snapshot(#[from] SnapshotError),
}

/// The set of files the operation in progress conflicted on.
///
/// The union, each path once, of (a) the paths unmerged now, in the status's
/// order; (b) the `Conflicts:` list git wrote into `MERGE_MSG`, read with
/// `core.commentChar` (see `merge_msg` for which kinds write it); (c) every
/// path held at a stage above 0 by an unmerged index state on the live
/// entry's backup chain — the states Mergewright kept before each step it ran
/// inside the operation.
///
/// (b) and (c) are only read while git has an operation in progress: a
/// `MERGE_MSG` with nothing in progress describes no operation of now, and
/// there is no live entry to have a chain.
///
/// # Errors
/// [`ConflictSetError`] when `MERGE_MSG`, the journal or the chain exists
/// and cannot be read. Absent ones are not errors: they are `recorded:
/// false`.
pub fn operation_conflicts(
    repo: &RepoHandle,
    facts: &RepoFacts,
) -> Result<ConflictSet, ConflictSetError> {
    let mut set = conflicts_of(repo, facts.in_progress, &facts.status)?;
    set.solved = solved_on_disk(repo, &facts.status)?;
    Ok(set)
}

/// Every unmerged path [`file_state`](super::file_state) calls solved, in
/// status order. One the resolver cannot open (not unmerged after all, too
/// large, markers that do not parse) is not solved — as the merge review
/// reads it (`finish::merge_review`); a file that would not read at all is
/// the error.
fn solved_on_disk(
    repo: &RepoHandle,
    status: &WorkingTreeStatus,
) -> Result<Vec<PathBuf>, ConflictSetError> {
    use super::{FileState, OpenError, file_state, open_conflict};
    let mut solved = Vec::new();
    for entry in status.entries.iter().filter(|entry| entry.unmerged) {
        match open_conflict(repo, status, &entry.path) {
            Ok(opened) if file_state(&opened, true) == FileState::Solved => {
                solved.push(entry.path.clone());
            }
            Ok(_)
            | Err(OpenError::Refused(_) | OpenError::TooLarge { .. } | OpenError::Markers(_)) => {
                // Open, done, or nothing the resolver solved: not solved.
            }
            Err(OpenError::Read(source)) => {
                return Err(ConflictSetError::Facts(FactsError::Markers {
                    path: entry.path.clone(),
                    source,
                }));
            }
        }
    }
    Ok(solved)
}

/// The same from the two facts it needs, for a caller that read them
/// without a whole [`RepoFacts`].
pub(crate) fn conflicts_of(
    repo: &RepoHandle,
    in_progress: Option<InProgress>,
    status: &WorkingTreeStatus,
) -> Result<ConflictSet, ConflictSetError> {
    let mut set = Union::default();
    set.extend(
        status
            .entries
            .iter()
            .filter(|entry| entry.unmerged)
            .map(|entry| entry.path.clone()),
    );
    let Some(kind) = in_progress else {
        return Ok(ConflictSet {
            paths: set.paths,
            recorded: false,
            solved: Vec::new(),
        });
    };
    let listed = match merge_msg::read(repo)? {
        Some(message) => merge_msg::conflicts(&message, &merge_msg::comment_prefix(repo, &())?),
        None => None,
    };
    let from_git = listed.is_some();
    set.extend(listed.into_iter().flatten());
    let chained = chained_conflicts(repo, kind)?;
    let from_chain = !chained.is_empty();
    set.extend(chained);
    Ok(ConflictSet {
        paths: set.paths,
        recorded: from_git || from_chain,
        solved: Vec::new(),
    })
}

/// Paths in the order first seen, each once.
#[derive(Default)]
struct Union {
    paths: Vec<PathBuf>,
    seen: HashSet<PathBuf>,
}

impl Union {
    fn extend(&mut self, paths: impl IntoIterator<Item = PathBuf>) {
        for path in paths {
            if self.seen.insert(path.clone()) {
                self.paths.push(path);
            }
        }
    }
}

/// Every path an unmerged state on the live entry's backup chain holds at a
/// stage above 0, newest state first. Empty when the operation is not one
/// of Mergewright's (begun in the shell) or Mergewright ran no step inside it.
fn chained_conflicts(
    repo: &RepoHandle,
    kind: InProgress,
) -> Result<Vec<PathBuf>, ConflictSetError> {
    let entries = journal_entries(repo)?;
    // `live_entry_facts`, not `live_entry`: its error is only ever a read of
    // the repository, so it goes through whole, with no arm guessing what
    // an `ApplyError` was (m7-power-tools/01a, gap :613).
    let live = live_entry_facts(repo, &entries, kind, &()).map_err(ConflictSetError::Facts)?;
    let Some(name) = live.and_then(|entry| entry.backup_ref.as_deref()) else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    // Only links written since the current step stopped (m6-conflicts/22).
    for state in snapshot::current_step_chain(repo, name, &())? {
        paths.extend(snapshot::conflicted_paths(repo, state, &())?);
    }
    Ok(paths)
}

/// The journal's entries, read without opening it: [`journal::Journal::open`]
/// cuts a torn tail, and a read must not write. No journal is no entries.
fn journal_entries(repo: &RepoHandle) -> Result<Vec<JournalEntry>, ConflictSetError> {
    let path = journal::journal_path(repo)?;
    match std::fs::read(path) {
        Ok(bytes) => Ok(journal::parse(&bytes)?.entries),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}
