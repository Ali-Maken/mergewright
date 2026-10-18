//! What became of a commit the screen went looking for (ease-pass 06c): the
//! worker's answer to a [`Wanted::Find`](super::Wanted::Find), the reveal
//! the app owes when the default fold hides it, and the search's ask for how
//! many of its hits are folded.
//!
//! A commit hidden by the default fold is revealed by any route that goes to
//! it: its merge opens and the cursor lands on it. One hidden by a view he
//! chose (first-parent `1`, hidden stashes) keeps "not in this view".

use std::sync::Arc;

use mergewright_core::oid::Oid;

/// How the worker answered a `Wanted::Find`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Found {
    /// The commit has a row, and the window holds the rows around it.
    Row,
    /// Hidden by the default fold behind this shown merge; the window holds
    /// the merge's rows.
    Folded {
        /// The shown merge to open: the commit, or the next merge in on the
        /// way to it, is on its side.
        behind: Oid,
    },
    /// Not in this view (a view he chose hides it), or not in history: the
    /// window holds the request's fallback rows.
    NotInView,
}

/// A reveal the app owes: open `merge`, then seek `target` again. Every
/// seek's find owes one when the default fold hides its commit — a seek a
/// promotion re-asks too: it is still a route going to the commit. Only the
/// promotion's own anchor (the cursor's commit, not sought) lands on the
/// merge and opens nothing ([`Sought::anchor_behind`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reveal {
    /// The shown merge to open.
    pub merge: Oid,
    /// The commit sought.
    pub target: Oid,
}

/// Asked beside rows: how many of a search's hits the default fold hides
/// now. Rides on every request until a [`HistoryUpdate::FoldedHits`] with
/// the same `search` comes back, so a superseded request never loses it.
///
/// [`HistoryUpdate::FoldedHits`]: super::HistoryUpdate::FoldedHits
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoldedAsk {
    /// This ask's number ([`History::ask_folded`](super::History::ask_folded)
    /// counts them): an answer to an older ask is told apart from the newest.
    pub search: u64,
    /// The hits, in walk order.
    pub ids: Arc<[Oid]>,
}

/// What became of the commits a request sought, when there is news: a
/// promotion's anchor this order folds away, and a find's answer.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Sought {
    /// On a promoted window: the request's anchor, when the new order folds
    /// it away, and the shown merge it now hides behind — where the cursor
    /// lands instead (ease-pass 06).
    pub anchor_behind: Option<(Oid, Oid)>,
    /// On the answer to a [`Wanted::Find`](super::Wanted::Find): how the
    /// worker found its commit.
    pub found: Option<Found>,
}
