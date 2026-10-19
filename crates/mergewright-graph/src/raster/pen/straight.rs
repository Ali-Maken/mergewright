//! Straight runs for [`super::Pen`]: a lane passing through many rows,
//! drawn by copying one rasterised scanline where the pixmap is clear.

use tiny_skia::{Pixmap, Transform};

use super::super::geometry::{count, line, pixels, pixels_len, stroke, whole};
use super::super::stroke::double_offset;
use super::super::{Ink, LaneStroke, Pen, Scanline};

impl Pen<'_> {
    /// A straight run down `x` from `top` to `bottom`, both whole pixels.
    ///
    /// Anti-aliased stroking costs per scanline, and a lane passing through
    /// a strip is most of its scanlines, so where the pixels under the line
    /// are still clear the scanline rasterised once is copied; where
    /// something is already drawn (an arc crossing), those rows are stroked
    /// as before, so blending stays tiny-skia's. Byte-identical either way
    /// (`tests/it/raster_pixels.rs`).
    pub(super) fn straight(&self, pixmap: &mut Pixmap, x: f32, top: f32, bottom: f32, ink: Ink) {
        match ink.shape {
            LaneStroke::Thick | LaneStroke::Thin => {}
            // Two plain runs either side of the centre line: each keeps the
            // copied-scanline path.
            LaneStroke::Double => {
                let offset = double_offset(ink.width);
                let plain = Ink {
                    shape: LaneStroke::Thin,
                    ..ink
                };
                self.straight(pixmap, x - offset, top, bottom, plain);
                self.straight(pixmap, x + offset, top, bottom, plain);
                return;
            }
            // Dashes are not one scanline repeated: stroked whole.
            LaneStroke::Dotted => {
                stroke(pixmap, line(x, top, x, bottom), ink, self.shift);
                return;
            }
        }
        // The scanline is sampled at the pixmap's x: whole pixels apart
        // from the strip's, so the same fraction and the same bytes.
        let Some(index) = self.scanline(pixmap.width(), x - self.origin, ink) else {
            stroke(pixmap, line(x, top, x, bottom), ink, self.shift);
            return;
        };
        let scanlines = self.scanlines.borrow();
        let scanline = &scanlines[index];
        let stride = pixels_len(pixmap.width());
        let (left, span) = (scanline.left * 4, scanline.pixels.len());
        // Pixmap rows: the strip's, less the pixmap's top (whole pixels).
        let (first, last) = (whole(top - self.top), whole(bottom - self.top));
        let strip_y = |y: usize| count(y) + self.top;
        let mut dirty: Option<usize> = None;
        for y in first..last {
            let start = y * stride + left;
            let under = &mut pixmap.data_mut()[start..start + span];
            if under.iter().all(|&byte| byte == 0) {
                under.copy_from_slice(&scanline.pixels);
                if let Some(from) = dirty.take() {
                    stroke(
                        pixmap,
                        line(x, strip_y(from), x, strip_y(y)),
                        ink,
                        self.shift,
                    );
                }
            } else if dirty.is_none() {
                dirty = Some(y);
            }
        }
        if let Some(from) = dirty {
            stroke(
                pixmap,
                line(x, strip_y(from), x, strip_y(last)),
                ink,
                self.shift,
            );
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
        stroke(
            &mut sample,
            line(local, 0.0, local, 3.0),
            ink,
            Transform::identity(),
        );
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
}
