//! Moving through history on screen: the cursor, the viewport, and asking the
//! walk for the rows they need.
//!
//! All of it is plain arithmetic on [`History`], so `update` stays pure: a
//! move returns the [`RowRequest`] it needs sent, if any, and the answer comes
//! back later through [`History::apply`].

use graft_graph::Row;
use graft_graph::layout::Anchor;

use std::ops::Range;
use std::sync::Arc;

use graft_core::oid::Oid;

use super::{History, HistoryUpdate, Phase, Promote, Promotion, RowRequest, View, Wanted};

/// A cursor movement the graph pane understands (DESIGN.md §2.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Motion {
    /// Down this many rows (`j`, `↓`).
    Down(usize),
    /// Up this many rows (`k`, `↑`).
    Up(usize),
    /// A screen down, keeping the cursor's place on screen (`PgDn`).
    PageDown,
    /// A screen up, keeping the cursor's place on screen (`PgUp`).
    PageUp,
    /// The first row (`g`).
    Top,
    /// The last row (`G`). On a walk not yet laid out to its end, a request
    /// that walks there while the viewport stays put.
    Bottom,
}

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
    /// [`Window::promoted`]: super::Window::promoted
    pub fn promote(&mut self, order: Arc<[Oid]>, height: usize) -> Option<RowRequest> {
        if matches!(self.phase, Phase::Cancelled | Phase::Failed(_)) {
            self.promotion = Some(Promotion::Kept);
            return None;
        }
        self.promotion = Some(Promotion::Swapping {
            order,
            anchor: None,
        });
        self.wanted(height.max(1), true)
    }

    /// Takes in the first rows in the promoted order: the cursor goes back on
    /// the selected commit, at the same place on screen.
    fn promoted(&mut self, window: super::Window, height: usize) {
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
        self.jump = None;
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

    /// The row the cursor is on, if it has arrived.
    #[must_use]
    pub fn selected(&self) -> Option<&Row> {
        self.row_at(self.cursor)
    }

    /// The row at absolute row number `at`, if it is in the window on hand.
    /// What [`History::selected`] asks for at the cursor, and what a mouse
    /// hover asks for at whatever row is under the pointer (m1-the-graph/17).
    #[must_use]
    pub fn row_at(&self, at: usize) -> Option<&Row> {
        let window = self.window.as_ref()?;
        window.rows.get(at.checked_sub(window.first)?)
    }

    /// Moves the cursor straight to row `at` — a click, not a step — clamped
    /// to the last row laid out so far. Scrolls the viewport to keep it on
    /// screen and returns the request for any rows that now need fetching.
    pub fn select_at(&mut self, at: usize, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        let withdrew = self.jump.take().is_some();
        self.seeking = None;
        self.cursor = at.min(self.rows.saturating_sub(1));
        self.follow(height);
        self.wanted(height, withdrew)
    }

    /// Scrolls the viewport by `lines` (negative is up) without moving the
    /// selection — the wheel (DESIGN.md §2.15), which is not the keyboard's
    /// `j`/`k` and does not follow the cursor onto or off screen.
    pub fn scroll(&mut self, lines: isize, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        let last = self.rows.saturating_sub(1);
        self.top = if lines.is_negative() {
            self.top.saturating_sub(lines.unsigned_abs())
        } else {
            self.top.saturating_add(lines.unsigned_abs()).min(last)
        };
        self.wanted(height, false)
    }

    /// Puts the cursor on `commit` (a search hit), centred on screen — at
    /// once if the rows on hand hold it, else by asking the walk to lay out
    /// until it has a row ([`Wanted::Find`]). A pending find is a jump: the
    /// header says so and `Esc` withdraws it. `None` when there is nothing to
    /// ask (the rows on hand did it, or the walk has gone).
    pub fn seek(&mut self, commit: Oid, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        let on_hand = self.window.as_ref().and_then(|window| {
            let at = window.rows.iter().position(|row| row.commit.id == commit)?;
            Some(window.first + at)
        });
        if let Some(row) = on_hand {
            let withdrew = self.jump.take().is_some();
            self.seeking = None;
            self.cursor = row;
            self.top = row.saturating_sub(height / 2);
            return self.wanted(height, withdrew);
        }
        if matches!(self.phase, Phase::Cancelled | Phase::Failed(_)) {
            return None;
        }
        let fallback = self.drawn(self.top.saturating_sub(height)..self.top + 2 * height);
        let request = self.ask(
            Wanted::Find {
                commit,
                rows: height,
                fallback,
            },
            height,
        );
        self.jump = Some(request.id);
        self.seeking = Some(commit);
        Some(request)
    }

    /// Puts the cursor and viewport back where they were (`Esc` on a running
    /// search), withdrawing any jump, and returns the request for their rows.
    pub fn restore(&mut self, cursor: usize, top: usize, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        self.jump = None;
        self.seeking = None;
        self.cursor = cursor;
        self.top = top;
        self.keep_within_rows(height);
        self.wanted(height, true)
    }

    /// Withdraws a `G` still walking, returning the request that replaces it
    /// — the rows where the viewport already is — or `None` if there was no
    /// jump to withdraw.
    pub fn withdraw_jump(&mut self, height: usize) -> Option<RowRequest> {
        self.jump.take()?;
        self.seeking = None;
        self.wanted(height.max(1), true)
    }

    /// The rows on screen in a `height`-row viewport, as far as they have
    /// arrived: fewer than `height` at the end of history or before the
    /// first answer.
    #[must_use]
    pub fn visible(&self, height: usize) -> &[Row] {
        let Some(window) = &self.window else {
            return &[];
        };
        let start = self.top.saturating_sub(window.first).min(window.rows.len());
        let end = self
            .top
            .saturating_add(height)
            .saturating_sub(window.first)
            .min(window.rows.len());
        window.rows.get(start..end).unwrap_or_default()
    }

    /// Pulls the cursor and the viewport back onto the rows there are, once
    /// the walk has said how many. The viewport is clamped on its own, not to
    /// the cursor: the wheel leaves the cursor off screen on purpose, and
    /// clamping the top to it snapped a wheel-scrolled view back to row 0 on
    /// every answer after the walk finished (m1-the-graph/32). Only a cursor
    /// that had to move is followed back into view.
    fn keep_within_rows(&mut self, height: usize) {
        let last = self.rows.saturating_sub(1);
        self.top = self.top.min(last);
        if self.cursor > last {
            self.cursor = last;
            self.follow(height);
        }
    }

    /// Scrolls just enough to keep the cursor inside the viewport.
    fn follow(&mut self, height: usize) {
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + height {
            self.top = self.cursor + 1 - height;
        }
    }

    /// The request for the rows a viewport at `top` needs, unless the window
    /// already covers them and enough rows are laid out below the cursor to
    /// keep moving. `force` asks regardless: it replaces a withdrawn jump.
    ///
    /// A screen of slack is asked for on both sides, so moving a row at a time
    /// asks again only once per screen.
    fn wanted(&mut self, height: usize, force: bool) -> Option<RowRequest> {
        let complete = match self.phase {
            Phase::Streaming => false,
            Phase::Complete => true,
            // The job has gone: nobody would answer.
            Phase::Cancelled | Phase::Failed(_) => return None,
        };
        let bottom = self.top + height;
        let needed = self.drawn(self.top..bottom);
        let covered = self.window.as_ref().is_some_and(|window| {
            let end = if complete {
                needed.end.min(self.rows)
            } else {
                needed.end
            };
            window.first <= needed.start && window.end() >= end
        });
        let room_below = complete || self.rows > self.cursor + height;
        if covered && room_below && !force {
            return None;
        }
        let asked = self.drawn(self.top.saturating_sub(height)..bottom + height);
        Some(self.ask(Wanted::Rows(asked), height))
    }

    /// The rows a renderer needs on hand to draw `rows`. The text tiers draw
    /// exactly those. The pixel tier draws whole aligned strips, and each
    /// strip's first node needs the row above it: so the range widens to
    /// strip boundaries, and one row further up. Padding here, rather than
    /// rasterising short strips, keeps a strip's cache key its real range.
    fn drawn(&self, rows: Range<usize>) -> Range<usize> {
        match self.strip_rows {
            None => rows,
            Some(strip) => {
                let strip = strip.max(1);
                let start = (rows.start / strip * strip).saturating_sub(1);
                start..rows.end.div_ceil(strip) * strip
            }
        }
    }

    /// The next request, numbered after every earlier one — carrying the
    /// promoted order and the selection while a promotion is swapping in.
    fn ask(&mut self, wanted: Wanted, height: usize) -> RowRequest {
        self.asked += 1;
        let selected = self.selected().map(|row| Anchor {
            commit: row.commit.id,
            offset: self.cursor.saturating_sub(self.top),
        });
        let promote = match &mut self.promotion {
            Some(Promotion::Swapping { order, anchor }) => {
                // The selection off the rows on hand keeps the last one known.
                if selected.is_some() {
                    *anchor = selected;
                }
                Some(Promote {
                    order: Arc::clone(order),
                    anchor: *anchor,
                    height,
                })
            }
            _ => None,
        };
        RowRequest {
            id: self.asked,
            wanted,
            view: self.view.clone(),
            promote,
        }
    }
}
