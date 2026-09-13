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

use graft_core::ref_name::RefName;
use graft_git::history::{Order, Walk};
use graft_graph::inclusion::reinclude;
use graft_graph::layout::{CommitSource, StreamConfig, StreamError, StreamingLayout};
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
    /// The row the cursor is on. A plain index; selection is ticket 16's.
    pub cursor: usize,
    /// The id of the newest request. Answers to any other are dropped.
    pub asked: u64,
    /// The `G` request still laying out, if one is. `Esc` withdraws it.
    pub jump: Option<u64>,
    /// The view every request asks for: first-parent or not, what is folded
    /// and hidden.
    pub view: View,
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
            asked: 0,
            jump: None,
            view: View::default(),
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
    let open = move || {
        let walk = match started.take() {
            Some(walk) => walk,
            None => Walk::all_refs(handle, order)?,
        };
        Ok(WalkSource::new(handle, walk))
    };
    let mut views = Views::new(seeds, StreamConfig::default(), Box::new(open));
    serve(&mut views, requests, stop, report).map_err(|error| error_chain(&error))
}

/// Opens a fresh source over the same walk, for a view laid out on its own.
pub type Open<'o, S> = Box<dyn FnMut() -> Result<S, <S as CommitSource>::Error> + 'o>;

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
        let slot = match view.follow {
            Follow::AllParents => &mut self.all_parents,
            Follow::FirstParent => &mut self.first_parent,
        };
        if let Some(layout) = slot {
            if layout.source().inclusion() != &view.inclusion {
                reinclude(layout, view.inclusion.clone())?;
            }
            Ok(layout)
        } else {
            {
                let source = (self.open)().map_err(StreamError::Source)?;
                let included = Included::new(
                    source,
                    self.seeds.clone(),
                    view.follow,
                    view.inclusion.clone(),
                );
                // Laid out from the view's seeds, not the walk's.
                let seeds = included.seeds().clone();
                Ok(slot.insert(StreamingLayout::new(included, seeds, self.config)))
            }
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
        // `answer` gives up on a request only for one in another view.
        interrupted = latest.filter(|newest| newest.view != view);
    }
}

/// Answers one request: lays out as far as it needs, a chunk at a time, then
/// reports the rows it names.
///
/// Between chunks it reports progress and asks `newer` whether a newer request
/// has arrived; one that has replaces this one, so a withdrawn `G` stops
/// walking within a chunk. Returns `None` once the answer is sent — or,
/// sending nothing, once a newer request names a different view: `layout`
/// cannot answer that one, and the caller must.
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
            Wanted::Last(_) => usize::MAX,
        };
        if layout.is_done() || layout.rows_emitted() >= reach {
            break;
        }
        let advanced = layout.advance(chunk, stop)?;
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
            if replacement.view != request.view {
                return Ok(None);
            }
            request = replacement;
        }
    }

    let laid_out = layout.rows_emitted();
    let range = match request.wanted {
        Wanted::Rows(rows) => rows.start.min(laid_out)..rows.end.min(laid_out),
        Wanted::Last(count) => laid_out.saturating_sub(count)..laid_out,
    };
    layout.set_viewport(range.clone());
    let rows = layout.rows(range.clone())?;
    let window = Window {
        request: request.id,
        first: range.start,
        rows: rows.into(),
        laid_out,
        done: layout.is_done(),
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
