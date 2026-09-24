//! The finish summary's two reads (m6-conflicts/19): what the merge about to
//! be committed changes, and one staged file's diff for `space`.
//!
//! Both run here, off the loop's thread; `update` receives one finished `Arc`.

use std::path::Path;
use std::sync::Arc;

use graft_git::diff::{DiffOptions, DiffSpec, file_diff, list_files};
use graft_git::run::Stop;
use graft_ops::finish::FinishSummary;
use graft_ui::detail::DiffView;

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
    let entry = list
        .files
        .iter()
        .find(|entry| entry.path == want)
        .ok_or_else(|| format!("{} is not staged", path.display()))?;
    let diff = file_diff(&handle, &DiffSpec::Staged, entry, &options, MAX_LINES, stop)
        .map_err(|e| e.to_string())?;
    // Word spans are always worked out; whether they tint is the widget's call.
    Ok(Arc::new(assemble_diff(&diff, true)))
}

#[cfg(test)]
mod tests;
