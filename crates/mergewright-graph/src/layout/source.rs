//! Where a streaming layout gets its commits.
//!
//! `graph` depends only on `core` (ARCHITECTURE.md), so it cannot open a
//! repository itself. A [`CommitSource`] is the seam: `app` implements it
//! over `git`'s history walk, and tests implement it over a `Vec`.

use mergewright_core::commit::Commit;
use mergewright_core::commit_handle::{CommitHandle, Link};
use mergewright_core::oid::Oid;

/// A walk the layout can pull commits from, and go back to for commits it
/// has already seen.
pub trait CommitSource {
    /// Why a pull or a re-read can fail.
    type Error: std::error::Error + 'static;

    /// The next links in walk order, at most `limit` of them: each commit
    /// by handle, parents as this source shows them (a view that trims
    /// parents trims them here too). No commit object need be read.
    ///
    /// `stop` is polled between commits; a pull that saw it return `true`
    /// reports `stopped` and the links it had by then, and the next call
    /// carries on where it left off.
    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error>;

    /// The whole commits behind `commits`, one per handle and in the same
    /// order: what a row is assembled from. Parents as stored in the object,
    /// untrimmed — the row takes its parents from the shape. Every handle
    /// came from this source.
    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Self::Error>;

    /// The links of `commits`, one per handle and in the same order: each
    /// commit's parents as this source shows them (a view that trims parents
    /// trims them here too). Every handle came from this source.
    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Self::Error>;

    /// The handle of the commit `id`, if the repository's table has one.
    fn handle_of(&self, id: &Oid) -> Option<CommitHandle>;

    /// The id behind `handle`. Handles come only from this source, so this
    /// cannot fail; a foreign handle is a bug and panics.
    fn id_of(&self, handle: CommitHandle) -> Oid;
}

/// What one [`CommitSource::pull`] call produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pull {
    /// The links, in walk order.
    pub links: Vec<Link>,
    /// Whether the walk has nothing more to give.
    pub done: bool,
    /// Whether the walk gave up early because `stop` asked it to.
    pub stopped: bool,
}
