//! The history worker's loop: answering [`RowRequest`]s against a walk's
//! [`Views`], laying out only as far as each request needs, and laying out
//! ahead between them.

use std::cell::RefCell;
use std::ops::Range;
use std::sync::mpsc::{Receiver, TryRecvError};

use std::sync::Arc;

use mergewright_core::oid::Oid;
use mergewright_graph::Included;
use mergewright_graph::inclusion::Whereabouts;
use mergewright_graph::layout::{CommitSource, StreamError, StreamingLayout};

use crate::jobs::Delivery;

use super::fold::{landing, lay_out};
use super::sought::{FoldedAsk, Found, Sought};
use super::state::{Ended, HistoryUpdate, RowRequest, Wanted, Window};
use super::views::{IdleWork, Views, aim, reach};

/// Answers requests from `requests` until it has no sender left.
///
/// Between requests it lays out ahead, a chunk per [`Views::idle_step`], to
/// the end of history, so a later `G` or search finds its rows laid out; a
/// request that arrives stops that within one pull and is answered at once.
/// Once caught up it waits: this is a worker thread, and a walk nobody is
/// scrolling costs nothing. Requests that queued while one was being answered
/// are skipped to the newest — only its answer would be looked at.
///
/// A request carrying a promotion (ADR-0008) is answered from the layout in
/// force until the promoted layout has the anchor's row (Part C of
/// m7-power-tools/12): idle steps lay it out, swap it in, and answer the last
/// request again from it.
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
        let mut request = match interrupted.take() {
            Some(request) => request,
            None => match next_request(views, requests, stop, report)? {
                Next::Asked(request) => *request,
                Next::Ended(ended) => return Ok(ended),
            },
        };
        let view = request.view.clone();
        let mut provisional = None;
        if let Some(promote) = request.promote.clone() {
            if views.begin_promotion(&view, &promote)? {
                // In force: laid out to the anchor, as far as a swap left it.
                let anchor = promote.anchor.map(|anchor| anchor.commit);
                if !reach(views.layout(&view)?, anchor, stop)? {
                    return Ok(Ended::Cancelled);
                }
                aim(&mut request, promote.around(views.layout(&view)?));
            } else {
                // Answered in the order on screen, so the window says so and
                // the screen keeps waiting for the promoted one.
                provisional = Some(request.clone());
                request.promote = None;
            }
        } else {
            views.abandon_promotion();
        }
        let mut latest = None;
        let mut newer = || {
            let newest = requests.try_iter().last();
            latest.clone_from(&newest);
            newest
        };
        if let Some(ended) = views.answer(request, &mut newer, stop, report)? {
            return Ok(ended);
        }
        if let Some(asked) = provisional {
            views.answered_provisionally(asked);
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
                // job was cancelled or Mergewright is leaving. Nobody can ask
                // again. With nothing arrived, `stop` said stop.
                Some(Arrival::SendersGone) | None => Next::Ended(Ended::Cancelled),
            });
        }
        match views.idle_step(&interrupted, report)? {
            IdleWork::Advanced { .. } | IdleWork::Promoting { .. } => {}
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
/// per step (mending what folding by default got wrong, `fold::lay_out`),
/// then reports the rows it names. The window's per-row fold facts are left
/// empty: [`Views::answer`] fills them.
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
    layout: &mut StreamingLayout<Included<S>>,
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
            Wanted::Find { commit, .. } if located(layout, *commit).is_some() => 0,
            Wanted::Last(_) | Wanted::Find { .. } => usize::MAX,
        };
        if layout.is_done() || layout.rows_emitted() >= reach {
            break;
        }
        // Lay out only what this request still needs, a chunk at most: a
        // whole chunk read from the repository costs about 30 ms unoptimised,
        // two frames, when a page key needs a screenful (ticket 30c).
        let needed = chunk.min(reach - layout.rows_emitted());
        let advanced = lay_out(layout, needed, stop)?.advance;
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
    let (range, sought) = answered(layout, &request);
    layout.set_viewport(range.clone());
    let rows = layout.rows(range.clone())?;
    let window = Window {
        request: request.id,
        first: range.start,
        rows: rows.into(),
        laid_out,
        done: layout.is_done(),
        promoted: request.promote.is_some(),
        fold: Arc::from([]),
        sought: (sought != Sought::default()).then(|| Box::new(sought)),
    };
    if report(HistoryUpdate::Window(window)) == Delivery::NobodyListening {
        return Ok(Some(Ended::NobodyListening));
    }
    if let Some(ask) = &request.folded
        && report(folded_hits(layout, ask)) == Delivery::NobodyListening
    {
        return Ok(Some(Ended::NobodyListening));
    }
    Ok(None)
}

/// The rows `request` is answered with, once laid out far enough, and what
/// a find or a promotion learnt on the way.
fn answered<S: CommitSource>(
    layout: &StreamingLayout<Included<S>>,
    request: &RowRequest,
) -> (Range<usize>, Sought) {
    let mut sought = Sought {
        anchor_behind: anchor_behind(layout, request),
        found: None,
    };
    let laid_out = layout.rows_emitted();
    let range = match &request.wanted {
        Wanted::Rows(rows) => rows.start.min(laid_out)..rows.end.min(laid_out),
        Wanted::Last(count) => laid_out.saturating_sub(*count)..laid_out,
        Wanted::Find {
            commit,
            rows,
            fallback,
        } => {
            let (at, found) = located(layout, *commit).unwrap_or((None, Found::NotInView));
            sought.found = Some(found);
            match at {
                Some(row) => row.saturating_sub(*rows)..row.saturating_add(rows + 1).min(laid_out),
                None => fallback.start.min(laid_out)..fallback.end.min(laid_out),
            }
        }
    };
    (range, sought)
}

/// Where a find's `commit` stands once the walk has decided it: the row to
/// centre on (its own, or the shown merge the default fold hides it behind)
/// and how it was found. `None` while the walk has not decided it — or
/// decided it shown, or folded behind a merge, without the row laid out yet
/// — so the find lays out further. Hidden by a view he chose: no row.
fn located<S: CommitSource>(
    layout: &StreamingLayout<Included<S>>,
    commit: Oid,
) -> Option<(Option<usize>, Found)> {
    if let Some(row) = layout.row_of(commit) {
        return Some((Some(row), Found::Row));
    }
    match layout.source().whereabouts(commit) {
        Whereabouts::Folded { behind } => {
            let row = layout.row_of(behind)?;
            Some((Some(row), Found::Folded { behind }))
        }
        Whereabouts::Filtered => Some((None, Found::NotInView)),
        Whereabouts::Shown | Whereabouts::NotWalked => None,
    }
}

/// The answer to `ask`, from what the walk has decided so far.
fn folded_hits<S: CommitSource>(
    layout: &StreamingLayout<Included<S>>,
    ask: &FoldedAsk,
) -> HistoryUpdate {
    let (mut folded, mut unwalked) = (0, 0);
    for &id in ask.ids.iter() {
        match layout.source().whereabouts(id) {
            Whereabouts::Folded { .. } => folded += 1,
            Whereabouts::NotWalked => unwalked += 1,
            Whereabouts::Shown | Whereabouts::Filtered => {}
        }
    }
    HistoryUpdate::FoldedHits {
        search: ask.search,
        folded,
        unwalked,
    }
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

/// A promoting request's anchor, when the new order folds it away, and the
/// shown merge it hides behind (ease-pass 06).
fn anchor_behind<S: CommitSource>(
    layout: &StreamingLayout<Included<S>>,
    request: &RowRequest,
) -> Option<(Oid, Oid)> {
    let anchor = request.promote.as_ref()?.anchor?.commit;
    Some((anchor, landing(layout, anchor)?))
}
