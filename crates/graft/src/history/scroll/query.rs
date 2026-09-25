//! Reading the window on hand, and asking the walk for more of it: the
//! selection, the visible rows, and building the next [`RowRequest`].

use std::ops::Range;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_graph::Row;
use graft_graph::layout::Anchor;

use super::super::{History, Phase, Promote, Promotion, RowRequest, Wanted};

impl History {
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
        let reach = self.reachable();
        self.cursor = at.min(reach.end.saturating_sub(1)).max(reach.start);
        self.follow(height);
        self.wanted(height, withdrew)
    }

    /// Scrolls the viewport by `lines` (negative is up) without moving the
    /// selection — the wheel (DESIGN.md §2.15), which is not the keyboard's
    /// `j`/`k` and does not follow the cursor onto or off screen.
    ///
    /// Withdraws a pending `G` or search jump, as [`History::select_at`]
    /// does: the request the wheel sends supersedes it, and no answer will
    /// ever carry the jump's id, so leaving it set would strand `seeking`
    /// with the header stuck on "seeking" (m1-the-graph/65).
    pub fn scroll(&mut self, lines: isize, height: usize) -> Option<RowRequest> {
        let height = height.max(1);
        let reach = self.reachable();
        let last = reach.end.saturating_sub(1).max(reach.start);
        self.top = if lines.is_negative() {
            self.top
                .saturating_sub(lines.unsigned_abs())
                .max(reach.start)
        } else {
            self.top.saturating_add(lines.unsigned_abs()).min(last)
        };
        let withdrew = self.jump.take().is_some();
        self.seeking = None;
        self.wanted(height, withdrew)
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

    /// The walk is working on an ask: rows the screen asked for have not
    /// come (a range, a `G`, a search's find, or the first screen). A walk
    /// resting between asks, complete, stopped or failed is not reading.
    /// Half of `update::walk_is_reading`, which also needs the walk to be
    /// the one the screen waits on (m6-conflicts/85 A1).
    #[must_use]
    pub fn is_reading(&self) -> bool {
        self.phase == Phase::Streaming
            && (self.jump.is_some() || self.outstanding.is_some() || self.window.is_none())
    }

    /// The rows a motion can land on. While a walk is alive, every row it
    /// has laid out: the rows not on hand are asked for. Once it has gone —
    /// stopped or failed — nobody answers an ask, so only the rows on hand:
    /// a cursor past them had nothing to draw, which is what `Esc` then `G`
    /// on a long history showed (m6-conflicts/85 A3).
    pub(super) fn reachable(&self) -> Range<usize> {
        match self.phase {
            Phase::Streaming | Phase::Complete => 0..self.rows,
            Phase::Cancelled | Phase::Failed(_) => self
                .window
                .as_ref()
                .map_or(0..0, |window| window.first..window.end()),
        }
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
    pub(super) fn keep_within_rows(&mut self, height: usize) {
        let last = self.rows.saturating_sub(1);
        self.top = self.top.min(last);
        if self.cursor > last {
            self.cursor = last;
            self.follow(height);
        }
    }

    /// Scrolls just enough to keep the cursor inside the viewport.
    pub(super) fn follow(&mut self, height: usize) {
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + height {
            self.top = self.cursor + 1 - height;
        }
    }

    /// The request that keeps the rows on hand ahead of the keys, or `None`
    /// while they already are. `force` asks regardless: it replaces a
    /// withdrawn jump.
    ///
    /// PROMPT.md's third principle: a key never waits on the worker. Every
    /// motion but `G` moves at most a screen, so while the window holds the
    /// viewport and two screens either side, the next two keys are drawn
    /// from rows on hand — the answer may land a key late on a slow CPU.
    /// Once it doesn't, this asks for three screens either side: the answer
    /// arrives as an event and redraws, and moving a row at a time asks about
    /// once a screen rather than on every key (m1-the-graph/30c). A request
    /// already out for those rows is not sent again.
    pub(super) fn wanted(&mut self, height: usize, force: bool) -> Option<RowRequest> {
        let complete = match self.phase {
            Phase::Streaming => false,
            Phase::Complete => true,
            // The job has gone: nobody would answer.
            Phase::Cancelled | Phase::Failed(_) => return None,
        };
        let bottom = self.top + height;
        let ahead = self.drawn(self.top.saturating_sub(2 * height)..bottom + 2 * height);
        let on_hand = self.window.as_ref().is_some_and(|window| {
            let end = if complete {
                ahead.end.min(self.rows)
            } else {
                ahead.end
            };
            window.first <= ahead.start && window.end() >= end
        });
        let asking = self.outstanding.as_ref().is_some_and(|rows| {
            let end = if complete {
                ahead.end.min(self.rows)
            } else {
                ahead.end
            };
            rows.start <= ahead.start && rows.end >= end
        });
        if (on_hand || asking) && !force {
            return None;
        }
        let asked = self.drawn(self.top.saturating_sub(3 * height)..bottom + 3 * height);
        Some(self.ask(Wanted::Rows(asked), height))
    }

    /// The rows a renderer needs on hand to draw `rows`. The text tiers draw
    /// exactly those. The pixel tier draws whole aligned strips, and each
    /// strip's first node needs the row above it: so the range widens to
    /// strip boundaries, and one row further up. Padding here, rather than
    /// rasterising short strips, keeps a strip's cache key its real range.
    pub(super) fn drawn(&self, rows: Range<usize>) -> Range<usize> {
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
    pub(super) fn ask(&mut self, wanted: Wanted, height: usize) -> RowRequest {
        self.asked += 1;
        self.outstanding = match &wanted {
            Wanted::Rows(rows) => Some(rows.clone()),
            Wanted::Last(_) | Wanted::Find { .. } => None,
        };
        let selected = self.selected().map(|row| Anchor {
            commit: row.commit.id,
            offset: self.cursor.saturating_sub(self.top),
        });
        let promote = match &mut self.promotion {
            Some(Promotion::Swapping { order, anchor }) => {
                // A cursor moved off the rows on hand has no commit to anchor
                // to yet: an older anchor would snap it back (27c).
                *anchor = selected;
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
