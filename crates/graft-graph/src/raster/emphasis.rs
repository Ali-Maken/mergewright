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
    Emphasis, FADED_ALPHA, Inked, Look, Marker, OFF_THREAD_ALPHA, RasterError, Ring, Stub, paint,
};
use crate::row::Row;

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

/// An emphasis image, cropped to the cell columns it touches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Emphasised {
    /// The image: the strip's height, `frame.columns` cells wide. Shared,
    /// so a terminal upload can tell this image from a new one by pointer.
    pub frame: Arc<RasterFrame>,
    /// The strip's cell column the image's left edge sits on.
    pub first_column: u16,
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
    paint(&strip.look(), &rows, None)
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
    let frame = paint(&strip.look(), &rows, Some(backdrop))?;
    let Some((first, last)) = painted_cells(&frame, strip.cell_width, reach) else {
        return Ok(None);
    };
    let first_column = u16::try_from(first).map_err(|_| RasterError::TooLarge {
        rows: strip.rows.len(),
        lane_columns: strip.lane_columns,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
    })?;
    Ok(Some(Emphasised {
        frame: Arc::new(crop(&frame, strip.cell_width, first, last)),
        first_column,
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

/// The first and last cell columns holding a painted pixel, or `None` when
/// none does. Premultiplied: a clear pixel is four zero bytes. Only the
/// pixels around lane columns `reach` are scanned: a node reaches less than
/// a cell either side of its lane's centre (`NodeSize::for_cell`), every
/// stroke less than that.
fn painted_cells(
    frame: &RasterFrame,
    cell_width: u32,
    reach: (usize, usize),
) -> Option<(u32, u32)> {
    let width = usize::try_from(frame.width).ok()?;
    let cell = usize::try_from(cell_width).ok()?;
    let lane = cell * usize::try_from(super::CELLS_PER_LANE).ok()?;
    if width == 0 || cell == 0 {
        return None;
    }
    // From a cell left of the first lane's own cell to the end of the cell
    // right of the last lane's own cell.
    let from = (reach.0 * lane).saturating_sub(cell).min(width);
    let to = (reach.1 * lane + 2 * cell).min(width);
    let painted = |byte: &u8| *byte != 0;
    // Both on cell edges, as `from` and `to` are.
    let start = |pixel: usize| pixel / cell * cell;
    let end = |pixel: usize| (pixel / cell + 1) * cell;
    // The painted cells found so far, as a pixel range `[left, right)`:
    // only pixels outside it can widen it, so a line scans those alone.
    let mut span: Option<(usize, usize)> = None;
    for line in frame.pixels.chunks_exact(width * 4) {
        let pixels = |range: std::ops::Range<usize>| &line[range.start * 4..range.end * 4];
        span = match span {
            None => {
                let Some(byte) = pixels(from..to).iter().position(painted) else {
                    continue;
                };
                let first = from + byte / 4;
                let after = end(first);
                let last = pixels(after..to)
                    .iter()
                    .rposition(painted)
                    .map_or(first, |byte| after + byte / 4);
                Some((start(first), end(last)))
            }
            Some((left, right)) => {
                let first = pixels(from..left).iter().position(painted);
                let last = pixels(right..to).iter().rposition(painted);
                Some((
                    first.map_or(left, |byte| start(from + byte / 4)),
                    last.map_or(right, |byte| end(right + byte / 4)),
                ))
            }
        };
    }
    let (left, right) = span?;
    let cell_of = |pixel: usize| u32::try_from(pixel / cell).ok();
    Some((cell_of(left)?, cell_of(right - 1)?))
}

/// Cell columns `first..=last` of `frame`.
fn crop(frame: &RasterFrame, cell_width: u32, first: u32, last: u32) -> RasterFrame {
    let columns = last - first + 1;
    let width = columns * cell_width;
    let bytes = |pixels: u32| pixels as usize * 4;
    let (stride, left, span) = (bytes(frame.width), bytes(first * cell_width), bytes(width));
    let mut pixels = Vec::with_capacity(span * frame.height as usize);
    for line in frame.pixels.chunks_exact(stride) {
        pixels.extend_from_slice(&line[left..left + span]);
    }
    RasterFrame {
        width,
        height: frame.height,
        rows: frame.rows,
        columns,
        pixels,
    }
}

/// `top` drawn over `base` at its column offset, premultiplied "over"
/// (ADR-0053): for iTerm2, which cannot layer two images in a cell.
///
/// `top` is the emphasis of a strip drawn as `base`, so it has `base`'s
/// height and lies within its width; anything of it outside `base` is not
/// drawn.
#[must_use]
pub fn composite(base: &RasterFrame, top: &Emphasised) -> RasterFrame {
    let mut out = base.clone();
    let Some(cell_width) = base.width.checked_div(base.columns) else {
        return out;
    };
    debug_assert_eq!(
        (top.frame.height, top.frame.rows),
        (base.height, base.rows),
        "an emphasis layer is its strip's height"
    );
    let left = u32::from(top.first_column) * cell_width;
    let width = top.frame.width.min(base.width.saturating_sub(left));
    let height = top.frame.height.min(base.height);
    let bytes = |pixels: u32| pixels as usize * 4;
    for y in 0..height {
        let from = bytes(y * top.frame.width);
        let to = bytes(y * base.width + left);
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
    #[allow(clippy::cast_possible_truncation)] // At most 255, shown above.
    let rounded = rounded as u8;
    rounded
}
