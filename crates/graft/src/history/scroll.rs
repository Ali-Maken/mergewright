//! Moving through history on screen: the cursor, the viewport, and asking the
//! walk for the rows they need.
//!
//! All of it is plain arithmetic on [`History`], so `update` stays pure: a
//! move returns the [`RowRequest`] it needs sent, if any, and the answer comes
//! back later through [`History::apply`].

use graft_graph::Row;

use super::{History, HistoryUpdate, Phase, RowRequest, View, Wanted};

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
                self.rows = window.laid_out;
                // A view with fewer rows can leave the cursor past the end.
                if window.done {
                    self.cursor = self.cursor.min(self.rows.saturating_sub(1));
                    self.top = self.top.min(self.cursor);
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
                if self.jump == Some(window.request) {
                    self.jump = None;
                    self.cursor = self.rows.saturating_sub(1);
                    self.top = self.rows.saturating_sub(height.max(1));
                }
                self.window = Some(window);
            }
        }
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
                let request = self.ask(Wanted::Last(2 * height));
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
        let window = self.window.as_ref()?;
        window.rows.get(self.cursor.checked_sub(window.first)?)
    }

    /// Withdraws a `G` still walking, returning the request that replaces it
    /// — the rows where the viewport already is — or `None` if there was no
    /// jump to withdraw.
    pub fn withdraw_jump(&mut self, height: usize) -> Option<RowRequest> {
        self.jump.take()?;
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
        let covered = self.window.as_ref().is_some_and(|window| {
            let needed = if complete {
                bottom.min(self.rows)
            } else {
                bottom
            };
            window.first <= self.top && window.end() >= needed
        });
        let room_below = complete || self.rows > self.cursor + height;
        if covered && room_below && !force {
            return None;
        }
        let from = self.top.saturating_sub(height);
        Some(self.ask(Wanted::Rows(from..bottom + height)))
    }

    /// The next request, numbered after every earlier one.
    fn ask(&mut self, wanted: Wanted) -> RowRequest {
        self.asked += 1;
        RowRequest {
            id: self.asked,
            wanted,
            view: self.view.clone(),
        }
    }
}
