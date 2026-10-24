//! Where the strips land in the graph column this frame: decided once, here,
//! so `Graphics::drawn` and the Ghostty capture cannot drift.

use super::layers::on_screen;
use super::tall::{Room, make_room};
use crate::update::AppState;
use crate::view::strips::{PixelRows, StripRows, TallRow};

/// The terminal-side slot of the pinned uncommitted row's image. Strips are
/// found by their first row; the pinned row's own strip is cached as row 0
/// (aligned, one row), so it needs a slot that is no strip's.
pub(super) const PINNED_SLOT: usize = usize::MAX;

/// Where one strip's rows land in the graph column this frame: the strip
/// (its index in [`PixelRows::strips`]) and its [`Room`] (the parts, as strip
/// rows and the graph line each starts on, and the cursor row's second line).
/// The shape of the frame is decided here, once: `Graphics::drawn` places
/// from it and the Ghostty capture (`tests/it/ghostty_shot.rs`) stacks from
/// it, so the two cannot drift.
pub struct Placed {
    /// Index into [`PixelRows::strips`]; `None` for the pinned uncommitted
    /// row's own image ([`PixelRows::pinned`]), see [`Placed::strip_of`].
    pub strip: Option<usize>,
    /// The terminal-side slot the image is kept in: the strip's first row,
    /// or `usize::MAX` for the pinned row (no strip's).
    pub slot: usize,
    /// Where its rows go.
    pub room: Room,
}

impl Placed {
    /// The strip this places, out of `pixels`.
    pub fn strip_of<'a>(&self, pixels: &'a PixelRows) -> Option<&'a StripRows> {
        match self.strip {
            Some(index) => pixels.strips.get(index),
            None => pixels.pinned.as_ref(),
        }
    }
}

/// Every strip on screen in a `graph_height`-line column, placed around the
/// cursor row's second line (07b): strips show one line fewer and make room.
pub fn placements(state: &AppState, pixels: &PixelRows, graph_height: u16) -> Vec<Placed> {
    let top = state.history.as_ref().map_or(0, |history| history.top);
    let tall = TallRow::of(state);
    let height = graph_height.saturating_sub(u16::from(tall.is_some()));
    // The pinned uncommitted row first: one row on the column's first line.
    let pinned = pixels.pinned.iter().map(|_| Placed {
        strip: None,
        slot: PINNED_SLOT,
        room: make_room(0..1, 0, tall),
    });
    let strips = pixels
        .strips
        .iter()
        .enumerate()
        .filter_map(|(strip, rows)| {
            let lowered = pixels.uncommitted_on_top && rows.uncommitted_at().is_none();
            let (shown, at) = on_screen(rows.first_row, rows.row_count(), lowered, top, height)?;
            Some(Placed {
                strip: Some(strip),
                slot: rows.first_row,
                room: make_room(shown, at, tall),
            })
        });
    pinned.chain(strips).collect()
}
