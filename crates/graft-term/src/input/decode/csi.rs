//! `ESC [` sequences: keys, mouse reports, paste and focus, or a reply.

use graft_core::event::{Event, KeyCode, Modifiers};

use super::mouse::{legacy_mouse, sgr_mouse};
use super::{Decoded, PASTE_END, Step, alt, cursor_key, key, plain, reply};

/// `ESC [`: a key, mouse report, paste, focus change or reply.
pub(super) fn csi(bytes: &[u8]) -> Step {
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

    // `n` is a device status report: `CSI 0 n` answers the per-frame fence
    // (ADR-0054). No key ends in `n`, so it can only be a reply.
    if params.first() == Some(&b'?')
        || matches!(final_byte, b't' | b'n')
        || !intermediates.is_empty()
    {
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

pub(super) fn number(params: &[u8], index: usize) -> Option<u32> {
    field(params, index).first().copied().flatten()
}

/// The xterm / kitty modifier parameter: one plus a bit mask.
pub(super) fn modifiers(parameter: Option<u32>) -> Modifiers {
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
