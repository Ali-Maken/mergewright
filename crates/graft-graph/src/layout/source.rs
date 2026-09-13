//! Where a streaming layout gets its commits.
//!
//! `graph` depends only on `core` (ARCHITECTURE.md), so it cannot open a
//! repository itself. A [`CommitSource`] is the seam: `app` implements it
//! over `git`'s history walk, and tests implement it over a `Vec`.

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;

/// A walk the layout can pull commits from, and go back to for commits it
/// has already seen.
pub trait CommitSource {
    /// Why a pull or a re-read can fail.
    type Error: std::error::Error + 'static;

    /// The next commits in walk order, at most `limit` of them.
    ///
    /// `stop` is polled between commits; a pull that saw it return `true`
    /// reports `stopped` and the commits it had by then, and the next call
    /// carries on where it left off.
    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error>;

    /// The commits with `ids`, one per id and in the same order: what a
    /// rebuild of evicted rows needs, and what a layout over a walk order
    /// already known in full (ADR-0008 promotion) is fed with.
    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Self::Error>;

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

/// What one [`CommitSource::next`] call produced.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pull {
    /// The commits, in walk order.
    pub commits: Vec<Commit>,
    /// Whether the walk has nothing more to give.
    pub done: bool,
    /// Whether the walk gave up early because `stop` asked it to.
    pub stopped: bool,
}
