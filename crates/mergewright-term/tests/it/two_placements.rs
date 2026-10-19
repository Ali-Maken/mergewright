//! Settle test 2 of ease-pass/13: the Pixel tier's tall row places the
//! cursor's strip twice from one raster — the rows up to the cursor at its
//! cell, the rows after it one cell row lower. What each protocol sends for
//! that, byte for byte. A pty accepts anything, so this proves only what
//! Mergewright emits (ADR-0012), not what a terminal draws.
#![allow(
    clippy::unwrap_used,
    reason = "test helpers: a panic is the intended failure (PROMPT.md 6.3)"
)]

use std::ops::Range;
use std::sync::Arc;

use mergewright_core::raster::RasterFrame;
use mergewright_term::iterm2::ItermImages;
use mergewright_term::kitty::{CellRect, KittyImages, Layer, Place, UploadKey};

/// A 32-row strip, 3 columns of 2×4 px cells, one colour.
fn strip() -> Arc<RasterFrame> {
    let height = 4 * 32;
    Arc::new(RasterFrame {
        width: 6,
        height,
        rows: 32,
        columns: 3,
        pixels: [128, 0, 0, 128].repeat(6 * height as usize),
    })
}

/// Strip rows `rows` of `frame`, on screen from row `at`.
fn place(frame: &Arc<RasterFrame>, rows: Range<usize>, at: u16) -> Place<'_> {
    Place {
        first_row: 32,
        key: UploadKey(7),
        frame: Some(&**frame),
        cells: CellRect {
            column: 2,
            row: at,
            columns: 3,
            rows: u16::try_from(rows.len()).unwrap(),
        },
        rows,
        layer: Layer::Strip,
        stack: 0,
    }
}

/// The cursor on strip row `cursor`, the viewport showing strip rows
/// 6..20: rows 6..=cursor from screen row 0, the second line under the
/// cursor, the rest one row lower.
fn split(frame: &Arc<RasterFrame>, cursor: usize) -> [Place<'_>; 2] {
    let above = 6..cursor + 1;
    let at = u16::try_from(above.len()).unwrap() + 1;
    [place(frame, above, 0), place(frame, cursor + 1..20, at)]
}

/// Every Kitty graphics command's control data, payloads cut.
fn commands(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .split("\x1b_G")
        .skip(1)
        .map(|command| {
            let end = command.find("\x1b\\").unwrap();
            command[..end].split(';').next().unwrap().to_owned()
        })
        .collect()
}

#[test]
fn kitty_places_one_upload_twice_with_disjoint_crops_and_one_transmit() {
    let frame = strip();
    let mut images = KittyImages::new(usize::MAX);
    let mut out = Vec::new();
    images.frame(&split(&frame, 10), &mut out);
    let sent = commands(&out);
    let transmits = sent.iter().filter(|c| c.starts_with("a=t")).count();
    assert_eq!(transmits, 1, "one transmit for both placements: {sent:?}");
    // Two placements of the one image: p=1 crops rows 6..11 (y = 6 × 4 px,
    // 5 rows), p=2 rows 11..20 (y = 44, 9 rows), one row lower on screen.
    assert_eq!(
        &sent[1..],
        [
            "a=p,i=1000,p=1,x=0,y=24,w=6,h=20,c=3,r=5,C=1,q=1",
            "a=p,i=1000,p=2,x=0,y=44,w=6,h=36,c=3,r=9,C=1,q=1",
        ]
    );
    // A `j` within the strip moves only the crops: no transmit.
    let mut again = Vec::new();
    images.frame(&split(&frame, 11), &mut again);
    let sent = commands(&again);
    assert!(sent.iter().all(|c| c.starts_with("a=p")), "{sent:?}");
    assert_eq!(sent.len(), 2);
}

/// iTerm2 has no ids and no crops (ADR-0012): each placement is its own PNG
/// band, and the split moving with the cursor makes both crops new bands,
/// so a `j` inside the strip encodes and sends two PNGs. Recorded, not
/// fixed here: ease-pass/13's measurement weighs it.
#[test]
fn iterm2_sends_two_bands_and_a_j_inside_the_strip_re_encodes_both() {
    let frame = strip();
    let mut images = ItermImages::new(16);
    let bands = |bytes: &[u8]| {
        String::from_utf8_lossy(bytes)
            .matches("\x1b]1337;File=")
            .count()
    };
    let mut first = Vec::new();
    images.frame(&split(&frame, 10), &mut first).unwrap();
    assert_eq!(bands(&first), 2);
    assert_eq!(images.encoded(), 2);

    let mut moved = Vec::new();
    images.frame(&split(&frame, 11), &mut moved).unwrap();
    assert_eq!(bands(&moved), 2);
    assert_eq!(images.encoded(), 4, "both crops changed, both re-encoded");
    // Back to the first split: both bands are still kept, none re-encoded.
    images.frame(&split(&frame, 10), &mut Vec::new()).unwrap();
    assert_eq!(images.encoded(), 4);
    eprintln!(
        "iTerm2 split bytes: first {} B, after one j {} B",
        first.len(),
        moved.len()
    );
}
