//! The Kitty stream, byte for byte. A pty accepts anything, so these tests
//! prove only what Graft emits; ADR-0012 is why that is never acceptance for
//! rendering.

use std::sync::Arc;

use super::*;

/// A strip of `rows` rows, 3 columns of 2×4 px cells, every pixel the same
/// premultiplied colour.
fn strip(rows: usize, pixel: [u8; 4]) -> Arc<RasterFrame> {
    let (width, height) = (6, u32::try_from(rows).unwrap() * 4);
    let count = usize::try_from(width * height).unwrap();
    Arc::new(RasterFrame {
        width,
        height,
        rows,
        columns: 3,
        pixels: pixel.repeat(count),
    })
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// Every APC graphics command in `bytes`, control data only (payloads cut).
fn commands(bytes: &[u8]) -> Vec<String> {
    text(bytes)
        .split("\x1b_G")
        .skip(1)
        .map(|command| {
            let end = command.find("\x1b\\").unwrap();
            let body = &command[..end];
            body.split(';').next().unwrap().to_owned()
        })
        .collect()
}

fn place(frame: &Arc<RasterFrame>, first_row: usize, rows: Range<usize>, at: u16) -> Place<'_> {
    Place {
        first_row,
        frame,
        rows,
        cells: CellRect {
            column: 2,
            row: at,
            columns: 3,
            rows: 0,
        },
    }
}

#[test]
fn a_new_strip_is_transmitted_once_then_placed_by_cell_rect() {
    let mut images = KittyImages::new(8);
    let frame = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(&[place(&frame, 0, 1..3, 5)], &mut out);

    assert_eq!(
        commands(&out),
        [
            "a=t,f=32,t=d,i=1000,s=6,v=16,q=1,m=0",
            "a=p,i=1000,p=1,x=0,y=4,w=6,h=8,c=3,r=2,C=1,q=1",
        ]
    );
    // Placed at the cell, cursor saved around it.
    assert!(text(&out).contains("\x1b7\x1b[6;3H\x1b_Ga=p"));
    assert!(text(&out).ends_with("\x1b8"));

    let mut again = Vec::new();
    images.frame(&[place(&frame, 0, 1..3, 5)], &mut again);
    assert_eq!(
        commands(&again),
        ["a=p,i=1000,p=1,x=0,y=4,w=6,h=8,c=3,r=2,C=1,q=1"],
        "no retransmit for a strip already uploaded"
    );
}

#[test]
fn scrolling_one_row_moves_the_crop_by_one_cell_height() {
    let mut images = KittyImages::new(8);
    let frame = strip(8, [0, 0, 0, 0]);
    let mut crops = Vec::new();
    for top in [0, 1, 2, 1] {
        let mut out = Vec::new();
        images.frame(&[place(&frame, 0, top..top + 4, 0)], &mut out);
        crops.extend(commands(&out).into_iter().filter(|c| c.starts_with("a=p")));
    }
    let ys: Vec<&str> = crops
        .iter()
        .map(|c| c.split(',').find(|kv| kv.starts_with("y=")).unwrap())
        .collect();
    assert_eq!(ys, ["y=0", "y=4", "y=8", "y=4"]);
}

#[test]
fn placement_replaces_one_fixed_id_and_never_clears() {
    let mut images = KittyImages::new(8);
    let frame = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    for top in 0..3 {
        images.frame(&[place(&frame, 0, top..4, 0)], &mut out);
    }
    let text = text(&out);
    assert!(
        !text.contains("\x1b[2J"),
        "ED frees kitty image data in Ghostty"
    );
    assert!(!text.contains("q=2"), "q=2 hides errors");
    assert!(
        !commands(&out).iter().any(|c| c.starts_with("a=d")),
        "replace, never delete-then-place"
    );
    assert!(
        commands(&out)
            .iter()
            .filter(|c| c.starts_with("a=p"))
            .all(|c| c.contains("i=1000,p=1,"))
    );
}

#[test]
fn a_large_strip_is_chunked_at_4096_base64_bytes() {
    let mut images = KittyImages::new(8);
    // 6 × 1024 × 4 = 24 576 bytes → 32 768 base64 bytes → 8 full chunks.
    let frame = strip(256, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(&[place(&frame, 0, 0..1, 0)], &mut out);

    let text = text(&out);
    let transmits: Vec<&str> = text
        .split("\x1b_G")
        .skip(1)
        .filter(|c| !c.starts_with("a=p"))
        .collect();
    assert_eq!(transmits.len(), 8);
    assert!(transmits[0].starts_with("a=t,f=32,t=d,i=1000,s=6,v=1024,q=1,m=1;"));
    for chunk in &transmits[1..7] {
        assert!(chunk.starts_with("m=1;"), "{}", &chunk[..8]);
    }
    assert!(transmits[7].starts_with("m=0;"));
    for chunk in &transmits {
        let payload = &chunk[chunk.find(';').unwrap() + 1..chunk.find("\x1b\\").unwrap()];
        assert!(payload.len() <= 4096);
    }
}

#[test]
fn pixels_are_sent_straight_not_premultiplied() {
    let mut images = KittyImages::new(8);
    // Premultiplied 50% white: (128,128,128,128) is straight (255,255,255,128).
    let frame = strip(1, [128, 128, 128, 128]);
    let mut out = Vec::new();
    images.frame(&[place(&frame, 0, 0..1, 0)], &mut out);
    let text = text(&out);
    let payload = &text[text.find(';').unwrap() + 1..text.find("\x1b\\").unwrap()];
    assert!(payload.starts_with("////gP///4D/"), "{payload}");
}

#[test]
fn base64_matches_the_standard_alphabet_with_padding() {
    let mut out = Vec::new();
    for (input, want) in [
        (&b""[..], ""),
        (b"f", "Zg=="),
        (b"fo", "Zm8="),
        (b"foo", "Zm9v"),
        (b"foobar", "Zm9vYmFy"),
    ] {
        out.clear();
        base64(input, &mut out);
        assert_eq!(text(&out), want);
    }
}

#[test]
fn a_changed_strip_frees_the_old_upload_and_gets_a_new_id() {
    let mut images = KittyImages::new(8);
    let old = strip(4, [0, 0, 0, 0]);
    let new = strip(4, [255, 0, 0, 255]);
    let mut out = Vec::new();
    images.frame(&[place(&old, 0, 0..4, 0)], &mut out);
    out.clear();
    images.frame(&[place(&new, 0, 0..4, 0)], &mut out);
    assert_eq!(
        commands(&out),
        [
            "a=d,d=I,i=1000,q=1",
            "a=t,f=32,t=d,i=1001,s=6,v=16,q=1,m=0",
            "a=p,i=1001,p=1,x=0,y=0,w=6,h=16,c=3,r=4,C=1,q=1",
        ]
    );
}

#[test]
fn a_strip_that_scrolls_out_loses_its_placement_but_keeps_its_data() {
    let mut images = KittyImages::new(8);
    let first = strip(4, [0, 0, 0, 0]);
    let second = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(
        &[place(&first, 0, 2..4, 0), place(&second, 4, 0..2, 2)],
        &mut out,
    );
    out.clear();
    images.frame(&[place(&second, 4, 0..4, 0)], &mut out);
    assert_eq!(
        commands(&out),
        [
            "a=p,i=1001,p=1,x=0,y=0,w=6,h=16,c=3,r=4,C=1,q=1",
            "a=d,d=i,i=1000,p=1,q=1"
        ]
    );
    out.clear();
    images.frame(&[place(&first, 0, 0..4, 0)], &mut out);
    assert!(
        !commands(&out).iter().any(|c| c.starts_with("a=t")),
        "data kept"
    );
}

#[test]
fn uploads_past_capacity_free_the_least_recently_placed() {
    let mut images = KittyImages::new(2);
    let strips: Vec<_> = (0..3).map(|_| strip(4, [0, 0, 0, 0])).collect();
    let mut out = Vec::new();
    for (n, frame) in strips.iter().enumerate() {
        out.clear();
        images.frame(&[place(frame, n * 4, 0..4, 0)], &mut out);
    }
    assert!(commands(&out).contains(&"a=d,d=I,i=1000,q=1".to_owned()));
    assert_eq!(images.uploaded(), 2);
}

#[test]
fn free_all_on_exit_deletes_every_upload_with_its_data() {
    let mut images = KittyImages::new(8);
    let (a, b) = (strip(4, [0, 0, 0, 0]), strip(4, [0, 0, 0, 0]));
    let mut out = Vec::new();
    images.frame(&[place(&a, 0, 0..4, 0), place(&b, 4, 0..4, 4)], &mut out);
    out.clear();
    images.free_all(&mut out);
    assert_eq!(commands(&out), ["a=d,d=I,i=1000,q=1", "a=d,d=I,i=1001,q=1"]);
    assert_eq!(images.uploaded(), 0);
}

#[test]
fn hide_removes_placements_and_keeps_data() {
    let mut images = KittyImages::new(8);
    let a = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(&[place(&a, 0, 0..4, 0)], &mut out);
    out.clear();
    images.hide(&mut out);
    assert_eq!(commands(&out), ["a=d,d=i,i=1000,p=1,q=1"]);
    out.clear();
    images.hide(&mut out);
    assert!(out.is_empty(), "nothing placed, nothing to say");
}

#[test]
fn a_missing_image_reply_makes_the_next_frame_retransmit() {
    let mut images = KittyImages::new(8);
    let a = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(&[place(&a, 0, 0..4, 0)], &mut out);

    let replies = replies(b"\x1b_Gi=1000,p=1;ENOENT:image not found\x1b\\");
    assert_eq!(replies.len(), 1);
    assert!(replies[0].is_error());
    images.answer(&replies[0]);

    out.clear();
    images.frame(&[place(&a, 0, 0..4, 0)], &mut out);
    assert_eq!(
        commands(&out),
        [
            "a=t,f=32,t=d,i=1001,s=6,v=16,q=1,m=0",
            "a=p,i=1001,p=1,x=0,y=0,w=6,h=16,c=3,r=4,C=1,q=1"
        ]
    );
}

#[test]
fn forget_uploads_after_a_clear_retransmits_without_deleting() {
    let mut images = KittyImages::new(8);
    let a = strip(4, [0, 0, 0, 0]);
    let mut out = Vec::new();
    images.frame(&[place(&a, 0, 0..4, 0)], &mut out);
    images.forget_uploads();
    out.clear();
    images.frame(&[place(&a, 0, 0..4, 0)], &mut out);
    assert_eq!(commands(&out)[0], "a=t,f=32,t=d,i=1001,s=6,v=16,q=1,m=0");
}

#[test]
fn replies_parse_ids_and_messages_and_skip_other_replies() {
    let found = replies(b"\x1b[?62;22c\x1b_Gi=1003;OK\x1b\\\x1b_Gi=7,p=1;EINVAL:bad crop\x1b\\\x1b]11;rgb:0/0/0\x1b\\");
    assert_eq!(
        found,
        [
            GraphicsReply {
                image: Some(1003),
                placement: None,
                message: "OK".to_owned()
            },
            GraphicsReply {
                image: Some(7),
                placement: Some(1),
                message: "EINVAL:bad crop".to_owned()
            },
        ]
    );
    assert!(!found[0].is_error());
    assert!(found[1].is_error());
}

#[test]
fn a_reply_message_is_sanitised_before_anyone_can_draw_it() {
    let found = replies(b"\x1b_Gi=1;ENOENT:\x07bad\x1b\\");
    assert_eq!(found[0].message, "ENOENT:\u{fffd}bad");
}
