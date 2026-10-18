//! What git combined on its own in a stopped merge: every staged change
//! but the paths the merge conflicted on. One walk, read by the finish
//! summary's `by_git` and by the clash screen's `Combined by git (N)`
//! (ease-pass/11b), so the two never disagree.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use mergewright_git::diff::{DiffOptions, DiffSpec, FileStatus, list_files};
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{RunError, Stop};
use mergewright_git::state::{self, InProgress};

use super::{ByGit, Change, SummaryError, path_of};
use crate::conflict::conflicts_of;
use crate::facts::FactsError;

/// Files the stopped merge changed without a clash: stage-0 index entries
/// differing from `HEAD`, every path the merge conflicted on left out
/// (unmerged now, or resolved and staged since). In git's diff order.
///
/// # Errors
/// [`SummaryError::NotMerging`] unless git has a merge in progress; the
/// status, conflict-set or staged-diff read's own failure otherwise.
pub fn combined_by_git(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<PathBuf>, SummaryError> {
    let in_progress = state::in_progress(repo).map_err(FactsError::State)?;
    if in_progress != Some(InProgress::Merge) {
        return Err(SummaryError::NotMerging);
    }
    let status = mergewright_git::status::read(repo, stop).map_err(FactsError::from)?;
    let set = conflicts_of(repo, in_progress, &status)?;
    Ok(changed_by_git(repo, &set.paths, stop)?
        .into_iter()
        .map(|change| change.path)
        .collect())
}

/// Every staged change but the conflicted paths, in git's diff order.
pub(super) fn changed_by_git(
    repo: &RepoHandle,
    conflicted: &[PathBuf],
    stop: &dyn Stop,
) -> Result<Vec<ByGit>, SummaryError> {
    let yours: HashSet<&Path> = conflicted.iter().map(PathBuf::as_path).collect();
    let staged = list_files(repo, &DiffSpec::Staged, &DiffOptions::default(), stop)?;
    let mut by_git = Vec::new();
    for file in staged.files {
        let path = path_of(file.path);
        if yours.contains(path.as_path()) {
            continue;
        }
        let change = match file.status {
            FileStatus::Added | FileStatus::Copied { .. } => Change::Added,
            FileStatus::Deleted => Change::Deleted,
            FileStatus::Renamed { .. } => Change::Renamed {
                from: path_of(file.old_path.ok_or_else(|| {
                    RunError::Unparsable(format!("a rename with no old path: {}", path.display()))
                })?),
            },
            FileStatus::TypeChanged => Change::TypeChanged,
            // `Staged` is `diff-index --cached`: it reports neither an
            // untracked file nor, with nothing unmerged, an unmerged one.
            FileStatus::Modified | FileStatus::Unmerged | FileStatus::Untracked => Change::Modified,
        };
        by_git.push(ByGit {
            path,
            change,
            // `None` is a binary file: no line counts to show.
            added: file.added.map_or(0, |n| n as usize),
            removed: file.removed.map_or(0, |n| n as usize),
        });
    }
    Ok(by_git)
}
