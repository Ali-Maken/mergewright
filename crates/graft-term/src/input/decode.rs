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

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};

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
enum Step {
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

fn alt() -> Modifiers {
    Modifiers {
        alt: true,
        ..Modifiers::NONE
    }
}

fn key(code: KeyCode, modifiers: Modifiers) -> Decoded {
    Decoded::Event(Event::Key(KeyPress { code, modifiers }))
}

/// One key that is not an escape sequence: a control byte or a UTF-8
/// character. `prefix` bytes (an `Alt` escape) were already used.
fn plain(bytes: &[u8], modifiers: Modifiers, prefix: usize, flushing: bool) -> Step {
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

fn cursor_key(final_byte: u8) -> Option<KeyCode> {
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

fn reply(bytes: &[u8], length: usize) -> Step {
    Step::Took(Some(Decoded::Reply(bytes[..length].to_vec())), length)
}

/// `ESC [`: a key, mouse report, paste, focus change or reply.
fn csi(bytes: &[u8]) -> Step {
    let mut end = 2;
    while matches!(bytes.get(end), Some(0x30..=0x3f)) {
        end += 1;
    }
    let params = &bytes[2..end];
    while matches!(bytes.get(end), Some(0x20..=0x2f)) {
        end += 1;
    }
    let intermediates = &bytes[2 + params.len()..end];
    let final_byte = match bytes.get(end) {
        None => return Step::Unfinished,
        Some(&byte @ 0x40..=0x7e) => byte,
        // A control byte where the final byte belongs: this was never a CSI,
        // so `ESC [` was `Alt+[` and what follows decodes on its own.
        Some(_) => return plain(&bytes[1..], alt(), 1, true),
    };
    let length = end + 1;

    if params.first() == Some(&b'?') || final_byte == b't' || !intermediates.is_empty() {
        return reply(bytes, length);
    }
    match (params, final_byte) {
        (b"200", b'~') => paste(bytes, length),
        (b"", b'M') => legacy_mouse(bytes),
        (b"", b'I') => Step::Took(Some(Decoded::Event(Event::FocusGained)), length),
        (b"", b'O') => Step::Took(Some(Decoded::Event(Event::FocusLost)), length),
        _ if params.first() == Some(&b'<') => {
            Step::Took(sgr_mouse(&params[1..], final_byte), length)
        }
        _ => Step::Took(csi_key(params, final_byte), length),
    }
}

fn paste(bytes: &[u8], start: usize) -> Step {
    let body = &bytes[start..];
    match body
        .windows(PASTE_END.len())
        .position(|window| window == PASTE_END)
    {
        Some(end) => {
            let text = String::from_utf8_lossy(&body[..end]).into_owned();
            Step::Took(
                Some(Decoded::Event(Event::Paste(text))),
                start + end + PASTE_END.len(),
            )
        }
        None => Step::Unfinished,
    }
}

/// The numbers in one `;`-separated field, split on `:`. Empty or
/// non-numeric sub-fields read as `None`.
fn field(params: &[u8], index: usize) -> Vec<Option<u32>> {
    params
        .split(|&byte| byte == b';')
        .nth(index)
        .map(|field| {
            field
                .split(|&byte| byte == b':')
                .map(|part| std::str::from_utf8(part).ok()?.parse().ok())
                .collect()
        })
        .unwrap_or_default()
}

fn number(params: &[u8], index: usize) -> Option<u32> {
    field(params, index).first().copied().flatten()
}

/// The xterm / kitty modifier parameter: one plus a bit mask.
fn modifiers(parameter: Option<u32>) -> Modifiers {
    let bits = parameter.unwrap_or(1).saturating_sub(1);
    Modifiers {
        shift: bits & 1 != 0,
        alt: bits & 2 != 0,
        control: bits & 4 != 0,
        command: bits & 8 != 0,
    }
}

fn csi_key(params: &[u8], final_byte: u8) -> Option<Decoded> {
    let held = modifiers(number(params, 1));
    let code = match final_byte {
        b'~' => tilde_key(number(params, 0)?)?,
        b'P'..=b'S' => KeyCode::Function(final_byte - b'P' + 1),
        b'Z' => KeyCode::BackTab,
        b'u' => return kitty_key(params),
        other => cursor_key(other)?,
    };
    Some(key(code, held))
}

fn tilde_key(number: u32) -> Option<KeyCode> {
    Some(match number {
        1 | 7 => KeyCode::Home,
        2 => KeyCode::Insert,
        3 => KeyCode::Delete,
        4 | 8 => KeyCode::End,
        5 => KeyCode::PageUp,
        6 => KeyCode::PageDown,
        11..=15 => KeyCode::Function(u8::try_from(number - 10).ok()?),
        17..=21 => KeyCode::Function(u8::try_from(number - 11).ok()?),
        23..=26 => KeyCode::Function(u8::try_from(number - 12).ok()?),
        _ => return None,
    })
}

/// `CSI codepoint ; modifiers:event u`, the kitty keyboard protocol. A
/// release (event 3) is dropped: every Graft binding fires on the way down.
fn kitty_key(params: &[u8]) -> Option<Decoded> {
    let codepoint = number(params, 0)?;
    let second = field(params, 1);
    let mut held = modifiers(second.first().copied().flatten());
    if second.get(1).copied().flatten() == Some(3) {
        return None;
    }
    let code = match codepoint {
        27 => KeyCode::Escape,
        13 => KeyCode::Enter,
        9 => KeyCode::Tab,
        127 => KeyCode::Backspace,
        // The protocol's private-use range: lock keys, keypad, media keys and
        // bare modifiers. None has a binding anywhere in Graft.
        57_344..=63_743 => return None,
        _ => {
            let character = char::from_u32(codepoint)?;
            if held.shift {
                KeyCode::Char(character.to_uppercase().next()?)
            } else {
                if character.is_uppercase() {
                    held.shift = true;
                }
                KeyCode::Char(character)
            }
        }
    };
    Some(key(code, held))
}

/// `CSI < button ; column ; row M|m`, 1-based coordinates.
fn sgr_mouse(params: &[u8], final_byte: u8) -> Option<Decoded> {
    let button = number(params, 0)?;
    let column = number(params, 1)?;
    let row = number(params, 2)?;
    Some(mouse(button, column, row, final_byte == b'm'))
}

/// `CSI M b x y`, each byte offset by 32 and coordinates 1-based.
fn legacy_mouse(bytes: &[u8]) -> Step {
    let Some(&[button, column, row]) = bytes.get(3..6) else {
        return Step::Unfinished;
    };
    let decoded = Some(mouse(
        u32::from(button.saturating_sub(32)),
        u32::from(column.saturating_sub(32)),
        u32::from(row.saturating_sub(32)),
        false,
    ));
    Step::Took(decoded, 6)
}

fn mouse(code: u32, column: u32, row: u32, released: bool) -> Decoded {
    let button = match code & 3 {
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        // 3 is "no button": a release in the legacy encoding, or motion.
        _ => MouseButton::Left,
    };
    let no_button = code & 3 == 3;
    let kind = if code & 64 != 0 {
        match code & 3 {
            0 => MouseKind::ScrollUp,
            1 => MouseKind::ScrollDown,
            2 => MouseKind::ScrollLeft,
            _ => MouseKind::ScrollRight,
        }
    } else if code & 32 != 0 {
        if no_button {
            MouseKind::Moved
        } else {
            MouseKind::Drag(button)
        }
    } else if released || no_button {
        MouseKind::Up(button)
    } else {
        MouseKind::Down(button)
    };
    let cell = |value: u32| u16::try_from(value.saturating_sub(1)).unwrap_or(u16::MAX);
    Decoded::Event(Event::Mouse(MouseInput {
        kind,
        column: cell(column),
        row: cell(row),
        modifiers: Modifiers {
            shift: code & 4 != 0,
            alt: code & 8 != 0,
            control: code & 16 != 0,
            command: false,
        },
    }))
}

#[cfg(test)]
mod tests;
