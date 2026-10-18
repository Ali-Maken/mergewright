//! The layouts one walk is shown through: opening a layout the first time a
//! view is asked for, promoting every layout to a topological order
//! (ADR-0008), and the work done between requests. Answering requests
//! against them is [`super::serve`].

mod promotion;
mod sent;

use std::collections::HashMap;
use std::sync::Arc;

use mergewright_core::commit_handle::CommitHandle;
use mergewright_core::oid::Oid;
use mergewright_graph::inclusion::reinclude;
use mergewright_graph::layout::{CommitSource, StreamConfig, StreamError, StreamingLayout};
use mergewright_graph::{Follow, Included, Seeds};

use crate::jobs::Delivery;

use super::fold::lay_out;
use super::serve::answer;
use super::state::{Ended, HistoryUpdate, RowRequest, View};

use promotion::Pending;
pub(super) use promotion::{aim, reach};
use sent::Sent;

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
/// the rows from the first commit they name (`mergewright_graph::reinclude`).
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
    /// The last window sent, sent again when idle work changes what it
    /// shows: a fold repair laying its rows out again, or the fold learning
    /// more about them (ease-pass 06).
    sent: Option<Sent>,
    /// Each opened merge's recovered branch name, read once from its row.
    labels: HashMap<Oid, Option<Arc<[u8]>>>,
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
            sent: None,
            labels: HashMap::new(),
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
            let chunk = layout.config().chunk_rows;
            let laid = lay_out(layout, chunk, stop)?;
            if self.refresh_sent(laid.invalidated, report)? == Delivery::NobodyListening {
                return Ok(IdleWork::NobodyListening);
            }
            return Ok(IdleWork::Advanced {
                rows: laid.advance.rows_emitted,
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
        // The window is held back until its fold facts are filled in, which
        // reads the layout `answer` is borrowing.
        let mut held = None;
        let ended = answer(layout, request, newer, stop, &mut |update| match update {
            HistoryUpdate::Window(window) => {
                held = Some(window);
                Delivery::Received
            }
            other => report(other),
        })?;
        let Some(window) = held else {
            return Ok(ended);
        };
        let window = self.with_folds(&view, window)?;
        self.answered = window.request;
        self.sent = Some(Sent::new(view, window.clone()));
        if report(HistoryUpdate::Window(window)) == Delivery::NobodyListening {
            return Ok(Some(Ended::NobodyListening));
        }
        Ok(ended)
    }

    /// Whether a promoted layout is pending: opened, not yet swapped in.
    #[cfg(test)]
    pub(super) fn is_promoting(&self) -> bool {
        self.pending.is_some()
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
