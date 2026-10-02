//! The pixel tier's rows: the history the viewport shows, cut into whole
//! aligned strips and painted the way the Unicode tier paints its rows.
//!
//! [`History`] pads its requests to strip boundaries when `strip_rows` is set
//! (`history/scroll.rs`), so every strip here is a real row range and its
//! cache key is honest. The uncommitted row, the `+N` fold, the selection's
//! thread and the cursor all come from the same state the text tiers read.
//! Each strip hands out two views (ADR-0053): its layout ([`PlainStrip`]) and
//! the selection's paint over it ([`Selection`]), at the frame's one
//! [`Backdrop`]. Drawing, placement and replies are not here.

use std::collections::BTreeSet;
use std::ops::Range;

use graft_core::capabilities::Rgb;
use graft_graph::fold::DrawnRow;
use graft_graph::raster::{
    Backdrop, Emphasis, Lit, Marker, PlainRow, PlainStrip, Ring, Selection, Stub,
};
use graft_graph::row::Row;
use graft_graph::{Fold, Kin};

use super::entries::{Thread, thread, uncommitted_row};
use super::rows::keep_for;
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
    /// The rail's hue.
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
    /// The uncommitted row is on screen, above the viewport's first history
    /// row: every history row sits one screen row lower.
    pub uncommitted_on_top: bool,
    /// The lowest alpha any row of any strip draws at
    /// ([`Backdrop::lowest`]): every strip this frame is drawn at it, so a
    /// cursor move that keeps it leaves every strip as it was.
    pub backdrop: Backdrop,
}

impl PixelRows {
    /// `strips` with the width and rail `fold` draws them at.
    fn drawn_through(strips: Vec<StripRows>, fold: &Fold, uncommitted_on_top: bool) -> PixelRows {
        let backdrop = Backdrop::lowest(
            strips
                .iter()
                .flat_map(|strip| strip.rows.iter().map(PaintedRow::lit)),
        );
        PixelRows {
            strips,
            uncommitted_on_top,
            backdrop,
            lane_columns: fold.drawn_columns(),
            marker: fold.marker().map(|column| Marker { column }),
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
    /// Edges the fold took out of `row`, for the rasteriser to draw as stubs.
    stubs: Vec<Stub>,
}

impl PaintedRow {
    /// The uncommitted-changes row, drawn through `fold`, never on a thread.
    fn uncommitted(fold: &Fold, row: &Row) -> PaintedRow {
        let drawn = fold.view(row);
        PaintedRow {
            stubs: stubs_of(&drawn),
            row: drawn.row.into_owned(),
            ring: Ring::Uncommitted,
            emphasis: Emphasis::Normal,
            thread: None,
        }
    }

    /// The row as the layout draws it.
    fn plain(&self) -> PlainRow<'_> {
        PlainRow {
            row: &self.row,
            ring: self.ring,
            stubs: self.stubs.clone(),
        }
    }

    /// The selection's paint on the row.
    fn lit(&self) -> Lit<'_> {
        Lit {
            emphasis: self.emphasis,
            thread: self.thread.as_ref(),
        }
    }
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

    /// Hands `draw` the strip's layout and the selection's paint over it,
    /// at `frame`'s backdrop: for `StripCache::{backdrop, emphasis,
    /// composited}`. The views borrow these rows; nothing is kept.
    pub fn with_layers<R>(
        &self,
        frame: &PixelRows,
        paint: &StripPaint,
        draw: impl FnOnce(&PlainStrip<'_>, &Selection<'_>) -> R,
    ) -> R {
        let plain: Vec<PlainRow<'_>> = self.rows.iter().map(PaintedRow::plain).collect();
        let lit: Vec<Lit<'_>> = self.rows.iter().map(PaintedRow::lit).collect();
        draw(
            &PlainStrip {
                rows: &plain,
                above: self.above.as_ref(),
                lane_columns: frame.lane_columns,
                marker: frame.marker,
                cell_width: paint.cell_width,
                cell_height: paint.cell_height,
                palette: &paint.palette,
                muted: paint.muted,
            },
            &Selection {
                rows: &lit,
                backdrop: frame.backdrop,
            },
        )
    }
}

/// The strips the graph viewport needs this frame, `strip_rows` rows each.
///
/// A strip is left out while its rows, or the row above it, have not arrived:
/// rasterising part of one would draw the wrong picture. The padded request
/// (`History::strip_rows`) makes that a moment between a scroll and its
/// answer. The last strip of a history may be short; so may one whose rows
/// past the window have not been laid out yet — its key says how many rows.
/// Every strip waits while the cursor's row is in the viewport but has not
/// arrived: without it there is no thread, and an unlit strip would be sent
/// twice. A cursor the wheel left off screen is not on its way, so strips
/// draw unlit instead of waiting for it (37).
pub fn pixel_rows(state: &AppState, max_lanes: usize, strip_rows: usize) -> PixelRows {
    let strip_rows = strip_rows.max(1);
    let Some(history) = &state.history else {
        return PixelRows::default();
    };
    let window_rows: &[Row] = history.window.as_ref().map_or(&[], |window| &window.rows);
    let first = history.window.as_ref().map_or(0, |window| window.first);
    let on_hand = first..first + window_rows.len();

    // The same predicates `visible_rows` reads: the row draws whenever the
    // tree is dirty, and hangs off `HEAD`'s lane only while `HEAD` is on top.
    let top_row = state
        .uncommitted_row_attached()
        .then(|| history.visible(1).first())
        .flatten();
    // Pinned, not merely shown: `graph_rows` already leaves the pinned row
    // out, so the strips cover exactly the rows the scrollers keep the
    // cursor among (48). A one-row pane draws its history row's lanes.
    let uncommitted = state.uncommitted_row_pinned();
    let height = state.graph_rows();

    let ranges = strip_ranges(history.top..history.top + height, &on_hand, strip_rows);
    let virtual_row = uncommitted.then(|| uncommitted_row(top_row));
    if ranges.is_empty() && virtual_row.is_none() {
        return PixelRows::default();
    }

    let row_at = |index: usize| &window_rows[index - first];
    let thread = thread(history);
    if thread.is_none() && cursor_row_on_its_way(history, height) && !window_rows.is_empty() {
        return PixelRows::default();
    }
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
                .map(|above| fold.view(row_at(above)).row.into_owned()),
        })
        .collect();
    if let Some(virtual_row) = &virtual_row {
        let on_top = PaintedRow::uncommitted(&fold, virtual_row);
        put_uncommitted(&mut strips, on_top, history.top, window_rows.is_empty());
    }

    PixelRows::drawn_through(strips, &fold, uncommitted)
}

/// Whether the cursor sits in the viewport and its row has not arrived yet.
///
/// Such a row is on its way. Until it comes there is no thread, and strips
/// drawn without one would be rasterised and sent unlit, then again lit once
/// the window arrives: a page key's frame and its answer each sent every
/// strip (30b). The wheel moves the viewport and leaves the cursor behind,
/// and a cursor off screen is never on its way: waiting for it blanked the
/// graph until a key brought the cursor back (37).
fn cursor_row_on_its_way(history: &History, height: usize) -> bool {
    history.selected().is_none() && (history.top..history.top + height).contains(&history.cursor)
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
/// boundaries. Weighs columns as the text tiers do — [`keep_for`], shared —
/// so a lane never folds differently between tiers.
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
    let reserved: BTreeSet<usize> = (1..=integration).collect();
    let indices: Vec<usize> = ranges.iter().flat_map(Range::clone).collect();
    let drawn: Vec<Row> = virtual_row
        .into_iter()
        .cloned()
        .chain(indices.iter().map(|&index| row_at(index).clone()))
        .collect();
    let cursor = indices
        .iter()
        .position(|&index| index == history.cursor)
        .map(|position| position + usize::from(virtual_row.is_some()));
    let keep = keep_for(
        &drawn,
        cursor,
        &reserved,
        thread.map(|thread| &thread.ancestry),
    );
    let max_lanes = if state.lanes_expanded {
        usize::MAX
    } else {
        max_lanes
    };
    Fold::plan(max_lanes, &keep, &drawn)
}

/// Paints a history row the way `visible_rows` does for text.
struct Painter<'a> {
    state: &'a AppState,
    thread: Option<&'a Thread>,
    fold: &'a Fold,
}

impl Painter<'_> {
    fn row(&self, index: usize, row: &Row) -> PaintedRow {
        let refs = self.state.refs.as_deref();
        // HEAD wins over the stash, as in `row_entry`.
        let ring = if refs.and_then(RefBadges::head) == Some(row.commit.id) {
            Ring::Head
        } else if refs.and_then(RefBadges::stash) == Some(row.commit.id) {
            Ring::Stash
        } else {
            Ring::of(row)
        };
        // The cursor's own row needs no row-level emphasis: it is always on
        // its own thread, and the rasteriser draws the thread at full alpha
        // regardless of `emphasis` (ADR-0024 — the thread is never a wider
        // stroke, only alpha).
        let mut emphasis = Emphasis::Normal;
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
        // A commit only a stash reaches is dimmed as its text row is
        // (`view::rows::mark_stash_only`, m7-power-tools/16). The `□` shape
        // is the text tiers' alone: no new ring.
        if self.state.stash_only.entry(row.commit.id).is_some() {
            emphasis = Emphasis::Dimmed;
        }
        let drawn = self.fold.view(row);
        PaintedRow {
            stubs: stubs_of(&drawn),
            row: drawn.row.into_owned(),
            ring,
            emphasis,
            thread,
        }
    }
}

/// The rasteriser's stubs for a drawn row: one per folded edge, in order.
fn stubs_of(drawn: &DrawnRow<'_>) -> Vec<Stub> {
    drawn
        .folded
        .iter()
        .map(|edge| Stub {
            identity: edge.identity,
            toward: edge.toward,
        })
        .collect()
}

#[cfg(test)]
pub(crate) mod tests;
