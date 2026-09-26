//! The layouts one walk is shown through, and answering [`RowRequest`]s
//! against them: opening a layout the first time a view is asked for,
//! promoting every layout to a topological order (ADR-0008), and laying out
//! only as far as each request needs.

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_graph::inclusion::reinclude;
use graft_graph::layout::{CommitSource, StreamConfig, StreamError, StreamingLayout};
use graft_graph::{Follow, Included, Seeds};

use crate::jobs::Delivery;

use super::state::{Ended, HistoryUpdate, Promote, RowRequest, View, Wanted, Window};

/// Opens a fresh source over the same walk, for a view laid out on its own —
/// or, given a promoted order, a source that hands out that order instead.
pub type Open<'o, S> =
    Box<dyn FnMut(Option<&Arc<[CommitHandle]>>) -> Result<S, <S as CommitSource>::Error> + 'o>;

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
    promoted: Option<Arc<[CommitHandle]>>,
    all_parents: Option<StreamingLayout<Included<S>>>,
    first_parent: Option<StreamingLayout<Included<S>>>,
    /// The view last asked for: the one idle steps lay out ahead in.
    shown: Option<View>,
    /// The view and row count last announced as laid out to the end
    /// ([`HistoryUpdate::LaidOut`]), so it is said once per layout.
    announced: Option<(View, usize)>,
}

/// What one [`Views::idle_step`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdleWork {
    /// The frontier moved; `rows` laid out in this step.
    Advanced {
        /// Rows laid out in this step.
        rows: usize,
    },
    /// Nothing to do: the frontier is at the end of history, or nothing has
    /// been asked yet. The caller parks until a request comes.
    Caught,
    /// Announcing the end found nobody listening: the loop has gone.
    NobodyListening,
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
            shown: None,
            announced: None,
        }
    }

    /// One bounded step of work nobody asked for: lays the view last asked
    /// for out by at most one chunk towards the end of history, checking
    /// `stop` before every pull (DESIGN.md §3.6). Sends nothing on the way;
    /// once the end is reached, [`HistoryUpdate::LaidOut`] once.
    ///
    /// `stop` is the caller's "a request is waiting, or the walk is
    /// cancelled": either ends the step within one pull.
    ///
    /// # Errors
    /// Opening the source, or laying out, failed.
    pub(crate) fn idle_step(
        &mut self,
        stop: &dyn Fn() -> bool,
        report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
    ) -> Result<IdleWork, StreamError<S::Error>> {
        // Nothing asked yet: nothing is open, and nothing is opened unasked.
        let Some(view) = self.shown.clone() else {
            return Ok(IdleWork::Caught);
        };
        let layout = self.layout(&view)?;
        if !layout.is_done() {
            let before = layout.rows_emitted();
            let chunk = layout.config().chunk_rows;
            layout.advance(chunk, stop)?;
            return Ok(IdleWork::Advanced {
                rows: layout.rows_emitted() - before,
            });
        }
        let rows = layout.rows_emitted();
        let end = (view, rows);
        if self.announced.as_ref() == Some(&end) {
            return Ok(IdleWork::Caught);
        }
        self.announced = Some(end);
        if report(HistoryUpdate::LaidOut { rows }) == Delivery::NobodyListening {
            return Ok(IdleWork::NobodyListening);
        }
        Ok(IdleWork::Caught)
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
        if self.shown.as_ref() != Some(view) {
            self.shown = Some(view.clone());
        }
        let layout = self.slot(view.follow).insert(layout);
        // A fold or a stash shown moves the frontier back (`truncate`); the
        // next idle step lays out from there.
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
        // A new layout, not yet at the end: announced again once it is.
        self.announced = None;
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
        promoted: Option<&Arc<[CommitHandle]>>,
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
/// Between requests it lays out ahead, a chunk per [`Views::idle_step`], to
/// the end of history, so a later `G` or search finds its rows laid out; a
/// request that arrives stops that within one pull and is answered at once.
/// Once caught up it waits: this is a worker thread, and a walk nobody is
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
            None => match next_request(views, requests, stop, report)? {
                Next::Asked(request) => *request,
                Next::Ended(ended) => return Ok(ended),
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

/// What [`next_request`] came back with.
enum Next {
    /// The newest request waiting. Boxed: a request is 200-odd bytes.
    Asked(Box<RowRequest>),
    /// No request will come: cancelled, every sender gone, or nobody
    /// listening.
    Ended(Ended),
}

/// What the "is a request waiting?" check found, kept for the loop to take:
/// a channel cannot be peeked, only received from.
enum Arrival {
    Asked(Box<RowRequest>),
    SendersGone,
}

/// Waits for the next request, laying out ahead while none is waiting.
///
/// Every check for a request is also a `stop` check, so a cancel or a
/// request ends the idle step within one pull. Once caught up it parks in
/// `recv`: no wake, no work, until a request or the senders going.
fn next_request<S: CommitSource>(
    views: &mut Views<'_, S>,
    requests: &Receiver<RowRequest>,
    stop: &dyn Fn() -> bool,
    report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
) -> Result<Next, StreamError<S::Error>> {
    let arrived = RefCell::new(None);
    let interrupted = || {
        if stop() {
            return true;
        }
        let mut arrived = arrived.borrow_mut();
        if arrived.is_none() {
            *arrived = match requests.try_recv() {
                Ok(request) => Some(Arrival::Asked(Box::new(request))),
                Err(TryRecvError::Disconnected) => Some(Arrival::SendersGone),
                Err(TryRecvError::Empty) => None,
            };
        }
        arrived.is_some()
    };
    loop {
        if interrupted() {
            return Ok(match arrived.take() {
                Some(Arrival::Asked(first)) => {
                    Next::Asked(requests.try_iter().last().map_or(first, Box::new))
                }
                // Every sender is gone: the runner dropped it because the
                // job was cancelled or Graft is leaving. Nobody can ask
                // again. With nothing arrived, `stop` said stop.
                Some(Arrival::SendersGone) | None => Next::Ended(Ended::Cancelled),
            });
        }
        match views.idle_step(&interrupted, report)? {
            IdleWork::Advanced { .. } => {}
            IdleWork::NobodyListening => return Ok(Next::Ended(Ended::NobodyListening)),
            // `recv` failing means every sender is gone, as above.
            IdleWork::Caught => {
                return Ok(match requests.recv() {
                    Ok(first) => Next::Asked(Box::new(requests.try_iter().last().unwrap_or(first))),
                    Err(_) => Next::Ended(Ended::Cancelled),
                });
            }
        }
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
pub(super) fn answer<S: CommitSource>(
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
pub(super) fn error_chain(error: &dyn std::error::Error) -> String {
    let mut text = error.to_string();
    let mut cause = error.source();
    while let Some(inner) = cause {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        cause = inner.source();
    }
    text
}
