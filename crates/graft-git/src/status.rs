//! Counting the working tree's changes: staged, unstaged and untracked
//! files, for the uncommitted-changes row (PROMPT.md §1.3).
//!
//! Read by a job, never by `update`: gix's status iterator walks the index
//! and the worktree, which touches disk. [`counts`] is the one entry point.
//! It reports numbers, not paths — the file tree those numbers break down
//! into is the staging view (M3); this ticket only needs "how many".

#[cfg(test)]
mod tests;

use crate::repo::RepoHandle;

/// How many files are staged, changed but unstaged, and untracked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkingTreeCounts {
    /// Staged: differences between `HEAD`'s tree and the index. On an
    /// unborn branch this compares against the empty tree (gix's own
    /// `HEAD^{tree}` fallback), so a staged file on a first commit still
    /// counts.
    pub staged: usize,
    /// Unstaged: tracked files that differ between the index and the
    /// worktree.
    pub unstaged: usize,
    /// Untracked: worktree entries the index does not know about.
    /// Ignored files are not counted — `gix::Repository::status`'s default
    /// matches `git status --ignored=no`.
    pub untracked: usize,
}

impl WorkingTreeCounts {
    /// No staged, unstaged or untracked changes: the uncommitted row does
    /// not draw at all.
    #[must_use]
    pub fn is_clean(self) -> bool {
        self.staged == 0 && self.unstaged == 0 && self.untracked == 0
    }
}

/// Why counting the working tree's changes failed.
#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    /// The status platform could not be built.
    #[error(transparent)]
    Start(#[from] Box<gix::status::Error>),
    /// The status walk could not begin.
    #[error(transparent)]
    Begin(#[from] Box<gix::status::into_iter::Error>),
    /// A status entry could not be read partway through the walk.
    #[error(transparent)]
    Item(#[from] Box<gix::status::iter::Error>),
}

/// Count staged, unstaged and untracked files.
///
/// A bare repository has no worktree to compare and reports a clean tree —
/// not an error, since there is truthfully nothing uncommitted to show.
///
/// # Errors
/// The status walk could not start, or broke partway through.
pub fn counts(handle: &RepoHandle) -> Result<WorkingTreeCounts, StatusError> {
    if handle.is_bare() {
        return Ok(WorkingTreeCounts::default());
    }

    let platform = handle
        .gix()
        .status(gix::progress::Discard)
        .map_err(Box::new)?;
    let mut result = WorkingTreeCounts::default();
    for item in platform.into_iter(Vec::new()).map_err(Box::new)? {
        match item.map_err(Box::new)? {
            gix::status::Item::TreeIndex(_) => result.staged += 1,
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::Modification { .. } => {
                    result.unstaged += 1;
                }
                gix::status::index_worktree::Item::DirectoryContents { .. } => {
                    result.untracked += 1;
                }
                // Rename tracking is off by default (`Repository::status`
                // leaves `rewrites: None`), so this arm is unreached in
                // practice; kept exhaustive rather than wildcarded so a
                // future default change is a compile error here, not a
                // silent miscount. A rewrite's source is an index entry
                // gone missing, which is a staged change.
                gix::status::index_worktree::Item::Rewrite { .. } => {
                    result.staged += 1;
                }
            },
        }
    }
    Ok(result)
}
