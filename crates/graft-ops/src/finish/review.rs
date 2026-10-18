//! The merge review's read (ease-pass/05b): which unmerged files are
//! solved on disk, and what the merge will commit with them staged.

use std::io;
use std::path::PathBuf;

use graft_git::repo::RepoHandle;
use graft_git::run::{GitInvocation, Stop};
use graft_git::state::{self, InProgress};

use super::{FinishSummary, SummaryError, summary_on};
use crate::conflict::{FileState, OpenError, file_state, open_conflict};
use crate::facts::FactsError;
use crate::snapshot::{SnapshotError, TempIndex};

/// A merge's review: what it will commit, and the files the finish stages.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MergeReview {
    /// What the merge will commit, as if `solved` were staged.
    pub summary: FinishSummary,
    /// Every path git still calls unmerged, each solved on disk: what the
    /// review's `y` hands [`Resume::Continue`](crate::apply::Resume::Continue).
    pub solved: Vec<PathBuf>,
}

/// The review of the merge in progress, with the solved files read from disk
/// rather than named (ease-pass/05b): every unmerged path is opened and
/// sorted by [`file_state`](crate::conflict::file_state). So the banner's
/// `c` and a relaunched app need nothing in memory to know what is solved.
///
/// # Errors
/// [`SummaryError::NotMerging`] unless git has a merge in progress;
/// [`SummaryError::ClashesOpen`] counting the unmerged files not solved
/// (a clash still open, a whole-file or binary conflict, markers that do
/// not parse) and their open clashes; a file that would not read as
/// [`FactsError::Markers`]; otherwise as [`finish_preview`].
pub fn merge_review(repo: &RepoHandle, stop: &dyn Stop) -> Result<MergeReview, SummaryError> {
    let in_progress = state::in_progress(repo).map_err(FactsError::State)?;
    if in_progress != Some(InProgress::Merge) {
        return Err(SummaryError::NotMerging);
    }
    let status = graft_git::status::read(repo, stop).map_err(FactsError::from)?;
    let mut solved = Vec::new();
    let (mut files, mut clashes) = (0, 0);
    for entry in status.entries.iter().filter(|entry| entry.unmerged) {
        let state = match open_conflict(repo, &status, &entry.path) {
            Ok(opened) => file_state(&opened, true),
            Err(OpenError::Read(source)) => {
                return Err(SummaryError::Facts(FactsError::Markers {
                    path: entry.path.clone(),
                    source,
                }));
            }
            // Not unmerged after all, too large, or markers that do not
            // parse: nothing the resolver solved, so it is open.
            Err(OpenError::Refused(_) | OpenError::TooLarge { .. } | OpenError::Markers(_)) => {
                FileState::Open { open: 0 }
            }
        };
        match state {
            FileState::Solved => solved.push(entry.path.clone()),
            // `Done` cannot come back for a path read as unmerged; counted
            // as open, not solved, should the rule ever change.
            FileState::Open { .. } | FileState::Done => {
                files += 1;
                clashes += state.open_clashes().max(1);
            }
        }
    }
    if files > 0 {
        return Err(SummaryError::ClashesOpen { files, clashes });
    }
    let summary = finish_preview(repo, &solved, stop)?;
    Ok(MergeReview { summary, solved })
}

/// The review's facts as if `solved` were staged: a scratch index, never the
/// real one (ease-pass/05b, hard choice 3). Crate-only: it stages what it
/// is given, markers included, so [`merge_review`] — which judges each path
/// on disk first — is the one public door (ease-pass/05s, Minor 5).
///
/// The real index is copied to a temporary file beside the journal,
/// `git add -- <solved>` runs on the copy (`GIT_INDEX_FILE`, the snapshot
/// plumbing's route), and each resolved file's result is read from it. The
/// copy is removed on every path out. With nothing in `solved` this is
/// [`finish_summary`], with no copy made.
///
/// `by_git` is read from the real index: it leaves out every path the merge
/// conflicted on, and `solved` are exactly such paths, so staging them on
/// the copy changes nothing it lists.
///
/// # Errors
/// [`SummaryError::NotMerging`] unless git has a merge in progress;
/// [`SummaryError::StillUnmerged`] counting the unmerged paths not in
/// `solved`; [`SummaryError::Scratch`] when the copy could not be made or
/// removed; the read's own failure otherwise.
pub(crate) fn finish_preview(
    repo: &RepoHandle,
    solved: &[PathBuf],
    stop: &dyn Stop,
) -> Result<FinishSummary, SummaryError> {
    summary_of(repo, solved, stop)
}

/// [`finish_summary`] and [`finish_preview`]: one body.
pub(super) fn summary_of(
    repo: &RepoHandle,
    solved: &[PathBuf],
    stop: &dyn Stop,
) -> Result<FinishSummary, SummaryError> {
    let in_progress = state::in_progress(repo).map_err(FactsError::State)?;
    if in_progress != Some(InProgress::Merge) {
        return Err(SummaryError::NotMerging);
    }
    let status = graft_git::status::read(repo, stop).map_err(FactsError::from)?;
    let unmerged: Vec<PathBuf> = status
        .entries
        .iter()
        .filter(|entry| entry.unmerged)
        .map(|entry| entry.path.clone())
        .collect();
    let count = unmerged
        .iter()
        .filter(|path| !solved.contains(path))
        .count();
    if count > 0 {
        return Err(SummaryError::StillUnmerged { count });
    }
    // Only the solved paths git still calls unmerged need the copy: one
    // staged already reads the same from the real index.
    let to_stage: Vec<PathBuf> = unmerged;
    let copy = if to_stage.is_empty() {
        None
    } else {
        Some(review_index(repo, &to_stage, stop)?)
    };
    let index = copy.as_ref().map(TempIndex::path);
    let summary = summary_on(repo, in_progress, &status, index, stop);
    match copy {
        // Removed before the summary is returned. A copy that would not go
        // is said when the read succeeded; when the read failed, its error
        // is the one returned.
        Some(copy) => {
            let removed = copy.remove();
            // Safe to drop the removal's error when the read failed: the
            // copy is inert scratch under a name no other run takes (as
            // `snapshot/temp_index.rs` argues for its own), and the read's
            // failure is the one he can act on.
            let summary = summary?;
            removed.map_err(|why| SummaryError::Scratch(SnapshotError::Io(why)))?;
            Ok(summary)
        }
        None => summary,
    }
}

/// A copy of the real index with `paths` staged on it: `GIT_INDEX_FILE`
/// set, nothing written to `.git/index`. Beside the journal, under a name
/// no other run takes; [`TempIndex`] removes it on every path.
fn review_index(
    repo: &RepoHandle,
    paths: &[PathBuf],
    stop: &dyn Stop,
) -> Result<TempIndex, SummaryError> {
    let journal = crate::journal::journal_path(repo)?;
    let scratch = journal.parent().ok_or_else(|| {
        SummaryError::Scratch(SnapshotError::Io(io::Error::other(
            "the journal has no directory to copy the index into",
        )))
    })?;
    std::fs::create_dir_all(scratch)?;
    let copy = TempIndex::copied_as(
        repo,
        scratch.join(TempIndex::unique_name("tmp-review-index")),
        stop,
    )?;
    let Some(add) = crate::operation::stage_solved_args(paths) else {
        return Ok(copy);
    };
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", copy.path())
        .args(add)
        .run(stop)?;
    Ok(copy)
}

#[cfg(test)]
mod tests;
