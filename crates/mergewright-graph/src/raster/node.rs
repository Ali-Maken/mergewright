//! Commit nodes: a disc punched clear of the lines under it, then the node
//! drawn in its ring style. Styles differ in *shape*, never only in colour,
//! so they read with any palette — the Unicode tier's rule for its glyphs.

use tiny_skia::{PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

use super::Ink;
use super::geometry::{eraser, fill_placed, paint, snap, stroke_placed};
use crate::row::Row;

/// How a commit's node is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ring {
    /// A filled disc (Unicode `●`).
    Commit,
    /// A ring with a dot inside (`◉`).
    Merge,
    /// A hollow ring: nothing came before it (`◇`).
    Root,
    /// A filled disc inside a thin halo (`◆`).
    Head,
    /// A filled square (`■`).
    Stash,
    /// A dashed ring (`◌`).
    Uncommitted,
}

impl Ring {
    /// Node style from git's parents ([`Row::is_merge`]): a folded merge is
    /// `Merge`, as `mergewright_ui`'s `NodeKind::of` draws it; none is a root.
    /// Stash-blind: `Head`, `Stash` and `Uncommitted`, and a stash entry's
    /// plain style, are the caller's to set (ease-pass 06r).
    pub fn of(row: &Row) -> Ring {
        if row.is_merge() {
            Ring::Merge
        } else if row.commit.parents.is_empty() {
            Ring::Root
        } else {
            Ring::Commit
        }
    }
}

/// Node dimensions for one cell size.
#[derive(Clone, Copy, Debug)]
pub(super) struct NodeSize {
    /// The node's outer radius.
    radius: f32,
    /// Ring line thickness.
    ring: f32,
    /// Clear space between the node's outer edge and any line.
    gap: f32,
}

impl NodeSize {
    /// The largest node whose widest style (the head's halo) plus its gap
    /// stays inside its band and short of the next lane.
    pub(super) fn for_cell(cell_width: f32, cell_height: f32, stroke: f32) -> Self {
        let gap = stroke * 0.6;
        let room = (cell_height / 2.0).min(cell_width) - gap;
        // The head's halo reaches 1.4 radii (see `draw`).
        let radius = (room / 1.4).max(1.0);
        NodeSize {
            radius,
            ring: (radius * 0.25).max(1.0),
            gap,
        }
    }

    /// From the node's centre to the nearest point a line may touch beside
    /// a plain node: its outer radius plus the clear gap.
    pub(super) fn clearance(self) -> f32 {
        self.radius + self.gap
    }
}

/// Punch one node's disc clear, centred at `centre`, and draw the node in
/// `ink`; with no ink (an emphasis layer leaving the node out) only punch.
pub(super) fn draw(
    pixmap: &mut Pixmap,
    ring: Ring,
    centre: (f32, f32),
    size: NodeSize,
    ink: Option<Ink>,
    shift: Transform,
) {
    let NodeSize { radius, gap, .. } = size;
    let reach = match ring {
        Ring::Head => radius * 1.4,
        _ => radius,
    };
    fill_circle(pixmap, centre, reach + gap, None, shift);
    if let Some(ink) = ink {
        draw_ring(pixmap, ring, centre, size, ink, shift);
    }
}

/// The node itself in `ink`, its disc already punched.
fn draw_ring(
    pixmap: &mut Pixmap,
    ring: Ring,
    centre: (f32, f32),
    size: NodeSize,
    ink: Ink,
    shift: Transform,
) {
    let NodeSize {
        radius,
        ring: thickness,
        ..
    } = size;
    match ring {
        Ring::Commit => fill_circle(pixmap, centre, radius, Some(ink), shift),
        Ring::Merge => {
            stroke_circle(
                pixmap,
                centre,
                radius - thickness / 2.0,
                thickness,
                ink,
                false,
                shift,
            );
            fill_circle(pixmap, centre, radius * 0.4, Some(ink), shift);
        }
        Ring::Root => stroke_circle(
            pixmap,
            centre,
            radius - thickness / 2.0,
            thickness,
            ink,
            false,
            shift,
        ),
        Ring::Head => {
            fill_circle(pixmap, centre, radius, Some(ink), shift);
            let halo = thickness * 0.6;
            stroke_circle(
                pixmap,
                centre,
                radius * 1.4 - halo / 2.0,
                halo,
                ink,
                false,
                shift,
            );
        }
        Ring::Stash => fill_square(pixmap, centre, radius * 0.85, ink, shift),
        Ring::Uncommitted => {
            stroke_circle(
                pixmap,
                centre,
                radius - thickness / 2.0,
                thickness,
                ink,
                true,
                shift,
            );
        }
    }
}

/// Fill a disc in `ink`, or erase it to transparent when `ink` is `None`.
fn fill_circle(
    pixmap: &mut Pixmap,
    (x, y): (f32, f32),
    radius: f32,
    ink: Option<Ink>,
    shift: Transform,
) {
    // `from_circle` is `None` only for a non-positive radius: nothing to draw.
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };
    let paint = ink.map_or_else(eraser, paint);
    fill_placed(pixmap, &path, &paint, shift);
}

fn stroke_circle(
    pixmap: &mut Pixmap,
    (x, y): (f32, f32),
    radius: f32,
    width: f32,
    ink: Ink,
    dashed: bool,
    shift: Transform,
) {
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };
    // Eight dashes round the circumference, dash longer than gap.
    let segment = (std::f32::consts::TAU * radius / 8.0).max(1.0);
    let dash = if dashed {
        StrokeDash::new(vec![segment * 0.6, segment * 0.4], 0.0)
    } else {
        None
    };
    let stroke = Stroke {
        width,
        dash,
        ..Stroke::default()
    };
    stroke_placed(pixmap, &path, &paint(ink), &stroke, shift);
}

fn fill_square(pixmap: &mut Pixmap, (x, y): (f32, f32), half: f32, ink: Ink, shift: Transform) {
    // Moved by hand, after the strip-pixel edges are found: `fill_rect`
    // under a transform is a path fill, which anti-aliases differently.
    // Snapped first, as every outline is (`stroke_placed`), so the move is
    // exact wherever the square lands.
    let (left, right) = (snap(x - half) + shift.tx, snap(x + half) + shift.tx);
    let (top, bottom) = (snap(y - half) + shift.ty, snap(y + half) + shift.ty);
    // `from_ltrb` is `None` only for an empty or non-finite rectangle.
    let Some(rect) = Rect::from_ltrb(left, top, right, bottom) else {
        return;
    };
    pixmap.fill_rect(rect, &paint(ink), Transform::identity(), None);
}

#[cfg(test)]
mod tests;
