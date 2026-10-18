//! The read behind the graph's "only in a stash" marks (m7-power-tools/16):
//! which commits a stash entry's first parent reaches that nothing else
//! does — the base a stash was made on, after that base was rebased away.

use std::sync::Arc;

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{StashOnly, only_in_stash};
use mergewright_git::run::Stop;

use crate::repos::RepoStore;

/// How many commits the answer holds at most. A stash's own base line is a
/// handful of commits; past this the marks stop, and `StashOnly::capped`
/// says so (the ticket's `cap`).
pub(crate) const CAP: usize = 500;

/// The work behind
/// [`Command::ReadStashOnly`](crate::update::Command::ReadStashOnly):
/// `stashes` is every entry's own commit, `stash@{0}` first.
///
/// # Errors
/// The repository not opening, or `git` failing (stopped included), as one
/// line.
pub(crate) fn read_stash_only(
    repos: &RepoStore,
    stashes: &[Oid],
    stop: &dyn Stop,
) -> Result<Arc<StashOnly>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    only_in_stash(&repo.handle(), stashes, CAP, stop)
        .map(Arc::new)
        .map_err(|error| error.to_string())
}
