//! The state a walk's history holds on the screen, and what a running walk
//! reports: [`History`] plus the requests and updates that flow between it
//! and `stream`/`views`. Moving `History` is in `scroll.rs`.

use std::ops::Range;
use std::sync::Arc;

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::Order;
use graft_graph::layout::{Anchor, CommitSource, StreamingLayout};
use graft_graph::{Follow, Inclusion, Row};

/// What a walk is, known before its first commit: the order it is in and the
/// branches its lanes are anchored on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WalkFacts {
    /// The order that was asked for.
    pub requested: Order,
    /// The order the commits actually come in. Different from
    /// [`WalkFacts::requested`] when there is no commit-graph file and
    /// ADR-0008 fell back to commit date.
    pub order: Order,
    /// Which branch owns lane 0 (ADR-0001), or [`None`] in a repository with
    /// no branches.
    pub default_branch: Option<RefName>,
    /// The branches that sit beside lane 0 and never compete for it.
    pub integration: Arc<[RefName]>,
}

impl WalkFacts {
    /// Is this an order the user should be told is not the final one?
    #[must_use]
    pub fn is_provisional(&self) -> bool {
        self.requested != self.order
    }
}

/// Which rows the screen wants from the walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Wanted {
    /// These rows. The walk is laid out as far as their end first, if it has
    /// not got there yet; past the end of history, fewer come back.
    Rows(Range<usize>),
    /// The last this-many rows of history. The walk is laid out to its end
    /// first — on a cold walk of a big repository, that is the long one (`G`).
    Last(usize),
    /// The rows around one commit: `rows` either side of it (search, `n`/`N`).
    /// The walk is laid out until the commit has a row, or to the end if it
    /// never gets one (it is not in this view); then `fallback` comes back
    /// instead, so the viewport keeps its rows.
    Find {
        /// The commit to reach.
        commit: Oid,
        /// Rows either side of it.
        rows: usize,
        /// The rows sent when the commit is not in this view.
        fallback: Range<usize>,
    },
}

/// Which history the screen is looking at: every parent or first parents
/// only, and what is folded or hidden (PROMPT.md §1.3, DESIGN.md §3.6).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct View {
    /// Every parent, or only `parents[0]` (the first-parent view).
    pub follow: Follow,
    /// Merges folded and stashes hidden.
    pub inclusion: Inclusion,
}

/// One ask for rows, numbered so its answer can be told from a stale one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRequest {
    /// Increases with every ask; only the newest ask's answer is wanted.
    pub id: u64,
    /// The rows asked for.
    pub wanted: Wanted,
    /// The view they are asked for in. Every request says it, so the newest
    /// request alone decides what the walk shows.
    pub view: View,
    /// The promoted order to show it in, while the screen is waiting for the
    /// walk to take it up (ADR-0008). Carried by *every* request until an
    /// answer says it is in force, so skipping to the newest request can
    /// never skip the promotion. Row numbers in `wanted` were counted in the
    /// order being replaced, so rows by number are asked for again around the
    /// anchor (kept as they are without one); a `G` or a search jump is kept,
    /// and lands in the new order.
    pub promote: Option<Promote>,
}

/// A topological order to swap in, and where the screen is in the order it
/// replaces (ADR-0008, DESIGN.md §3.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Promote {
    /// Every commit, in strict topological order.
    pub order: Arc<[CommitHandle]>,
    /// The selected commit and its place in the viewport, if one is selected.
    pub anchor: Option<Anchor>,
    /// How many rows the viewport shows.
    pub height: usize,
}

impl Promote {
    /// The rows a viewport needs in `layout` so that the anchor sits where it
    /// sat — or `None` with no anchor, or one this view does not show.
    pub(super) fn around<S: CommitSource>(
        &self,
        layout: &StreamingLayout<S>,
    ) -> Option<Range<usize>> {
        let anchor = self.anchor?;
        let top = layout.row_of(anchor.commit)?.saturating_sub(anchor.offset);
        Some(top.saturating_sub(self.height)..top + 2 * self.height)
    }
}

/// The rows that answer one [`RowRequest`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    /// The request this answers.
    pub request: u64,
    /// The row number of `rows[0]`.
    pub first: usize,
    /// The rows, in order. Shared rather than cloned: the state is copied on
    /// every event.
    pub rows: Arc<[Row]>,
    /// How many rows the walk had laid out when it answered.
    pub laid_out: usize,
    /// Whether that is every row there is.
    pub done: bool,
    /// Whether these rows answer a request's [`Promote`]: their row numbers
    /// are the promoted order's, and the screen re-finds its anchor in them.
    pub promoted: bool,
}

impl Window {
    /// The row number one past the last row held.
    #[must_use]
    pub fn end(&self) -> usize {
        self.first + self.rows.len()
    }
}

/// One thing a running walk tells the screen.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryUpdate {
    /// The walk has started; this is what it is. Always the first update.
    Started(Box<WalkFacts>),
    /// Still laying out towards request `request`; `laid_out` rows so far.
    /// What the screen shows while `G` walks a cold history.
    Progress {
        /// The request being worked towards.
        request: u64,
        /// Rows laid out so far.
        laid_out: usize,
    },
    /// The rows a request asked for.
    Window(Window),
    /// Laid out to the end of history while nobody was asking: `rows` is
    /// every row the view has. Sent once per layout that gets there idle —
    /// again after a fold or a promotion changes the layout — and nothing
    /// is sent on the way (m7-power-tools/12 Part B).
    LaidOut {
        /// Every row of the view.
        rows: usize,
    },
}

/// How far promoting a provisional order to strict topological order has
/// got (ADR-0008). Only a walk whose order is provisional has one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Promotion {
    /// The topological order is being worked out in the background.
    Sorting,
    /// The order has been handed to the walk, attached to every request until
    /// an answer comes back in it. `anchor` is the selection it re-finds.
    Swapping {
        /// Every commit, in topological order.
        order: Arc<[CommitHandle]>,
        /// The selected commit and its place on screen when last known.
        anchor: Option<Anchor>,
    },
    /// Given up — `Esc`, or the sort failed: the order on screen stays
    /// provisional.
    Kept,
}

/// Where a walk is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Not every commit has a row yet; more are laid out as the screen asks.
    Streaming,
    /// Every commit has a row.
    Complete,
    /// The user stopped it. The rows on screen stay.
    Cancelled,
    /// It broke, and this is why.
    Failed(String),
}

/// The history on screen: a window of rows around the viewport, and where the
/// viewport and the cursor are.
///
/// Never the whole history. The rows held are the latest [`Window`] the walk
/// answered with, so memory follows the screen, not the repository (DESIGN.md
/// §3.6); moving is in `scroll.rs`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct History {
    /// What the walk is, once it has said.
    pub walk: Option<WalkFacts>,
    /// The rows last sent for the newest request, if any have come.
    pub window: Option<Window>,
    /// How many rows the walk has laid out, as far as the screen has heard.
    pub rows: usize,
    /// Where the walk is.
    pub phase: Phase,
    /// The row at the top of the viewport.
    pub top: usize,
    /// The row the cursor is on: the selection. A row number in the current
    /// view, so it names the same commit however its chunk comes and goes;
    /// [`History::selected`] reads the commit.
    pub cursor: usize,
    /// The commit to put the cursor back on once the next window arrives:
    /// set by a view change, which can move every row number.
    pub reanchor: Option<Oid>,
    /// The id of the newest request. Answers to any other are dropped.
    pub asked: u64,
    /// The `G` request still laying out, if one is. `Esc` withdraws it.
    pub jump: Option<u64>,
    /// The commit a pending jump is seeking (`History::seek`), rather than
    /// the bottom of history; `None` for a `G`.
    pub seeking: Option<Oid>,
    /// The view every request asks for: first-parent or not, what is folded
    /// and hidden.
    pub view: View,
    /// Promotion to topological order, while one is under way or was given
    /// up; `None` once promoted, or when the order was never provisional.
    pub promotion: Option<Promotion>,
    /// The pixel tier's strip height, or `None` for the text tiers. When set,
    /// every request is widened to whole strips plus the row above the first
    /// (whose edges reach into it), so the strip cache only ever sees full
    /// aligned strips; the text tiers ask for exactly what they did before.
    pub strip_rows: Option<usize>,
    /// The rows the newest request asked for, until its window arrives: a
    /// key that finds them already asked for does not ask again
    /// (m1-the-graph/30c). `None` when nothing is out or the newest request
    /// is not a range (`G`, a search's find).
    pub outstanding: Option<Range<usize>>,
    /// Every row the walk last said it had, once it laid out to the end of
    /// history unasked ([`HistoryUpdate::LaidOut`]); `None` until then. Kept
    /// as heard: nothing is drawn from it yet.
    pub laid_out: Option<usize>,
}

impl History {
    /// A walk that has been asked for and has sent nothing yet.
    #[must_use]
    pub fn started() -> Self {
        History {
            walk: None,
            window: None,
            rows: 0,
            phase: Phase::Streaming,
            top: 0,
            cursor: 0,
            reanchor: None,
            asked: 0,
            jump: None,
            seeking: None,
            view: View::default(),
            promotion: None,
            strip_rows: None,
            outstanding: None,
            laid_out: None,
        }
    }
}

/// How a [`stream`] or [`serve`] call ended, when it did not fail.
///
/// There is no "complete": a walk that has laid out every row still answers
/// requests for as long as the screen asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ended {
    /// `stop` asked it to stop, or nobody will ask for rows again.
    Cancelled,
    /// A report found nobody listening: the loop has gone.
    NobodyListening,
}
