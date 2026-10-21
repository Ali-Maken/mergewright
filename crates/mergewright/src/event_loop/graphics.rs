//! The pixel tier's output: Kitty images or iTerm2 inline images placed in
//! the graph column, and the terminal's replies about them (tickets
//! m1-the-graph/24c and 25, ADR-0012).
//!
//! Every command goes out with `q=1`, so the terminal answers only failures.
//! Those replies reach the loop through the input mux like any other; each
//! wake drains them without waiting, and an `ENOENT` makes the next frame
//! transmit that strip again rather than trusting uploads survive.

use std::collections::HashMap;
use std::io;
use std::sync::Arc;
use std::time::Instant;

use mergewright_core::capabilities::{Capabilities, PixelProtocol, RenderingTier};
use mergewright_graph::raster::cache::{ByteBudget, CacheError, Flush, STRIP_ROWS, StripCache};
use mergewright_term::input::ReplyWaiter;
use mergewright_term::iterm2::ItermImages;
use mergewright_term::kitty::{self, KittyImages, UploadKey};
use mergewright_term::{Transport, sanitize_line};
use mergewright_ui::graph_screen::GraphicsStatus;
use ratatui::layout::Rect;

use crate::session::Session;
use crate::update::AppState;
use crate::view::strips::{PixelRows, StripPaint, TallRow, pixel_rows};

mod layers;
mod tall;

use layers::{Drawn, Held, layers, on_screen, places};
use tall::{Band, make_room};

/// iTerm2 bands kept: a tall viewport plus a strip either side
/// (`StripCache::capacity_for(32, 256, 64)` is 11).
const STRIPS_KEPT: usize = 16;

/// Decoded bytes the terminal may hold for us in images no frame places:
/// strips scrolled away and emphasis pieces the cursor left (ADR-0053
/// amendment 2026-10-10), so scrolling or moving back places them for
/// nothing. Kitty's own quota is 320 MB per buffer (`KittyImages::new`).
const UNPLACED_BYTES: usize = 64 << 20;

/// Pixel bytes the strip cache keeps across its layers (m8 04,
/// `docs/perf/scroll-pixel-memory-budget.txt`). A strip weighs lanes × cell,
/// and every cached entry holds a full-strip pool buffer (DESIGN §3.8): on
/// MONOREPO at the user's 17×37 cell an entry weighs ~5.6 MB. On Kitty only
/// backdrops are cached (emphasis pieces are sent once and dropped, ADR-0053
/// amendment 2026-10-10), so this keeps ~17 of them; on iTerm2 a strip also
/// caches its emphasis and composite. The viewport's strips stay whatever
/// they weigh (`keep_at_least`).
const STRIP_BUDGET: ByteBudget = ByteBudget(96 << 20);

/// The terminal-side slot of the pinned uncommitted row's image. Strips are
/// found by their first row; the pinned row's own strip is cached as row 0
/// (aligned, one row), so it needs a slot that is no strip's.
const PINNED_SLOT: usize = usize::MAX;

/// Rows kept above and below the viewport, on top of its own strips.
const BUFFER_ROWS: usize = STRIP_ROWS;

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
    /// Where each emphasis piece the terminal holds sits, by key: its first
    /// cell column and width in cells, so a held piece is placed without
    /// being rasterised. Kept to the keys `images` holds.
    pieces: HashMap<UploadKey, layers::Crop>,
    replies: R,
    status: GraphicsStatus,
}

impl<R: GraphicsReplies> Graphics<R> {
    /// Nothing uploaded yet; replies read from `replies`. Kitty uploads go
    /// zlib-deflated when `capabilities` says the terminal takes it
    /// (`kitty_deflate`, the probe the tier was settled from); an error
    /// reply about one turns that off again ([`KittyImages::answer`]).
    pub fn new(replies: R, capabilities: &Capabilities) -> Self {
        Graphics {
            images: KittyImages::new(UNPLACED_BYTES).deflating(capabilities.kitty_deflate),
            bands: ItermImages::new(STRIPS_KEPT),
            strips: StripCache::with_budget(
                STRIP_BUDGET,
                StripCache::capacity_for(STRIP_ROWS, 0, BUFFER_ROWS),
            ),
            pieces: HashMap::new(),
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
            // The tier in force, not the column's (`view::graph_tier`): an
            // open preview drawing the column in text only hides the strips,
            // so closing it places them again from the cache (ADR-0060 3).
            if crate::view::tier_in_force(session, state) == RenderingTier::Pixel {
                self.images.hide(out);
            } else {
                self.images.free_all(out);
            }
            self.bands.hide(out);
            return;
        };
        self.strips.keep_at_least(StripCache::capacity_for(
            STRIP_ROWS,
            usize::from(graph.height),
            BUFFER_ROWS,
        ));
        let pixels = pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS);
        let paint = StripPaint::for_frame(session, state);
        let iterm2 = session.capabilities.pixel_protocol == Some(PixelProtocol::Iterm2);
        let (drawn, band) = self.drawn(state, (&pixels, &paint), graph.height, iterm2);

        let mut places = places(&drawn, graph);
        places.extend(band.as_ref().map(|band| band.place(graph)));
        if iterm2 {
            if let Err(why) = self.bands.frame(&places, out) {
                self.status = GraphicsStatus::Failed(sanitize_line(&why.to_string()));
            }
        } else {
            self.images.frame(&places, out);
            let images = &self.images;
            self.pieces.retain(|key, _| images.holds(*key));
        }
    }

    /// Every strip on screen this frame, from the cache or rasterised, each
    /// placed in the part or parts the cursor row's second line leaves it
    /// (ease-pass/13), and that line's continuation band. A strip that
    /// fails is left out and the status says why; the rest still draw.
    fn drawn(
        &mut self,
        state: &AppState,
        (pixels, paint): (&PixelRows, &StripPaint),
        graph_height: u16,
        iterm2: bool,
    ) -> (Vec<Drawn>, Option<Band>) {
        let top = state.history.as_ref().map_or(0, |history| history.top);
        // The second line takes a line of the column (07b): strips show one
        // row fewer, and make room for it.
        let tall = TallRow::of(state);
        let height = graph_height.saturating_sub(u16::from(tall.is_some()));
        let mut drawn: Vec<Drawn> = Vec::new();
        let mut band: Option<Band> = None;
        // The pinned uncommitted row first: one row on the column's first
        // line, in a slot of its own (a strip's slot is its first row).
        let pinned = pixels
            .pinned
            .iter()
            .map(|strip| (strip, PINNED_SLOT, Some((0..1, 0))));
        let strips = pixels.strips.iter().map(|strip| {
            let lowered = pixels.uncommitted_on_top && strip.uncommitted_at().is_none();
            let shown = on_screen(strip.first_row, strip.row_count(), lowered, top, height);
            (strip, strip.first_row, shown)
        });
        for (strip, slot, shown) in pinned.chain(strips) {
            let Some((rows, at)) = shown else {
                continue;
            };
            let room = make_room(rows, at, tall);
            let mut held = Held {
                images: &self.images,
                pieces: &mut self.pieces,
            };
            let cache = &mut self.strips;
            let kitty = (!iterm2).then_some(held.images);
            let layers = strip.with_layers(pixels, paint, |plain, selection| {
                let layers = layers(cache, &mut held, strip.first_row, plain, selection, iterm2)?;
                let band = room
                    .band
                    .map(|(row, at)| Band::of(plain, selection, (slot, row, at), kitty))
                    .transpose()
                    .map_err(CacheError::Raster)?
                    .flatten();
                Ok::<_, CacheError>((layers, band))
            });
            match layers {
                Ok((((frame, key), emphasis), this_band)) => {
                    band = band.or(this_band);
                    drawn.extend(room.parts.into_iter().map(|(rows, at)| Drawn {
                        first_row: slot,
                        frame: Arc::clone(&frame),
                        key,
                        emphasis: emphasis.clone(),
                        rows,
                        at,
                    }));
                }
                Err(why) => self.status = GraphicsStatus::Failed(sanitize_line(&why.to_string())),
            }
        }
        (drawn, band)
    }

    /// Rasterise, without drawing or uploading, the first strip `state`
    /// would show that is not cached yet; then the next, until none is left
    /// or `waiting` says a wake has come. Asked before every strip, so work
    /// already begun when a wake arrives is one strip at most. Backdrops
    /// only: the emphasis follows the cursor, which a page key has not moved
    /// yet (ADR-0053). Returns the strips rasterised.
    pub fn prefetch(
        &mut self,
        session: &Session,
        state: &AppState,
        waiting: &mut impl FnMut() -> bool,
    ) -> usize {
        let pixels = pixel_rows(state, crate::view::max_lanes(state), STRIP_ROWS);
        let paint = StripPaint::for_frame(session, state);
        let mut rasterised = 0;
        for strip in &pixels.strips {
            let cache = &mut self.strips;
            if strip.with_layers(&pixels, &paint, |plain, selection| {
                cache.contains_backdrop(strip.first_row, plain, selection.backdrop)
            }) {
                continue;
            }
            if waiting() {
                break;
            }
            match strip.with_layers(&pixels, &paint, |plain, selection| {
                cache.backdrop(strip.first_row, plain, selection.backdrop)
            }) {
                Ok(_frame) => rasterised += 1,
                // The same failure `frame` shows when this strip comes on
                // screen; shown now too, so it is never lost.
                Err(why) => self.status = GraphicsStatus::Failed(sanitize_line(&why.to_string())),
            }
        }
        rasterised
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
    pub fn strip_stats(&self) -> mergewright_graph::raster::cache::CacheStats {
        self.strips.stats()
    }

    /// The same, with the strip cache held to `budget`, for tests.
    #[cfg(test)]
    #[must_use]
    pub fn with_strip_budget(mut self, budget: ByteBudget) -> Self {
        self.strips = StripCache::with_budget(budget, self.strips.min_strips());
        self
    }

    /// The strip cache's bytes and whether the viewport's floor held it
    /// over budget, for tests.
    #[cfg(test)]
    pub fn strip_budget_report(&self) -> (usize, bool) {
        (self.strips.resident_bytes(), self.strips.minimum_wins())
    }

    /// The viewport floor the last frame set, per layer, for tests.
    #[cfg(test)]
    pub fn strip_floor(&self) -> usize {
        self.strips.min_strips()
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
