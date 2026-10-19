//! Answers too large to sit inline in [`super::Product`]: moved out of
//! `events` (ease-pass 09, move-only) to keep it under its size row.

use mergewright_git::version::SystemGit;

/// A history place's whole file at one commit (m7-power-tools/21c).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WholeFileAnswer {
    /// The file, assembled as the detail pane assembles a diff.
    pub view: std::sync::Arc<mergewright_ui::detail::DiffView>,
    /// `Some` only when the diff is binary. `Err`: the size read's `Display`.
    pub sizes: Option<Result<mergewright_git::trace::BlobSizes, String>>,
}

/// What running `git --version` found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitCheck {
    /// `git` ran and said which version it is.
    Ran(SystemGit),
    /// `git` could not be run at all, and this is why. Not fatal at M0 —
    /// nothing has shelled out to it yet — but the user needs to know now
    /// rather than at the first operation that needs it.
    Unavailable(String),
}

/// A finished topological sort: the order, and how long it took to make.
/// The time decides the commit-graph offer (m7-power-tools/15, :778): only a
/// sort the user had to wait for is worth a file to speed up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SortedOrder {
    /// Every commit in strict topological order.
    pub order: std::sync::Arc<[mergewright_core::commit_handle::CommitHandle]>,
    /// Wall time of `history::topological_order`, measured in the job.
    pub took: core::time::Duration,
}
