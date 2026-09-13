//! The pixel renderer's output: a premultiplied-alpha RGBA buffer and its
//! geometry.
//!
//! `graph` draws it and `term` encodes it into Kitty, iTerm2 or Sixel bytes.
//! Those two crates are siblings, so the shape they share lives here
//! (DESIGN.md §8.2).

/// One rasterised strip of graph rows.
///
/// `pixels` holds `width × height` pixels, four bytes each, row-major, in
/// **premultiplied** RGBA: each colour channel is already scaled by alpha,
/// so no channel exceeds its pixel's alpha. The background is never painted
/// (ADR-0003): every pixel no line or node touches is `[0, 0, 0, 0]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RasterFrame {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels: `rows × cell_height`.
    pub height: u32,
    /// Graph rows the strip covers, one band of `cell_height` pixels each.
    pub rows: usize,
    /// Terminal columns the strip covers, `cell_width` pixels each.
    pub columns: u32,
    /// Premultiplied RGBA bytes, `width × height × 4` of them.
    pub pixels: Vec<u8>,
}

impl RasterFrame {
    /// The premultiplied RGBA of the pixel at (`x`, `y`), or `None` outside
    /// the frame.
    pub fn pixel(&self, x: u32, y: u32) -> Option<[u8; 4]> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let start =
            usize::try_from((u64::from(y) * u64::from(self.width) + u64::from(x)) * 4).ok()?;
        let bytes = self.pixels.get(start..start + 4)?;
        Some([bytes[0], bytes[1], bytes[2], bytes[3]])
    }
}
