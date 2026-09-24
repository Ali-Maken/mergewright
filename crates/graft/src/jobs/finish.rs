//! The finish summary's two reads (m6-conflicts/19): what the merge about to
//! be committed changes, and one staged file's diff for `space`.
//!
//! Both run here, off the loop's thread; `update` receives one finished `Arc`.

use std::path::Path;
use std::sync::Arc;

use graft_git::diff::{DiffOptions, DiffSpec, file_diff, list_files};
use graft_git::run::Stop;
use graft_ops::finish::FinishSummary;
use graft_ui::detail::{DiffView, FileStatusView};

use super::detail::assemble_diff;
use crate::repos::RepoStore;
use crate::update::detail::MAX_LINES;

/// What the merge in progress will commit, or why it cannot be said, in
/// `graft_ops`' own words.
pub(super) fn summary(repos: &RepoStore) -> Result<Arc<FinishSummary>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    graft_ops::finish::finish_summary(&repo.handle())
        .map(Arc::new)
        .map_err(|error| error.to_string())
}

/// `path`'s staged change — the index against `HEAD`, which is exactly what
/// the merge commit will record for it — as the detail pane's view model.
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
    let list = list_files(&handle, &DiffSpec::Staged, &options, stop).map_err(|e| e.to_string())?;
    let want = path.as_os_str().as_encoded_bytes();
    let Some(entry) = list.files.iter().find(|entry| entry.path == want) else {
        return Ok(Arc::new(unchanged(path)));
    };
    let diff = file_diff(&handle, &DiffSpec::Staged, entry, &options, MAX_LINES, stop)
        .map_err(|e| e.to_string())?;
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
