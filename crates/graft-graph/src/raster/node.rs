//! Commit nodes: a disc punched clear of the lines under it, then the node
//! drawn in its ring style. Styles differ in *shape*, never only in colour,
//! so they read with any palette — the Unicode tier's rule for its glyphs.

use tiny_skia::{FillRule, PathBuilder, Pixmap, Rect, Stroke, StrokeDash, Transform};

use super::Ink;
use super::geometry::{eraser, paint};
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
    /// The ring a row's parent count implies: none is a root, several a
    /// merge. `Head`, `Stash` and `Uncommitted` are the caller's to set.
    pub fn of(row: &Row) -> Ring {
        match row.commit.parents.len() {
            0 => Ring::Root,
            1 => Ring::Commit,
            _ => Ring::Merge,
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

/// Draw one node centred at `centre`.
pub(super) fn draw(pixmap: &mut Pixmap, ring: Ring, centre: (f32, f32), size: NodeSize, ink: Ink) {
    let NodeSize {
        radius,
        ring: thickness,
        gap,
    } = size;
    let reach = match ring {
        Ring::Head => radius * 1.4,
        _ => radius,
    };
    fill_circle(pixmap, centre, reach + gap, None);
    match ring {
        Ring::Commit => fill_circle(pixmap, centre, radius, Some(ink)),
        Ring::Merge => {
            stroke_circle(
                pixmap,
                centre,
                radius - thickness / 2.0,
                thickness,
                ink,
                false,
            );
            fill_circle(pixmap, centre, radius * 0.4, Some(ink));
        }
        Ring::Root => stroke_circle(
            pixmap,
            centre,
            radius - thickness / 2.0,
            thickness,
            ink,
            false,
        ),
        Ring::Head => {
            fill_circle(pixmap, centre, radius, Some(ink));
            let halo = thickness * 0.6;
            stroke_circle(pixmap, centre, radius * 1.4 - halo / 2.0, halo, ink, false);
        }
        Ring::Stash => fill_square(pixmap, centre, radius * 0.85, ink),
        Ring::Uncommitted => {
            stroke_circle(
                pixmap,
                centre,
                radius - thickness / 2.0,
                thickness,
                ink,
                true,
            );
        }
    }
}

/// Fill a disc in `ink`, or erase it to transparent when `ink` is `None`.
fn fill_circle(pixmap: &mut Pixmap, (x, y): (f32, f32), radius: f32, ink: Option<Ink>) {
    // `from_circle` is `None` only for a non-positive radius: nothing to draw.
    let Some(path) = PathBuilder::from_circle(x, y, radius) else {
        return;
    };
    let paint = ink.map_or_else(eraser, paint);
    pixmap.fill_path(
        &path,
        &paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );
}

fn stroke_circle(
    pixmap: &mut Pixmap,
    (x, y): (f32, f32),
    radius: f32,
    width: f32,
    ink: Ink,
    dashed: bool,
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
    pixmap.stroke_path(&path, &paint(ink), &stroke, Transform::identity(), None);
}

fn fill_square(pixmap: &mut Pixmap, (x, y): (f32, f32), half: f32, ink: Ink) {
    // `from_ltrb` is `None` only for an empty or non-finite rectangle.
    let Some(rect) = Rect::from_ltrb(x - half, y - half, x + half, y + half) else {
        return;
    };
    pixmap.fill_rect(rect, &paint(ink), Transform::identity(), None);
}
