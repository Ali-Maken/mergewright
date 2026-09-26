//! Edits an undo or redo replaced over drift, kept only in its backup
//! (M6/77; note journey 15, m7-power-tools/23).
//!
//! An undo `U` that reverses entry `X` writes `X`'s before-state over what
//! `X` left. Where he edited a file after `X` (drift) and `U` writes that
//! file, his edit survives only in `U`'s backup (`U.worktree_before`). A
//! path is lost when all three hold:
//!
//! 1. **He changed it since `X`**: `U`'s backup differs there from
//!    `X.worktree_after`. Without this, every file the undo itself put back
//!    — a checkout's files, all in commits — would read as lost.
//! 2. **`U` wrote over it**: the backup differs there from
//!    `U.worktree_after`.
//! 3. **It is still lost now**: the file on disk differs from the backup's
//!    copy.
//!
//! A redo is the same over the undo it reverses. The same rule as
//! `undo::replaced` (the drift box's "replaces your edit to …"), asked
//! after the fact.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use graft_core::journal::{JournalEntry, OperationKind};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;

use super::LostEdit;
use crate::facts::FactsError;
use crate::journal::in_force_all;
use crate::snapshot::{self, SnapshotError};
use crate::undo::tree_diff;

/// How many undo and redo entries [`lost_edits`] looks at, newest first.
pub const LOST_EDITS_CAP: usize = 50;

/// Every edit an in-force undo or redo replaced and that is not back on
/// disk, newest entry first. Reads the working tree once, as a tree (the
/// way the backups were taken), then two `diff-tree`s per entry.
///
/// # Errors
/// [`FactsError::Stopped`] when `stop` asks; [`FactsError::Snapshot`] when
/// a read fails.
pub fn lost_edits(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    stop: &dyn Stop,
) -> Result<Vec<LostEdit>, FactsError> {
    let candidates: Vec<&JournalEntry> = entries
        .iter()
        .zip(in_force_all(entries))
        .rev()
        .filter(|(entry, in_force)| {
            *in_force
                && matches!(entry.kind, OperationKind::Undo | OperationKind::Redo)
                && entry.worktree_before.is_some()
        })
        .map(|(entry, _)| entry)
        .take(LOST_EDITS_CAP)
        .collect();
    if candidates.is_empty() {
        return Ok(Vec::new());
    }
    let now = snapshot::worktree_tree(repo, repo.git_dir(), stop).map_err(snapshot_error)?;
    let mut found = Vec::new();
    for entry in candidates {
        if stop.should_stop() {
            return Err(FactsError::Stopped);
        }
        if let Some(edit) = lost_in(repo, entries, entry, now, stop)? {
            found.push(edit);
        }
    }
    Ok(found)
}

/// One entry's lost paths, or `None` when it replaced nothing still lost.
fn lost_in(
    repo: &RepoHandle,
    entries: &[JournalEntry],
    entry: &JournalEntry,
    now: Oid,
    stop: &dyn Stop,
) -> Result<Option<LostEdit>, FactsError> {
    let Some(backup) = entry.worktree_before else {
        return Ok(None);
    };
    let reversed_after = entry
        .reverses
        .as_ref()
        .and_then(|id| entries.iter().find(|other| other.id == *id))
        .and_then(|reversed| reversed.worktree_after);
    // Each side is a set of paths where the backup differs; an unknown side
    // (a state that was not recorded) narrows nothing, since unknown is not
    // unchanged. Both unknown: there is nothing to tell his edits from the
    // undo's own writes, so nothing is claimed.
    let drifted = differs_from(repo, reversed_after, backup, stop)?;
    let written = differs_from(repo, entry.worktree_after, backup, stop)?;
    let candidates: BTreeSet<PathBuf> = match (drifted, written) {
        (None, None) => return Ok(None),
        (Some(one), None) | (None, Some(one)) => one,
        (Some(drifted), Some(written)) => drifted.intersection(&written).cloned().collect(),
    };
    if candidates.is_empty() {
        return Ok(None);
    }
    let on_disk = paths_of(tree_diff(repo, now, backup, stop, &mut Vec::new())?);
    let paths: Vec<PathBuf> = candidates
        .into_iter()
        .filter(|path| on_disk.contains(path))
        .collect();
    Ok((!paths.is_empty()).then(|| LostEdit {
        entry: entry.id.clone(),
        backup_tree: backup,
        paths,
    }))
}

/// The paths where `backup` differs from `other`; `None` when `other` was
/// not recorded.
fn differs_from(
    repo: &RepoHandle,
    other: Option<Oid>,
    backup: Oid,
    stop: &dyn Stop,
) -> Result<Option<BTreeSet<PathBuf>>, FactsError> {
    match other {
        None => Ok(None),
        Some(other) if other == backup => Ok(Some(BTreeSet::new())),
        Some(other) => Ok(Some(paths_of(tree_diff(
            repo,
            other,
            backup,
            stop,
            &mut Vec::new(),
        )?))),
    }
}

fn paths_of<T>(diff: BTreeMap<PathBuf, T>) -> BTreeSet<PathBuf> {
    diff.into_keys().collect()
}

fn snapshot_error(why: SnapshotError) -> FactsError {
    match why {
        SnapshotError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(other),
    }
}
