//! Reading history for the screen: the walk, laid out as it arrives.
//!
//! The walk lives in `graft-git` and the layout engine in `graft-graph`, and
//! neither may see the other (ARCHITECTURE.md). This is where they meet:
//! [`WalkSource`] puts a `Walk` behind the layout's `CommitSource` seam,
//! [`seeds_for`] turns ADR-0001's branch resolution into reserved lanes, and
//! [`stream`] drives a `StreamingLayout` for as long as the screen shows it,
//! answering each [`RowRequest`] with just the rows it names — laying out
//! further only when a request reaches past the frontier. The screen holds one
//! [`Window`] of rows, never the whole history (DESIGN.md §3.6).
//!
//! Nothing here knows about jobs or threads beyond a `stop` check, a
//! request channel and a `report` callback; `crate::jobs` supplies both.

mod scroll;
mod seeds;
mod source;

#[cfg(test)]
mod tests;

use std::ops::Range;
use std::sync::Arc;
use std::sync::mpsc::Receiver;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::{CHUNK, Order, Walk};
use graft_graph::inclusion::reinclude;
use graft_graph::layout::{Anchor, CommitSource, StreamConfig, StreamError, StreamingLayout};
use graft_graph::{Follow, Included, Inclusion, Row, Seeds};

use crate::jobs::Delivery;
use crate::repos::RepoStore;

pub use scroll::Motion;
pub use seeds::seeds_for;
pub use source::WalkSource;

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
    pub order: Arc<[Oid]>,
    /// The selected commit and its place in the viewport, if one is selected.
    pub anchor: Option<Anchor>,
    /// How many rows the viewport shows.
    pub height: usize,
}

impl Promote {
    /// The rows a viewport needs in `layout` so that the anchor sits where it
    /// sat — or `None` with no anchor, or one this view does not show.
    fn around<S: CommitSource>(&self, layout: &StreamingLayout<S>) -> Option<Range<usize>> {
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
        order: Arc<[Oid]>,
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

/// Walk `store`'s repository in `order`, answering each [`RowRequest`] that
/// arrives on `requests` with the rows it names.
///
/// Returns once `requests` has no sender left (the job was cancelled, or Graft
/// is leaving), `stop` says stop, or a report finds nobody listening.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, its
/// refs or branches could not be read, or the walk or the layout failed.
/// Formed here because it crosses a thread as part of a job result.
pub fn stream(
    store: &RepoStore,
    order: Order,
    requests: &Receiver<RowRequest>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Ended, String> {
    let branches = store.branches().map_err(|error| error.to_string())?;
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let refs = handle.references().map_err(|error| error.to_string())?;
    let seeds = seeds_for(&refs, &branches);

    let walk = Walk::all_refs(&handle, order).map_err(|error| error.to_string())?;
    let facts = WalkFacts {
        requested: walk.requested_order(),
        order: walk.order_in_force(),
        default_branch: branches.default_branch().cloned(),
        integration: branches.integration().to_vec().into(),
    };
    if report(HistoryUpdate::Started(Box::new(facts))) == Delivery::NobodyListening {
        return Ok(Ended::NobodyListening);
    }

    // The first view opened takes the walk already started; a second one
    // (the other `Follow`) starts its own.
    let handle = &handle;
    let mut started = Some(walk);
    let open = move |promoted: Option<&Arc<[Oid]>>| {
        if let Some(ids) = promoted {
            return Ok(WalkSource::in_order(handle, Arc::clone(ids)));
        }
        let walk = match started.take() {
            Some(walk) => walk,
            None => Walk::all_refs(handle, order)?,
        };
        Ok(WalkSource::new(handle, walk))
    };
    let mut views = Views::new(seeds, StreamConfig::default(), Box::new(open));
    serve(&mut views, requests, stop, report).map_err(|error| error_chain(&error))
}

/// The whole range's strict topological order, worked out without a
/// commit-graph file: what the view promotes to (ADR-0008). `Ok(None)` if
/// `stop` asked it to stop.
///
/// # Errors
/// The failure text, ready to show: the repository, its refs or a commit
/// could not be read. Formed here because it crosses a thread as part of a
/// job result.
pub fn topological_order(
    store: &RepoStore,
    stop: &dyn Fn() -> bool,
) -> Result<Option<Arc<[Oid]>>, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    // The walk reads each commit once, so a cache would fill and never be hit
    // (m1-the-graph/27b, fix D).
    let handle = repo.handle_with_object_cache(0);
    let tips = handle.commit_tips().map_err(|error| error.to_string())?;
    let mut walk = Walk::full_topological(&handle, &tips).map_err(|error| error_chain(&error))?;
    let mut order = Vec::new();
    loop {
        let chunk = walk
            .next_ids(CHUNK, stop)
            .map_err(|error| error_chain(&error))?;
        order.extend(chunk.ids);
        if chunk.stopped {
            return Ok(None);
        }
        if chunk.done {
            return Ok(Some(order.into()));
        }
    }
}

/// Opens a fresh source over the same walk, for a view laid out on its own —
/// or, given a promoted order, a source that hands out that order instead.
pub type Open<'o, S> =
    Box<dyn FnMut(Option<&Arc<[Oid]>>) -> Result<S, <S as CommitSource>::Error> + 'o>;

/// The layouts one walk is shown through: one per [`Follow`], each opened
/// the first time a request asks for it and kept while the walk lives.
///
/// First-parent view changes the seeds and so every row: it is its own
/// layout, and switching to it and back leaves the other's cache alone.
/// Folding and stash visibility change within a layout, throwing away only
/// the rows from the first commit they name (`graft_graph::reinclude`).
pub struct Views<'o, S: CommitSource> {
    seeds: Seeds,
    config: StreamConfig,
    open: Open<'o, S>,
    /// The promoted order every layout is opened over, once there is one.
    promoted: Option<Arc<[Oid]>>,
    all_parents: Option<StreamingLayout<Included<S>>>,
    first_parent: Option<StreamingLayout<Included<S>>>,
}

impl<'o, S: CommitSource> Views<'o, S> {
    /// Views of the walk `open` starts, laid out from `seeds`. Opens nothing
    /// until a request asks.
    #[must_use]
    pub fn new(seeds: Seeds, config: StreamConfig, open: Open<'o, S>) -> Self {
        Views {
            seeds,
            config,
            open,
            promoted: None,
            all_parents: None,
            first_parent: None,
        }
    }

    /// The layout showing `view`: opened if this `Follow` has none yet,
    /// re-included if what it hides has changed.
    ///
    /// # Errors
    /// Opening the source, or re-reading history to re-include, failed.
    pub fn layout(
        &mut self,
        view: &View,
    ) -> Result<&mut StreamingLayout<Included<S>>, StreamError<S::Error>> {
        let layout = if let Some(layout) = self.slot(view.follow).take() {
            layout
        } else {
            let promoted = self.promoted.clone();
            self.open_layout(view, promoted.as_ref())?
        };
        let layout = self.slot(view.follow).insert(layout);
        if layout.source().inclusion() != &view.inclusion {
            reinclude(layout, view.inclusion.clone())?;
        }
        Ok(layout)
    }

    /// Swaps every layout for ones over `promote`'s order, unless that order
    /// is already in force, and lays `view`'s out as far as the anchor.
    ///
    /// The layouts it replaces stay until the new one has reached the anchor,
    /// so the swap is one step: never an empty graph refilling. Returns
    /// `false` if `stop` asked it to stop first, leaving the old layouts in
    /// force.
    ///
    /// # Errors
    /// Opening the source, or laying out, failed.
    pub fn promote(
        &mut self,
        view: &View,
        promote: &Promote,
        stop: &dyn Fn() -> bool,
    ) -> Result<bool, StreamError<S::Error>> {
        let anchor = promote.anchor.map(|anchor| anchor.commit);
        let in_force = self
            .promoted
            .as_ref()
            .is_some_and(|order| Arc::ptr_eq(order, &promote.order));
        if in_force {
            return reach(self.layout(view)?, anchor, stop);
        }
        let mut layout = self.open_layout(view, Some(&promote.order))?;
        if !reach(&mut layout, anchor, stop)? {
            return Ok(false);
        }
        self.promoted = Some(Arc::clone(&promote.order));
        self.all_parents = None;
        self.first_parent = None;
        *self.slot(view.follow) = Some(layout);
        Ok(true)
    }

    /// Where `follow`'s layout is kept.
    fn slot(&mut self, follow: Follow) -> &mut Option<StreamingLayout<Included<S>>> {
        match follow {
            Follow::AllParents => &mut self.all_parents,
            Follow::FirstParent => &mut self.first_parent,
        }
    }

    /// A new layout of `view`, over the walk or over a promoted order.
    fn open_layout(
        &mut self,
        view: &View,
        promoted: Option<&Arc<[Oid]>>,
    ) -> Result<StreamingLayout<Included<S>>, StreamError<S::Error>> {
        let source = (self.open)(promoted).map_err(StreamError::Source)?;
        let included = Included::new(
            source,
            self.seeds.clone(),
            view.follow,
            view.inclusion.clone(),
        );
        // Laid out from the view's seeds, not the walk's.
        let seeds = included.seeds().clone();
        Ok(StreamingLayout::new(included, seeds, self.config))
    }
}

/// Lays `layout` out until `anchor` has a row, or history ends. Returns
/// `false` if `stop` asked it to stop first.
fn reach<S: CommitSource>(
    layout: &mut StreamingLayout<S>,
    anchor: Option<Oid>,
    stop: &dyn Fn() -> bool,
) -> Result<bool, StreamError<S::Error>> {
    let chunk = layout.config().chunk_rows;
    loop {
        let found = anchor.is_none_or(|id| {
            layout
                .row_of(id)
                .is_some_and(|row| row < layout.rows_emitted())
        });
        if found || layout.is_done() {
            return Ok(true);
        }
        let advanced = layout.advance(chunk, stop)?;
        if advanced.stopped || stop() {
            return Ok(false);
        }
    }
}

/// Answers requests from `requests` until it has no sender left.
///
/// Waits between requests: this is a worker thread, and a walk nobody is
/// scrolling costs nothing. Requests that queued while one was being answered
/// are skipped to the newest — only its answer would be looked at.
///
/// # Errors
/// Whatever the layout reports: the source failed, repeated a commit, stalled,
/// or a rebuilt chunk differed.
///
/// Each request is answered in the view it names; a newer request in another
/// view interrupts the one being answered and is answered next.
pub fn serve<S: CommitSource>(
    views: &mut Views<'_, S>,
    requests: &Receiver<RowRequest>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Ended, StreamError<S::Error>> {
    let mut interrupted: Option<RowRequest> = None;
    loop {
        let request = match interrupted.take() {
            Some(request) => request,
            // `recv` failing means every sender is gone: the runner dropped
            // it because the job was cancelled or Graft is leaving. Nobody
            // can ask again.
            None => match requests.recv() {
                Ok(first) => requests.try_iter().last().unwrap_or(first),
                Err(_) => return Ok(Ended::Cancelled),
            },
        };
        let view = request.view.clone();
        let mut request = request;
        if let Some(promote) = request.promote.clone() {
            if !views.promote(&view, &promote, stop)? {
                return Ok(Ended::Cancelled);
            }
            // With nothing to anchor to, row numbers are the best guess left.
            if let Some(around) = promote.around(views.layout(&view)?) {
                request.wanted = match request.wanted {
                    Wanted::Rows(_) => Wanted::Rows(around),
                    Wanted::Find { commit, rows, .. } => Wanted::Find {
                        commit,
                        rows,
                        fallback: around,
                    },
                    // Counted from the end, which the new order shares.
                    last @ Wanted::Last(_) => last,
                };
            }
        }
        let layout = views.layout(&view)?;
        let mut latest = None;
        let mut newer = || {
            let newest = requests.try_iter().last();
            latest.clone_from(&newest);
            newest
        };
        if let Some(ended) = answer(layout, request, &mut newer, stop, report)? {
            return Ok(ended);
        }
        // `answer` gives up on a request only for one in another view, or
        // one carrying a promotion.
        interrupted = latest.filter(|newest| newest.view != view || newest.promote.is_some());
    }
}

/// Answers one request: lays out only as far as it needs, at most a chunk
/// per step, then reports the rows it names.
///
/// Between chunks it reports progress and asks `newer` whether a newer request
/// has arrived; one that has replaces this one, so a withdrawn `G` stops
/// walking within a chunk. Returns `None` once the answer is sent — or,
/// sending nothing, once a newer request names a different view or carries a
/// promotion: `layout` cannot answer that one, and the caller must.
///
/// # Errors
/// Whatever the layout reports.
fn answer<S: CommitSource>(
    layout: &mut StreamingLayout<S>,
    mut request: RowRequest,
    newer: &mut dyn FnMut() -> Option<RowRequest>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Option<Ended>, StreamError<S::Error>> {
    let chunk = layout.config().chunk_rows;
    loop {
        let reach = match &request.wanted {
            Wanted::Rows(rows) => rows.end,
            // A far hit is reached by laying out forward from the frontier;
            // rows already emitted are found through the retained order and
            // rebuilt from their checkpoint, never re-walked (DESIGN.md §3.6).
            Wanted::Find { commit, .. } if layout.row_of(*commit).is_some() => 0,
            Wanted::Last(_) | Wanted::Find { .. } => usize::MAX,
        };
        if layout.is_done() || layout.rows_emitted() >= reach {
            break;
        }
        // Lay out only what this request still needs, a chunk at most: a
        // whole chunk read from the repository costs about 30 ms unoptimised,
        // two frames, when a page key needs a screenful (ticket 30c).
        let needed = chunk.min(reach - layout.rows_emitted());
        let advanced = layout.advance(needed, stop)?;
        if advanced.stopped || stop() {
            return Ok(Some(Ended::Cancelled));
        }
        let progress = HistoryUpdate::Progress {
            request: request.id,
            laid_out: layout.rows_emitted(),
        };
        if report(progress) == Delivery::NobodyListening {
            return Ok(Some(Ended::NobodyListening));
        }
        if let Some(replacement) = newer() {
            if replacement.view != request.view || replacement.promote.is_some() {
                return Ok(None);
            }
            request = replacement;
        }
    }

    let laid_out = layout.rows_emitted();
    let range = match request.wanted {
        Wanted::Rows(rows) => rows.start.min(laid_out)..rows.end.min(laid_out),
        Wanted::Last(count) => laid_out.saturating_sub(count)..laid_out,
        Wanted::Find {
            commit,
            rows,
            fallback,
        } => match layout.row_of(commit) {
            Some(row) => row.saturating_sub(rows)..row.saturating_add(rows + 1).min(laid_out),
            None => fallback.start.min(laid_out)..fallback.end.min(laid_out),
        },
    };
    layout.set_viewport(range.clone());
    let rows = layout.rows(range.clone())?;
    let window = Window {
        request: request.id,
        first: range.start,
        rows: rows.into(),
        laid_out,
        done: layout.is_done(),
        promoted: request.promote.is_some(),
    };
    if report(HistoryUpdate::Window(window)) == Delivery::NobodyListening {
        return Ok(Some(Ended::NobodyListening));
    }
    Ok(None)
}

/// An error and every cause under it, on one line: `StreamError::Source`
/// says only "reading history", and the reason is in its source.
fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    text
}
