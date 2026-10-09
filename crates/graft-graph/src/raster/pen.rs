//! Drawing methods for [`super::Pen`]: bands, strokes and node discs
//! into the strip's pixmap.

use tiny_skia::{
    Color, GradientStop, LinearGradient, Paint, Pixmap, Point, Rect, SpreadMode, Transform,
};

use super::geometry::{
    arc_in, arc_out, count, goes_down, line, paint, pixels, pixels_len, stroke, stroke_with, whole,
};
use super::{
    Band, BranchIdentity, CELLS_PER_LANE, Ink, Inked, Look, Marker, Pen, RefCell, Rgb, Row,
    Scanline, Toward, node, stroke_width,
};

impl<'a> Pen<'a> {
    pub(super) fn new(look: &'a Look<'a>, rows: &'a [Inked<'a>], lift: Option<f32>) -> Self {
        let cell_width = pixels(look.cell_width);
        let cell_height = pixels(look.cell_height);
        let stroke = stroke_width(look.cell_width);
        Pen {
            look,
            rows,
            lift,
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
        let rgb = identity.map_or(self.look.muted, |identity| {
            self.look.palette[identity.slot(self.look.palette.len())]
        });
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "alpha is 0.0..=1.0."
        )]
        let alpha = (alpha.clamp(0.0, 1.0) * 255.0).round() as u8;
        Ink {
            rgb,
            alpha,
            width: self.stroke,
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
        if alpha > backdrop {
            return Some((alpha - backdrop) / (1.0 - backdrop));
        }
        debug_assert!(
            alpha >= backdrop,
            "an element at alpha {alpha} under a backdrop at {backdrop} would be overdrawn by it"
        );
        None
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
    pub(super) fn column(&self, pixmap: &mut Pixmap, column: usize) {
        let is_marker = self.look.marker.map(|marker| marker.column) == Some(column);
        let x = self.x(column);
        let flush = |pixmap: &mut Pixmap, run: Option<(f32, f32, Ink)>| {
            if let Some((top, bottom, ink)) = run {
                self.straight(pixmap, x, top, bottom, ink);
            }
        };
        // The straight run being extended: its top, its bottom, its ink.
        let mut run: Option<(f32, f32, Ink)> = None;
        for (index, inked) in self.rows.iter().enumerate() {
            let row = inked.row;
            let band = self.band(index);
            let identity = row.lanes.get(column).copied().flatten();
            if identity.is_none() || column == row.lane {
                flush(pixmap, run.take());
                continue;
            }
            let incoming = row.incoming.contains(&column);
            let outgoing = !incoming && row.outgoing.iter().any(|&(out, _)| out == column);
            // The marker column draws no edge of its own; an element this
            // pass leaves out ends the run as a gap would.
            let alpha = if incoming || outgoing || !is_marker {
                self.tone(inked.lit.alpha_at(column))
            } else {
                None
            };
            let Some(alpha) = alpha else {
                flush(pixmap, run.take());
                continue;
            };
            let ink = self.ink(identity, alpha);
            let node = self.x(row.lane);
            if incoming {
                flush(pixmap, run.take());
                stroke(pixmap, arc_in((x, band.top), (node, band.mid)), ink);
            } else if outgoing {
                flush(pixmap, run.take());
                stroke(pixmap, arc_out((node, band.mid), (x, band.bottom)), ink);
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
    pub(super) fn row(
        &self,
        pixmap: &mut Pixmap,
        index: usize,
        inked: &Inked<'_>,
        above: Option<&Row>,
    ) {
        let band = self.band(index);
        let row = inked.row;
        let lane_ink = self
            .tone(inked.lit.alpha_at(row.lane))
            .map(|alpha| self.ink(row.lanes.get(row.lane).copied().flatten(), alpha));
        let x = self.x(row.lane);
        if let Some(lane_ink) = lane_ink {
            if above.is_some_and(|above| goes_down(above, row.lane)) {
                stroke(pixmap, line(x, band.top, x, band.mid), lane_ink);
            }
            if row.outgoing.iter().any(|&(column, _)| column == row.lane) {
                stroke(pixmap, line(x, band.mid, x, band.bottom), lane_ink);
            }
        }
        self.tail(pixmap, inked, band);
        if let Some(marker) = self.look.marker {
            self.rail(pixmap, inked, marker, band);
        }
        // The disc is punched whether or not this pass draws the node: a
        // lit edge under an unlit node is cleared in the single pass too.
        node::draw(pixmap, inked.ring, (x, band.mid), self.node, lane_ink);
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
    fn tail(&self, pixmap: &mut Pixmap, inked: &Inked<'_>, band: Band) {
        let Some(first) = inked.stubs.first() else {
            return;
        };
        let row = inked.row;
        let Some(alpha) = self.tone(inked.lit.alpha_at(row.lane)) else {
            return;
        };
        let ink = self.ink(Some(first.identity), alpha);
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
    fn rail(&self, pixmap: &mut Pixmap, inked: &Inked<'_>, marker: Marker, band: Band) {
        let Some(alpha) = self.tone(inked.lit.rail_alpha()) else {
            return;
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
