//! Taking a walk's updates in, and the movements that drive the cursor and
//! the view: what `keys` and `history::walk_progress` call into.

use std::sync::Arc;

use graft_core::commit_handle::CommitHandle;
use graft_graph::layout::Anchor;

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
            HistoryUpdate::LaidOut { rows } => self.laid_out = Some(rows),
            // Answered: requests stop carrying the ask. The count itself is
            // the search's (`update::search::folded_hits`).
            HistoryUpdate::FoldedHits { search, .. } => {
                if self.folded_ask.as_ref().map(|ask| ask.search) == Some(search) {
                    self.folded_ask = None;
                }
            }
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
                    self.land_find(&window, id, height.max(1));
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
        self.promotion = Some(Promotion::Swapping { order });
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
    ///
    /// The selected commit is the one the cursor is on **now**, read from the
    /// rows still on hand in the old order. While a promotion swaps in, rows
    /// keep coming in that order (m7-power-tools/12 Part C) and the cursor
    /// keeps moving — by keys that ask for no rows, too — so any commit noted
    /// earlier may be one he has since moved off (m8/13m: `j j` between the
    /// two answers put the cursor back on the top row).
    fn promoted(&mut self, window: Window, height: usize) {
        let selected = self.selected().map(|row| Anchor {
            commit: row.commit.id,
            offset: self.cursor.saturating_sub(self.top),
        });
        let anchor = match self.promotion.take() {
            Some(Promotion::Swapping { .. }) => selected,
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
        // Folded away in the new order (ease-pass 06): the cursor lands on
        // the merge it now hides behind.
        let behind = window
            .sought
            .as_deref()
            .and_then(|sought| sought.anchor_behind);
        let landed = |commit| match behind {
            Some((anchor, merge)) if anchor == commit => merge,
            _ => commit,
        };
        let found = anchor.and_then(|anchor| {
            let shown = window.rows.iter().any(|row| row.commit.id == anchor.commit);
            let commit = if shown {
                anchor.commit
            } else {
                landed(anchor.commit)
            };
            let at = window.rows.iter().position(|row| row.commit.id == commit)?;
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
                self.land_find(&window, id, height);
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
    /// Nothing while [`History::head_pending`].
    pub fn show(&mut self, height: usize) -> Option<RowRequest> {
        if self.head_pending {
            return None;
        }
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
        let reach = self.reachable();
        let (first, last) = (reach.start, reach.end.saturating_sub(1).max(reach.start));
        let offset = self.cursor.saturating_sub(self.top);
        match motion {
            Motion::Down(count) => self.cursor = self.cursor.saturating_add(count).min(last),
            Motion::Up(count) => self.cursor = self.cursor.saturating_sub(count).max(first),
            Motion::PageDown => {
                self.cursor = self.cursor.saturating_add(height).min(last);
                self.top = self.cursor.saturating_sub(offset);
            }
            Motion::PageUp => {
                self.cursor = self.cursor.saturating_sub(height).max(first);
                self.top = self.cursor.saturating_sub(offset);
            }
            Motion::Top => {
                self.cursor = first;
                self.top = first;
            }
            Motion::Bottom if self.phase == Phase::Streaming => {
                let request = self.ask(Wanted::Last(2 * height), height);
                self.jump = Some(request.id);
                return Some(request);
            }
            Motion::Bottom => {
                self.cursor = last;
                self.top = reach.end.saturating_sub(height);
            }
        }
        // A viewport above the rows there are draws blank lines.
        self.top = self.top.max(first);
        self.follow(height);
        self.wanted(height, withdrew)
    }

    /// Switches to `view`, returning the request that shows the viewport in
    /// it — or `None` if nothing changes or the walk has gone.
    ///
    /// Switching `Follow` starts at the top: the other view has different
    /// rows, and the row numbers here mean nothing in it. A fold or a hidden
    /// stash keeps the viewport. Either way the selected commit is looked for
    /// in the rows that come back, and the cursor lands on it if it is there
    /// — unless a `G` or a seek is still out: that is asked again in the new
    /// view and lands as it would have.
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
        // The rows below the change are laid out again.
        self.phase = Phase::Streaming;
        let height = height.max(1);
        // A `G` or a seek not yet answered is superseded by this request:
        // ask it again in the new view, or it is lost — as `promote` does.
        // The launch's seek to `HEAD` is one: the stash list arrives with the
        // refs and changes the view (m8-polish-and-ship/45 A).
        if self.jump.is_some() {
            if let Some(commit) = self.seeking {
                return Some(self.find(commit, height));
            }
            let request = self.ask(Wanted::Last(2 * height), height);
            self.jump = Some(request.id);
            return Some(request);
        }
        self.wanted(height, true)
    }
}
