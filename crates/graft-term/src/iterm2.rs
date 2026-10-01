//! iTerm2 inline images (OSC 1337): rasterised strips to bytes.
//!
//! The protocol has no image ids and no crops, so every frame sends each
//! visible band again (ADR-0012). What it does not have to do is encode it
//! again: a band's PNG is kept, keyed by what went into it — the strip's
//! pixels, the crop of rows (the offset) and the cell rectangle it is scaled
//! into (the viewport) — so a frame whose bands did not change only moves the
//! cursor and writes bytes it already has.
//!
//! Rules, each pinned by a byte-stream test:
//!
//! - an image is written **at the cursor**, so each band is preceded by a
//!   cursor move and wrapped in save/restore (`ESC 7` … `ESC 8`), with
//!   `doNotMoveCursor=1` so a band ending on the last row cannot scroll;
//! - it is scaled into a **cell rectangle** (`width`, `height` in cells,
//!   `preserveAspectRatio=0`), so alignment never depends on pixel-exact cell
//!   size;
//! - an iTerm2 image is cell content: it stays until something overwrites
//!   those cells. The cells a frame covered are **erased first** (`ECH`)
//!   next frame, so a band that moved or narrowed leaves nothing behind.
//!
//! The encoder is pure: bytes go into a caller's buffer.

use std::io::{self, Write as _};
use std::ops::Range;
use std::sync::Arc;

use graft_core::raster::RasterFrame;

use crate::kitty::{self, CellRect, Place};
use crate::tmux::Transport;

/// One encoded band and the key it was encoded under.
#[derive(Debug)]
struct Band {
    first_row: usize,
    frame: Arc<RasterFrame>,
    rows: Range<usize>,
    columns: u16,
    cell_rows: u16,
    /// The whole OSC 1337 sequence, base64 payload included.
    sequence: Vec<u8>,
    last_used: u64,
}

/// Encoded bands kept for re-sending, and the cells the last frame covered.
#[derive(Debug)]
pub struct ItermImages {
    bands: Vec<Band>,
    capacity: usize,
    covered: Vec<CellRect>,
    encoded: u64,
    clock: u64,
    transport: Transport,
}

impl ItermImages {
    /// Keep at most `capacity` encoded bands (every band of the current
    /// frame is kept, whatever the capacity).
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            bands: Vec::new(),
            capacity: capacity.max(1),
            covered: Vec::new(),
            encoded: 0,
            clock: 0,
            transport: Transport::Direct,
        }
    }

    /// Send each band's `OSC 1337` over `transport`: inside tmux, one
    /// passthrough wrapper per band; the cursor moves and erases stay outside
    /// it, for tmux to see.
    #[must_use]
    pub fn with_transport(mut self, transport: Transport) -> Self {
        self.transport = transport;
        self
    }

    /// PNG encodes done since this was made: a band re-sent unchanged does
    /// not count.
    #[must_use]
    pub fn encoded(&self) -> u64 {
        self.encoded
    }

    /// Append the bytes that make the graph column show exactly `places`:
    /// erase what the last frame covered, then write every band at its cell.
    ///
    /// # Errors
    ///
    /// A band the PNG encoder refused (an empty or malformed frame). Bands
    /// before it are already in `out`; the cells are erased either way.
    pub fn frame(&mut self, places: &[Place<'_>], out: &mut Vec<u8>) -> io::Result<()> {
        self.clock += 1;
        self.hide(out);
        for place in places {
            let index = self.band_for(place)?;
            let band = &mut self.bands[index];
            band.last_used = self.clock;
            out.extend_from_slice(b"\x1b7");
            move_to(out, place.cells.row, place.cells.column);
            self.transport.write(&band.sequence, out);
            out.extend_from_slice(b"\x1b8");
            self.covered.push(CellRect {
                rows: band.cell_rows,
                ..place.cells
            });
        }
        self.evict();
        Ok(())
    }

    /// Erase every cell the last frame drew an image into (the graph is not
    /// on screen, or the tier dropped away from pixels). Encoded bands stay.
    pub fn hide(&mut self, out: &mut Vec<u8>) {
        if self.covered.is_empty() {
            return;
        }
        out.extend_from_slice(b"\x1b7");
        for rect in self.covered.drain(..) {
            for row in rect.row..rect.row.saturating_add(rect.rows) {
                move_to(out, row, rect.column);
                // `write!` into a `Vec` cannot fail.
                write!(out, "\x1b[{}X", rect.columns).ok();
            }
        }
        out.extend_from_slice(b"\x1b8");
    }

    /// The index of `place`'s encoded band, encoding it on a miss.
    fn band_for(&mut self, place: &Place<'_>) -> io::Result<usize> {
        let cell_rows = if place.cells.rows == 0 {
            u16::try_from(place.rows.len()).unwrap_or(u16::MAX)
        } else {
            place.cells.rows
        };
        if let Some(index) = self.bands.iter().position(|band| {
            band.first_row == place.first_row
                && Arc::ptr_eq(&band.frame, place.frame)
                && band.rows == place.rows
                && band.columns == place.cells.columns
                && band.cell_rows == cell_rows
        }) {
            return Ok(index);
        }
        let png = band_png(place.frame, &place.rows)?;
        self.encoded += 1;
        let mut sequence = Vec::with_capacity(png.len().div_ceil(3) * 4 + 96);
        // `write!` into a `Vec` cannot fail.
        write!(
            sequence,
            "\x1b]1337;File=inline=1;size={};width={};height={cell_rows};\
             preserveAspectRatio=0;doNotMoveCursor=1:",
            png.len(),
            place.cells.columns,
        )
        .ok();
        kitty::base64(&png, &mut sequence);
        sequence.push(0x07);
        self.bands.push(Band {
            first_row: place.first_row,
            frame: Arc::clone(place.frame),
            rows: place.rows.clone(),
            columns: place.cells.columns,
            cell_rows,
            sequence,
            last_used: self.clock,
        });
        Ok(self.bands.len() - 1)
    }

    /// Drop least-recently-used bands past capacity, never one of this frame.
    fn evict(&mut self) {
        while self.bands.len() > self.capacity {
            let Some(index) = self
                .bands
                .iter()
                .enumerate()
                .filter(|(_, band)| band.last_used != self.clock)
                .min_by_key(|(_, band)| band.last_used)
                .map(|(index, _)| index)
            else {
                return;
            };
            self.bands.remove(index);
        }
    }
}

/// `CSI row ; column H`, one-based.
fn move_to(out: &mut Vec<u8>, row: u16, column: u16) {
    // `write!` into a `Vec` cannot fail.
    write!(
        out,
        "\x1b[{};{}H",
        u32::from(row) + 1,
        u32::from(column) + 1
    )
    .ok();
}

/// The strip's `rows` as a straight-alpha RGBA PNG.
fn band_png(frame: &RasterFrame, rows: &Range<usize>) -> io::Result<Vec<u8>> {
    let cell_height = frame.height as usize / frame.rows.max(1);
    let row_bytes = frame.width as usize * 4;
    let start = rows.start * cell_height * row_bytes;
    let end = rows.end * cell_height * row_bytes;
    let pixels = frame.pixels.get(start..end).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("band rows {rows:?} lie outside a {}-row strip", frame.rows),
        )
    })?;
    let height = u32::try_from(rows.len() * cell_height)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "band too tall"))?;

    let mut png = Vec::new();
    let mut encoder = png::Encoder::new(&mut png, frame.width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    // Encode time is frame budget on this tier (ADR-0012).
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer
        .write_image_data(&kitty::straight_alpha(pixels))
        .map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)?;
    Ok(png)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A strip of `rows` rows, 3 columns of 2×4 px cells, one colour.
    fn strip(rows: usize) -> Arc<RasterFrame> {
        let height = u32::try_from(rows).unwrap() * 4;
        Arc::new(RasterFrame {
            width: 6,
            height,
            rows,
            columns: 3,
            pixels: [128, 0, 0, 128].repeat(6 * height as usize),
        })
    }

    fn place(frame: &Arc<RasterFrame>, rows: Range<usize>, row: u16) -> Place<'_> {
        Place {
            first_row: 0,
            frame,
            cells: CellRect {
                column: 2,
                row,
                columns: 3,
                rows: u16::try_from(rows.len()).unwrap(),
            },
            rows,
            layer: crate::kitty::Layer::Strip,
        }
    }

    fn text(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[test]
    fn a_band_is_written_at_its_cell_scaled_into_its_cells() {
        let frame = strip(8);
        let mut images = ItermImages::new(4);
        let mut out = Vec::new();
        images.frame(&[place(&frame, 2..6, 5)], &mut out).unwrap();
        let out = text(&out);
        let prefix = "\x1b7\x1b[6;3H\x1b]1337;File=inline=1;size=";
        assert!(out.starts_with(prefix), "{out:?}");
        assert!(
            out.contains(";width=3;height=4;preserveAspectRatio=0;doNotMoveCursor=1:iVBORw0KGgo"),
            "a PNG, base64: {out:?}"
        );
        assert!(out.ends_with("\x07\x1b8"), "{out:?}");
    }

    #[test]
    fn the_size_argument_is_the_png_byte_count() {
        let frame = strip(8);
        let mut out = Vec::new();
        ItermImages::new(4)
            .frame(&[place(&frame, 0..8, 0)], &mut out)
            .unwrap();
        let png = band_png(&frame, &(0..8)).unwrap();
        assert!(text(&out).contains(&format!("size={};", png.len())));
        // 6×32 px, straight alpha: the premultiplied half-red is full red.
        let decoder = png::Decoder::new(std::io::Cursor::new(png));
        let mut reader = decoder.read_info().unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!((info.width, info.height), (6, 32));
        assert_eq!(&pixels[..4], &[255, 0, 0, 128]);
    }

    #[test]
    fn an_unchanged_band_is_resent_but_not_re_encoded() {
        let frame = strip(8);
        let mut images = ItermImages::new(4);
        let mut first = Vec::new();
        images.frame(&[place(&frame, 0..8, 1)], &mut first).unwrap();
        let mut second = Vec::new();
        images
            .frame(&[place(&frame, 0..8, 1)], &mut second)
            .unwrap();
        assert_eq!(images.encoded(), 1);
        let (first, second) = (text(&first), text(&second));
        let sequence = |stream: &str| stream[stream.find("\x1b]1337").unwrap()..].to_owned();
        assert_eq!(sequence(&first), sequence(&second), "same bytes re-sent");
    }

    #[test]
    fn a_band_moved_on_screen_is_not_re_encoded_but_a_new_crop_is() {
        let frame = strip(8);
        let mut images = ItermImages::new(4);
        let mut out = Vec::new();
        images.frame(&[place(&frame, 0..8, 1)], &mut out).unwrap();
        // Same crop, same cells, lower on screen (a resize moved the pane).
        images.frame(&[place(&frame, 0..8, 3)], &mut out).unwrap();
        assert_eq!(images.encoded(), 1);
        // Scrolled one row: a different crop is a different band.
        images.frame(&[place(&frame, 1..8, 1)], &mut out).unwrap();
        assert_eq!(images.encoded(), 2);
    }

    #[test]
    fn new_pixels_under_the_same_rows_are_encoded_again() {
        let (old, new) = (strip(8), strip(8));
        let mut images = ItermImages::new(4);
        let mut out = Vec::new();
        images.frame(&[place(&old, 0..8, 0)], &mut out).unwrap();
        images.frame(&[place(&new, 0..8, 0)], &mut out).unwrap();
        assert_eq!(images.encoded(), 2);
    }

    #[test]
    fn the_next_frame_erases_the_cells_the_last_one_covered() {
        let frame = strip(8);
        let mut images = ItermImages::new(4);
        images
            .frame(&[place(&frame, 0..2, 4)], &mut Vec::new())
            .unwrap();
        let mut out = Vec::new();
        images.frame(&[], &mut out).unwrap();
        assert_eq!(text(&out), "\x1b7\x1b[5;3H\x1b[3X\x1b[6;3H\x1b[3X\x1b8");
        // Nothing covered any more: hiding again writes nothing.
        let mut out = Vec::new();
        images.hide(&mut out);
        assert!(out.is_empty());
    }

    /// Inside tmux each band is one passthrough and the erase/move stay
    /// outside; what tmux forwards is the direct stream exactly.
    #[test]
    fn through_tmux_each_band_is_wrapped_and_the_cursor_moves_are_not() {
        let frame = strip(4);
        let places = [place(&frame, 0..2, 5), place(&frame, 2..4, 7)];
        let mut direct = Vec::new();
        ItermImages::new(4).frame(&places, &mut direct).unwrap();
        let mut tmux = Vec::new();
        let mut images = ItermImages::new(4).with_transport(Transport::TmuxPassthrough);
        images.frame(&places, &mut tmux).unwrap();

        let (forwarded, wrappers) = crate::tmux::unwrap_all(&tmux);
        assert_eq!(forwarded, direct);
        assert_eq!(wrappers, 2);
        assert!(text(&tmux).starts_with("\x1b7\x1b[6;3H\x1bPtmux;\x1b\x1b]1337;File="));

        let mut erase = Vec::new();
        images.hide(&mut erase);
        assert!(
            !erase.windows(2).any(|pair| pair == b"\x1bP"),
            "erases go to tmux"
        );
    }

    #[test]
    fn a_crop_outside_the_strip_is_an_error_not_a_panic() {
        let frame = strip(4);
        let mut images = ItermImages::new(4);
        let error = images
            .frame(&[place(&frame, 2..6, 0)], &mut Vec::new())
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn bands_past_capacity_are_dropped_oldest_first() {
        let frame = strip(8);
        let mut images = ItermImages::new(1);
        let mut out = Vec::new();
        images.frame(&[place(&frame, 0..8, 0)], &mut out).unwrap();
        images.frame(&[place(&frame, 1..8, 0)], &mut out).unwrap();
        // The first crop was evicted, so showing it again encodes it again.
        images.frame(&[place(&frame, 0..8, 0)], &mut out).unwrap();
        assert_eq!(images.encoded(), 3);
    }
}
