//! The detail pane's jobs: the changed-file list, one file's diff, and which
//! refs contain a commit.
//!
//! Every git read runs here, off the loop's thread, and so does the view-model
//! assembly: sanitising, pairing, word diffs and syntax highlighting. `update`
//! receives one finished `Arc` and never touches git bytes.

use std::sync::Arc;

use graft_core::oid::Oid;
use graft_git::contained::{Contained, contained_in};
use graft_git::diff::{
    DiffOptions, DiffSpec, FileDiff, FileEntry, FileList, file_diff, list_files,
};
use graft_git::run::Stop;
use graft_ui::detail::{DiffView, FileListView};

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::detail::DiffKey;

mod assemble;
mod blame;
mod working_tree;

pub use assemble::{assemble_diff, assemble_files};
pub(super) use blame::read_blame;
use working_tree::shapes_of;
pub use working_tree::{UnmergedShapes, assemble_working_tree, diff_spec_for};

impl Stop for Cancel {
    fn should_stop(&self) -> bool {
        self.is_cancelled()
    }
}

/// List a spec's changed files, with the commit body for a single commit.
///
/// The changes pane is the one spec read twice: `Staged` (index against
/// `HEAD`) and `Unstaged` (working tree against the index, untracked files
/// included) are what the three sections *are*, and one
/// `WorkingTreeVsHead` read could tell neither apart nor list an untracked
/// file at all.
pub(super) fn read_files(
    repos: &RepoStore,
    spec: DiffSpec,
    options: DiffOptions,
    stop: &Cancel,
) -> Result<(Arc<FileListView>, Arc<FileList>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    if spec == DiffSpec::WorkingTreeVsHead {
        let staged =
            list_files(&handle, &DiffSpec::Staged, &options, stop).map_err(|e| e.to_string())?;
        let unstaged =
            list_files(&handle, &DiffSpec::Unstaged, &options, stop).map_err(|e| e.to_string())?;
        // The shape of each conflict ("both modified", "deleted by
        // them") is the status' to say, not the diff's, so the changes
        // pane's own read fetches it alongside the two diffs. A status
        // that cannot be read costs the CONFLICTS rows their wording and
        // nothing else, so it is not this job's failure.
        let how = match graft_git::status::read(&handle, stop) {
            Ok(status) => shapes_of(&status),
            Err(_) => UnmergedShapes::new(),
        };
        let (view, list) = assemble_working_tree(&staged, &unstaged, &how);
        return Ok((Arc::new(view), Arc::new(list)));
    }
    let list = list_files(&handle, &spec, &options, stop).map_err(|e| e.to_string())?;
    let message = match spec {
        DiffSpec::Commit { commit: id, .. } => Some(
            graft_git::history::read_commit(&handle, id)
                .map_err(|e| e.to_string())?
                .message,
        ),
        DiffSpec::Range { .. }
        | DiffSpec::WorkingTreeVsHead
        | DiffSpec::Staged
        | DiffSpec::Unstaged => None,
    };
    let view = assemble_files(&list, message.as_deref().map(body_of));
    Ok((Arc::new(view), Arc::new(list)))
}

/// Read and assemble one file's diff, handing back both the drawable view
/// model and the raw diff it was built from: `graft_git::diff::hunk_patch`
/// needs git's own bytes to build a stage or discard patch, and the view
/// model has been sanitised past the point of reversing.
pub(super) fn read_file_diff(
    repos: &RepoStore,
    key: &DiffKey,
    entry: &FileEntry,
    stop: &Cancel,
) -> Result<(Arc<DiffView>, Arc<FileDiff>), String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let spec = diff_spec_for(key.spec, key.section);
    let diff = file_diff(
        &repo.handle(),
        &spec,
        entry,
        &key.options,
        key.max_lines,
        stop,
    )
    .map_err(|e| e.to_string())?;
    // Word spans are always worked out; whether they tint is the widget's call.
    let view = assemble_diff(&diff, true);
    Ok((Arc::new(view), Arc::new(diff)))
}

#[cfg(test)]
mod blame_tests;

/// Which refs contain a commit.
pub(super) fn read_contained(
    repos: &RepoStore,
    oid: Oid,
    stop: &Cancel,
) -> Result<Contained, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    contained_in(&repo.handle(), oid, stop).map_err(|e| e.to_string())
}

/// A commit message without its subject line and the blank lines after it.
fn body_of(message: &[u8]) -> &[u8] {
    let rest = match message.iter().position(|&b| b == b'\n') {
        Some(end) => &message[end + 1..],
        None => &[],
    };
    let start = rest
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .unwrap_or(rest.len());
    &rest[start..]
}
