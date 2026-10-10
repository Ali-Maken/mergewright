//! The Kitty stream, byte for byte. A pty accepts anything, so these tests prove only what
//! Graft emits; ADR-0012 is why that is never acceptance for rendering. Pieces: `pieces`.

use std::sync::Arc;

use super::*;
use crate::tmux::Transport;
mod pieces;

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

/// The key the strip cache would give `frame`: here, its allocation, so a
/// new `Arc` is new pixels as it was before uploads were keyed (m8 04).
fn key_of(frame: &Arc<RasterFrame>) -> UploadKey {
    UploadKey(Arc::as_ptr(frame) as u64)
}

fn place(frame: &Arc<RasterFrame>, first_row: usize, rows: Range<usize>, at: u16) -> Place<'_> {
    Place {
        first_row,
        key: key_of(frame),
        frame: Some(&**frame),
        rows,
        cells: CellRect {
            column: 2,
            row: at,
            columns: 3,
            rows: 0,
        },
        layer: Layer::Strip,
    }
}

#[test]
fn a_new_strip_is_transmitted_once_then_placed_by_cell_rect() {
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(6 * 16 * 4); // one `strip(4, _)`
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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
    let mut images = KittyImages::new(usize::MAX);
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

/// Inside tmux every command, each upload chunk included, is its own
/// passthrough; the cursor save, move and restore around a placement stay
/// outside for tmux; and what tmux forwards is the direct stream exactly.
#[test]
fn through_tmux_each_command_and_each_chunk_gets_its_own_wrapper() {
    let frame = strip(40, [255, 0, 0, 255]);
    let places = [place(&frame, 0, 0..40, 0)];
    let mut direct = Vec::new();
    KittyImages::new(usize::MAX).frame(&places, &mut direct);
    let mut tmux = Vec::new();
    KittyImages::new(usize::MAX)
        .with_transport(Transport::TmuxPassthrough)
        .frame(&places, &mut tmux);

    let sent = commands(&direct);
    assert_eq!(
        sent.iter()
            .filter(|command| command.starts_with("a=t") || command.starts_with("m="))
            .count(),
        2,
        "the strip must be big enough to chunk: {sent:?}"
    );
    let (forwarded, wrappers) = crate::tmux::unwrap_all(&tmux);
    assert_eq!(forwarded, direct);
    assert_eq!(wrappers, sent.len());
    assert!(
        tmux.starts_with(b"\x1bPtmux;\x1b\x1b_Ga=t"),
        "{:?}",
        text(&tmux[..20])
    );
    let moves = text(&tmux);
    assert!(
        moves.contains("\x1b\\\x1b7\x1b[1;3H\x1bPtmux;\x1b\x1b_Ga=p"),
        "{moves:?}"
    );
}

#[test]
fn through_tmux_hide_and_free_are_wrapped_too() {
    let frame = strip(1, [0, 0, 0, 255]);
    let mut images = KittyImages::new(usize::MAX).with_transport(Transport::TmuxPassthrough);
    images.frame(&[place(&frame, 0, 0..1, 0)], &mut Vec::new());

    let mut out = Vec::new();
    images.hide(&mut out);
    images.free_all(&mut out);

    assert_eq!(
        text(&out),
        "\x1bPtmux;\x1b\x1b_Ga=d,d=i,i=1000,p=1,q=1\x1b\x1b\\\x1b\\\
         \x1bPtmux;\x1b\x1b_Ga=d,d=I,i=1000,q=1\x1b\x1b\\\x1b\\"
    );
}

/// The straight-alpha conversion as first written, one division per
/// channel: the reference the fast path must match byte for byte.
fn straight_alpha_reference(pixels: &[u8]) -> Vec<u8> {
    let mut straight = Vec::with_capacity(pixels.len());
    for &[red, green, blue, alpha] in pixels.as_chunks::<4>().0 {
        let channel = |value: u8| {
            (u16::from(value) * 255 + u16::from(alpha) / 2)
                .checked_div(u16::from(alpha))
                .map_or(0, |straight| {
                    u8::try_from(straight.min(255)).unwrap_or(u8::MAX)
                })
        };
        straight.extend_from_slice(&[channel(red), channel(green), channel(blue), alpha]);
    }
    straight
}

#[test]
fn straight_alpha_matches_the_per_channel_division_for_every_value_and_alpha() {
    let mut pixels = Vec::new();
    for alpha in 0..=255u8 {
        for value in 0..=255u8 {
            // Malformed values above alpha included: they clamp, never wrap.
            pixels.extend_from_slice(&[value, value / 2, 255 - value, alpha]);
        }
    }
    assert_eq!(straight_alpha(&pixels), straight_alpha_reference(&pixels));
}

#[test]
fn base64_matches_the_standard_alphabet_for_every_length_and_byte() {
    fn reference(input: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut out = String::new();
        for group in input.chunks(3) {
            let bits = group.iter().enumerate().fold(0u32, |bits, (at, byte)| {
                bits | u32::from(*byte) << (16 - 8 * at)
            });
            for at in 0..4 {
                if at <= group.len() {
                    out.push(char::from(ALPHABET[(bits >> (18 - 6 * at) & 63) as usize]));
                } else {
                    out.push('=');
                }
            }
        }
        out
    }
    let bytes: Vec<u8> = (0..=255u8).cycle().take(1000).collect();
    for length in 0..40 {
        let mut out = vec![b'x'];
        base64(&bytes[length..length * 2], &mut out);
        assert_eq!(
            text(&out[1..]),
            reference(&bytes[length..length * 2]),
            "length {length}"
        );
    }
    let mut out = Vec::new();
    base64(&bytes, &mut out);
    assert_eq!(text(&out), reference(&bytes));
}

/// A fixed run of frames: two strips, one replaced, one dropped, hide, free.
fn pinned_run(images: &mut KittyImages) -> Vec<u8> {
    let a = strip(2, [10, 20, 30, 255]);
    let b = strip(1, [0, 0, 0, 0]);
    let b2 = strip(1, [64, 32, 16, 128]);
    let mut out = Vec::new();
    images.frame(&[place(&a, 0, 0..2, 1), place(&b, 2, 0..1, 3)], &mut out);
    images.frame(&[place(&a, 0, 1..2, 0), place(&b2, 2, 0..1, 2)], &mut out);
    images.frame(&[place(&a, 0, 0..1, 4)], &mut out);
    images.hide(&mut out);
    images.free_all(&mut out);
    out
}

/// `pinned_run`'s bytes as captured before layers and deflate existed
/// (ticket 45, at e6f12848).
const PINNED_BEFORE_DEFLATE: &str = "\u{1b}_Ga=t,f=32,t=d,i=1000,s=6,v=8,q=1,m=0;ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/ChQe/woUHv8KFB7/\u{1b}\\\u{1b}7\u{1b}[2;3H\u{1b}_Ga=p,i=1000,p=1,x=0,y=0,w=6,h=8,c=3,r=2,C=1,q=1\u{1b}\\\u{1b}8\u{1b}_Ga=t,f=32,t=d,i=1001,s=6,v=4,q=1,m=0;AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA\u{1b}\\\u{1b}7\u{1b}[4;3H\u{1b}_Ga=p,i=1001,p=1,x=0,y=0,w=6,h=4,c=3,r=1,C=1,q=1\u{1b}\\\u{1b}8\u{1b}7\u{1b}[1;3H\u{1b}_Ga=p,i=1000,p=1,x=0,y=4,w=6,h=4,c=3,r=1,C=1,q=1\u{1b}\\\u{1b}8\u{1b}_Ga=d,d=I,i=1001,q=1\u{1b}\\\u{1b}_Ga=t,f=32,t=d,i=1002,s=6,v=4,q=1,m=0;gEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCAgEAggIBAIICAQCCA\u{1b}\\\u{1b}7\u{1b}[3;3H\u{1b}_Ga=p,i=1002,p=1,x=0,y=0,w=6,h=4,c=3,r=1,C=1,q=1\u{1b}\\\u{1b}8\u{1b}7\u{1b}[5;3H\u{1b}_Ga=p,i=1000,p=1,x=0,y=0,w=6,h=4,c=3,r=1,C=1,q=1\u{1b}\\\u{1b}8\u{1b}_Ga=d,d=i,i=1002,p=1,q=1\u{1b}\\\u{1b}_Ga=d,d=i,i=1000,p=1,q=1\u{1b}\\\u{1b}_Ga=d,d=I,i=1000,q=1\u{1b}\\\u{1b}_Ga=d,d=I,i=1002,q=1\u{1b}\\";

#[test]
fn deflate_off_is_unchanged() {
    let out = pinned_run(&mut KittyImages::new(usize::MAX).deflating(false));
    assert_eq!(text(&out), PINNED_BEFORE_DEFLATE);
    // Off is the default.
    assert_eq!(
        text(&pinned_run(&mut KittyImages::new(usize::MAX))),
        PINNED_BEFORE_DEFLATE
    );
}

/// Standard base64 back to bytes (test-side inverse of `base64`).
fn unbase64(text: &[u8]) -> Vec<u8> {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut bits = 0u32;
    let mut held = 0;
    let mut out = Vec::new();
    for &symbol in text.iter().filter(|&&symbol| symbol != b'=') {
        let value = ALPHABET.iter().position(|&a| a == symbol).unwrap();
        bits = bits << 6 | u32::try_from(value).unwrap();
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push(u8::try_from(bits >> held & 0xff).unwrap());
        }
    }
    out
}

/// Every transmit in `bytes`: its first chunk's control data and its whole
/// payload, chunks joined and base64-decoded.
fn transmits(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut found: Vec<(String, Vec<u8>)> = Vec::new();
    for command in text(bytes).split("\x1b_G").skip(1) {
        let body = &command[..command.find("\x1b\\").unwrap()];
        let (control, payload) = body.split_once(';').unwrap_or((body, ""));
        if control.starts_with("a=t") {
            found.push((control.to_owned(), payload.as_bytes().to_vec()));
        } else if control.starts_with("m=") {
            found
                .last_mut()
                .unwrap()
                .1
                .extend_from_slice(payload.as_bytes());
        }
    }
    found
        .into_iter()
        .map(|(control, payload)| (control, unbase64(&payload)))
        .collect()
}

fn inflate(bytes: &[u8]) -> Vec<u8> {
    miniz_oxide::inflate::decompress_to_vec_zlib(bytes).unwrap()
}

/// `place`, as the emphasis layer over the strip at `first_row`, one column
/// to the right.
fn emphasis(frame: &Arc<RasterFrame>, first_row: usize, rows: Range<usize>, at: u16) -> Place<'_> {
    let strip = place(frame, first_row, rows, at);
    Place {
        layer: Layer::Emphasis,
        cells: CellRect {
            column: 3,
            ..strip.cells
        },
        ..strip
    }
}

const FIRST_EMPHASIS: u32 = EMPHASIS_ID_BIT | FIRST_IMAGE_ID;

fn base_and_glow() -> (Arc<RasterFrame>, Arc<RasterFrame>) {
    (strip(4, [0, 0, 0, 0]), strip(4, [9, 9, 9, 9]))
}

#[test]
fn deflated_transmit_round_trips() {
    let frame = strip(40, [64, 32, 16, 128]);
    let mut out = Vec::new();
    KittyImages::new(usize::MAX)
        .deflating(true)
        .frame(&[place(&frame, 0, 0..1, 0)], &mut out);
    let sent = transmits(&out);
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "a=t,f=32,t=d,o=z,i=1000,s=6,v=160,q=1,m=0");
    assert_eq!(inflate(&sent[0].1), straight_alpha(&frame.pixels));
}

#[test]
fn deflated_chunks_wrap_in_tmux() {
    // Hashed noise does not compress, so the deflated upload still chunks.
    let pixels: Vec<u8> = (0..6 * 4000u32)
        .flat_map(|at| {
            let [r, g, b, _] = at.wrapping_mul(2_654_435_761).to_le_bytes();
            [r, g, b, 255]
        })
        .collect();
    let frame = Arc::new(RasterFrame {
        width: 6,
        height: 4000,
        rows: 1000,
        columns: 3,
        pixels,
    });
    let places = [place(&frame, 0, 0..1, 0)];
    let mut direct = Vec::new();
    KittyImages::new(usize::MAX)
        .deflating(true)
        .frame(&places, &mut direct);
    let mut tmux = Vec::new();
    KittyImages::new(usize::MAX)
        .deflating(true)
        .with_transport(Transport::TmuxPassthrough)
        .frame(&places, &mut tmux);

    let sent = commands(&direct);
    assert!(sent[0].contains("o=z"), "{sent:?}");
    assert!(sent.iter().any(|c| c.starts_with("m=")), "{sent:?}");
    let (forwarded, wrappers) = crate::tmux::unwrap_all(&tmux);
    assert_eq!(forwarded, direct);
    assert_eq!(wrappers, sent.len());
}

#[test]
fn emphasis_sits_above_its_strip() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    let mut out = Vec::new();
    // Emphasis listed first: the frame still emits every strip first.
    images.frame(
        &[emphasis(&glow, 0, 0..2, 1), place(&base, 0, 0..2, 1)],
        &mut out,
    );
    assert_eq!(
        commands(&out),
        [
            "a=t,f=32,t=d,i=1000,s=6,v=16,q=1,m=0".to_owned(),
            "a=p,i=1000,p=1,x=0,y=0,w=6,h=8,c=3,r=2,C=1,q=1".to_owned(),
            format!("a=t,f=32,t=d,i={FIRST_EMPHASIS},s=6,v=16,q=1,m=0"),
            format!("a=p,i={FIRST_EMPHASIS},p=1,x=0,y=0,w=6,h=8,c=3,r=2,C=1,z=1,q=1"),
        ]
    );
    assert!(text(&out).contains(&format!("\x1b7\x1b[2;4H\x1b_Ga=p,i={FIRST_EMPHASIS}")));
}

#[test]
fn emphasis_goes_strip_stays() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    images.frame(
        &[place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)],
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    images.frame(&[place(&base, 0, 0..2, 1)], &mut out);
    assert_eq!(
        commands(&out),
        [
            "a=p,i=1000,p=1,x=0,y=0,w=6,h=8,c=3,r=2,C=1,q=1".to_owned(),
            format!("a=d,d=i,i={FIRST_EMPHASIS},p=1,q=1"),
        ]
    );
}

#[test]
fn emphasis_follows_a_moved_strip() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    images.frame(
        &[place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)],
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    images.frame(
        &[emphasis(&glow, 0, 1..3, 0), place(&base, 0, 1..3, 0)],
        &mut out,
    );
    assert_eq!(
        commands(&out),
        [
            "a=p,i=1000,p=1,x=0,y=4,w=6,h=8,c=3,r=2,C=1,q=1".to_owned(),
            format!("a=p,i={FIRST_EMPHASIS},p=1,x=0,y=4,w=6,h=8,c=3,r=2,C=1,z=1,q=1"),
        ],
        "no retransmit; the emphasis is re-placed after its strip"
    );
}

#[test]
fn a_deflate_error_falls_back_to_raw() {
    let frame = strip(4, [64, 32, 16, 128]);
    let mut images = KittyImages::new(usize::MAX).deflating(true);
    assert!(images.is_deflating());
    images.frame(&[place(&frame, 0, 0..4, 0)], &mut Vec::new());

    // ENOENT behaves as today: forgotten, retransmitted, still deflated.
    images.answer(&replies(b"\x1b_Gi=1000;ENOENT:gone\x1b\\")[0]);
    assert!(images.is_deflating());
    let mut out = Vec::new();
    images.frame(&[place(&frame, 0, 0..4, 0)], &mut out);
    assert_eq!(
        commands(&out)[0],
        "a=t,f=32,t=d,o=z,i=1001,s=6,v=16,q=1,m=0"
    );

    images.answer(&replies(b"\x1b_Gi=1001;EINVAL:bad compression\x1b\\")[0]);
    assert!(!images.is_deflating());
    out.clear();
    images.frame(&[place(&frame, 0, 0..4, 0)], &mut out);
    assert_eq!(commands(&out)[0], "a=t,f=32,t=d,i=1002,s=6,v=16,q=1,m=0");
    assert!(!text(&out).contains("o=z"));
    assert_eq!(transmits(&out)[0].1, straight_alpha(&frame.pixels));
}

#[test]
fn deflate_shrinks_a_sparse_strip() {
    // 187 columns of 10×20 px cells, 20 rows: six 2-px opaque lanes on a
    // transparent background, as a graph strip mostly is.
    let (width, rows, cell_height) = (1870u32, 20usize, 20u32);
    let height = u32::try_from(rows).unwrap() * cell_height;
    let mut pixels = vec![0u8; usize::try_from(width * height * 4).unwrap()];
    for y in 0..height {
        for lane in 0..6u32 {
            for x in [lane * 30 + 5, lane * 30 + 6] {
                let at = usize::try_from((y * width + x) * 4).unwrap();
                pixels[at..at + 4].copy_from_slice(&[200, 80, 40, 255]);
            }
        }
    }
    let frame = Arc::new(RasterFrame {
        width,
        height,
        rows,
        columns: 187,
        pixels,
    });
    let places = [Place {
        first_row: 0,
        key: key_of(&frame),
        frame: Some(&frame),
        rows: 0..rows,
        cells: CellRect {
            column: 0,
            row: 0,
            columns: 187,
            rows: 0,
        },
        layer: Layer::Strip,
    }];
    let mut raw = Vec::new();
    KittyImages::new(2).frame(&places, &mut raw);
    let started = std::time::Instant::now();
    let mut deflated = Vec::new();
    KittyImages::new(2)
        .deflating(true)
        .frame(&places, &mut deflated);
    let took = started.elapsed();
    #[expect(clippy::cast_precision_loss, reason = "a printed percentage")]
    let percent = deflated.len() as f64 * 100.0 / raw.len() as f64;
    println!(
        "deflate_shrinks_a_sparse_strip: raw {} B, deflated {} B ({percent:.2}%), encode {:.3} ms",
        raw.len(),
        deflated.len(),
        took.as_secs_f64() * 1000.0
    );
    assert!(
        deflated.len() * 10 <= raw.len(),
        "{} vs {}",
        deflated.len(),
        raw.len()
    );
    assert_eq!(
        inflate(&transmits(&deflated)[0].1),
        straight_alpha(&frame.pixels)
    );
}

#[test]
fn emphasis_enoent_retransmits() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    let places = [place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)];
    images.frame(&places, &mut Vec::new());
    images.answer(&replies(format!("\x1b_Gi={FIRST_EMPHASIS};ENOENT:gone\x1b\\").as_bytes())[0]);
    let mut out = Vec::new();
    images.frame(&places, &mut out);
    let sent = commands(&out);
    assert_eq!(sent.len(), 3, "{sent:?}");
    assert_eq!(
        sent[1],
        format!("a=t,f=32,t=d,i={},s=6,v=16,q=1,m=0", FIRST_EMPHASIS + 1)
    );
}

#[test]
fn forget_covers_emphasis() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    let places = [place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)];
    images.frame(&places, &mut Vec::new());
    images.forget_uploads();
    assert_eq!(images.uploaded(), 0);
    let mut out = Vec::new();
    images.frame(&places, &mut out);
    assert_eq!(transmits(&out).len(), 2);
}

#[test]
fn free_all_frees_emphasis() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    images.frame(
        &[place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)],
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    images.free_all(&mut out);
    assert_eq!(
        commands(&out),
        [
            "a=d,d=I,i=1000,q=1".to_owned(),
            format!("a=d,d=I,i={FIRST_EMPHASIS},q=1"),
        ]
    );
}

#[test]
fn hide_hides_emphasis() {
    let (base, glow) = base_and_glow();
    let mut images = KittyImages::new(usize::MAX);
    images.frame(
        &[place(&base, 0, 0..2, 1), emphasis(&glow, 0, 0..2, 1)],
        &mut Vec::new(),
    );
    let mut out = Vec::new();
    images.hide(&mut out);
    assert_eq!(
        commands(&out),
        [
            "a=d,d=i,i=1000,p=1,q=1".to_owned(),
            format!("a=d,d=i,i={FIRST_EMPHASIS},p=1,q=1"),
        ]
    );
}

/// m8 04: an upload is known by its key, not by the pixels it came from.
/// Once transmitted, Kitty keeps nothing of the frame: the cache that made
/// it is the only owner, and its eviction frees the pixels.
#[test]
fn an_upload_holds_no_pixels_after_transmit() {
    let mut images = KittyImages::new(usize::MAX);
    let frame = strip(4, [10, 20, 30, 255]);
    let mut out = Vec::new();
    images.frame(&[place(&frame, 0, 0..4, 1)], &mut out);
    assert_eq!(images.uploaded(), 1);
    assert_eq!(Arc::strong_count(&frame), 1, "Kitty kept a reference");
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

/// m8 04: a transmit works in buffers kept from the last one. Two strips of
/// one size, one sparse and one noise (which overflows the first guess at the
/// deflated size): each sends exactly what `compress_to_vec_zlib` would, and
/// the second makes no buffer the first did not.
#[test]
fn a_transmit_reuses_its_buffers_and_sends_the_same_bytes() {
    let noise = |seed: u32| {
        let pixels: Vec<u8> = (0..6 * 400u32)
            .flat_map(|at| {
                let [r, g, b, _] = (at ^ seed).wrapping_mul(2_654_435_761).to_le_bytes();
                [r, g, b, 255]
            })
            .collect();
        Arc::new(RasterFrame {
            pixels,
            width: 6,
            height: 400,
            rows: 100,
            columns: 3,
        })
    };
    let mut images = KittyImages::new(usize::MAX).deflating(true);
    let (sparse, loud, louder) = (strip(100, [0, 0, 0, 0]), noise(1), noise(2));
    for (frame, row) in [(&sparse, 0), (&loud, 1)] {
        let mut out = Vec::new();
        images.frame(&[place(frame, row, 0..100, 0)], &mut out);
        let sent: Vec<u8> = transmits(&out)
            .into_iter()
            .flat_map(|(_, bytes)| bytes)
            .collect();
        let straight = straight_alpha(&frame.pixels);
        assert_eq!(
            sent,
            miniz_oxide::deflate::compress_to_vec_zlib(&straight, 1)
        );
    }
    let buffers = |images: &KittyImages| {
        let scratch = &images.scratch;
        [
            scratch.straight.as_ptr() as usize,
            scratch.deflated.as_ptr() as usize,
            scratch.encoded.as_ptr() as usize,
        ]
    };
    let before = buffers(&images);
    let mut out = Vec::new();
    images.frame(&[place(&louder, 2, 0..100, 0)], &mut out);
    assert_eq!(buffers(&images), before, "a new buffer for the same size");
}
