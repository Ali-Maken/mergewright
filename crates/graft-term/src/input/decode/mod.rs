//! Turning the bytes a terminal sends into key events and terminal replies.
//!
//! Pure: no file descriptor, no clock. The mux feeds bytes in as they are
//! read and asks for a flush when nothing more has arrived in time; every
//! decision about what a byte *means* is made here, so every corner of it is
//! a plain function call in a test.
//!
//! Two kinds of thing come out, and telling them apart is the whole job:
//!
//! - **Replies** — answers to queries Graft sent (`CSI ? … c`, `CSI … t`,
//!   `CSI ? … u`, `CSI ? … $ y`, and every OSC, APC and DCS string). No key
//!   ever produces one of these shapes, so they are routed to whoever is
//!   waiting and never reach the keymap.
//! - **Events** — keys, mouse, paste and focus, in Graft's own vocabulary.
//!
//! A sequence still arriving is held, not guessed at: [`Decoder::feed`] keeps
//! an unfinished tail for the next read, so a reply split across two reads is
//! still one reply. [`Decoder::flush`] is the mux saying "nothing more is
//! coming": whatever is held is read as keys, which is how a lone `Esc` press
//! is told apart from the start of a sequence.
//!
//! Split across three files by concern: this one is the escape-sequence
//! framing (deciding what kind of thing is starting and where it ends);
//! [`csi`] reads `CSI` sequences into keys; [`mouse`] turns a button code
//! into a [`MouseInput`](graft_core::event::MouseInput).

mod csi;
mod mouse;

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

use csi::csi;

const ESCAPE: u8 = 0x1b;
const BELL: u8 = 0x07;
const PASTE_END: &[u8] = b"\x1b[201~";

/// One thing the terminal sent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decoded {
    /// A key, mouse, paste or focus event for the application.
    Event(Event),
    /// A reply to a query, as the exact bytes the terminal sent, escape
    /// introducer and terminator included, so a reply parser reads it
    /// unchanged.
    Reply(Vec<u8>),
}

/// Bytes in, [`Decoded`] out, holding back a sequence that has not finished
/// arriving.
#[derive(Debug, Default)]
pub struct Decoder {
    held: Vec<u8>,
}

/// What reading one item at the front of the buffer found.
pub(super) enum Step {
    /// An item (or nothing worth reporting) and how many bytes it used.
    Took(Option<Decoded>, usize),
    /// The buffer ends mid-item: wait for more bytes, or a flush.
    Unfinished,
}

impl Decoder {
    /// A decoder holding nothing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Is a sequence waiting for its remaining bytes?
    pub fn is_holding(&self) -> bool {
        !self.held.is_empty()
    }

    /// Decode `bytes` after whatever was held, appending finished items to
    /// `out` in the order they were sent. An unfinished tail is kept.
    pub fn feed(&mut self, bytes: &[u8], out: &mut Vec<Decoded>) {
        self.held.extend_from_slice(bytes);
        let used = drain(&self.held, false, out);
        self.held.drain(..used);
    }

    /// Nothing more arrived in time: read whatever is held as keys.
    pub fn flush(&mut self, out: &mut Vec<Decoded>) {
        let used = drain(&self.held, true, out);
        debug_assert_eq!(used, self.held.len(), "a flush always finishes");
        self.held.clear();
    }
}

/// Decode items from the front of `bytes` until one is unfinished; returns
/// how many bytes were used. With `flushing`, nothing is unfinished.
fn drain(bytes: &[u8], flushing: bool, out: &mut Vec<Decoded>) -> usize {
    let mut position = 0;
    while position < bytes.len() {
        match step(&bytes[position..], flushing) {
            Step::Took(item, used) => {
                out.extend(item);
                position += used;
            }
            Step::Unfinished => break,
        }
    }
    position
}

fn step(bytes: &[u8], flushing: bool) -> Step {
    if bytes[0] != ESCAPE {
        return plain(bytes, Modifiers::NONE, 0, flushing);
    }
    let Some(&introducer) = bytes.get(1) else {
        return finished_or(flushing, Some(key(KeyCode::Escape, Modifiers::NONE)), 1);
    };
    let step = match introducer {
        b'[' => csi(bytes),
        b']' => string_sequence(bytes, true),
        b'_' | b'P' => string_sequence(bytes, false),
        b'O' => ss3(bytes),
        // `Esc Esc`: the first is a key; the second starts over, so an
        // `Esc` pressed just before an arrow key still leaves the arrow.
        ESCAPE => Step::Took(Some(key(KeyCode::Escape, Modifiers::NONE)), 1),
        _ => return plain(&bytes[1..], alt(), 1, flushing),
    };
    match step {
        // Nothing more is coming, so the introducer was typed: `Alt+[` and
        // so on. Only the first two bytes are taken; the rest decode again.
        Step::Unfinished if flushing => plain(&bytes[1..], alt(), 1, true),
        step => step,
    }
}

fn finished_or(flushing: bool, item: Option<Decoded>, used: usize) -> Step {
    if flushing {
        Step::Took(item, used)
    } else {
        Step::Unfinished
    }
}

pub(super) fn alt() -> Modifiers {
    Modifiers {
        alt: true,
        ..Modifiers::NONE
    }
}

pub(super) fn key(code: KeyCode, modifiers: Modifiers) -> Decoded {
    Decoded::Event(Event::Key(KeyPress { code, modifiers }))
}

/// One key that is not an escape sequence: a control byte or a UTF-8
/// character. `prefix` bytes (an `Alt` escape) were already used.
pub(super) fn plain(bytes: &[u8], modifiers: Modifiers, prefix: usize, flushing: bool) -> Step {
    let first = bytes[0];
    let control = |character: u8| {
        let modifiers = Modifiers {
            control: true,
            ..modifiers
        };
        Step::Took(
            Some(key(KeyCode::Char(char::from(character)), modifiers)),
            prefix + 1,
        )
    };
    match first {
        b'\r' => Step::Took(Some(key(KeyCode::Enter, modifiers)), prefix + 1),
        b'\t' => Step::Took(Some(key(KeyCode::Tab, modifiers)), prefix + 1),
        0x7f => Step::Took(Some(key(KeyCode::Backspace, modifiers)), prefix + 1),
        0x00 => control(b' '),
        0x01..=0x1a => control(first - 0x01 + b'a'),
        0x1c..=0x1f => control(first - 0x1c + b'4'),
        _ => character(bytes, modifiers, prefix, flushing),
    }
}

fn character(bytes: &[u8], mut modifiers: Modifiers, prefix: usize, flushing: bool) -> Step {
    let width = match bytes[0] {
        0x00..=0x7f => 1,
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        // A continuation byte or garbage where a character should start. Not
        // a key anyone pressed; stepping over it is the only way forward.
        _ => return Step::Took(None, prefix + 1),
    };
    let Some(encoded) = bytes.get(..width) else {
        // A multi-byte character split across reads; a flush means the rest
        // never came, which is no character at all.
        return finished_or(flushing, None, prefix + bytes.len());
    };
    let Some(character) = std::str::from_utf8(encoded)
        .ok()
        .and_then(|text| text.chars().next())
    else {
        return Step::Took(None, prefix + 1);
    };
    if character.is_uppercase() {
        modifiers.shift = true;
    }
    Step::Took(
        Some(key(KeyCode::Char(character), modifiers)),
        prefix + width,
    )
}

/// `ESC O x`: F1–F4 and the cursor keys in application mode.
fn ss3(bytes: &[u8]) -> Step {
    let Some(&last) = bytes.get(2) else {
        return Step::Unfinished;
    };
    let code = match last {
        b'P'..=b'S' => Some(KeyCode::Function(last - b'P' + 1)),
        b'M' => Some(KeyCode::Enter),
        other => cursor_key(other),
    };
    Step::Took(code.map(|code| key(code, Modifiers::NONE)), 3)
}

pub(super) fn cursor_key(final_byte: u8) -> Option<KeyCode> {
    Some(match final_byte {
        b'A' => KeyCode::Up,
        b'B' => KeyCode::Down,
        b'C' => KeyCode::Right,
        b'D' => KeyCode::Left,
        b'H' => KeyCode::Home,
        b'F' => KeyCode::End,
        _ => return None,
    })
}

/// `ESC ]` (OSC), `ESC _` (APC) or `ESC P` (DCS): always a reply. OSC may end
/// with `BEL` as well as `ST`.
fn string_sequence(bytes: &[u8], bell_ends: bool) -> Step {
    let mut end = 2;
    while let Some(&byte) = bytes.get(end) {
        match byte {
            BELL if bell_ends => return reply(bytes, end + 1),
            ESCAPE => {
                return match bytes.get(end + 1) {
                    None => Step::Unfinished,
                    Some(b'\\') => reply(bytes, end + 2),
                    // Another sequence began before this one closed, so this
                    // was never a string: the introducer was a key.
                    Some(_) => plain(&bytes[1..], alt(), 1, true),
                };
            }
            _ => end += 1,
        }
    }
    Step::Unfinished
}

pub(super) fn reply(bytes: &[u8], length: usize) -> Step {
    Step::Took(Some(Decoded::Reply(bytes[..length].to_vec())), length)
}

#[cfg(test)]
mod tests;
