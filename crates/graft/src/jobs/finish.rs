//! The finish summary's two reads (m6-conflicts/19): what the merge about to
//! be committed changes, and one staged file's diff for `space`.
//!
//! Both run here, off the loop's thread; `update` receives one finished `Arc`.

use std::path::Path;
use std::sync::Arc;

use graft_git::diff::{DiffOptions, DiffSpec, file_diff, list_files};
use graft_git::run::Stop;
use graft_ops::finish::SummaryError;
use graft_ui::detail::{DiffView, FileStatusView};

use super::detail::assemble_diff;
use crate::repos::RepoStore;
use crate::update::detail::MAX_LINES;
use crate::update::finish::{ReviewFailed, Reviewed};

/// The merge review (ease-pass/05b): what the merge in progress will
/// commit and the files solved on disk that its `y` stages, read with
/// `graft_ops::finish::merge_review` — nothing is staged by reading.
/// Files with a clash still open come back as
/// [`ReviewFailed::StillOpen`] so the app can say how many, files and
/// clashes; any other failure in `graft_ops`' own words.
pub(super) fn summary(repos: &RepoStore) -> Result<Reviewed, ReviewFailed> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    match graft_ops::finish::merge_review(&handle, &()) {
        Ok(review) => Ok(Reviewed {
            summary: Arc::new(review.summary),
            solved: review.solved,
            // A failed count is not a failed review: the head line draws
            // `? commits from …`, as a folded merge's unknown count does
            // (ease-pass 06). The review is still right without it.
            incoming: incoming_count(&handle).ok(),
        }),
        Err(SummaryError::ClashesOpen { files, clashes }) => {
            Err(ReviewFailed::StillOpen { files, clashes })
        }
        Err(error) => Err(ReviewFailed::Failed(error.to_string())),
    }
}

/// The commits the merge brings in: `HEAD..MERGE_HEAD`, the review's
/// `N commits from <branch>` (ease-pass/12), counted with the read a folded
/// merge's tail uses (`count_between`).
///
/// # Errors
/// `HEAD` or `MERGE_HEAD` not reading, or `git` failing, as one line.
fn incoming_count(handle: &graft_git::repo::RepoHandle) -> Result<u32, String> {
    let head = handle
        .head_commit()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "HEAD names no commit".to_owned())?;
    let detail = graft_git::state::in_progress_detail(handle)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no merge is in progress".to_owned())?;
    let theirs = detail
        .theirs
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "MERGE_HEAD names no commit".to_owned())?;
    let count = graft_git::history_reads::count_between(handle, head, theirs.id, &())
        .map_err(|error| error.to_string())?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// `path`'s staged change — the index against `HEAD`, which is exactly what
/// the merge commit will record for it — as the detail pane's view model.
/// A path git still calls unmerged is a solved file the finish stages
/// (ease-pass/05b): its change is the file on disk against `HEAD`.
///
/// A path with no entry is staged but equal to `HEAD` (a resolution that
/// kept yours): an empty view, which the summary draws as `no change to
/// show` in the muted style. Nothing is wrong, so it is not an `Err`
/// (review m6-amend-19-34 Major 3). `Err` is for reads that failed.
pub(super) fn staged_diff(
    repos: &RepoStore,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Arc<DiffView>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let options = DiffOptions::default();
    let status = graft_git::status::read(&handle, stop).map_err(|e| e.to_string())?;
    let solved = status
        .entries
        .iter()
        .any(|entry| entry.unmerged && entry.path == path);
    let spec = if solved {
        DiffSpec::WorkingTreeVsHead
    } else {
        DiffSpec::Staged
    };
    let list = list_files(&handle, &spec, &options, stop).map_err(|e| e.to_string())?;
    let want = path.as_os_str().as_encoded_bytes();
    let Some(entry) = list.files.iter().find(|entry| entry.path == want) else {
        return Ok(Arc::new(unchanged(path)));
    };
    let diff =
        file_diff(&handle, &spec, entry, &options, MAX_LINES, stop).map_err(|e| e.to_string())?;
    // Word spans are always worked out; whether they tint is the widget's call.
    Ok(Arc::new(assemble_diff(&diff, true)))
}

/// The view for a path the merge leaves as it is on `HEAD`: no hunks.
fn unchanged(path: &Path) -> DiffView {
    DiffView {
        path: graft_term::sanitize_path(path),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        untracked: false,
    }
}

#[cfg(test)]
mod tests;
