//! Taking a walk's updates in, and the movements that drive the cursor and
//! the view: what `keys` and `history::walk_progress` call into.

use std::sync::Arc;

use graft_core::commit_handle::CommitHandle;

use super::super::{History, HistoryUpdate, Phase, Promotion, RowRequest, View, Wanted, Window};
use super::Motion;

impl History {
    /// Takes in one update from the walk. `height` is how many rows the
    /// viewport shows, which is where a finished `G` puts the last row.
    ///
    /// Progress and rows for any request but the newest are dropped: they
    /// answer a question that has since been replaced or withdrawn.
    pub fn apply(&mut self, update: HistoryUpdate, height: usize) {
        match update {
            HistoryUpdate::Started(facts) => self.walk = Some(*facts),
            HistoryUpdate::Progress { request, laid_out } => {
                if request == self.asked {
                    self.rows = self.rows.max(laid_out);
                }
            }
            HistoryUpdate::Window(window) => {
                if window.request != self.asked {
                    return;
                }
                // The newest request is answered: nothing is out any more.
                self.outstanding = None;
                if window.promoted {
                    self.promoted(window, height.max(1));
                    return;
                }
                self.rows = window.laid_out;
                // A view with fewer rows can leave the cursor past the end.
                if window.done {
                    self.keep_within_rows(height.max(1));
                }
                if window.done && self.phase == Phase::Streaming {
                    self.phase = Phase::Complete;
                }
                // A view change moves row numbers: find the selected commit
                // among the new rows. Not there, the cursor keeps its index.
                if let Some(id) = self.reanchor.take()
                    && let Some(at) = window.rows.iter().position(|row| row.commit.id == id)
                {
                    self.cursor = window.first + at;
                    self.follow(height.max(1));
                }
                if self.jump == Some(window.request)
                    && let Some(id) = self.seeking.take()
                {
                    self.jump = None;
                    // Not in this view: the cursor keeps its row, and the
                    // caller sees `selected` is not the commit it sought.
                    if let Some(at) = window.rows.iter().position(|row| row.commit.id == id) {
                        self.cursor = window.first + at;
                        self.top = self.cursor.saturating_sub(height.max(1) / 2);
                    }
                }
                if self.jump == Some(window.request) {
                    self.jump = None;
                    self.cursor = self.rows.saturating_sub(1);
                    self.top = self.rows.saturating_sub(height.max(1));
                }
                self.window = Some(window);
            }
        }
    }

    /// Hands the walk a topological `order` to promote to, returning the
    /// request that carries it — or `None` if the walk has gone, in which case
    /// the order on screen stays provisional.
    ///
    /// Every request from here on carries the order and the selection, until
    /// an answer comes back in the new order ([`Window::promoted`]).
    ///
    /// [`Window::promoted`]: Window::promoted
    pub fn promote(&mut self, order: Arc<[CommitHandle]>, height: usize) -> Option<RowRequest> {
        if matches!(self.phase, Phase::Cancelled | Phase::Failed(_)) {
            self.promotion = Some(Promotion::Kept);
            return None;
        }
        self.promotion = Some(Promotion::Swapping {
            order,
            anchor: None,
        });
        let height = height.max(1);
        // A `G` or a search jump not yet answered is superseded by this
        // request: ask it again, carrying the order, or it is lost (27c).
        if self.jump.is_some() {
            if let Some(commit) = self.seeking {
                return self.seek(commit, height);
            }
            let request = self.ask(Wanted::Last(2 * height), height);
            self.jump = Some(request.id);
            return Some(request);
        }
        self.wanted(height, true)
    }

    /// Takes in the first rows in the promoted order: the cursor goes back on
    /// the selected commit, at the same place on screen — unless the request
    /// was a `G` or a search jump, which the walk kept and answered in the new
    /// order: then it lands as it would have (m1-the-graph/27c).
    fn promoted(&mut self, window: Window, height: usize) {
        let anchor = match self.promotion.take() {
            Some(Promotion::Swapping { anchor, .. }) => anchor,
            other => {
                // Only a request made while swapping carries an order, and
                // only the newest request's answer gets here.
                self.promotion = other;
                None
            }
        };
        if let Some(walk) = self.walk.as_mut() {
            walk.order = walk.requested;
        }
        self.rows = window.laid_out;
        self.reanchor = None;
        if matches!(self.phase, Phase::Streaming | Phase::Complete) {
            self.phase = if window.done {
                Phase::Complete
            } else {
                Phase::Streaming
            };
        }
        let found = anchor.and_then(|anchor| {
            let at = window
                .rows
                .iter()
                .position(|row| row.commit.id == anchor.commit)?;
            Some((window.first + at, anchor.offset))
        });
        if let Some((row, offset)) = found {
            self.cursor = row;
            self.top = row.saturating_sub(offset);
        } else {
            // Nothing selected, or a commit this view does not show.
            self.cursor = self.cursor.min(self.rows.saturating_sub(1));
            self.top = self.top.min(self.cursor);
        }
        self.follow(height);
        if self.jump == Some(window.request) {
            if let Some(id) = self.seeking {
                // Not in this view: the cursor stays on the anchor.
                if let Some(at) = window.rows.iter().position(|row| row.commit.id == id) {
                    self.cursor = window.first + at;
                    self.top = self.cursor.saturating_sub(height / 2);
                }
            } else {
                self.cursor = self.rows.saturating_sub(1);
                self.top = self.rows.saturating_sub(height);
            }
        }
        // Answered, or a jump no request is still asking for: either way no
        // answer is coming, and a search waiting on the seek must hear so.
        self.jump = None;
        self.seeking = None;
        self.window = Some(window);
    }

    /// The request that fills a `height`-row viewport where it is now, if the
    /// rows held do not already. For a walk just started, and after a resize.
    pub fn show(&mut self, height: usize) -> Option<RowRequest> {
        self.wanted(height.max(1), false)
    }

    /// Moves the cursor, scrolling the viewport to keep it on screen, and
    /// returns the request for any rows that now need fetching.
    ///
    /// Any move replaces a `G` still walking: the newest ask wins.
    pub fn move_cursor(&mut self, motion: Motion, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        let withdrew = self.jump.take().is_some();
        self.seeking = None;
        let last = self.rows.saturating_sub(1);
        let offset = self.cursor.saturating_sub(self.top);
        match motion {
            Motion::Down(count) => self.cursor = self.cursor.saturating_add(count).min(last),
            Motion::Up(count) => self.cursor = self.cursor.saturating_sub(count),
            Motion::PageDown => {
                self.cursor = self.cursor.saturating_add(height).min(last);
                self.top = self.cursor.saturating_sub(offset);
            }
            Motion::PageUp => {
                self.cursor = self.cursor.saturating_sub(height);
                self.top = self.cursor.saturating_sub(offset);
            }
            Motion::Top => {
                self.cursor = 0;
                self.top = 0;
            }
            Motion::Bottom if self.phase == Phase::Streaming => {
                let request = self.ask(Wanted::Last(2 * height), height);
                self.jump = Some(request.id);
                return Some(request);
            }
            Motion::Bottom => {
                self.cursor = last;
                self.top = self.rows.saturating_sub(height);
            }
        }
        self.follow(height);
        self.wanted(height, withdrew)
    }

    /// Switches to `view`, returning the request that shows the viewport in
    /// it — or `None` if nothing changes or the walk has gone.
    ///
    /// Switching `Follow` starts at the top: the other view has different
    /// rows, and the row numbers here mean nothing in it. A fold or a hidden
    /// stash keeps the viewport. Either way the selected commit is looked for
    /// in the rows that come back, and the cursor lands on it if it is there.
    pub fn change_view(&mut self, view: View, height: usize) -> Option<RowRequest> {
        if view == self.view || matches!(self.phase, Phase::Cancelled | Phase::Failed(_)) {
            return None;
        }
        self.reanchor = self.selected().map(|row| row.commit.id);
        if view.follow != self.view.follow {
            self.top = 0;
            self.cursor = 0;
        }
        self.view = view;
        self.jump = None;
        // The rows below the change are laid out again.
        self.phase = Phase::Streaming;
        self.wanted(height.max(1), true)
    }
}
