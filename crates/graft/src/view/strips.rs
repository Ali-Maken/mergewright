//! The pixel tier's rows: the history the viewport shows, cut into whole
//! aligned strips and painted the way the Unicode tier paints its rows.
//!
//! [`History`] pads its requests to strip boundaries when `strip_rows` is set
//! (`history/scroll.rs`), so every strip here is a real row range and its
//! cache key is honest. The uncommitted row, the `+N` fold, the selection's
//! thread and the cursor all reach [`Strip`] from the same state the text
//! tiers read. Drawing, placement and replies are not here.

use std::collections::BTreeSet;
use std::ops::Range;

use graft_core::capabilities::Rgb;
use graft_graph::raster::{Emphasis, Marker, RasterRow, Ring, Strip};
use graft_graph::row::Row;
use graft_graph::{Fold, Kin};

use super::{Thread, head_is_topmost, thread, uncommitted_row};
use crate::history::History;
use crate::refs::RefBadges;
use crate::session::Session;
use crate::update::AppState;

/// What every strip this session draws shares: cell size and colours.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripPaint {
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Lane hues (`Theme::raster_lanes`).
    pub palette: Vec<Rgb>,
    /// The `+N` marker's hue when no lane joins it.
    pub muted: Rgb,
}

impl StripPaint {
    /// The session's measured cell and its theme's truecolor hues.
    pub fn of(session: &Session) -> StripPaint {
        StripPaint {
            cell_width: session.rendering.cell_size.width,
            cell_height: session.rendering.cell_size.height,
            palette: session.theme.raster_lanes(),
            muted: session.theme.raster_muted(),
        }
    }
}

/// One frame's strips: those covering the viewport whose rows are on hand,
/// all drawn through one fold so lanes line up across strip boundaries.
#[derive(Clone, Debug, Default)]
pub struct PixelRows {
    /// Strips top to bottom.
    pub strips: Vec<StripRows>,
    /// Lane columns every strip is wide, marker included.
    pub lane_columns: usize,
    /// The `+N` column, if anything folds.
    pub marker: Option<Marker>,
}

impl PixelRows {
    /// `strips` with the width and `+N` marker `fold` draws them at.
    fn drawn_through(strips: Vec<StripRows>, fold: &Fold) -> PixelRows {
        PixelRows {
            strips,
            lane_columns: fold.drawn_columns(),
            marker: fold.marker().map(|column| Marker {
                column,
                folded: fold.folded().len(),
            }),
        }
    }
}

/// One aligned strip's rows, in drawn columns, painted.
#[derive(Clone, Debug)]
pub struct StripRows {
    /// The history row the strip starts on: a multiple of the strip height.
    pub first_row: usize,
    rows: Vec<PaintedRow>,
    above: Option<Row>,
}

#[derive(Clone, Debug)]
struct PaintedRow {
    row: Row,
    ring: Ring,
    emphasis: Emphasis,
    thread: Option<BTreeSet<usize>>,
}

impl StripRows {
    /// Rows in the strip, the uncommitted row included (never zero).
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Where in the strip the uncommitted row sits, if it is in this one.
    pub fn uncommitted_at(&self) -> Option<usize> {
        self.rows
            .iter()
            .position(|painted| painted.ring == Ring::Uncommitted)
    }

    /// Hands `draw` the [`Strip`] these rows make — for `StripCache::get`.
    pub fn with_strip<R>(
        &self,
        frame: &PixelRows,
        paint: &StripPaint,
        draw: impl FnOnce(&Strip<'_>) -> R,
    ) -> R {
        let rows: Vec<RasterRow<'_>> = self
            .rows
            .iter()
            .map(|painted| RasterRow {
                row: &painted.row,
                ring: painted.ring,
                emphasis: painted.emphasis,
                thread: painted.thread.as_ref(),
            })
            .collect();
        draw(&Strip {
            rows: &rows,
            above: self.above.as_ref(),
            lane_columns: frame.lane_columns,
            marker: frame.marker,
            cell_width: paint.cell_width,
            cell_height: paint.cell_height,
            palette: &paint.palette,
            muted: paint.muted,
        })
    }
}

/// The strips the graph viewport needs this frame, `strip_rows` rows each.
///
/// A strip is left out while its rows, or the row above it, have not arrived:
/// rasterising part of one would draw the wrong picture. The padded request
/// (`History::strip_rows`) makes that a moment between a scroll and its
/// answer. The last strip of a history may be short; so may one whose rows
/// past the window have not been laid out yet — its key says how many rows.
pub fn pixel_rows(state: &AppState, max_lanes: usize, strip_rows: usize) -> PixelRows {
    let strip_rows = strip_rows.max(1);
    let Some(history) = &state.history else {
        return PixelRows::default();
    };
    let window_rows: &[Row] = history.window.as_ref().map_or(&[], |window| &window.rows);
    let first = history.window.as_ref().map_or(0, |window| window.first);
    let on_hand = first..first + window_rows.len();

    // The same facts, asked the same way, as `visible_rows`.
    let top_row = history.visible(1).first();
    let dirty = state.working_tree.is_some_and(|counts| !counts.is_clean());
    let uncommitted = dirty && head_is_topmost(state.refs.as_ref(), top_row);
    let height = state.graph_rows().saturating_sub(usize::from(uncommitted));

    let ranges = strip_ranges(history.top..history.top + height, &on_hand, strip_rows);
    let virtual_row = uncommitted.then(|| uncommitted_row(top_row));
    if ranges.is_empty() && virtual_row.is_none() {
        return PixelRows::default();
    }

    let row_at = |index: usize| &window_rows[index - first];
    let thread = thread(history);
    let fold = plan(
        state,
        history,
        thread.as_ref(),
        &ranges,
        virtual_row.as_ref(),
        row_at,
        max_lanes,
    );
    let painter = Painter {
        state,
        history,
        thread: thread.as_ref(),
        fold: &fold,
    };

    let mut strips: Vec<StripRows> = ranges
        .iter()
        .map(|range| StripRows {
            first_row: range.start,
            rows: range
                .clone()
                .map(|index| painter.row(index, row_at(index)))
                .collect(),
            above: range
                .start
                .checked_sub(1)
                .map(|above| fold.view(row_at(above))),
        })
        .collect();
    if let Some(virtual_row) = &virtual_row {
        let on_top = PaintedRow {
            row: fold.view(virtual_row),
            ring: Ring::Uncommitted,
            emphasis: Emphasis::Normal,
            thread: None,
        };
        put_uncommitted(&mut strips, on_top, history.top, window_rows.is_empty());
    }

    PixelRows::drawn_through(strips, &fold)
}

/// The aligned strips covering `viewport` whose rows and row above are
/// `on_hand`, each clipped to the rows there.
fn strip_ranges(
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
fn aligned(viewport: Range<usize>, strip_rows: usize) -> impl Iterator<Item = Range<usize>> {
    let first = viewport.start / strip_rows;
    let end = viewport.end.max(viewport.start + 1).div_ceil(strip_rows);
    (first..end).map(move |index| index * strip_rows..(index + 1) * strip_rows)
}

/// One fold over every strip drawn, so a lane keeps its column across strip
/// boundaries. Pinned as the text tiers pin: the integration lanes and the
/// selection's thread over the rows drawn.
fn plan<'r>(
    state: &AppState,
    history: &History,
    thread: Option<&Thread>,
    ranges: &[Range<usize>],
    virtual_row: Option<&'r Row>,
    row_at: impl Fn(usize) -> &'r Row,
    max_lanes: usize,
) -> Fold {
    let integration = history
        .walk
        .as_ref()
        .map_or(0, |walk| walk.integration.len());
    let rows = || ranges.iter().flat_map(Range::clone);
    let pinned: BTreeSet<usize> = (1..=integration)
        .chain(
            thread
                .into_iter()
                .flat_map(|thread| rows().flat_map(move |index| thread.lit(index).iter().copied())),
        )
        .collect();
    let max_lanes = if state.lanes_expanded {
        usize::MAX
    } else {
        max_lanes
    };
    Fold::plan(
        virtual_row.into_iter().chain(rows().map(&row_at)),
        max_lanes,
        &pinned,
    )
}

/// Paints a history row the way `visible_rows` does for text.
struct Painter<'a> {
    state: &'a AppState,
    history: &'a History,
    thread: Option<&'a Thread>,
    fold: &'a Fold,
}

impl Painter<'_> {
    fn row(&self, index: usize, row: &Row) -> PaintedRow {
        let refs = self.state.refs.as_ref();
        // HEAD wins over the stash, as in `row_entry`.
        let ring = if refs.and_then(RefBadges::head) == Some(row.commit.id) {
            Ring::Head
        } else if refs.and_then(RefBadges::stash) == Some(row.commit.id) {
            Ring::Stash
        } else {
            Ring::of(row)
        };
        let mut emphasis = if index == self.history.cursor {
            Emphasis::Highlighted
        } else {
            Emphasis::Normal
        };
        let thread = self.thread.map(|thread| {
            if self.state.focus && thread.kin(index) == Kin::Unrelated {
                emphasis = Emphasis::Dimmed;
            }
            // The thread is in real columns; the rasteriser wants drawn ones.
            thread
                .lit(index)
                .iter()
                .filter_map(|&column| self.fold.drawn(column))
                .collect()
        });
        PaintedRow {
            row: self.fold.view(row),
            ring,
            emphasis,
            thread,
        }
    }
}

#[cfg(test)]
mod tests;
