//! Line and arc geometry, and pixel-unit conversions, for the rasteriser
//! in [`super`].

use tiny_skia::{BlendMode, LineCap, Paint, PathBuilder, Pixmap, Stroke, Transform};

use super::{Ink, Row};

/// Whether a row's drawing leaves through the bottom of `column`.
pub(super) fn goes_down(row: &Row, column: usize) -> bool {
    if column == row.lane {
        return row.outgoing.iter().any(|&(out, _)| out == column);
    }
    row.lanes.get(column).copied().flatten().is_some() && !row.incoming.contains(&column)
}

pub(super) fn line(x0: f32, y0: f32, x1: f32, y1: f32) -> Option<tiny_skia::Path> {
    let mut path = PathBuilder::new();
    path.move_to(x0, y0);
    path.line_to(x1, y1);
    path.finish()
}

/// The cubic control distance that best fits a quarter circle.
const QUARTER: f32 = 0.552_284_8;

/// A closing edge: leaves `from` heading straight down, arrives level at
/// `to`. A quarter-ellipse, so it meets the lane above without a kink.
pub(super) fn arc_in(from: (f32, f32), to: (f32, f32)) -> Option<tiny_skia::Path> {
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
pub(super) fn arc_out(from: (f32, f32), to: (f32, f32)) -> Option<tiny_skia::Path> {
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

pub(super) fn stroke(pixmap: &mut Pixmap, path: Option<tiny_skia::Path>, ink: Ink) {
    stroke_with(pixmap, path, &paint(ink), ink.width);
}

pub(super) fn stroke_with(
    pixmap: &mut Pixmap,
    path: Option<tiny_skia::Path>,
    paint: &Paint<'_>,
    width: f32,
) {
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

pub(super) fn paint(ink: Ink) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(ink.rgb.red, ink.rgb.green, ink.rgb.blue, ink.alpha);
    paint.anti_alias = true;
    paint
}

/// A paint that erases to transparent: node discs punch through whatever
/// lines were drawn under them.
pub(super) fn eraser() -> Paint<'static> {
    Paint {
        blend_mode: BlendMode::Clear,
        anti_alias: true,
        ..Paint::default()
    }
}

/// A pixel dimension as a coordinate.
#[allow(clippy::cast_precision_loss)] // Cell sizes are far below f32's 2^24 exact-integer range.
pub(super) fn pixels(value: u32) -> f32 {
    value as f32
}

/// A whole, non-negative coordinate as a pixel index.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // Callers pass whole pixels inside the pixmap.
pub(super) fn whole(value: f32) -> usize {
    value as usize
}

/// Bytes in one scanline `width` pixels wide.
pub(super) fn pixels_len(width: u32) -> usize {
    usize::try_from(width).map_or(usize::MAX, |width| width * 4)
}

/// A row or column index as a coordinate multiplier.
#[allow(clippy::cast_precision_loss)] // Strip rows and lanes are far below f32's 2^24 exact-integer range.
pub(super) fn count(value: usize) -> f32 {
    value as f32
}
