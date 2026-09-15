//! Turning a terminal mouse report's button code into an [`Event::Mouse`].

use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};

use super::csi::number;
use super::{Decoded, Step};

/// `CSI < button ; column ; row M|m`, 1-based coordinates.
pub(super) fn sgr_mouse(params: &[u8], final_byte: u8) -> Option<Decoded> {
    let button = number(params, 0)?;
    let column = number(params, 1)?;
    let row = number(params, 2)?;
    Some(mouse(button, column, row, final_byte == b'm'))
}

/// `CSI M b x y`, each byte offset by 32 and coordinates 1-based.
pub(super) fn legacy_mouse(bytes: &[u8]) -> Step {
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
