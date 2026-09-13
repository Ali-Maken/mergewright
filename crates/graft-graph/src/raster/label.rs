//! The `+N` marker's text, drawn from a tiny built-in bitmap font. A strip
//! has no font to hand, and the label only ever needs `+` and digits.

use tiny_skia::{Paint, Pixmap, Rect, Transform};

use super::{Ink, count, eraser, paint, pixels};

/// Glyph cells: 3 wide, 5 tall.
const GLYPH_WIDTH: u32 = 3;
const GLYPH_HEIGHT: u32 = 5;

/// `0`–`9` then `+`, 15 bits each, row-major, top-left in the high bit.
const GLYPHS: [u16; 11] = [
    0b111_101_101_101_111,
    0b010_110_010_010_111,
    0b111_001_111_100_111,
    0b111_001_111_001_111,
    0b101_101_111_001_001,
    0b111_100_111_001_111,
    0b111_100_111_101_111,
    0b111_001_001_001_001,
    0b111_101_111_101_111,
    0b111_101_111_001_111,
    0b000_010_111_010_000,
];
const PLUS: usize = 10;

/// Draw `+folded` centred on `centre`, on a cleared backing, scaled to the
/// cell height.
pub(super) fn draw(
    pixmap: &mut Pixmap,
    folded: usize,
    centre: (f32, f32),
    cell_height: f32,
    ink: Ink,
) {
    let glyphs: Vec<usize> = std::iter::once(PLUS)
        .chain(
            folded
                .to_string()
                .bytes()
                .map(|digit| usize::from(digit - b'0')),
        )
        .collect();
    let scale = (cell_height / 12.0).floor().max(1.0);
    let advance = pixels(GLYPH_WIDTH + 1) * scale;
    let width = count(glyphs.len()) * advance - scale;
    let height = pixels(GLYPH_HEIGHT) * scale;
    let left = (centre.0 - width / 2.0).round();
    let top = (centre.1 - height / 2.0).round();
    fill(
        pixmap,
        (
            left - scale,
            top - scale,
            width + 2.0 * scale,
            height + 2.0 * scale,
        ),
        &eraser(),
    );
    let mut crisp = paint(ink);
    crisp.anti_alias = false;
    for (index, &glyph) in glyphs.iter().enumerate() {
        let glyph_left = left + count(index) * advance;
        for bit in 0..GLYPH_WIDTH * GLYPH_HEIGHT {
            if GLYPHS[glyph] & (1 << (GLYPH_WIDTH * GLYPH_HEIGHT - 1 - bit)) == 0 {
                continue;
            }
            let column = pixels(bit % GLYPH_WIDTH);
            let line = pixels(bit / GLYPH_WIDTH);
            fill(
                pixmap,
                (
                    glyph_left + column * scale,
                    top + line * scale,
                    scale,
                    scale,
                ),
                &crisp,
            );
        }
    }
}

fn fill(pixmap: &mut Pixmap, (x, y, width, height): (f32, f32, f32, f32), paint: &Paint<'_>) {
    // `None` only for an empty rectangle: nothing to draw.
    let Some(rect) = Rect::from_xywh(x, y, width, height) else {
        return;
    };
    pixmap.fill_rect(rect, paint, Transform::identity(), None);
}
