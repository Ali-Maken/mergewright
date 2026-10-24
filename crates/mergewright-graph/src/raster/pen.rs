//! Drawing methods for [`super::Pen`]: bands, strokes and node discs
//! into the strip's pixmap.

pub(super) mod continuation;
mod straight;

use tiny_skia::{Pixmap, Rect, Transform};

use super::geometry::{arc_in, arc_out, count, goes_down, line, paint, pixels, stroke};
use super::{
    Band, BranchIdentity, CELLS_PER_LANE, Edge, Ink, Inked, LaneStroke, Look, Marker, Part, Pen,
    PieceKind, RefCell, Row, STRIP_LEAD_CELLS, Toward, alpha_byte, edge_at, lifted, node,
    stroke_width,
};

/// x of a lane column's centre in strip pixels: the centre of the lane's own
/// cell, where the Unicode tier draws the node glyph, counted from the
/// strip's left edge, which is [`STRIP_LEAD_CELLS`] cells left of the graph
/// column (D5). Lane 0's node so has a full cell of room on its left.
pub fn lane_centre_x(column: usize, cell_width: f32) -> f32 {
    let cells = pixels(STRIP_LEAD_CELLS) + count(column) * pixels(CELLS_PER_LANE);
    cells * cell_width + cell_width / 2.0
}

impl<'a> Pen<'a> {
    pub(super) fn new(
        look: &'a Look<'a>,
        rows: &'a [Inked<'a>],
        lift: Option<f32>,
        part: Option<Part>,
        window: std::ops::Range<usize>,
    ) -> Self {
        let cell_width = pixels(look.cell_width);
        let cell_height = pixels(look.cell_height);
        let origin = part.map_or(0.0, |part| pixels(part.first_cell) * cell_width);
        let top = part.map_or(0.0, |part| count(part.first_row) * cell_height);
        let stroke = stroke_width(look.cell_width);
        Pen {
            look,
            rows,
            lift,
            only: part.map(|part| (part.column, part.kind)),
            window,
            origin,
            top,
            shift: Transform::from_translate(-origin, -top),
            cell_width,
            cell_height,
            stroke,
            node: node::NodeSize::for_cell(cell_width, cell_height, stroke),
            scanlines: RefCell::new(Vec::new()),
        }
    }

    /// The whole pass, or the piece's share of it.
    pub(super) fn pass(&self, pixmap: &mut Pixmap) {
        // Edges column by column down the whole strip, so a lane passing
        // straight through many rows is one stroke, not one per row. Within
        // any one band the order is unchanged (columns left to right), and
        // edges stay confined to their band, so the pixels are the per-row
        // drawing's.
        match self.only {
            Some((column, PieceKind::Run | PieceKind::Arc { .. })) => self.column(pixmap, column),
            Some((_, PieceKind::Node)) => {}
            None => {
                let widest = self
                    .rows
                    .iter()
                    .map(|inked| inked.row.lanes.len())
                    .max()
                    .unwrap_or(0);
                for column in 0..widest {
                    self.column(pixmap, column);
                }
            }
        }
        // Then what sits on each row's node: its lane's own stubs, a folded
        // edge's tail, the rail, the node disc punched over the edges. A run
        // piece has none of these, and no disc reaches another lane's run.
        if self.only.is_some_and(|(_, kind)| kind == PieceKind::Run) {
            return;
        }
        for index in self.window.clone() {
            let above = match index.checked_sub(1) {
                Some(previous) => Some(self.rows[previous].row),
                None => self.look.above,
            };
            self.row(pixmap, index, &self.rows[index], above);
        }
    }

    /// The x of a lane column's centre line, in strip pixels.
    fn x(&self, column: usize) -> f32 {
        lane_centre_x(column, self.cell_width)
    }

    /// Whether this pass draws what a node piece of lane column `column`
    /// owns: the nodes, lane stubs and tails of rows on that lane, and the
    /// rail when it is the marker column.
    fn owns_node(&self, column: usize) -> bool {
        self.only
            .is_none_or(|only| only == (column, PieceKind::Node))
    }

    /// Whether this pass draws edge `edge` of lane column `column` on strip
    /// row `index`: a run piece its straight lines, an arc piece the arc on
    /// its row.
    fn owns_edge(&self, column: usize, index: usize, edge: Edge) -> bool {
        match (self.only, edge) {
            (None, _) => true,
            (Some((only, PieceKind::Run)), Edge::Straight) => only == column,
            (Some((only, PieceKind::Arc { row })), Edge::Arc { .. }) => {
                only == column && usize::from(row) == index
            }
            _ => false,
        }
    }

    /// `alpha` is 0.0–1.0; the stroke width never varies with emphasis
    /// (ADR-0024) so a line reads as a line whether it is the thread or not.
    /// A role may scale the width and shape the stroke
    /// ([`super::LaneStroke`]); the muted hue (`identity` `None`) is one
    /// plain stroke.
    fn ink(&self, identity: Option<BranchIdentity>, alpha: f32) -> Ink {
        let (rgb, shape) = identity.map_or((self.look.muted, LaneStroke::Thin), |identity| {
            self.look.lane(identity)
        });
        Ink {
            rgb,
            alpha: alpha_byte(alpha),
            width: shape.width(self.stroke),
            shape,
        }
    }

    /// The alpha an element of ADR-0024 alpha `alpha` draws at in this
    /// pass, or `None` when this pass leaves it out.
    ///
    /// A whole strip draws every element at its own alpha. An emphasis
    /// layer over a backdrop of alpha `b` draws only what is above `b`, at
    /// `(a − b) / (1 − b)`: over the backdrop's copy at `b` that is `a`
    /// again. An element *below* `b` cannot be expressed — the backdrop
    /// already drew it brighter — and no caller builds one (`Backdrop` is
    /// the lowest alpha in play).
    fn tone(&self, alpha: f32) -> Option<f32> {
        let Some(backdrop) = self.lift else {
            return Some(alpha);
        };
        if let Some(lifted) = lifted(alpha, backdrop) {
            return Some(lifted);
        }
        debug_assert!(
            alpha >= backdrop,
            "an element at alpha {alpha} under a backdrop at {backdrop} would be overdrawn by it"
        );
        None
    }

    /// An element anchored on lane column `column`'s own cell and strip
    /// row `index`, as it is built: its band in coordinates local to that
    /// cell's top-left corner (top 0), and the whole-pixel move placing it
    /// in the pixmap. Built local, an element's outline is the same bytes
    /// wherever it sits, so equal piece keys are equal pixels; moved after
    /// stroking by whole pixels, it is the whole pass's too (`shift`).
    /// Local, the anchor lane's centre is `self.x(0)`, the next's `self.x(1)`.
    fn local(&self, column: usize, index: usize) -> (Band, Transform) {
        let left = count(column) * pixels(CELLS_PER_LANE) * self.cell_width;
        let top = self.band(index).top;
        let band = Band {
            top: 0.0,
            mid: self.cell_height / 2.0,
            bottom: self.cell_height,
        };
        (
            band,
            Transform::from_translate(left - self.origin, top - self.top),
        )
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
        let x = self.x(column);
        let flush = |pixmap: &mut Pixmap, run: Option<(f32, f32, Ink)>| {
            if let Some((top, bottom, ink)) = run {
                self.straight(pixmap, x, top, bottom, ink);
            }
        };
        // The straight run being extended: its top, its bottom, its ink.
        let mut run: Option<(f32, f32, Ink)> = None;
        for index in self.window.clone() {
            let inked = &self.rows[index];
            let row = inked.row;
            let band = self.band(index);
            // An element this pass leaves out ends the run as a gap would.
            let Some((identity, edge)) = edge_at(row, self.look.marker, column)
                .filter(|&(_, edge)| self.owns_edge(column, index, edge))
            else {
                flush(pixmap, run.take());
                continue;
            };
            let Some(alpha) = self.tone(inked.lit.alpha_at(column)) else {
                flush(pixmap, run.take());
                continue;
            };
            let ink = self.ink(Some(identity), alpha);
            // An arc is built local to its own column and row (`local`).
            let (local, place) = self.local(column, index);
            let (lane, node) = (self.x(0), self.x(row.lane) - (x - self.x(0)));
            match edge {
                Edge::Arc { incoming: true } => {
                    flush(pixmap, run.take());
                    stroke(
                        pixmap,
                        arc_in((lane, local.top), (node, local.mid)),
                        ink,
                        place,
                    );
                }
                Edge::Arc { incoming: false } => {
                    flush(pixmap, run.take());
                    stroke(
                        pixmap,
                        arc_out((node, local.mid), (lane, local.bottom)),
                        ink,
                        place,
                    );
                }
                Edge::Straight => {
                    run = match run {
                        Some((top, _, run_ink)) if run_ink == ink => Some((top, band.bottom, ink)),
                        other => {
                            flush(pixmap, other);
                            Some((band.top, band.bottom, ink))
                        }
                    };
                }
            }
        }
        flush(pixmap, run);
    }

    /// A row's own lane stubs, its folded edge's tail, the rail and its node,
    /// drawn over the edges.
    fn row(&self, pixmap: &mut Pixmap, index: usize, inked: &Inked<'_>, above: Option<&Row>) {
        let band = self.band(index);
        let row = inked.row;
        // Stubs, tail and node are built local to the node's lane and row.
        let (local, place) = self.local(row.lane, index);
        let lane_ink = self
            .owns_node(row.lane)
            .then(|| self.tone(inked.lit.alpha_at(row.lane)))
            .flatten()
            .map(|alpha| self.ink(row.lanes.get(row.lane).copied().flatten(), alpha));
        let x = self.x(0);
        if let Some(lane_ink) = lane_ink {
            if above.is_some_and(|above| goes_down(above, row.lane)) {
                stroke(pixmap, line(x, local.top, x, local.mid), lane_ink, place);
            }
            if row.outgoing.iter().any(|&(column, _)| column == row.lane) {
                stroke(pixmap, line(x, local.mid, x, local.bottom), lane_ink, place);
            }
        }
        if let Some(marker) = self.look.marker {
            self.rail(pixmap, inked, marker, band);
        }
        // The disc is punched whether or not this pass draws the node: a
        // lit edge under an unlit node is cleared in the single pass too.
        // A ring is one plain stroke at the strip's width whatever its
        // lane's role: the node's shape is its ring style alone.
        let ring_ink = lane_ink.map(|ink| Ink {
            width: self.stroke,
            shape: LaneStroke::Thin,
            ..ink
        });
        node::draw(
            pixmap,
            inked.ring,
            (x, local.mid),
            self.node,
            ring_ink,
            place,
        );
        self.tail(pixmap, inked, local, place);
    }

    /// A folded edge's stub (ADR-0022), ease-pass/29 D3: a short straight
    /// tail in the node's own lane, on the ray from the node's centre to the
    /// lane's corner on its side (down-right for `Below`, up-right for
    /// `Above`). It starts at the node's rim, over the punched gap, and ends
    /// where its stroke is still one pixel inside the lane and the row, so it
    /// never reaches another lane or the image edge; the Unicode tier puts
    /// its `▾`/`▴` in the same gap cell. Several folded edges on a row draw
    /// one tail, in the first edge's colour.
    /// `band` and `place` are the node's, local (`local`); drawn after the
    /// node, whose punched disc would otherwise clear its start.
    fn tail(&self, pixmap: &mut Pixmap, inked: &Inked<'_>, band: Band, place: Transform) {
        let Some(first) = inked.stubs.first() else {
            return;
        };
        let row = inked.row;
        if !self.owns_node(row.lane) {
            return;
        }
        let Some(alpha) = self.tone(inked.lit.alpha_at(row.lane)) else {
            return;
        };
        let ink = self.ink(Some(first.identity), alpha);
        let centre = (self.x(0), band.mid);
        let corner_dy = match first.toward {
            Toward::Below => self.cell_height / 2.0,
            Toward::Above => -self.cell_height / 2.0,
        };
        let norm = self.cell_width.hypot(corner_dy);
        let (ux, uy) = (self.cell_width / norm, corner_dy / norm);
        // Longest reach keeping the stroke's half-width and a pixel inside.
        let margin = 1.0 + ink.width / 2.0;
        let along_x = (self.cell_width - margin) / ux;
        let along_y = (self.cell_height / 2.0 - margin) / uy.abs();
        let end = along_x.min(along_y);
        let start = self.node.reach(inked.ring);
        if end <= start {
            return;
        }
        let point = |t: f32| (centre.0 + ux * t, centre.1 + uy * t);
        let (from, to) = (point(start), point(end));
        stroke(pixmap, line(from.0, from.1, to.0, to.1), ink, place);
    }

    /// The rail on one band: a 1 px dotted vertical on the marker column's
    /// centre line (where the Unicode tier puts its `⋮`), in the muted hue.
    /// Dashes are a sixth of a band on, a sixth off, starting at the band's
    /// top, so every band draws the same three dashes and the pattern never
    /// jumps at a row or strip edge.
    fn rail(&self, pixmap: &mut Pixmap, inked: &Inked<'_>, marker: Marker, band: Band) {
        if !self.owns_node(marker.column) {
            return;
        }
        let Some(alpha) = self.tone(inked.lit.rail_alpha()) else {
            return;
        };
        let paint = paint(self.ink(None, alpha));
        // A whole pixel column, so the rail is crisp, not two half-lit ones.
        // Moved to the pixmap by hand: `fill_rect` under a transform is a
        // path fill, which anti-aliases differently.
        let left = (self.x(marker.column) - 0.5).round() + self.shift.tx;
        let dash = self.cell_height / 6.0;
        for index in 0..3 {
            let top = band.top + count(index) * 2.0 * dash + self.shift.ty;
            // `from_xywh` is `None` only for an empty or non-finite rectangle.
            if let Some(rect) = Rect::from_xywh(left, top, 1.0, dash) {
                pixmap.fill_rect(rect, &paint, Transform::identity(), None);
            }
        }
    }
}

#[cfg(test)]
mod tests;
