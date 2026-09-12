//! Where a streaming layout gets its commits.
//!
//! `graph` depends only on `core` (ARCHITECTURE.md), so it cannot open a
//! repository itself. A [`CommitSource`] is the seam: `app` implements it
//! over `git`'s history walk, and tests implement it over a `Vec`.

use graft_core::commit::Commit;
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
