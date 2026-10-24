//! Which strips cover the viewport, and where the uncommitted row sits in
//! them (split from `strips.rs` to keep it under its size cap).

use std::ops::Range;

use super::{PaintedRow, StripRows};
use crate::history::History;
use crate::update::AppState;

/// Whether the cursor sits in the viewport and its row has not arrived yet.
///
/// Such a row is on its way. Until it comes there is no thread, and strips
/// drawn without one would be rasterised and sent unlit, then again lit once
/// the window arrives: a page key's frame and its answer each sent every
/// strip (30b). The wheel moves the viewport and leaves the cursor behind,
/// and a cursor off screen is never on its way: waiting for it blanked the
/// graph until a key brought the cursor back (37).
pub(super) fn cursor_row_on_its_way(history: &History, height: usize) -> bool {
    history.selected().is_none() && (history.top..history.top + height).contains(&history.cursor)
}

/// The aligned strips covering `viewport` whose rows and row above are
/// `on_hand`, each clipped to the rows there.
pub(super) fn strip_ranges(
    viewport: Range<usize>,
    on_hand: &Range<usize>,
    strip_rows: usize,
) -> Vec<Range<usize>> {
    aligned(viewport, strip_rows)
        .filter_map(|range| {
            let above_on_hand = range.start == 0 || on_hand.contains(&(range.start - 1));
            let end = range.end.min(on_hand.end);
            (on_hand.contains(&range.start) && above_on_hand && range.start < end)
                .then_some(range.start..end)
        })
        .collect()
}

/// Puts the uncommitted row on top of `HEAD`'s row `top`, in whichever strip
/// holds it. An unborn branch has no rows, so there the virtual row makes a
/// strip of its own.
fn put_uncommitted(strips: &mut Vec<StripRows>, on_top: PaintedRow, top: usize, no_history: bool) {
    let holding = strips
        .iter()
        .position(|strip| (strip.first_row..strip.first_row + strip.rows.len()).contains(&top));
    match holding {
        Some(at) => {
            let strip = &mut strips[at];
            strip.rows.insert(top - strip.first_row, on_top);
        }
        None if strips.is_empty() && no_history => strips.push(StripRows {
            first_row: 0,
            rows: vec![on_top],
            above: None,
        }),
        // `HEAD`'s strip is still waiting on its rows; it draws once they
        // come, with the virtual row on top. Nothing is lost by waiting.
        None => {}
    }
}

/// The aligned strips covering `viewport`, top to bottom (`StripCache::strips_for`).
pub(super) fn aligned(
    viewport: Range<usize>,
    strip_rows: usize,
) -> impl Iterator<Item = Range<usize>> {
    let first = viewport.start / strip_rows;
    let end = viewport.end.max(viewport.start + 1).div_ceil(strip_rows);
    (first..end).map(move |index| index * strip_rows..(index + 1) * strip_rows)
}

/// Where the uncommitted row draws: on `HEAD` it hangs off its lane, so it is
/// part of `HEAD`'s strip (that strip's index for it does not move with the
/// viewport). Off `HEAD` it is an overlay on the pane's first line, and comes
/// back as an image of its own, so scrolling re-keys no strip.
pub(super) fn place_uncommitted(
    state: &AppState,
    strips: &mut Vec<StripRows>,
    on_top: PaintedRow,
    top: usize,
    no_history: bool,
) -> Option<StripRows> {
    if state.uncommitted_row_attached() {
        put_uncommitted(strips, on_top, top, no_history);
        return None;
    }
    Some(StripRows {
        first_row: 0,
        rows: vec![on_top],
        above: None,
    })
}
