//! The layouts one walk is shown through: opening a layout the first time a
//! view is asked for, promoting every layout to a topological order
//! (ADR-0008), and the work done between requests. Answering requests
//! against them is [`super::serve`].

use std::ops::Range;
use std::sync::Arc;

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_graph::inclusion::reinclude;
use graft_graph::layout::{CommitSource, StreamConfig, StreamError, StreamingLayout};
use graft_graph::{Follow, Included, Seeds};

use crate::jobs::Delivery;

use super::serve::answer;
use super::state::{Ended, HistoryUpdate, Promote, RowRequest, View, Wanted};

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
    /// A promoted layout opened and not yet swapped in: it reaches its
    /// anchor between requests, while requests are answered from the layout
    /// in force (m7-power-tools/12 Part C).
    pending: Option<Pending<S>>,
    /// The id of the newest request answered: tells a request answered
    /// provisionally from one a newer request replaced.
    answered: u64,
}

/// A promoted layout on its way to the anchor.
struct Pending<S: CommitSource> {
    /// The order it is laid out in.
    order: Arc<[CommitHandle]>,
    /// The `Follow` it was opened for.
    follow: Follow,
    /// The commit that must have a row before it is swapped in (with at
    /// least a chunk laid out): the selection the newest request carried.
    anchor: Option<Oid>,
    layout: StreamingLayout<Included<S>>,
    /// The newest request answered from the provisional layout meanwhile,
    /// with its `Promote`: answered again from this layout once swapped in.
    ask: Option<RowRequest>,
}

/// What one [`Views::idle_step`] did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IdleWork {
    /// The frontier moved; `rows` laid out in this step.
    Advanced {
        /// Rows laid out in this step.
        rows: usize,
    },
    /// A pending promotion moved toward its anchor — or reached it, was
    /// swapped in, and the last request was answered again.
    Promoting {
        /// Rows of the promoted layout laid out towards the anchor.
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
            pending: None,
            answered: 0,
        }
    }

    /// One bounded step of work nobody asked for, checking `stop` before
    /// every pull (DESIGN.md §3.6). A pending promotion first: its layout
    /// moves a chunk towards the anchor, and once the anchor has a row it is
    /// swapped in and the last request answered again from it — the one
    /// window idle work sends. Otherwise the view last asked for moves at
    /// most one chunk towards the end of history, sending nothing on the
    /// way; once the end is reached, [`HistoryUpdate::LaidOut`] once.
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
        if self.pending.is_some() {
            return self.promotion_step(stop, report);
        }
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

    /// Answers `request` in its view (see [`answer`]), remembering its id as
    /// the newest answered once its window is sent.
    ///
    /// # Errors
    /// Opening, re-including or laying out failed.
    pub(super) fn answer(
        &mut self,
        request: RowRequest,
        newer: &mut dyn FnMut() -> Option<RowRequest>,
        stop: &dyn Fn() -> bool,
        report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
    ) -> Result<Option<Ended>, StreamError<S::Error>> {
        let view = request.view.clone();
        let layout = self.layout(&view)?;
        let mut sent = None;
        let ended = answer(layout, request, newer, stop, &mut |update| {
            if let HistoryUpdate::Window(window) = &update {
                sent = Some(window.request);
            }
            report(update)
        })?;
        if let Some(id) = sent {
            self.answered = id;
        }
        Ok(ended)
    }

    /// Opens a layout over `promote`'s order for `view` and leaves it
    /// pending: [`Views::idle_step`] lays it out to the anchor between
    /// requests. Returns whether the order is in force now — already, or
    /// because a pending layout kept from an earlier request is ready to
    /// swap; then the caller answers from it. A pending layout for the same
    /// order and `Follow` is kept, aimed at the newest anchor.
    ///
    /// # Errors
    /// Opening the source failed.
    pub(crate) fn begin_promotion(
        &mut self,
        view: &View,
        promote: &Promote,
    ) -> Result<bool, StreamError<S::Error>> {
        let in_force = self
            .promoted
            .as_ref()
            .is_some_and(|order| Arc::ptr_eq(order, &promote.order));
        if in_force {
            return Ok(true);
        }
        let anchor = promote.anchor.map(|anchor| anchor.commit);
        let kept = self.pending.as_ref().is_some_and(|pending| {
            Arc::ptr_eq(&pending.order, &promote.order) && pending.follow == view.follow
        });
        if !kept {
            let layout = self.open_layout(view, Some(&promote.order))?;
            self.pending = Some(Pending {
                order: Arc::clone(&promote.order),
                follow: view.follow,
                anchor,
                layout,
                ask: None,
            });
        }
        let Some(pending) = self.pending.as_mut() else {
            return Ok(false);
        };
        pending.anchor = anchor;
        if ready_to_swap(&pending.layout, anchor) {
            // Ready: in force before this request is answered.
            self.swap();
            return Ok(true);
        }
        Ok(false)
    }

    /// Promotes at once: begins the promotion and lays the promoted layout
    /// out to the anchor before returning, as `serve` did before
    /// m7-power-tools/12 Part C. The worker promotes between requests
    /// instead ([`Views::idle_step`]); this is for measuring the swap on its
    /// own (`examples/budgets`). Returns `false` if `stop` stopped it first,
    /// the promoted layout left pending.
    ///
    /// # Errors
    /// Opening the source, or laying out, failed.
    pub fn promote(
        &mut self,
        view: &View,
        promote: &Promote,
        stop: &dyn Fn() -> bool,
    ) -> Result<bool, StreamError<S::Error>> {
        if self.begin_promotion(view, promote)? {
            return Ok(true);
        }
        let Some(pending) = self.pending.as_mut() else {
            return Ok(false);
        };
        if !reach(&mut pending.layout, pending.anchor, stop)? {
            return Ok(false);
        }
        self.swap();
        Ok(true)
    }

    /// Whether a promoted layout is pending: opened, not yet swapped in.
    #[cfg(test)]
    pub(super) fn is_promoting(&self) -> bool {
        self.pending.is_some()
    }

    /// After a request answered from the provisional layout while a
    /// promotion is pending: `asked` (with its `Promote`) is answered again
    /// once the promoted layout is swapped in — if it was the request
    /// answered. A newer request without the order replaced it: the screen
    /// no longer asks for the order, and the pending layout is dropped.
    pub(super) fn answered_provisionally(&mut self, asked: RowRequest) {
        let answered = self.answered;
        let Some(pending) = self.pending.as_mut() else {
            return;
        };
        if answered == asked.id {
            pending.ask = Some(asked);
        } else if answered > asked.id {
            self.pending = None;
        }
    }

    /// The screen has stopped asking for the pending order (a request came
    /// without it): the promoted layout is dropped unswapped.
    pub(super) fn abandon_promotion(&mut self) {
        self.pending = None;
    }

    /// The pending promotion's step: a chunk towards the anchor, and once it
    /// has a row, the swap and the last request answered again.
    fn promotion_step(
        &mut self,
        stop: &dyn Fn() -> bool,
        report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
    ) -> Result<IdleWork, StreamError<S::Error>> {
        let Some(pending) = self.pending.as_mut() else {
            return Ok(IdleWork::Caught);
        };
        let before = pending.layout.rows_emitted();
        if !ready_to_swap(&pending.layout, pending.anchor) {
            let chunk = pending.layout.config().chunk_rows;
            pending.layout.advance(chunk, stop)?;
        }
        let rows = pending.layout.rows_emitted() - before;
        if !ready_to_swap(&pending.layout, pending.anchor) {
            return Ok(IdleWork::Promoting { rows });
        }
        let Some(mut request) = self.swap() else {
            return Ok(IdleWork::Promoting { rows });
        };
        if let Some(promote) = request.promote.clone() {
            let layout = self.layout(&request.view)?;
            aim(&mut request, promote.around(layout));
        }
        // The rows were on screen already, in the order being replaced: no
        // progress to show, only the one window in the new order.
        let mut quiet = |update: HistoryUpdate| match update {
            HistoryUpdate::Progress { .. } => Delivery::Received,
            other => report(other),
        };
        match self.answer(request, &mut || None, stop, &mut quiet)? {
            Some(Ended::NobodyListening) => Ok(IdleWork::NobodyListening),
            // Stopped: a request is waiting and the caller answers it from
            // the layout now in force — or the walk is cancelled, which the
            // caller's own `stop` check sees.
            Some(Ended::Cancelled) | None => Ok(IdleWork::Promoting { rows }),
        }
    }

    /// Puts the pending layout in force: every other layout goes, and the
    /// order is the one every layout opens over from now on. Returns the
    /// request to answer again from it, if one was answered meanwhile.
    fn swap(&mut self) -> Option<RowRequest> {
        let pending = self.pending.take()?;
        self.promoted = Some(pending.order);
        self.all_parents = None;
        self.first_parent = None;
        // A new layout, not yet at the end: announced again once it is.
        self.announced = None;
        *self.slot(pending.follow) = Some(pending.layout);
        pending.ask
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

/// Whether a pending promoted layout is swapped in: once the anchor has a
/// row and at least its first chunk is laid out, or history has ended.
///
/// The first chunk even with no anchor: the swap then lands with the rows
/// the normal path lands with. A promotion that reached the walk before its
/// first window carries no anchor, and swapping that in with nothing laid
/// out answered only the rows asked for, never `done` on a history one chunk
/// holds — the walk then stayed short of `Complete` for good, where the
/// normal path, anchored and so a chunk laid out, completed (M7/04b, 12c).
fn ready_to_swap<S: CommitSource>(layout: &StreamingLayout<S>, anchor: Option<Oid>) -> bool {
    layout.is_done()
        || (layout.rows_emitted() >= layout.config().chunk_rows && has_row(layout, anchor))
}

/// Whether `anchor` has a row in `layout`: at once with no anchor, and once
/// history has ended, found or not.
fn has_row<S: CommitSource>(layout: &StreamingLayout<S>, anchor: Option<Oid>) -> bool {
    layout.is_done()
        || anchor.is_none_or(|id| {
            layout
                .row_of(id)
                .is_some_and(|row| row < layout.rows_emitted())
        })
}

/// Lays `layout` out until `anchor` has a row, or history ends. Returns
/// `false` if `stop` asked it to stop first.
pub(super) fn reach<S: CommitSource>(
    layout: &mut StreamingLayout<S>,
    anchor: Option<Oid>,
    stop: &dyn Fn() -> bool,
) -> Result<bool, StreamError<S::Error>> {
    let chunk = layout.config().chunk_rows;
    loop {
        if has_row(layout, anchor) {
            return Ok(true);
        }
        let advanced = layout.advance(chunk, stop)?;
        if advanced.stopped || stop() {
            return Ok(false);
        }
    }
}

/// Re-aims a request whose row numbers were counted in the order being
/// replaced: rows by number are asked for `around` the anchor instead. With
/// nothing to anchor to, row numbers are the best guess left.
pub(super) fn aim(request: &mut RowRequest, around: Option<Range<usize>>) {
    let Some(around) = around else {
        return;
    };
    request.wanted = match request.wanted.clone() {
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
