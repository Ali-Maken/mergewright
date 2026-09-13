//! The pixel tier's output: Kitty images or iTerm2 inline images placed in
//! the graph column, and the terminal's replies about them (tickets
//! m1-the-graph/24c and 25, ADR-0012).
//!
//! Every command goes out with `q=1`, so the terminal answers only failures.
//! Those replies reach the loop through the input mux like any other; each
//! wake drains them without waiting, and an `ENOENT` makes the next frame
//! transmit that strip again rather than trusting uploads survive.

use std::io;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use graft_core::capabilities::{PixelProtocol, RenderingTier};
use graft_core::raster::RasterFrame;
use graft_graph::raster::cache::{Flush, STRIP_ROWS, StripCache};
use graft_term::input::ReplyWaiter;
use graft_term::iterm2::ItermImages;
use graft_term::kitty::{self, CellRect, KittyImages, Place};
use graft_term::{Transport, sanitize_line};
use graft_ui::graph_screen::GraphicsStatus;
use ratatui::layout::Rect;

use crate::session::Session;
use crate::update::AppState;
use crate::view::strips::{StripPaint, pixel_rows};

/// Strips kept rasterised, and images kept uploaded: a tall viewport plus a
/// strip either side (`StripCache::capacity_for(32, 256, 64)` is 11).
const STRIPS_KEPT: usize = 16;

/// Where the terminal's replies about images come from.
pub trait GraphicsReplies {
    /// Every reply byte that has arrived; never waits for more.
    fn take(&mut self) -> io::Result<Vec<u8>>;
}

impl GraphicsReplies for ReplyWaiter {
    fn take(&mut self) -> io::Result<Vec<u8>> {
        let mut bytes = Vec::new();
        let mut chunk = [0u8; 4096];
        loop {
            // A deadline already passed: what has arrived, and nothing more.
            let count = self.read(&mut chunk, Instant::now())?;
            if count == 0 {
                return Ok(bytes);
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
    }
}

/// No terminal to hear from: a loop driven without a tty.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoReplies;

impl GraphicsReplies for NoReplies {
    fn take(&mut self) -> io::Result<Vec<u8>> {
        Ok(Vec::new())
    }
}

/// The images on screen, the strips behind them, and what the terminal last
/// said about them.
#[derive(Debug)]
pub struct Graphics<R> {
    images: KittyImages,
    bands: ItermImages,
    strips: StripCache,
    replies: R,
    status: GraphicsStatus,
}

impl<R: GraphicsReplies> Graphics<R> {
    /// Nothing uploaded yet; replies read from `replies`.
    pub fn new(replies: R) -> Self {
        Graphics {
            images: KittyImages::new(STRIPS_KEPT),
            bands: ItermImages::new(STRIPS_KEPT),
            strips: StripCache::new(STRIP_ROWS, STRIPS_KEPT),
            replies,
            status: GraphicsStatus::Ok,
        }
    }

    /// Send image commands over `transport` (tmux's passthrough inside tmux,
    /// DESIGN.md §4.3).
    #[must_use]
    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.images = self.images.with_transport(transport);
        self.bands = self.bands.with_transport(transport);
        self
    }

    /// What the header says while pixels are drawn.
    pub fn status(&self) -> &GraphicsStatus {
        &self.status
    }

    /// Take in every reply that has arrived since the last wake. The status
    /// is the last error among them, or `gfx ok` when none failed. `true`
    /// when the status changed, so the screen should say so.
    pub fn drain(&mut self) -> io::Result<bool> {
        let bytes = self.replies.take()?;
        let mut status = GraphicsStatus::Ok;
        for reply in kitty::replies(&bytes) {
            self.images.answer(&reply);
            if reply.is_error() {
                status = GraphicsStatus::Failed(sanitize_line(&reply.message));
            }
        }
        let changed = status != self.status;
        self.status = status;
        Ok(changed)
    }

    /// Append the commands that put this frame's strips into `graph`, the
    /// graph column; `None` (no pixels this frame) removes every placement.
    /// Off the pixel tier (`F3`) it frees every upload too: the terminal may
    /// evict unplaced images while the text tiers draw, so coming back to
    /// pixels transmits afresh instead of placing ids it may no longer hold
    /// (m1-the-graph/33). The status screen only hides them.
    pub fn frame(
        &mut self,
        session: &Session,
        state: &AppState,
        graph: Option<Rect>,
        out: &mut Vec<u8>,
    ) {
        let Some(graph) = graph.filter(|area| !area.is_empty()) else {
            if crate::view::tier_in_force(session, state) == RenderingTier::Pixel {
                self.images.hide(out);
            } else {
                self.images.free_all(out);
            }
            self.bands.hide(out);
            return;
        };
        let top = state.history.as_ref().map_or(0, |history| history.top);
        let pixels = pixel_rows(state, session.max_lanes, STRIP_ROWS);
        let paint = StripPaint::of(session);

        let mut drawn: Vec<(usize, Arc<RasterFrame>, Range<usize>, u16)> = Vec::new();
        for strip in &pixels.strips {
            let lowered = pixels.uncommitted_on_top && strip.uncommitted_at().is_none();
            let Some((rows, at)) = on_screen(
                strip.first_row,
                strip.row_count(),
                lowered,
                top,
                graph.height,
            ) else {
                continue;
            };
            let cache = &mut self.strips;
            match strip.with_strip(&pixels, &paint, |painted| {
                cache.get(strip.first_row, painted)
            }) {
                Ok(frame) => drawn.push((strip.first_row, frame, rows, at)),
                // Shown, and the strip left out: the rest still draw.
                Err(why) => self.status = GraphicsStatus::Failed(sanitize_line(&why.to_string())),
            }
        }

        let places: Vec<Place<'_>> = drawn
            .iter()
            .map(|(first_row, frame, rows, at)| Place {
                first_row: *first_row,
                frame,
                rows: rows.clone(),
                cells: CellRect {
                    column: graph.x,
                    row: graph.y + at,
                    columns: placed_columns(graph.width, frame.columns),
                    rows: u16::try_from(rows.len()).unwrap_or(u16::MAX),
                },
            })
            .collect();
        if session.capabilities.pixel_protocol == Some(PixelProtocol::Iterm2) {
            if let Err(why) = self.bands.frame(&places, out) {
                self.status = GraphicsStatus::Failed(sanitize_line(&why.to_string()));
            }
        } else {
            self.images.frame(&places, out);
        }
    }

    /// Throw away every rasterised strip: the theme or the cell size
    /// changed, so every strip's pixels did.
    ///
    /// A resize is not a reason. A strip's pixels depend on its rows, lane
    /// columns (in the layout hash), theme and cell size — not the window —
    /// so a resize re-places what it can and rasterises only strips whose key
    /// changed. Uploads survive it too: the loop's backend clears row by row
    /// (`row_clear`), never with the `ESC[2J` Ghostty frees image data on.
    pub fn flush(&mut self, why: Flush) {
        self.strips.flush(why);
    }

    /// The strip cache's counters, for tests.
    #[cfg(test)]
    pub fn strip_stats(&self) -> graft_graph::raster::cache::CacheStats {
        self.strips.stats()
    }

    /// PNG encodes on the iTerm2 tier, for tests.
    #[cfg(test)]
    pub fn bands_encoded(&self) -> u64 {
        self.bands.encoded()
    }

    /// Append the commands deleting every upload. Call once, leaving.
    pub fn release(&mut self, out: &mut Vec<u8>) {
        self.images.free_all(out);
    }
}

/// Cells a strip is placed across: its own lane columns, never more than the
/// graph column has. Placing a narrower strip across the whole column would
/// make the terminal stretch it sideways.
fn placed_columns(graph_width: u16, frame_columns: u32) -> u16 {
    u16::try_from(frame_columns).map_or(graph_width, |columns| columns.min(graph_width))
}

/// The strip rows that land in a `height`-row graph column whose first row
/// shows history row `top`, and the column row the first of them lands on.
/// `lowered`: the uncommitted row sits above this strip's rows on screen.
fn on_screen(
    first_row: usize,
    row_count: usize,
    lowered: bool,
    top: usize,
    height: u16,
) -> Option<(Range<usize>, u16)> {
    // Strip row `j` is drawn on column row `start + j - top`.
    let start = first_row + usize::from(lowered);
    let first = top.saturating_sub(start);
    let end = (top + usize::from(height))
        .saturating_sub(start)
        .min(row_count);
    let at = u16::try_from(start + first - top).ok()?;
    (first < end).then_some((first..end, at))
}

#[cfg(test)]
mod tests {
    use super::{on_screen, placed_columns};

    #[test]
    fn a_strip_is_placed_across_its_own_columns_not_stretched() {
        assert_eq!(placed_columns(20, 6), 6);
        assert_eq!(placed_columns(20, 20), 20);
        // Wider than the column: clamped (and squeezed; KNOWN_GAPS).
        assert_eq!(placed_columns(20, 30), 20);
    }

    #[test]
    fn a_strip_starting_above_the_viewport_is_cropped_to_it() {
        // Rows 50..70 of the strip 32..64: strip rows 18..32 on rows 0..14.
        assert_eq!(on_screen(32, 32, false, 50, 20), Some((18..32, 0)));
        // The next strip fills the rest: rows 64..70 on rows 14..20.
        assert_eq!(on_screen(64, 32, false, 50, 20), Some((0..6, 14)));
    }

    #[test]
    fn the_uncommitted_row_takes_the_first_screen_row() {
        // Top 40, uncommitted row inserted at strip index 8 of 32..64 (33 rows).
        assert_eq!(on_screen(32, 33, false, 40, 20), Some((8..28, 0)));
        // A later strip sits one row lower than its history rows say.
        assert_eq!(on_screen(64, 32, true, 50, 20), Some((0..5, 15)));
    }

    #[test]
    fn a_strip_off_screen_places_nothing() {
        assert_eq!(on_screen(96, 32, false, 50, 20), None);
        assert_eq!(on_screen(0, 32, false, 50, 20), None);
    }
}
