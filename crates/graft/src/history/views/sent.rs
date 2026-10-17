//! The last window sent, and sending it again when idle work changes what
//! it shows (ease-pass 06). Folding by default decides as the walk goes: a
//! repair can lay rows the screen holds out again (a commit hidden too
//! early, a merge's edge left out), and a merge's fold or an opened side is
//! only known once the walk has reached far enough. Nothing asks for those
//! rows again by itself, so the worker sends them unasked, under the same
//! request id — dropped by the screen if a newer request has since gone out.
//! A child of [`super`], so it reads `Views`'s fields directly.

use graft_graph::layout::{CommitSource, StreamError};

use crate::jobs::Delivery;

use super::super::fold::row_folds;
use super::super::state::{HistoryUpdate, View, Window};
use super::Views;

/// A window sent, in the view it was sent in.
pub(super) struct Sent {
    view: View,
    window: Window,
    /// The first row a repair has laid out again since the window was sent,
    /// if one has.
    stale_from: Option<usize>,
}

impl Sent {
    pub(super) fn new(view: View, window: Window) -> Self {
        Sent {
            view,
            window,
            stale_from: None,
        }
    }
}

impl<S: CommitSource> Views<'_, S> {
    /// `window`, which `view`'s layout answered, with its rows' fold facts.
    ///
    /// # Errors
    /// Reading an opened merge's row back failed.
    pub(super) fn with_folds(
        &mut self,
        view: &View,
        mut window: Window,
    ) -> Result<Window, StreamError<S::Error>> {
        let mut labels = std::mem::take(&mut self.labels);
        let folds = match self.slot(view.follow).as_mut() {
            Some(layout) => row_folds(layout, &window.rows, &mut labels),
            None => Ok(window.fold.clone()),
        };
        self.labels = labels;
        window.fold = folds?;
        Ok(window)
    }

    /// After idle work laid out more (`invalidated`: the first row a repair
    /// threw away, if one did): sends the last window again if its rows
    /// were laid out anew — once the layout is past them again — or if the
    /// fold now says more about them. Says whether anyone was listening.
    ///
    /// # Errors
    /// Reading the rows or an opened merge's row back failed.
    pub(super) fn refresh_sent(
        &mut self,
        invalidated: Option<usize>,
        report: &mut dyn FnMut(HistoryUpdate) -> Delivery,
    ) -> Result<Delivery, StreamError<S::Error>> {
        let Some(mut sent) = self.sent.take() else {
            return Ok(Delivery::Received);
        };
        if let Some(row) = invalidated {
            sent.stale_from = Some(sent.stale_from.map_or(row, |from| from.min(row)));
        }
        let refreshed = self.refreshed(&sent);
        let window = match refreshed {
            Ok(Some(window)) => window,
            Ok(None) => {
                self.sent = Some(sent);
                return Ok(Delivery::Received);
            }
            Err(error) => {
                self.sent = Some(sent);
                return Err(error);
            }
        };
        sent.stale_from = None;
        sent.window = window.clone();
        self.sent = Some(sent);
        Ok(report(HistoryUpdate::Window(window)))
    }

    /// The window `sent` should now be, or `None` when it is still right or
    /// cannot be told yet.
    fn refreshed(&mut self, sent: &Sent) -> Result<Option<Window>, StreamError<S::Error>> {
        let mut labels = std::mem::take(&mut self.labels);
        let refreshed = match self.slot(sent.view.follow).as_mut() {
            // Another inclusion since: a newer request's answer replaces it.
            Some(layout) if layout.source().inclusion() == &sent.view.inclusion => {
                let end = sent.window.end();
                let laid_out = layout.rows_emitted();
                let done = layout.is_done();
                if sent.stale_from.is_some_and(|from| from < end) {
                    if laid_out < end && !done {
                        // Not laid out that far again yet: the next step.
                        Ok(None)
                    } else {
                        let range = sent.window.first.min(laid_out)..end.min(laid_out);
                        layout.rows(range.clone()).and_then(|rows| {
                            let fold = row_folds(layout, &rows, &mut labels)?;
                            Ok(Some(Window {
                                first: range.start,
                                rows: rows.into(),
                                laid_out,
                                done,
                                promoted: false,
                                fold,
                                anchor_behind: None,
                                ..sent.window.clone()
                            }))
                        })
                    }
                } else {
                    row_folds(layout, &sent.window.rows, &mut labels).map(|fold| {
                        (fold != sent.window.fold).then(|| Window {
                            laid_out,
                            done,
                            promoted: false,
                            fold,
                            anchor_behind: None,
                            ..sent.window.clone()
                        })
                    })
                }
            }
            _ => Ok(None),
        };
        self.labels = labels;
        refreshed
    }
}
