use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};
use proptest::prelude::*;

use super::{Decoded, Decoder};

fn decode_all(chunks: &[&[u8]]) -> Vec<Decoded> {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    for chunk in chunks {
        decoder.feed(chunk, &mut out);
    }
    decoder.flush(&mut out);
    out
}

fn keys(modifiers: Modifiers, codes: &[KeyCode]) -> Vec<Decoded> {
    codes
        .iter()
        .map(|&code| Decoded::Event(Event::Key(KeyPress { code, modifiers })))
        .collect()
}

fn one(code: KeyCode, modifiers: Modifiers) -> Decoded {
    Decoded::Event(Event::Key(KeyPress { code, modifiers }))
}

fn reply(bytes: &[u8]) -> Decoded {
    Decoded::Reply(bytes.to_vec())
}

const SHIFT: Modifiers = Modifiers {
    shift: true,
    ..Modifiers::NONE
};
const ALT: Modifiers = Modifiers {
    alt: true,
    ..Modifiers::NONE
};

#[test]
fn plain_keys_come_out_in_order() {
    assert_eq!(
        decode_all(&[b"jk1 "]),
        keys(
            Modifiers::NONE,
            &[
                KeyCode::Char('j'),
                KeyCode::Char('k'),
                KeyCode::Char('1'),
                KeyCode::Char(' ')
            ]
        )
    );
}

#[test]
fn an_uppercase_letter_carries_shift() {
    assert_eq!(decode_all(&[b"H"]), vec![one(KeyCode::Char('H'), SHIFT)]);
}

#[test]
fn control_bytes_are_named_keys_or_control_chords() {
    assert_eq!(
        decode_all(&[b"\r\t\x7f\x03"]),
        vec![
            one(KeyCode::Enter, Modifiers::NONE),
            one(KeyCode::Tab, Modifiers::NONE),
            one(KeyCode::Backspace, Modifiers::NONE),
            one(KeyCode::Char('c'), Modifiers::CONTROL),
        ]
    );
}

#[test]
fn a_lone_escape_is_held_until_the_flush_then_is_a_key() {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    decoder.feed(b"\x1b", &mut out);
    assert!(out.is_empty(), "held, not guessed at");
    assert!(decoder.is_holding());

    decoder.flush(&mut out);

    assert_eq!(out, vec![one(KeyCode::Escape, Modifiers::NONE)]);
    assert!(!decoder.is_holding());
}

#[test]
fn escape_then_a_character_is_alt() {
    assert_eq!(decode_all(&[b"\x1bj"]), vec![one(KeyCode::Char('j'), ALT)]);
}

#[test]
fn escape_escape_leaves_the_following_sequence_intact() {
    assert_eq!(
        decode_all(&[b"\x1b\x1b[A"]),
        vec![
            one(KeyCode::Escape, Modifiers::NONE),
            one(KeyCode::Up, Modifiers::NONE)
        ]
    );
}

#[test]
fn a_reply_split_across_two_reads_is_one_reply() {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    decoder.feed(b"\x1b[?62;", &mut out);
    assert!(out.is_empty());
    decoder.feed(b"4c", &mut out);

    assert_eq!(out, vec![reply(b"\x1b[?62;4c")]);
}

#[test]
fn a_fence_answer_is_a_reply_never_a_key() {
    // ADR-0054: `CSI 0 n` answers the per-frame fence. It goes to the reply
    // path whole, and the keys either side of it survive.
    assert_eq!(
        decode_all(&[b"j\x1b[0nk"]),
        vec![
            one(KeyCode::Char('j'), Modifiers::NONE),
            reply(b"\x1b[0n"),
            one(KeyCode::Char('k'), Modifiers::NONE),
        ]
    );
}

#[test]
fn a_fence_answer_split_across_reads_is_one_reply() {
    let mut decoder = Decoder::new();
    let mut out = Vec::new();
    decoder.feed(b"\x1b[0", &mut out);
    assert!(out.is_empty(), "held until it finishes");
    decoder.feed(b"n\x1b[", &mut out);
    decoder.feed(b"0n", &mut out);

    assert_eq!(out, vec![reply(b"\x1b[0n"), reply(b"\x1b[0n")]);
    let answers: usize = out
        .iter()
        .map(|item| match item {
            Decoded::Reply(bytes) => crate::pace::fence_answers(bytes),
            Decoded::Event(_) => 0,
        })
        .sum();
    assert_eq!(answers, 2);
}

#[test]
fn every_probe_reply_shape_is_a_reply_and_keys_between_them_survive_in_order() {
    let decoded = decode_all(&[
        b"a\x1b_Gi=31;OK\x1b\\b\x1b[6;16;8tc\x1b[?1ud\x1b[?2026;2$ye\x1b]11;rgb:0000/0000/0000\x07f\x1b[?62;4c",
    ]);

    let character = |c| one(KeyCode::Char(c), Modifiers::NONE);
    assert_eq!(
        decoded,
        vec![
            character('a'),
            reply(b"\x1b_Gi=31;OK\x1b\\"),
            character('b'),
            reply(b"\x1b[6;16;8t"),
            character('c'),
            reply(b"\x1b[?1u"),
            character('d'),
            reply(b"\x1b[?2026;2$y"),
            character('e'),
            reply(b"\x1b]11;rgb:0000/0000/0000\x07"),
            character('f'),
            reply(b"\x1b[?62;4c"),
        ]
    );
}

#[test]
fn function_keys_in_every_encoding_graft_binds() {
    assert_eq!(
        decode_all(&[b"\x1bOQ\x1bOR\x1b[13~\x1b[1;2R"]),
        vec![
            one(KeyCode::Function(2), Modifiers::NONE),
            one(KeyCode::Function(3), Modifiers::NONE),
            one(KeyCode::Function(3), Modifiers::NONE),
            one(KeyCode::Function(3), SHIFT),
        ]
    );
}

#[test]
fn cursor_and_editing_keys_with_modifiers() {
    assert_eq!(
        decode_all(&[b"\x1b[A\x1b[1;5B\x1b[5~\x1b[3~\x1b[Z"]),
        vec![
            one(KeyCode::Up, Modifiers::NONE),
            one(KeyCode::Down, Modifiers::CONTROL),
            one(KeyCode::PageUp, Modifiers::NONE),
            one(KeyCode::Delete, Modifiers::NONE),
            one(KeyCode::BackTab, Modifiers::NONE),
        ]
    );
}

#[test]
fn kitty_keyboard_keys_and_a_dropped_release() {
    assert_eq!(
        decode_all(&[b"\x1b[27u\x1b[99;5u\x1b[106;1:3u\x1b[102;2u"]),
        vec![
            one(KeyCode::Escape, Modifiers::NONE),
            one(KeyCode::Char('c'), Modifiers::CONTROL),
            one(KeyCode::Char('F'), SHIFT),
        ]
    );
}

fn mouse(kind: MouseKind, column: u16, row: u16) -> Decoded {
    Decoded::Event(Event::Mouse(MouseInput {
        kind,
        column,
        row,
        modifiers: Modifiers::NONE,
    }))
}

#[test]
fn sgr_mouse_reports_are_zero_based() {
    assert_eq!(
        decode_all(&[
            b"\x1b[<0;10;5M\x1b[<0;10;5m\x1b[<64;1;1M\x1b[<65;1;1M\x1b[<35;3;4M\x1b[<32;2;2M"
        ]),
        vec![
            mouse(MouseKind::Down(MouseButton::Left), 9, 4),
            mouse(MouseKind::Up(MouseButton::Left), 9, 4),
            mouse(MouseKind::ScrollUp, 0, 0),
            mouse(MouseKind::ScrollDown, 0, 0),
            mouse(MouseKind::Moved, 2, 3),
            mouse(MouseKind::Drag(MouseButton::Left), 1, 1),
        ]
    );
}

#[test]
fn a_paste_split_across_reads_is_one_paste() {
    assert_eq!(
        decode_all(&[b"\x1b[200~two w", b"ords\x1b[20", b"1~q"]),
        vec![
            Decoded::Event(Event::Paste("two words".to_owned())),
            one(KeyCode::Char('q'), Modifiers::NONE),
        ]
    );
}

#[test]
fn focus_changes() {
    assert_eq!(
        decode_all(&[b"\x1b[I\x1b[O"]),
        vec![
            Decoded::Event(Event::FocusGained),
            Decoded::Event(Event::FocusLost)
        ]
    );
}

#[test]
fn a_character_split_across_reads_is_one_character() {
    let encoded = "é".as_bytes();
    assert_eq!(
        decode_all(&[&encoded[..1], &encoded[1..]]),
        vec![one(KeyCode::Char('é'), Modifiers::NONE)]
    );
}

#[test]
fn a_sequence_that_never_finished_is_read_as_the_keys_it_was() {
    assert_eq!(
        decode_all(&[b"\x1b[1;"]),
        vec![
            one(KeyCode::Char('['), ALT),
            one(KeyCode::Char('1'), Modifiers::NONE),
            one(KeyCode::Char(';'), Modifiers::NONE),
        ]
    );
}

fn tier() -> ProptestConfig {
    ProptestConfig::with_cases(graft_fixtures::proptest_cases())
}

proptest! {
    #![proptest_config(tier())]

    /// How the bytes were split across reads never changes what they mean.
    #[test]
    fn splitting_the_input_changes_nothing(
        bytes in proptest::collection::vec(
            prop_oneof![Just(0x1b_u8), Just(b'['), Just(b'?'), Just(b';'), Just(b'c'), any::<u8>()],
            0..64,
        ),
        cut in any::<prop::sample::Index>(),
    ) {
        let cut = cut.index(bytes.len() + 1);
        let whole = decode_all(&[&bytes]);
        let split = decode_all(&[&bytes[..cut], &bytes[cut..]]);
        prop_assert_eq!(whole, split);
    }
}
