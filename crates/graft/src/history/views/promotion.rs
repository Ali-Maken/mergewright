//! Promoting a walk's layouts to a topological order (ADR-0008): the
//! promoted layout is opened pending, laid out towards its anchor between
//! requests, and swapped in once it has a row there (m7-power-tools/12
//! Part C). A child of [`super`], so it reads `Views`'s fields directly.

use std::ops::Range;
use std::sync::Arc;

use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;
use graft_graph::layout::{CommitSource, StreamError, StreamingLayout};
use graft_graph::{Follow, Included};

use crate::jobs::Delivery;

use super::super::state::{Ended, HistoryUpdate, Promote, RowRequest, View, Wanted};
use super::{IdleWork, Views};

/// A promoted layout on its way to the anchor.
pub(super) struct Pending<S: CommitSource> {
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

impl<S: CommitSource> Views<'_, S> {
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

    /// After a request answered from the provisional layout while a
    /// promotion is pending: `asked` (with its `Promote`) is answered again
    /// once the promoted layout is swapped in — if it was the request
    /// answered. A newer request without the order replaced it: the screen
    /// no longer asks for the order, and the pending layout is dropped.
    pub(in crate::history) fn answered_provisionally(&mut self, asked: RowRequest) {
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
    pub(in crate::history) fn abandon_promotion(&mut self) {
        self.pending = None;
    }

    /// The pending promotion's step: a chunk towards the anchor, and once it
    /// has a row, the swap and the last request answered again.
    pub(super) fn promotion_step(
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
pub(in crate::history) fn reach<S: CommitSource>(
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
pub(in crate::history) fn aim(request: &mut RowRequest, around: Option<Range<usize>>) {
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
