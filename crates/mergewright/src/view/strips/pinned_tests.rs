//! The pinned uncommitted row is an overlay on the viewport, not part of any
//! strip (ease-pass/20): no strip's cache key may depend on where the
//! viewport is.

use std::sync::Arc;

use mergewright_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache};

use super::tests::{MAX_LANES, scrolled, state};
use super::{PixelRows, StripPaint, pixel_rows};
use crate::session::for_tests::session;
use crate::update::AppState;
use crate::view::tests::{counts, history_of, oid, refs_with_head};

/// A linear history of 120 commits scrolled to `top`, a dirty tree, `HEAD`
/// (row 0) scrolled off: the uncommitted row is pinned and detached.
fn detached(top: usize) -> AppState {
    let mut state = state(scrolled(history_of(120, 0), top), 20);
    state.refs = Some(Arc::new(refs_with_head(Some(oid(120)))));
    state.working_tree = Some(counts(1, 0, 0));
    assert!(state.uncommitted_row_pinned() && !state.uncommitted_row_attached());
    state
}

/// Every strip's backdrop cache key, by the history row it starts on.
fn keys(rows: &PixelRows) -> Vec<(usize, u64)> {
    let paint = StripPaint::of(&session());
    let cache = StripCache::with_budget(ByteBudget(usize::MAX), 8);
    rows.strips
        .iter()
        .map(|strip| {
            let key = strip
                .with_layers(rows, &paint, |plain, selection| {
                    cache.backdrop_key(strip.first_row, plain, selection.backdrop)
                })
                .expect("an aligned strip has a key");
            (strip.first_row, key)
        })
        .collect()
}

#[test]
fn a_pinned_uncommitted_row_leaves_strip_keys_alone() {
    let at = pixel_rows(&detached(40), MAX_LANES, STRIP_ROWS);
    let next = pixel_rows(&detached(41), MAX_LANES, STRIP_ROWS);

    let (at, next) = (keys(&at), keys(&next));
    let shared: Vec<_> = at.iter().filter(|(first, _)| next.iter().any(|n| n.0 == *first)).collect();
    assert!(!shared.is_empty(), "the two frames share a strip");
    for (first, key) in shared {
        let other = next.iter().find(|n| n.0 == *first).map(|n| n.1);
        assert_eq!(other, Some(*key), "strip {first} re-keyed when the viewport moved one row");
    }
}

/// The pinned row's band, backdrop layer: its own one-row image.
fn pinned_band(rows: &PixelRows) -> Vec<u8> {
    let paint = StripPaint::of(&session());
    let mut cache = StripCache::with_budget(ByteBudget(usize::MAX), 8);
    let strip = rows.pinned.as_ref().expect("a pinned image");
    assert_eq!(strip.row_count(), 1);
    assert!(rows.strips.iter().all(|s| s.uncommitted_at().is_none()));
    let frame = strip
        .with_layers(rows, &paint, |plain, selection| {
            cache.backdrop(strip.first_row, plain, selection.backdrop)
        })
        .expect("raster");
    assert_eq!(frame.rows, 1);
    frame.pixels.clone()
}

/// The band of history row `row` (its backdrop layer), wherever its strip
/// holds it: past the uncommitted row when that sits earlier in the strip.
fn history_band(rows: &PixelRows, row: usize) -> Vec<u8> {
    let paint = StripPaint::of(&session());
    let mut cache = StripCache::with_budget(ByteBudget(usize::MAX), 8);
    let strip = rows
        .strips
        .iter()
        .find(|s| (s.first_row..s.first_row + s.row_count()).contains(&row))
        .expect("strip");
    let mut at = row - strip.first_row;
    if strip.uncommitted_at().is_some_and(|k| k <= at) {
        at += 1;
    }
    let frame = strip
        .with_layers(rows, &paint, |plain, selection| {
            cache.backdrop(strip.first_row, plain, selection.backdrop)
        })
        .expect("raster");
    let band = frame.width as usize * 4 * paint.cell_height as usize;
    frame.pixels[at * band..(at + 1) * band].to_vec()
}

/// Golden bytes: the pinned row's band as HEAD 3123840e rasterised it inside
/// its strip, with no row above it (strip 0, `top` 0, `HEAD` on row 1), taken
/// before the row became its own image.
const GOLDEN: &[u8] = include_bytes!("pinned_row.golden");

/// The pinned row alone in its image equals the virtual row as it was
/// rasterised with no history row above it. (With a history row above, HEAD
/// drew that row's lane into the pinned row's top edge: pixel rows 0-2 of the
/// node's column. A one-row image has no row above, so it never does; that is
/// the ruling's point, the key no longer depends on `top`.) It also does not
/// depend on `top`.
#[test]
fn the_pinned_row_image_matches_the_virtual_row() {
    let mut at_top = state(scrolled(history_of(120, 0), 0), 20);
    at_top.refs = Some(Arc::new(refs_with_head(Some(oid(119)))));
    at_top.working_tree = Some(counts(1, 0, 0));
    assert!(at_top.uncommitted_row_pinned() && !at_top.uncommitted_row_attached());
    let bands = [
        pinned_band(&pixel_rows(&at_top, MAX_LANES, STRIP_ROWS)),
        pinned_band(&pixel_rows(&detached(40), MAX_LANES, STRIP_ROWS)),
    ];
    for (band, top) in bands.iter().zip([0, 40]) {
        assert!(band == GOLDEN, "top {top}: the pinned row's pixels changed");
    }
}

#[test]
fn under_the_pinned_row_the_top_history_row_is_its_real_self() {
    let top = 40;
    let pinned = pixel_rows(&detached(top), MAX_LANES, STRIP_ROWS);
    let mut clean = detached(top);
    clean.working_tree = Some(counts(0, 0, 0));
    let clean = pixel_rows(&clean, MAX_LANES, STRIP_ROWS);
    assert!(
        history_band(&pinned, top) == history_band(&clean, top),
        "the row under the pinned row is drawn from the pinned row, not from the row above it"
    );
}
