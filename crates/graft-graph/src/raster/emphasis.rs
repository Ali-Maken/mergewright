//! The selection's paint as its own layer (ADR-0053).
//!
//! A strip used to bake the selection's thread into its pixels, so every
//! cursor move on a branchy history missed every strip and re-sent it. Here
//! a strip is drawn from layout only, every element at one **backdrop**
//! alpha; the selection's paint is a second, smaller image, the
//! **emphasis**, holding every element ADR-0024 draws brighter than the
//! backdrop, at `(a − b) / (1 − b)`. Composited over the strip it gives
//! alpha `a` again: the same look, sent as a layer that changes on its own.
//!
//! Both passes are one pen ([`super::paint`]) with a different paint per
//! row, so geometry and draw order cannot drift apart.

use std::collections::BTreeSet;
use std::sync::Arc;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;

use super::{
    Emphasis, FADED_ALPHA, Inked, Look, Marker, OFF_THREAD_ALPHA, RasterError, RasterPool, Ring,
    Stub, paint,
};
use crate::row::Row;

mod cells;
mod pieces;

use cells::{crop, painted_cells, painted_cells_in, painted_rows};

pub use pieces::{Piece, PieceKind, emphasis_pieces, rasterise_piece};

/// The one alpha a strip draws every element at: the lowest in play for the
/// frame. It changes only when Focus toggles or a thread comes or goes,
/// never on a cursor move.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Backdrop {
    /// No thread on screen: everything at full alpha, no emphasis layer.
    #[default]
    Full,
    /// A thread on screen: everything at [`OFF_THREAD_ALPHA`].
    OffThread,
    /// Focus mode: everything at [`FADED_ALPHA`].
    Faded,
}

impl Backdrop {
    /// The alpha the strip draws at.
    #[must_use]
    pub fn alpha(self) -> f32 {
        match self {
            Backdrop::Full => 1.0,
            Backdrop::OffThread => OFF_THREAD_ALPHA,
            Backdrop::Faded => FADED_ALPHA,
        }
    }

    /// The lowest alpha any of these rows draws at (ADR-0053 Decision 2):
    /// `Faded` if some row has `thread: None` and [`Emphasis::Dimmed`];
    /// else `OffThread` if some row has `thread: Some(_)` (∅ included);
    /// else `Full`. Read off the paint, never off the features that made it,
    /// so a new reason to dim a row cannot be missed here.
    #[must_use]
    pub fn lowest<'a>(lits: impl IntoIterator<Item = Lit<'a>>) -> Backdrop {
        let mut lowest = Backdrop::Full;
        for lit in lits {
            match (lit.thread, lit.emphasis) {
                (None, Emphasis::Dimmed) => return Backdrop::Faded,
                (Some(_), _) => lowest = Backdrop::OffThread,
                (None, Emphasis::Normal) => {}
            }
        }
        lowest
    }

    /// The paint that draws every element at this alpha, through the same
    /// rules as a row's own paint.
    fn paint(self) -> Lit<'static> {
        /// No column on the thread: every column off it.
        static NONE_LIT: BTreeSet<usize> = BTreeSet::new();
        match self {
            Backdrop::Full => Lit::default(),
            Backdrop::OffThread => Lit {
                emphasis: Emphasis::Normal,
                thread: Some(&NONE_LIT),
            },
            Backdrop::Faded => Lit {
                emphasis: Emphasis::Dimmed,
                thread: None,
            },
        }
    }
}

/// A row as the layout draws it: no selection in it.
#[derive(Clone, Debug)]
pub struct PlainRow<'a> {
    /// The row, in drawn columns (a `Fold::view` when anything folds).
    pub row: &'a Row,
    /// The node's ring style.
    pub ring: Ring,
    /// Edges into folded lanes, drawn as stubs on the node.
    pub stubs: Vec<Stub>,
}

impl<'a> PlainRow<'a> {
    /// `Ring::of` the row, no stubs.
    #[must_use]
    pub fn of(row: &'a Row) -> PlainRow<'a> {
        PlainRow {
            row,
            ring: Ring::of(row),
            stubs: Vec::new(),
        }
    }
}

/// One row's selection paint: its own emphasis and the drawn columns on the
/// selection's thread.
///
/// `thread: None` means "every column at `emphasis`"; `Some(∅)` means
/// "every column off the thread", which is a different alpha.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Lit<'a> {
    /// The whole row's emphasis when no thread is given.
    pub emphasis: Emphasis,
    /// Drawn columns carrying the selection's thread on this row: those at
    /// full alpha, every other at [`OFF_THREAD_ALPHA`], `emphasis` ignored.
    pub thread: Option<&'a BTreeSet<usize>>,
}

impl Lit<'_> {
    /// The alpha a column draws at: full on the thread, [`OFF_THREAD_ALPHA`]
    /// off it, or the row's own [`Emphasis`] when no thread is given.
    pub(super) fn alpha_at(&self, column: usize) -> f32 {
        match self.thread {
            Some(thread) if thread.contains(&column) => 1.0,
            Some(_) => OFF_THREAD_ALPHA,
            None => self.emphasis.alpha(),
        }
    }

    /// The rail's alpha: it stands for lanes off the thread, so a thread
    /// dims it.
    pub(super) fn rail_alpha(&self) -> f32 {
        if self.thread.is_some() {
            OFF_THREAD_ALPHA
        } else {
            self.emphasis.alpha()
        }
    }

    /// The highest alpha anything on the row can draw at.
    fn peak(&self) -> f32 {
        match self.thread {
            Some(thread) if !thread.is_empty() => 1.0,
            Some(_) => OFF_THREAD_ALPHA,
            None => self.emphasis.alpha(),
        }
    }
}

/// Everything one strip is drawn from: layout-only rows and how they look.
#[derive(Clone, Copy, Debug)]
pub struct PlainStrip<'a> {
    /// The strip's rows, top to bottom.
    pub rows: &'a [PlainRow<'a>],
    /// The row just above the strip, if any. Its edges into the first row
    /// draw at that row's paint.
    pub above: Option<&'a Row>,
    /// Lane columns the strip is wide, marker included.
    pub lane_columns: usize,
    /// The rail column, if anything folds.
    pub marker: Option<Marker>,
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Lane colours; an identity draws in `palette[identity.slot(len)]`.
    pub palette: &'a [Rgb],
    /// The rail's colour.
    pub muted: Rgb,
}

impl PlainStrip<'_> {
    fn look(&self) -> Look<'_> {
        Look {
            above: self.above,
            lane_columns: self.lane_columns,
            marker: self.marker,
            cell_width: self.cell_width,
            cell_height: self.cell_height,
            palette: self.palette,
            muted: self.muted,
        }
    }
}

/// The selection over one strip: one [`Lit`] per [`PlainStrip::rows`], and
/// the frame's backdrop.
#[derive(Clone, Copy, Debug)]
pub struct Selection<'a> {
    /// Each strip row's paint, top to bottom.
    pub rows: &'a [Lit<'a>],
    /// The alpha the strip under it is drawn at.
    pub backdrop: Backdrop,
}

/// An emphasis image, cropped to the cells it touches: a whole strip's
/// emphasis to its cell columns, a piece to its cell columns and rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emphasised {
    /// The image, `frame.columns` cells wide and `frame.rows` rows tall.
    /// Shared between the strip cache and the frame drawing it.
    pub frame: Arc<RasterFrame>,
    /// The strip's cell column the image's left edge sits on.
    pub first_column: u16,
    /// The strip row the image's top edge sits on: always 0 for a whole
    /// strip's emphasis ([`rasterise_emphasis`]), which is the strip's height.
    pub first_row: u16,
}

/// A strip drawn from layout only, every element at `backdrop`'s alpha.
///
/// # Errors
///
/// [`RasterError::EmptyPalette`], or [`RasterError::TooLarge`] for a strip
/// too large to draw.
pub fn rasterise_backdrop(
    strip: &PlainStrip<'_>,
    backdrop: Backdrop,
) -> Result<RasterFrame, RasterError> {
    rasterise_backdrop_in(strip, backdrop, &mut RasterPool::new())
}

/// [`rasterise_backdrop`], drawn in a buffer from `pool`.
pub(super) fn rasterise_backdrop_in(
    strip: &PlainStrip<'_>,
    backdrop: Backdrop,
    pool: &mut RasterPool,
) -> Result<RasterFrame, RasterError> {
    let lit = backdrop.paint();
    let rows: Vec<Inked<'_>> = strip
        .rows
        .iter()
        .map(|plain| Inked {
            row: plain.row,
            ring: plain.ring,
            stubs: &plain.stubs,
            lit,
        })
        .collect();
    paint(&strip.look(), &rows, None, pool)
}

/// The selection's paint over a strip drawn at `sel.backdrop`: every element
/// ADR-0024 draws above the backdrop's alpha `b`, at `(a − b) / (1 − b)`,
/// cropped to the cell columns it touches. `None` when nothing is above the
/// backdrop — always under [`Backdrop::Full`].
///
/// # Errors
///
/// [`RasterError::Selection`] when `sel` does not have one entry per strip
/// row; otherwise as [`rasterise_backdrop`].
pub fn rasterise_emphasis(
    strip: &PlainStrip<'_>,
    sel: &Selection<'_>,
) -> Result<Option<Emphasised>, RasterError> {
    rasterise_emphasis_in(strip, sel, &mut RasterPool::new())
}

/// [`rasterise_emphasis`], drawn in a buffer from `pool` and cropped in
/// place; a buffer that ends up holding nothing goes back.
pub(super) fn rasterise_emphasis_in(
    strip: &PlainStrip<'_>,
    sel: &Selection<'_>,
    pool: &mut RasterPool,
) -> Result<Option<Emphasised>, RasterError> {
    if sel.rows.len() != strip.rows.len() {
        return Err(RasterError::Selection {
            rows: strip.rows.len(),
            lit: sel.rows.len(),
        });
    }
    let backdrop = sel.backdrop.alpha();
    if sel.backdrop == Backdrop::Full || !sel.rows.iter().any(|lit| lit.peak() > backdrop) {
        return Ok(None);
    }
    let rows: Vec<Inked<'_>> = strip
        .rows
        .iter()
        .zip(sel.rows)
        .map(|(plain, lit)| Inked {
            row: plain.row,
            ring: plain.ring,
            stubs: &plain.stubs,
            lit: *lit,
        })
        .collect();
    let Some(reach) = lit_reach(&rows, strip.marker, backdrop) else {
        return Ok(None);
    };
    let frame = paint(&strip.look(), &rows, Some(backdrop), pool)?;
    let Some((first, last)) = painted_cells(&frame, strip.cell_width, reach) else {
        pool.give(frame.pixels);
        return Ok(None);
    };
    let first_column = u16::try_from(first).map_err(|_| RasterError::TooLarge {
        rows: strip.rows.len(),
        lane_columns: strip.lane_columns,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
    })?;
    let rows = (0, frame.rows.saturating_sub(1));
    Ok(Some(Emphasised {
        frame: Arc::new(crop(frame, strip.cell_width, (first, last), rows)),
        first_column,
        first_row: 0,
    }))
}

/// The drawn lane columns the emphasis pass can draw in, first and last, or
/// `None` when it draws nothing: a bound for [`painted_cells`]' scan, so a
/// thread a few lanes wide does not cost a scan of the whole strip. It
/// mirrors which elements [`super::Pen`] draws above the backdrop; a column
/// it missed would lose lit pixels.
fn lit_reach(rows: &[Inked<'_>], marker: Option<Marker>, backdrop: f32) -> Option<(usize, usize)> {
    let mut reach: Option<(usize, usize)> = None;
    let mut take = |from: usize, to: usize| {
        let (low, high) = (from.min(to), from.max(to));
        reach = Some(reach.map_or((low, high), |(first, last)| {
            (first.min(low), last.max(high))
        }));
    };
    for inked in rows {
        let row = inked.row;
        for (column, identity) in row.lanes.iter().enumerate() {
            if identity.is_some() && column != row.lane && inked.lit.alpha_at(column) > backdrop {
                // An arc runs from its column to the node; a straight line
                // stays in its column.
                let ends_here = row.incoming.contains(&column)
                    || row.outgoing.iter().any(|&(out, _)| out == column);
                take(column, if ends_here { row.lane } else { column });
            }
        }
        if inked.lit.alpha_at(row.lane) > backdrop {
            // Stubs and the node; a tail reaches toward the next column.
            take(row.lane, row.lane + usize::from(!inked.stubs.is_empty()));
        }
        if let Some(marker) = marker
            && inked.lit.rail_alpha() > backdrop
        {
            take(marker.column, marker.column);
        }
    }
    reach
}

/// `top` drawn over `base` at its cell column and row, premultiplied
/// "over" (ADR-0053): for iTerm2, which cannot layer two images in a cell.
///
/// `top` is the emphasis of a strip drawn as `base` (or a piece of it), so
/// it lies within `base`; anything of it outside `base` is not drawn.
#[must_use]
pub fn composite(base: &RasterFrame, top: &Emphasised) -> RasterFrame {
    composite_in(base, top, &mut RasterPool::new())
}

/// [`composite`], drawn in a buffer from `pool`.
pub(super) fn composite_in(
    base: &RasterFrame,
    top: &Emphasised,
    pool: &mut RasterPool,
) -> RasterFrame {
    let mut pixels = pool.take(base.pixels.len());
    pixels.copy_from_slice(&base.pixels);
    let mut out = RasterFrame {
        width: base.width,
        height: base.height,
        rows: base.rows,
        columns: base.columns,
        pixels,
    };
    let Some(cell_width) = base.width.checked_div(base.columns) else {
        return out;
    };
    let Some(cell_height) = u32::try_from(base.rows)
        .ok()
        .and_then(|rows| base.height.checked_div(rows))
    else {
        // No rows: no pixels to draw over.
        return out;
    };
    debug_assert!(
        usize::from(top.first_row) + top.frame.rows <= base.rows,
        "an emphasis layer lies within its strip's rows"
    );
    let left = u32::from(top.first_column) * cell_width;
    let below = u32::from(top.first_row) * cell_height;
    let width = top.frame.width.min(base.width.saturating_sub(left));
    let height = top.frame.height.min(base.height.saturating_sub(below));
    let bytes = |pixels: u32| pixels as usize * 4;
    for y in 0..height {
        let from = bytes(y * top.frame.width);
        let to = bytes((below + y) * base.width + left);
        let source = &top.frame.pixels[from..from + bytes(width)];
        let target = &mut out.pixels[to..to + bytes(width)];
        for (over, under) in source
            .as_chunks::<4>()
            .0
            .iter()
            .zip(target.as_chunks_mut::<4>().0)
        {
            match over[3] {
                0 => {}
                255 => under.copy_from_slice(over),
                alpha => {
                    let keep = 255 - u32::from(alpha);
                    for (channel, &on_top) in under.iter_mut().zip(over) {
                        // Never saturates on premultiplied input (`div255`).
                        *channel = on_top.saturating_add(div255(u32::from(*channel) * keep));
                    }
                }
            }
        }
    }
    out
}

/// `value / 255`, rounded to nearest, for `value` up to `255 × 255`.
fn div255(value: u32) -> u8 {
    let rounded = (value + 128 + ((value + 128) >> 8)) >> 8;
    // Premultiplied: a channel is at most its alpha, so `channel × (255 −
    // alpha) / 255` is at most `255 − alpha` and the sum stays in a byte.
    #[allow(clippy::cast_possible_truncation, reason = "At most 255, shown above.")]
    let rounded = rounded as u8;
    rounded
}
