//! Emphasis pieces known by key (m8 05 Part 2, ADR-0053 amendment 2026-10-10).

use std::sync::Arc;

use super::super::*;
use super::{FIRST_EMPHASIS, commands, emphasis, place, strip};

/// A byte budget no test here reaches.
const ROOMY: usize = 1 << 30;
/// Decoded bytes of `strip(4, _)`: 6 × 16 px × 4.
const STRIP_4_BYTES: usize = 6 * 16 * 4;

/// An emphasis piece of the strip at row 0, keyed `key`, in cell column
/// `column`; `pixels: false` is a place sent after `holds` said yes.
fn piece(frame: &Arc<RasterFrame>, key: u64, column: u16, pixels: bool) -> Place<'_> {
    let base = emphasis(frame, 0, 0..4, 0);
    Place {
        key: UploadKey(key),
        frame: pixels.then_some(&**frame),
        cells: CellRect {
            column,
            ..base.cells
        },
        ..base
    }
}

/// m8 05 Part 2: a held `j` for 1000 keys, each placing four pieces of
/// which one is new: the pieces left behind stay in the terminal only up
/// to the budget, oldest placed freed first, and nothing placed is freed.
#[test]
fn held_unplaced_bytes_stay_under_budget() {
    let budget = 10 * STRIP_4_BYTES;
    let mut images = KittyImages::new(budget);
    let base = strip(4, [0, 0, 0, 0]);
    let glow = strip(4, [9, 9, 9, 9]);
    let mut freed = 0;
    for key in 0..1000u64 {
        let mut places = vec![place(&base, 0, 0..4, 0)];
        for (column, held) in (key..key + 4).enumerate() {
            let pixels = !images.holds(UploadKey(held));
            places.push(piece(&glow, held, u16::try_from(column).unwrap(), pixels));
        }
        let mut out = Vec::new();
        images.frame(&places, &mut out);
        let sent = commands(&out);
        let placements = sent.iter().filter(|c| c.starts_with("a=p")).count();
        assert_eq!(placements, 5, "key {key}: every place placed: {sent:?}");
        let transmitted = sent.iter().filter(|c| c.starts_with("a=t")).count();
        // The first frame sends the strip and four pieces; then one piece.
        assert_eq!(transmitted, if key == 0 { 5 } else { 1 }, "key {key}");
        freed += sent.iter().filter(|c| c.starts_with("a=d,d=I")).count();
        assert!(
            images.unplaced() <= budget,
            "key {key}: {}",
            images.unplaced()
        );
    }
    assert!(freed > 900, "old pieces were freed ({freed})");
    assert!(images.uploaded() <= 5 + 10, "{}", images.uploaded());
}

/// An `ENOENT` about a piece makes `holds` say no, so the next frame sends
/// its pixels again rather than placing an image the terminal lost.
#[test]
fn holds_false_after_missing_image_reply() {
    let mut images = KittyImages::new(ROOMY);
    let glow = strip(4, [9, 9, 9, 9]);
    images.frame(&[piece(&glow, 7, 0, true)], &mut Vec::new());
    assert!(images.holds(UploadKey(7)));
    images.answer(&replies(format!("\x1b_Gi={FIRST_EMPHASIS};ENOENT:gone\x1b\\").as_bytes())[0]);
    assert!(!images.holds(UploadKey(7)));
    // A place without pixels that is not held is not placed blank.
    let mut out = Vec::new();
    images.frame(&[piece(&glow, 7, 0, false)], &mut out);
    assert!(commands(&out).is_empty(), "{:?}", commands(&out));
    images.frame(&[piece(&glow, 7, 0, true)], &mut out);
    assert!(commands(&out).iter().any(|c| c.starts_with("a=t")));
}

/// Pieces are known by key, not slot: a piece left unplaced and wanted
/// again is placed for nothing; two pieces of one strip are two images.
#[test]
fn a_piece_moved_away_from_and_back_is_not_sent_again() {
    let mut images = KittyImages::new(ROOMY);
    let glow = strip(4, [9, 9, 9, 9]);
    images.frame(
        &[piece(&glow, 1, 0, true), piece(&glow, 2, 2, true)],
        &mut Vec::new(),
    );
    images.frame(&[piece(&glow, 3, 0, true)], &mut Vec::new());
    let mut out = Vec::new();
    images.frame(
        &[piece(&glow, 1, 0, false), piece(&glow, 2, 2, false)],
        &mut out,
    );
    let sent = commands(&out);
    assert!(sent.iter().all(|c| !c.starts_with("a=t")), "{sent:?}");
    assert_eq!(sent.iter().filter(|c| c.starts_with("a=p")).count(), 2);
}
