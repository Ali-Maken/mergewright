//! The pixel tier's rasteriser: laid-out rows in, a premultiplied RGBA strip
//! out (PROMPT.md §1.1, DESIGN.md §3.8).
//!
//! Pure drawing: no I/O, no cache (that is the strip cache's job), no escape
//! sequences (that is `term`'s). Each row is one band `cell_height` pixels
//! tall; each lane column is [`CELLS_PER_LANE`] cells wide, with the lane's
//! centre over the first cell, where the Unicode tier puts its glyph.
//!
//! What a band holds, read from the row alone (plus the row above, for the
//! node's upper stub):
//! - a lane passing straight through: a vertical line, band top to bottom;
//! - a lane closing into the node: a quarter-ellipse from the band's top
//!   edge into the node, leaving vertically and arriving level;
//! - a lane opening from the node: the mirror image, down to the band's
//!   bottom edge;
//! - an edge into a folded lane: a short tail leaving the node's right edge
//!   and fading to nothing before the next column (ADR-0022);
//! - the rail, when anything folds: a 1 px dotted vertical in the muted hue
//!   on the marker column's centre line, three dashes a band;
//! - the node itself, on a disc punched clear of every line, drawn in its
//!   [`Ring`] style.
//!
//! The background is never painted (ADR-0003): a pixel no line, node or
//! rail touches stays `[0, 0, 0, 0]`, and the terminal composites.
//!
//! Colour depth never dashes a lane here: an image carries full RGB whatever
//! the terminal's text palette, so the Unicode tier's odd-column dashing
//! (low colour depth only) has no pixel counterpart.

pub mod cache;
mod node;

use std::cell::RefCell;
use std::collections::BTreeSet;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use tiny_skia::{
    BlendMode, Color, GradientStop, LineCap, LinearGradient, Paint, PathBuilder, Pixmap, Point,
    Rect, SpreadMode, Stroke, Transform,
};

use crate::fold::Toward;
use crate::identity::BranchIdentity;
use crate::row::Row;

pub use node::Ring;

/// Terminal cells one lane column takes, matching the Unicode tier: the
/// lane's own cell and the gap right of it.
pub const CELLS_PER_LANE: u32 = 2;

/// Alpha for a column off the selection's thread (ADR-0024): the thread is
/// emphasised by contrast, never by a heavier stroke.
pub const OFF_THREAD_ALPHA: f32 = 0.5;

/// Alpha for a whole row faded by Focus mode ([`Emphasis::Dimmed`]).
pub const FADED_ALPHA: f32 = 0.3;

/// The one stroke width every line and ring in a strip draws at: lanes,
/// curves and node rings alike (ADR-0024). Emphasis is never a wider
/// stroke, only alpha, so a lane reads as a line to follow at any size.
pub(crate) fn stroke_width(cell_width: u32) -> f32 {
    #[allow(clippy::cast_precision_loss)] // Cell widths are far below f32's exact range.
    let cell_width = cell_width as f32;
    (cell_width / 8.0).clamp(1.5, 2.5)
}

/// How strongly a whole row is drawn when no thread singles out its columns.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emphasis {
    /// Drawn as laid out.
    #[default]
    Normal,
    /// Faded: the row is off the selection's thread while Focus is on.
    Dimmed,
}

impl Emphasis {
    fn alpha(self) -> f32 {
        match self {
            Emphasis::Normal => 1.0,
            Emphasis::Dimmed => FADED_ALPHA,
        }
    }
}

/// An edge a fold took out of a row (`fold::FoldedEdge`), to draw as a short
/// tail on the node rather than a line to the rail (ADR-0022).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Stub {
    /// The folded branch: the tail's colour.
    pub identity: BranchIdentity,
    /// Whether the tail leads down or up.
    pub toward: Toward,
}

/// One row to rasterise and how to paint it.
#[derive(Clone, Debug)]
pub struct RasterRow<'a> {
    /// The row, in drawn columns (a `Fold::view` when anything folds).
    pub row: &'a Row,
    /// The node's ring style.
    pub ring: Ring,
    /// The whole row's emphasis when no thread is given.
    pub emphasis: Emphasis,
    /// Drawn columns carrying the selection's thread on this row. When set,
    /// those columns draw at full alpha and every other column at
    /// [`OFF_THREAD_ALPHA`], overriding `emphasis` (the Unicode tier's
    /// bold/dim rule, ticket 16). Never a wider stroke (ADR-0024).
    pub thread: Option<&'a BTreeSet<usize>>,
    /// Edges into folded lanes, drawn as stubs on the node; empty when
    /// nothing on this row folds.
    pub stubs: Vec<Stub>,
}

impl<'a> RasterRow<'a> {
    /// A row with the ring its parent count implies and normal emphasis.
    pub fn plain(row: &'a Row) -> Self {
        RasterRow {
            row,
            ring: Ring::of(row),
            emphasis: Emphasis::Normal,
            thread: None,
            stubs: Vec::new(),
        }
    }

    /// The alpha a column draws at: full on the thread, [`OFF_THREAD_ALPHA`]
    /// off it, or the row's own [`Emphasis`] when no thread is given.
    fn alpha_at(&self, column: usize) -> f32 {
        match self.thread {
            Some(thread) if thread.contains(&column) => 1.0,
            Some(_) => OFF_THREAD_ALPHA,
            None => self.emphasis.alpha(),
        }
    }
}

/// The rail: the drawn column folded lanes sit behind. It carries no count
/// (ADR-0022); the screen header says how many lanes fold.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Marker {
    /// The drawn column the rail takes (`Fold::marker`).
    pub column: usize,
}

/// Everything one strip is drawn from.
#[derive(Clone, Copy, Debug)]
pub struct Strip<'a> {
    /// The strip's rows, top to bottom.
    pub rows: &'a [RasterRow<'a>],
    /// The row just above the strip, if any: whether the first node's lane
    /// arrives from above depends on it.
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

/// Why a strip could not be rasterised.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum RasterError {
    /// No lane colours were given, so no lane could be drawn.
    #[error("the lane palette is empty, so no lane has a colour to draw in")]
    EmptyPalette,
    /// The pixel buffer would not fit in memory's addressable size.
    #[error(
        "a strip of {rows} rows and {lane_columns} lanes at a {cell_width}×{cell_height} px cell is too large to rasterise"
    )]
    TooLarge {
        /// Rows asked for.
        rows: usize,
        /// Lane columns asked for.
        lane_columns: usize,
        /// Cell width asked for.
        cell_width: u32,
        /// Cell height asked for.
        cell_height: u32,
    },
}

/// Rasterise a strip of rows into a premultiplied RGBA frame.
pub fn rasterise(strip: &Strip<'_>) -> Result<RasterFrame, RasterError> {
    if strip.palette.is_empty() {
        return Err(RasterError::EmptyPalette);
    }
    let too_large = || RasterError::TooLarge {
        rows: strip.rows.len(),
        lane_columns: strip.lane_columns,
        cell_width: strip.cell_width,
        cell_height: strip.cell_height,
    };
    let columns = u32::try_from(strip.lane_columns)
        .ok()
        .and_then(|lanes| lanes.checked_mul(CELLS_PER_LANE))
        .ok_or_else(too_large)?;
    let width = columns
        .checked_mul(strip.cell_width)
        .ok_or_else(too_large)?;
    let height = u32::try_from(strip.rows.len())
        .ok()
        .and_then(|rows| rows.checked_mul(strip.cell_height))
        .ok_or_else(too_large)?;
    let frame = |pixels| RasterFrame {
        width,
        height,
        rows: strip.rows.len(),
        columns,
        pixels,
    };
    if width == 0 || height == 0 {
        return Ok(frame(Vec::new()));
    }
    let mut pixmap = Pixmap::new(width, height).ok_or_else(too_large)?;
    let pen = Pen::new(strip);
    // Edges column by column down the whole strip, so a lane passing
    // straight through many rows is one stroke, not one per row. Within any
    // one band the order is unchanged (columns left to right), and edges
    // stay confined to their band, so the pixels are the per-row drawing's.
    let widest = strip
        .rows
        .iter()
        .map(|raster_row| raster_row.row.lanes.len())
        .max()
        .unwrap_or(0);
    for column in 0..widest {
        pen.column(&mut pixmap, column);
    }
    // Then what sits on each row's node: its lane's own stubs, a folded
    // edge's tail, the rail, the node disc punched over the edges.
    for (index, raster_row) in strip.rows.iter().enumerate() {
        let above = match index.checked_sub(1) {
            Some(previous) => Some(strip.rows[previous].row),
            None => strip.above,
        };
        pen.row(&mut pixmap, index, raster_row, above);
    }
    Ok(frame(pixmap.take()))
}

/// A row's vertical extent in pixels.
#[derive(Clone, Copy, Debug)]
struct Band {
    top: f32,
    mid: f32,
    bottom: f32,
}

/// A colour, its opacity and the stroke width it draws at.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Ink {
    rgb: Rgb,
    alpha: u8,
    width: f32,
}

/// The strip's fixed geometry, shared by every row.
struct Pen<'a> {
    strip: &'a Strip<'a>,
    cell_width: f32,
    cell_height: f32,
    stroke: f32,
    node: node::NodeSize,
    /// Scanlines of straight runs already rasterised, by x and ink.
    scanlines: RefCell<Vec<Scanline>>,
}

/// One scanline of a straight vertical line, rasterised once on a clear
/// background: every scanline of the line between integer y bounds is the
/// same, so a run over a clear background is these bytes, copied.
struct Scanline {
    x: f32,
    ink: Ink,
    /// The pixmap column of `pixels`' first pixel.
    left: usize,
    /// Premultiplied RGBA, clear margin included.
    pixels: Vec<u8>,
}

impl<'a> Pen<'a> {
    fn new(strip: &'a Strip<'a>) -> Self {
        let cell_width = pixels(strip.cell_width);
        let cell_height = pixels(strip.cell_height);
        let stroke = stroke_width(strip.cell_width);
        Pen {
            strip,
            cell_width,
            cell_height,
            stroke,
            node: node::NodeSize::for_cell(cell_width, cell_height, stroke),
            scanlines: RefCell::new(Vec::new()),
        }
    }

    /// The x of a lane column's centre line.
    fn x(&self, column: usize) -> f32 {
        (count(column) * pixels(CELLS_PER_LANE) + 0.5) * self.cell_width
    }

    /// `alpha` is 0.0–1.0; the stroke width never varies with emphasis
    /// (ADR-0024) so a line reads as a line whether it is the thread or not.
    fn ink(&self, identity: Option<BranchIdentity>, alpha: f32) -> Ink {
        let rgb = identity.map_or(self.strip.muted, |identity| {
            self.strip.palette[identity.slot(self.strip.palette.len())]
        });
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // alpha is 0.0..=1.0.
        let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        Ink {
            rgb,
            alpha,
            width: self.stroke,
        }
    }

    fn band(&self, index: usize) -> Band {
        let top = count(index) * self.cell_height;
        Band {
            top,
            mid: top + self.cell_height / 2.0,
            bottom: top + self.cell_height,
        }
    }

    /// One non-node column's edges down the whole strip: a closing arc, an
    /// opening arc, or a line straight through the band. Consecutive rows
    /// passing straight through in the same ink are one stroke.
    fn column(&self, pixmap: &mut Pixmap, column: usize) {
        let is_marker = self.strip.marker.map(|marker| marker.column) == Some(column);
        let x = self.x(column);
        let flush = |pixmap: &mut Pixmap, run: Option<(f32, f32, Ink)>| {
            if let Some((top, bottom, ink)) = run {
                self.straight(pixmap, x, top, bottom, ink);
            }
        };
        // The straight run being extended: its top, its bottom, its ink.
        let mut run: Option<(f32, f32, Ink)> = None;
        for (index, raster_row) in self.strip.rows.iter().enumerate() {
            let row = raster_row.row;
            let band = self.band(index);
            let identity = row.lanes.get(column).copied().flatten();
            if identity.is_none() || column == row.lane {
                flush(pixmap, run.take());
                continue;
            }
            let ink = self.ink(identity, raster_row.alpha_at(column));
            let node = self.x(row.lane);
            if row.incoming.contains(&column) {
                flush(pixmap, run.take());
                stroke(pixmap, arc_in((x, band.top), (node, band.mid)), ink);
            } else if row.outgoing.iter().any(|&(out, _)| out == column) {
                flush(pixmap, run.take());
                stroke(pixmap, arc_out((node, band.mid), (x, band.bottom)), ink);
            } else if is_marker {
                flush(pixmap, run.take());
            } else {
                run = match run {
                    Some((top, _, run_ink)) if run_ink == ink => Some((top, band.bottom, ink)),
                    other => {
                        flush(pixmap, other);
                        Some((band.top, band.bottom, ink))
                    }
                };
            }
        }
        flush(pixmap, run);
    }

    /// A row's own lane stubs, its folded edge's tail, the rail and its node,
    /// drawn over the edges.
    fn row(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        raster_row: &RasterRow<'_>,
        above: Option<&Row>,
    ) {
        let band = self.band(index);
        let row = raster_row.row;
        let lane_ink = self.ink(
            row.lanes.get(row.lane).copied().flatten(),
            raster_row.alpha_at(row.lane),
        );
        let x = self.x(row.lane);
        if above.is_some_and(|above| goes_down(above, row.lane)) {
            stroke(pixmap, line(x, band.top, x, band.mid), lane_ink);
        }
        if row.outgoing.iter().any(|&(column, _)| column == row.lane) {
            stroke(pixmap, line(x, band.mid, x, band.bottom), lane_ink);
        }
        self.tail(pixmap, raster_row, band);
        if let Some(marker) = self.strip.marker {
            self.rail(pixmap, raster_row, marker, band);
        }
        node::draw(pixmap, raster_row.ring, (x, band.mid), self.node, lane_ink);
    }

    /// A straight run down `x` from `top` to `bottom`, both whole pixels.
    ///
    /// Anti-aliased stroking costs per scanline, and a lane passing through
    /// a strip is most of its scanlines, so where the pixels under the line
    /// are still clear the scanline rasterised once is copied; where
    /// something is already drawn (an arc crossing), those rows are stroked
    /// as before, so blending stays tiny-skia's. Byte-identical either way
    /// (`tests/it/raster_pixels.rs`).
    fn straight(&self, pixmap: &mut Pixmap, x: f32, top: f32, bottom: f32, ink: Ink) {
        let Some(index) = self.scanline(pixmap.width(), x, ink) else {
            stroke(pixmap, line(x, top, x, bottom), ink);
            return;
        };
        let scanlines = self.scanlines.borrow();
        let scanline = &scanlines[index];
        let stride = pixels_len(pixmap.width());
        let (left, span) = (scanline.left * 4, scanline.pixels.len());
        let (first, last) = (whole(top), whole(bottom));
        let mut dirty: Option<usize> = None;
        for y in first..last {
            let start = y * stride + left;
            let under = &mut pixmap.data_mut()[start..start + span];
            if under.iter().all(|&byte| byte == 0) {
                under.copy_from_slice(&scanline.pixels);
                if let Some(from) = dirty.take() {
                    stroke(pixmap, line(x, count(from), x, count(y)), ink);
                }
            } else if dirty.is_none() {
                dirty = Some(y);
            }
        }
        if let Some(from) = dirty {
            stroke(pixmap, line(x, count(from), x, count(last)), ink);
        }
    }

    /// The index of `x`'s scanline in `ink`, rasterising it on first use;
    /// `None` when its clear margin would leave the pixmap.
    fn scanline(&self, width: u32, x: f32, ink: Ink) -> Option<usize> {
        let mut scanlines = self.scanlines.borrow_mut();
        if let Some(index) = scanlines
            .iter()
            .position(|scanline| scanline.x.to_bits() == x.to_bits() && scanline.ink == ink)
        {
            return Some(index);
        }
        let half = ink.width / 2.0;
        // One clear pixel either side of anything the line can touch.
        let left = (x - half).floor() - 1.0;
        let right = (x + half).ceil() + 1.0;
        if left < 0.0 || right > pixels(width) {
            return None;
        }
        let (left, right) = (whole(left), whole(right));
        let span = u32::try_from(right - left).ok()?;
        let mut sample = Pixmap::new(span, 3)?;
        // `left` is a whole pixel, so the fraction of `x` is kept exactly.
        let local = x - count(left);
        stroke(&mut sample, line(local, 0.0, local, 3.0), ink);
        let stride = pixels_len(span);
        let pixels = sample.data()[stride..2 * stride].to_vec();
        scanlines.push(Scanline {
            x,
            ink,
            left,
            pixels,
        });
        Some(scanlines.len() - 1)
    }

    /// A folded edge's tail (ADR-0022): leaves the node's right edge level,
    /// curves toward where the folded branch went (down for `Below`, up for
    /// `Above`) over half a band, and fades from the lane's alpha to nothing
    /// before the next column's centre line, so it never reads as reaching
    /// another lane. Several folded edges on a row draw one tail, in the
    /// first edge's colour.
    fn tail(&self, pixmap: &mut Pixmap, raster_row: &RasterRow<'_>, band: Band) {
        let Some(first) = raster_row.stubs.first() else {
            return;
        };
        let row = raster_row.row;
        let ink = self.ink(Some(first.identity), raster_row.alpha_at(row.lane));
        let from = (self.x(row.lane) + self.node.clearance(), band.mid);
        // Half a stroke short, and half again for anti-aliasing: the tail's
        // widest reach stays left of the next centre line.
        let to_x = self.x(row.lane + 1) - self.stroke;
        let to_y = match first.toward {
            Toward::Below => band.bottom,
            Toward::Above => band.top,
        };
        let Rgb { red, green, blue } = ink.rgb;
        // A gradient across x: alpha falls with distance from the node,
        // whichever way the curve bends.
        let shader = LinearGradient::new(
            Point::from_xy(from.0, band.mid),
            Point::from_xy(to_x, band.mid),
            vec![
                GradientStop::new(0.0, Color::from_rgba8(red, green, blue, ink.alpha)),
                GradientStop::new(1.0, Color::from_rgba8(red, green, blue, 0)),
            ],
            SpreadMode::Pad,
            Transform::identity(),
        );
        // `new` is `None` only when start and end coincide; the tail then
        // has no length and there is nothing to draw.
        let Some(shader) = shader else { return };
        let paint = Paint {
            shader,
            anti_alias: true,
            ..Paint::default()
        };
        stroke_with(pixmap, arc_out(from, (to_x, to_y)), &paint, ink.width);
    }

    /// The rail on one band: a 1 px dotted vertical on the marker column's
    /// centre line (where the Unicode tier puts its `⋮`), in the muted hue.
    /// Dashes are a sixth of a band on, a sixth off, starting at the band's
    /// top, so every band draws the same three dashes and the pattern never
    /// jumps at a row or strip edge.
    fn rail(&self, pixmap: &mut Pixmap, raster_row: &RasterRow<'_>, marker: Marker, band: Band) {
        // The rail stands for lanes off the thread, so a thread dims it.
        let alpha = if raster_row.thread.is_some() {
            OFF_THREAD_ALPHA
        } else {
            raster_row.emphasis.alpha()
        };
        let paint = paint(self.ink(None, alpha));
        // A whole pixel column, so the rail is crisp, not two half-lit ones.
        let left = (self.x(marker.column) - 0.5).round();
        let dash = self.cell_height / 6.0;
        for index in 0..3 {
            let top = band.top + count(index) * 2.0 * dash;
            // `from_xywh` is `None` only for an empty or non-finite rectangle.
            if let Some(rect) = Rect::from_xywh(left, top, 1.0, dash) {
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }
}

/// Whether a row's drawing leaves through the bottom of `column`.
fn goes_down(row: &Row, column: usize) -> bool {
    if column == row.lane {
        return row.outgoing.iter().any(|&(out, _)| out == column);
    }
    row.lanes.get(column).copied().flatten().is_some() && !row.incoming.contains(&column)
}

fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<tiny_skia::Path> {
    let mut path = PathBuilder::new();
    path.move_to(x0, y0);
    path.line_to(x1, y1);
    path.finish()
}

/// The cubic control distance that best fits a quarter circle.
const QUARTER: f32 = 0.552_284_8;

/// A closing edge: leaves `from` heading straight down, arrives level at
/// `to`. A quarter-ellipse, so it meets the lane above without a kink.
fn arc_in(from: (f32, f32), to: (f32, f32)) -> Option<tiny_skia::Path> {
    let ((x0, y0), (x1, y1)) = (from, to);
    let mut path = PathBuilder::new();
    path.move_to(x0, y0);
    path.cubic_to(
        x0,
        y0 + QUARTER * (y1 - y0),
        x1 + QUARTER * (x0 - x1),
        y1,
        x1,
        y1,
    );
    path.finish()
}

/// An opening edge: leaves `from` level, arrives at `to` heading straight
/// down, so it meets the lane below without a kink.
fn arc_out(from: (f32, f32), to: (f32, f32)) -> Option<tiny_skia::Path> {
    let ((x0, y0), (x1, y1)) = (from, to);
    let mut path = PathBuilder::new();
    path.move_to(x0, y0);
    path.cubic_to(
        x0 + QUARTER * (x1 - x0),
        y0,
        x1,
        y1 - QUARTER * (y1 - y0),
        x1,
        y1,
    );
    path.finish()
}

fn stroke(pixmap: &mut Pixmap, path: Option<tiny_skia::Path>, ink: Ink) {
    stroke_with(pixmap, path, &paint(ink), ink.width);
}

fn stroke_with(pixmap: &mut Pixmap, path: Option<tiny_skia::Path>, paint: &Paint<'_>, width: f32) {
    // `finish` yields `None` only for a path with no length; there is then
    // nothing to draw.
    let Some(path) = path else { return };
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, paint, &stroke, Transform::identity(), None);
}

fn paint(ink: Ink) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(ink.rgb.red, ink.rgb.green, ink.rgb.blue, ink.alpha);
    paint.anti_alias = true;
    paint
}

/// A paint that erases to transparent: node discs punch through whatever
/// lines were drawn under them.
fn eraser() -> Paint<'static> {
    Paint {
        blend_mode: BlendMode::Clear,
        anti_alias: true,
        ..Paint::default()
    }
}

/// A pixel dimension as a coordinate.
#[allow(clippy::cast_precision_loss)] // Cell sizes are far below f32's 2^24 exact-integer range.
fn pixels(value: u32) -> f32 {
    value as f32
}

/// A whole, non-negative coordinate as a pixel index.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Callers pass whole pixels inside the pixmap.
fn whole(value: f32) -> usize {
    value as usize
}

/// Bytes in one scanline `width` pixels wide.
fn pixels_len(width: u32) -> usize {
    usize::try_from(width).map_or(usize::MAX, |width| width * 4)
}

/// A row or column index as a coordinate multiplier.
#[allow(clippy::cast_precision_loss)] // Strip rows and lanes are far below f32's 2^24 exact-integer range.
fn count(value: usize) -> f32 {
    value as f32
}

#[cfg(test)]
mod tests {
    use super::stroke_width;

    #[test]
    #[allow(clippy::float_cmp)] // Exact clamp bounds and an exact division; no rounding involved.
    fn stroke_width_clamps_by_cell_width() {
        assert_eq!(stroke_width(8), 1.5);
        assert_eq!(stroke_width(24), 2.5);
        assert_eq!(stroke_width(17), 17.0 / 8.0);
    }
}
