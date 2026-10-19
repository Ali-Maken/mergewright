//! Line and arc geometry, and pixel-unit conversions, for the rasteriser
//! in [`super`].

use tiny_skia::{
    BlendMode, FillRule, LineCap, Paint, Path, PathBuilder, PathSegment, Pixmap, Stroke, Transform,
};

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

/// Stroke `path`, built in strip pixels, into `pixmap` moved by `shift`
/// (`Pen::shift`).
pub(super) fn stroke(
    pixmap: &mut Pixmap,
    path: Option<tiny_skia::Path>,
    ink: Ink,
    shift: Transform,
) {
    // `finish` yields `None` only for a path with no length; there is then
    // nothing to draw.
    let Some(path) = path else { return };
    super::stroke::styled(pixmap, &path, &paint(ink), ink.width, ink.shape, shift);
}

pub(super) fn stroke_with(
    pixmap: &mut Pixmap,
    path: Option<tiny_skia::Path>,
    paint: &Paint<'_>,
    width: f32,
    shift: Transform,
) {
    // `finish` yields `None` only for a path with no length; there is then
    // nothing to draw.
    let Some(path) = path else { return };
    let stroke = Stroke {
        width,
        line_cap: LineCap::Butt,
        ..Stroke::default()
    };
    stroke_placed(pixmap, &path, paint, &stroke, shift);
}

/// Pixels per [`snap`] step: outlines sit on a 1/1024 px grid.
const GRID: f32 = 1024.0;

/// `value` on the [`GRID`]. A grid point plus a whole number of pixels is
/// exact in `f32` up to 2^14 px (10 fraction bits + 14 integer bits = the
/// 24-bit mantissa), wider than any strip, so a snapped outline moved by a
/// whole-pixel transform lands on the same fractions wherever it goes.
pub(super) fn snap(value: f32) -> f32 {
    (value * GRID).round() / GRID
}

/// `path` with every point on the [`GRID`]; `None` only for a path left
/// with no length, which draws nothing.
pub(super) fn snapped(path: &Path) -> Option<Path> {
    let point = |p: tiny_skia::Point| (snap(p.x), snap(p.y));
    let mut out = PathBuilder::with_capacity(path.len(), path.len());
    for segment in path.segments() {
        match segment {
            PathSegment::MoveTo(p) => {
                let (x, y) = point(p);
                out.move_to(x, y);
            }
            PathSegment::LineTo(p) => {
                let (x, y) = point(p);
                out.line_to(x, y);
            }
            PathSegment::QuadTo(p1, p) => {
                let ((x1, y1), (x, y)) = (point(p1), point(p));
                out.quad_to(x1, y1, x, y);
            }
            PathSegment::CubicTo(p1, p2, p) => {
                let ((x1, y1), (x2, y2), (x, y)) = (point(p1), point(p2), point(p));
                out.cubic_to(x1, y1, x2, y2, x, y);
            }
            PathSegment::Close => out.close(),
        }
    }
    out.finish()
}

/// Stroke `path` into `pixmap` moved by `place`, a whole-pixel translate,
/// so that the pixels are the same bytes wherever `place` puts them
/// (ADR-0053 amendment: equal piece keys are equal pixels, and a piece is
/// the whole pass's pixels).
///
/// tiny-skia strokes a path *before* it moves the outline, and the move is
/// an `f32` add whose rounding depends on where it lands: the same outline
/// placed at two x could flip an anti-aliasing sample (seen: alpha 191 vs
/// 175 on one arc pixel). So the outline is made here, snapped to the
/// [`GRID`], and filled: the move is then exact. A stroke tiny-skia draws as
/// a hairline (anti-aliased, at most 1 px wide) rasterises the path itself,
/// so the path is snapped instead. Dashes are cut first, likewise snapped.
pub(super) fn stroke_placed(
    pixmap: &mut Pixmap,
    path: &Path,
    paint: &Paint<'_>,
    stroke: &Stroke,
    place: Transform,
) {
    debug_assert!(
        !place.has_scale() && !place.has_skew(),
        "a placement is a translate"
    );
    let dashed;
    let path = match &stroke.dash {
        // `dash` is `None` only when nothing of the path is left to draw.
        Some(dash) => match path.dash(dash, 1.0) {
            Some(path) => {
                dashed = path;
                &dashed
            }
            None => return,
        },
        None => path,
    };
    let solid = Stroke {
        dash: None,
        ..stroke.clone()
    };
    if paint.anti_alias && solid.width <= 1.0 {
        if let Some(path) = snapped(path) {
            pixmap.stroke_path(&path, paint, &solid, place, None);
        }
        return;
    }
    // `stroke` is `None` only for an outline with no area: nothing to fill.
    if let Some(outline) = path.stroke(&solid, 1.0).as_ref().and_then(snapped) {
        pixmap.fill_path(&outline, paint, FillRule::Winding, place, None);
    }
}

/// Fill `path` into `pixmap` moved by `place`, snapped as
/// [`stroke_placed`] says.
pub(super) fn fill_placed(pixmap: &mut Pixmap, path: &Path, paint: &Paint<'_>, place: Transform) {
    if let Some(path) = snapped(path) {
        pixmap.fill_path(&path, paint, FillRule::Winding, place, None);
    }
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
#[allow(
    clippy::cast_precision_loss,
    reason = "Cell sizes are far below f32's 2^24 exact-integer range."
)]
pub(super) fn pixels(value: u32) -> f32 {
    value as f32
}

/// A whole, non-negative coordinate as a pixel index.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "Callers pass whole pixels inside the pixmap."
)]
pub(super) fn whole(value: f32) -> usize {
    value as usize
}

/// Bytes in one scanline `width` pixels wide.
pub(super) fn pixels_len(width: u32) -> usize {
    usize::try_from(width).map_or(usize::MAX, |width| width * 4)
}

/// A row or column index as a coordinate multiplier.
#[allow(
    clippy::cast_precision_loss,
    reason = "Strip rows and lanes are far below f32's 2^24 exact-integer range."
)]
pub(super) fn count(value: usize) -> f32 {
    value as f32
}
