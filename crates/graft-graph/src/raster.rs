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
//! - the node itself, on a disc punched clear of every line, drawn in its
//!   [`Ring`] style.
//!
//! The background is never painted (ADR-0003): a pixel no line, node or
//! label touches stays `[0, 0, 0, 0]`, and the terminal composites.
//!
//! Colour depth never dashes a lane here: an image carries full RGB whatever
//! the terminal's text palette, so the Unicode tier's odd-column dashing
//! (low colour depth only) has no pixel counterpart.

mod label;
mod node;

use std::collections::BTreeSet;

use graft_core::capabilities::Rgb;
use graft_core::raster::RasterFrame;
use tiny_skia::{BlendMode, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::identity::BranchIdentity;
use crate::row::Row;

pub use node::Ring;

/// Terminal cells one lane column takes, matching the Unicode tier: the
/// lane's own cell and the gap right of it.
pub const CELLS_PER_LANE: u32 = 2;

/// How strongly a row, or one column of it, is drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Emphasis {
    /// Drawn as laid out.
    #[default]
    Normal,
    /// Faded: off the selection's thread.
    Dimmed,
    /// Heavier strokes: on the selection's thread.
    Highlighted,
}

/// One row to rasterise and how to paint it.
#[derive(Clone, Copy, Debug)]
pub struct RasterRow<'a> {
    /// The row, in drawn columns (a `Fold::view` when anything folds).
    pub row: &'a Row,
    /// The node's ring style.
    pub ring: Ring,
    /// The whole row's emphasis when no thread is given.
    pub emphasis: Emphasis,
    /// Drawn columns carrying the selection's thread on this row. When set,
    /// those columns draw [`Emphasis::Highlighted`] and every other column
    /// [`Emphasis::Dimmed`], overriding `emphasis` (the Unicode tier's
    /// bold/dim rule, ticket 16).
    pub thread: Option<&'a BTreeSet<usize>>,
}

impl<'a> RasterRow<'a> {
    /// A row with the ring its parent count implies and normal emphasis.
    pub fn plain(row: &'a Row) -> Self {
        RasterRow {
            row,
            ring: Ring::of(row),
            emphasis: Emphasis::Normal,
            thread: None,
        }
    }

    fn emphasis_at(&self, column: usize) -> Emphasis {
        match self.thread {
            Some(thread) if thread.contains(&column) => Emphasis::Highlighted,
            Some(_) => Emphasis::Dimmed,
            None => self.emphasis,
        }
    }
}

/// The `+N` column: where it draws and how many lanes fold behind it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Marker {
    /// The drawn column the marker takes (`Fold::marker`).
    pub column: usize,
    /// Lanes folded behind it (`Fold::folded().len()`).
    pub folded: usize,
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
    /// The `+N` column, if anything folds.
    pub marker: Option<Marker>,
    /// Cell width in pixels.
    pub cell_width: u32,
    /// Cell height in pixels.
    pub cell_height: u32,
    /// Lane colours; an identity draws in `palette[identity.slot(len)]`.
    pub palette: &'a [Rgb],
    /// The colour of a marker with no joining lane.
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
#[derive(Clone, Copy, Debug)]
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
}

impl<'a> Pen<'a> {
    fn new(strip: &'a Strip<'a>) -> Self {
        let cell_width = pixels(strip.cell_width);
        let cell_height = pixels(strip.cell_height);
        let stroke = (cell_width * 0.2).max(1.5);
        Pen {
            strip,
            cell_width,
            cell_height,
            stroke,
            node: node::NodeSize::for_cell(cell_width, cell_height, stroke),
        }
    }

    /// The x of a lane column's centre line.
    fn x(&self, column: usize) -> f32 {
        (count(column) * pixels(CELLS_PER_LANE) + 0.5) * self.cell_width
    }

    fn ink(&self, identity: Option<BranchIdentity>, emphasis: Emphasis) -> Ink {
        let rgb = identity.map_or(self.strip.muted, |identity| {
            self.strip.palette[identity.slot(self.strip.palette.len())]
        });
        let (alpha, width) = match emphasis {
            Emphasis::Normal => (255, self.stroke),
            Emphasis::Dimmed => (90, self.stroke),
            Emphasis::Highlighted => (255, self.stroke * 1.6),
        };
        Ink { rgb, alpha, width }
    }

    fn row(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        raster_row: &RasterRow<'_>,
        above: Option<&Row>,
    ) {
        let top = count(index) * self.cell_height;
        let band = Band {
            top,
            mid: top + self.cell_height / 2.0,
            bottom: top + self.cell_height,
        };
        let row = raster_row.row;
        let marker = self.strip.marker.map(|marker| marker.column);
        for (column, identity) in row.lanes.iter().enumerate() {
            if identity.is_none() || column == row.lane {
                continue;
            }
            let ink = self.ink(*identity, raster_row.emphasis_at(column));
            self.edge(pixmap, row, column, band, ink, marker == Some(column));
        }
        let lane_ink = self.ink(
            row.lanes.get(row.lane).copied().flatten(),
            raster_row.emphasis_at(row.lane),
        );
        let x = self.x(row.lane);
        if above.is_some_and(|above| goes_down(above, row.lane)) {
            stroke(pixmap, line(x, band.top, x, band.mid), lane_ink);
        }
        if row.outgoing.iter().any(|&(column, _)| column == row.lane) {
            stroke(pixmap, line(x, band.mid, x, band.bottom), lane_ink);
        }
        if let Some(marker) = self.strip.marker {
            self.marker(pixmap, raster_row, marker, band);
        }
        node::draw(pixmap, raster_row.ring, (x, band.mid), self.node, lane_ink);
    }

    /// One non-node column's edge on a row.
    fn edge(
        &self,
        pixmap: &mut Pixmap,
        row: &Row,
        column: usize,
        band: Band,
        ink: Ink,
        is_marker: bool,
    ) {
        let x = self.x(column);
        let node = self.x(row.lane);
        if row.incoming.contains(&column) {
            stroke(pixmap, arc_in((x, band.top), (node, band.mid)), ink);
        } else if row.outgoing.iter().any(|&(out, _)| out == column) {
            stroke(pixmap, arc_out((node, band.mid), (x, band.bottom)), ink);
        } else if !is_marker {
            stroke(pixmap, line(x, band.top, x, band.bottom), ink);
        }
    }

    fn marker(&self, pixmap: &mut Pixmap, raster_row: &RasterRow<'_>, marker: Marker, band: Band) {
        let joined = raster_row.row.lanes.get(marker.column).copied().flatten();
        // The marker stands for lanes off the thread, so a thread dims it.
        let emphasis = if raster_row.thread.is_some() {
            Emphasis::Dimmed
        } else {
            raster_row.emphasis
        };
        let ink = self.ink(joined, emphasis);
        let centre_x = self.x(marker.column) + self.cell_width / 2.0;
        label::draw(
            pixmap,
            marker.folded,
            (centre_x, band.mid),
            self.cell_height,
            ink,
        );
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
    // `finish` yields `None` only for a path with no length; there is then
    // nothing to draw.
    let Some(path) = path else { return };
    let stroke = Stroke {
        width: ink.width,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    pixmap.stroke_path(&path, &paint(ink), &stroke, Transform::identity(), None);
}

fn paint(ink: Ink) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(ink.rgb.red, ink.rgb.green, ink.rgb.blue, ink.alpha);
    paint.anti_alias = true;
    paint
}

/// A paint that erases to transparent: node discs and label backs punch
/// through whatever lines were drawn under them.
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

/// A row or column index as a coordinate multiplier.
#[allow(clippy::cast_precision_loss)] // Strip rows and lanes are far below f32's 2^24 exact-integer range.
fn count(value: usize) -> f32 {
    value as f32
}
