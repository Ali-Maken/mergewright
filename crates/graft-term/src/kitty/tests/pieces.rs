//! Emphasis pieces known by key (m8 05 Part 2, ADR-0053 amendment 2026-10-10).

use std::sync::Arc;

use graft_core::raster::RasterFrame;

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

/// m8 05 amendment 1: one content key turns up twice in a frame (the same
/// arc on two rows). It is sent once and placed twice, each place its own
/// placement id, so the second does not replace the first; a frame that
/// shows it once again removes the second placement only.
#[test]
fn one_upload_two_places() {
    let mut images = KittyImages::new(ROOMY);
    let glow = strip(4, [9, 9, 9, 9]);
    let id = FIRST_EMPHASIS;
    let mut out = Vec::new();
    images.frame(
        &[piece(&glow, 7, 0, true), piece(&glow, 7, 2, true)],
        &mut out,
    );
    let sent = commands(&out);
    let transmits = sent.iter().filter(|c| c.starts_with("a=t")).count();
    assert_eq!(transmits, 1, "one upload: {sent:?}");
    let placed: Vec<&String> = sent.iter().filter(|c| c.starts_with("a=p")).collect();
    assert_eq!(placed.len(), 2, "two placements: {sent:?}");
    assert!(
        placed[0].starts_with(&format!("a=p,i={id},p=1,")),
        "{sent:?}"
    );
    assert!(
        placed[1].starts_with(&format!("a=p,i={id},p=2,")),
        "{sent:?}"
    );

    // Held now: both places come without pixels; nothing sent or removed.
    let mut out = Vec::new();
    images.frame(
        &[piece(&glow, 7, 0, false), piece(&glow, 7, 2, false)],
        &mut out,
    );
    let sent = commands(&out);
    assert_eq!(sent.iter().filter(|c| c.starts_with("a=p")).count(), 2);
    assert!(
        sent.iter()
            .all(|c| !c.starts_with("a=t") && !c.starts_with("a=d")),
        "{sent:?}"
    );

    // Once: placement 1 replaced, placement 2 removed, data kept.
    let mut out = Vec::new();
    images.frame(&[piece(&glow, 7, 0, false)], &mut out);
    let sent = commands(&out);
    assert!(
        sent.contains(&format!("a=d,d=i,i={id},p=2,q=1")),
        "{sent:?}"
    );
    assert!(
        !sent.contains(&format!("a=d,d=i,i={id},p=1,q=1")),
        "{sent:?}"
    );
    assert!(images.holds(UploadKey(7)));

    // Twice again, then hidden: both placements go.
    images.frame(
        &[piece(&glow, 7, 0, false), piece(&glow, 7, 2, false)],
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    images.hide(&mut out);
    let sent = commands(&out);
    assert!(
        sent.contains(&format!("a=d,d=i,i={id},p=1,q=1")),
        "{sent:?}"
    );
    assert!(
        sent.contains(&format!("a=d,d=i,i={id},p=2,q=1")),
        "{sent:?}"
    );
}

/// m8 05 amendment 1: `stack` is a piece's rank, and it reaches the
/// terminal as `z`, the field the protocol stacks by: "If two images with
/// the same z-index overlap then the image with the lower id is considered
/// to have the lower z-index" (sw.kovidgoyal.net/kitty/graphics-protocol,
/// "Controlling displayed image layout"). Every rank sits above the strip
/// (`z` 0), and the screen order depends neither on the order the commands
/// are written in nor on which id an older upload happened to get.
#[test]
fn stack_reaches_the_command() {
    let mut images = KittyImages::new(ROOMY);
    let glow = strip(4, [9, 9, 9, 9]);
    // Key 8 is uploaded first, so it has the lower id, but ranks above 7.
    images.frame(&[piece(&glow, 8, 0, true)], &mut Vec::new());
    let places = [
        Place {
            stack: 5,
            ..piece(&glow, 8, 0, false)
        },
        Place {
            stack: 2,
            ..piece(&glow, 7, 0, true)
        },
    ];
    let mut out = Vec::new();
    images.frame(&places, &mut out);
    let sent = commands(&out);
    let z_of = |id: u32| {
        sent.iter()
            .find(|c| c.starts_with(&format!("a=p,i={id},")))
            .and_then(|c| c.split(',').find_map(|field| field.strip_prefix("z=")))
            .map(|z| z.parse::<u32>().unwrap())
    };
    assert_eq!(z_of(FIRST_EMPHASIS), Some(6), "key 8, stack 5: {sent:?}");
    assert_eq!(
        z_of(FIRST_EMPHASIS + 1),
        Some(3),
        "key 7, stack 2: {sent:?}"
    );
    // A strip's place carries no `z` (protocol default 0), whatever its stack.
    let base = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(
        &[Place {
            stack: 3,
            ..place(&base, 0, 0..4, 0)
        }],
        &mut out,
    );
    let sent = commands(&out);
    let strip_place = sent.iter().find(|c| c.starts_with("a=p")).unwrap();
    assert!(!strip_place.contains("z="), "{sent:?}");
}

/// The same key in a new allocation is the same picture: the strip cache
/// evicted it and drew it again, and the terminal still holds it. No
/// transmit; a different key in the same slot is a new upload.
#[test]
fn an_upload_is_matched_by_its_key() {
    let mut images = KittyImages::new(usize::MAX);
    let (first, again) = (strip(4, [1, 2, 3, 255]), strip(4, [1, 2, 3, 255]));
    let keyed = |frame, key| Place {
        key: UploadKey(key),
        ..place(frame, 0, 0..4, 1)
    };
    let mut out = Vec::new();
    images.frame(&[keyed(&first, 7)], &mut out);
    let mut out = Vec::new();
    images.frame(&[keyed(&again, 7)], &mut out);
    assert!(
        commands(&out).iter().all(|c| !c.contains("a=t")),
        "same key, no transmit: {:?}",
        commands(&out)
    );
    let mut out = Vec::new();
    images.frame(&[keyed(&again, 8)], &mut out);
    assert!(commands(&out).iter().any(|c| c.contains("a=t")));
    assert_eq!(images.uploaded(), 1, "the old one was freed");
}
