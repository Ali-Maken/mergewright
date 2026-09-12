//! Terminal input: crossterm's events, turned into `core`'s.
//!
//! Two things live here, and they are kept apart on purpose. [`translate`] is a
//! pure function — no terminal, no thread — so every corner of the mapping can
//! be tested as a plain call. [`spawn_reader`] is the thread that blocks on the
//! real terminal and feeds the loop; it contains as little logic as possible.
//!
//! Translating at this boundary is what keeps crossterm out of `core`: below
//! this module the application speaks only [`graft_core::event::Event`], so
//! swapping the terminal library would change this file and nothing else.

use std::io;
use std::sync::mpsc::Sender;
use std::thread;

use crossterm::event::{
    Event as TerminalEvent, KeyCode as TerminalKeyCode, KeyEvent, KeyEventKind, KeyModifiers,
    MouseButton as TerminalMouseButton, MouseEvent, MouseEventKind,
};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind};

use super::Wake;
use crate::update::AppEvent;

/// Read the terminal forever on a thread of its own, sending each event to the
/// loop.
///
/// Reading blocks, which is the point: an idle Graft costs nothing, because
/// nothing polls. The thread ends when the receiver is gone (the loop has
/// finished) or when reading fails.
pub fn spawn_reader(wakes: Sender<Wake>) -> io::Result<()> {
    thread::Builder::new()
        .name("graft-input".to_owned())
        .spawn(move || read_until_done(&wakes))?;
    Ok(())
}

fn read_until_done(wakes: &Sender<Wake>) {
    loop {
        let wake = match crossterm::event::read() {
            // Events with no meaning to Graft are dropped here rather than
            // waking the loop to do nothing.
            Ok(event) => match translate(event) {
                Some(event) => Wake::Event(AppEvent::Core(event)),
                None => continue,
            },
            // A failed read is not survivable — there is no input any more —
            // so it goes to the loop, which stops and reports it. It is never
            // retried and never ignored.
            Err(error) => Wake::InputFailed(error),
        };

        if wakes.send(wake).is_err() {
            // The loop has already finished and dropped the receiver. There is
            // nobody left to tell, and nothing left to do.
            return;
        }
    }
}

/// Map one crossterm event onto Graft's own vocabulary.
///
/// `None` means "nothing Graft can act on": a key release, a bare modifier
/// press, a key with no binding anywhere in the application. Dropping those
/// here keeps `update` free of cases that can never matter.
#[must_use]
pub fn translate(event: TerminalEvent) -> Option<Event> {
    match event {
        TerminalEvent::Key(key) => translate_key(key).map(Event::Key),
        TerminalEvent::Mouse(mouse) => Some(Event::Mouse(translate_mouse(mouse))),
        TerminalEvent::Resize(columns, rows) => Some(Event::Resize { columns, rows }),
        TerminalEvent::Paste(text) => Some(Event::Paste(text)),
        TerminalEvent::FocusGained => Some(Event::FocusGained),
        TerminalEvent::FocusLost => Some(Event::FocusLost),
    }
}

fn translate_key(key: KeyEvent) -> Option<KeyPress> {
    // Releases are dropped: every Graft binding fires on the way down. They
    // only arrive at all because the kitty protocol reports them, and acting
    // on both halves would run each binding twice.
    if key.kind == KeyEventKind::Release {
        return None;
    }

    Some(KeyPress {
        code: translate_key_code(key.code)?,
        modifiers: translate_modifiers(key.modifiers),
    })
}

fn translate_key_code(code: TerminalKeyCode) -> Option<KeyCode> {
    let translated = match code {
        TerminalKeyCode::Char(character) => KeyCode::Char(character),
        TerminalKeyCode::Enter => KeyCode::Enter,
        TerminalKeyCode::Esc => KeyCode::Escape,
        TerminalKeyCode::Backspace => KeyCode::Backspace,
        TerminalKeyCode::Tab => KeyCode::Tab,
        TerminalKeyCode::BackTab => KeyCode::BackTab,
        TerminalKeyCode::Delete => KeyCode::Delete,
        TerminalKeyCode::Insert => KeyCode::Insert,
        TerminalKeyCode::Left => KeyCode::Left,
        TerminalKeyCode::Right => KeyCode::Right,
        TerminalKeyCode::Up => KeyCode::Up,
        TerminalKeyCode::Down => KeyCode::Down,
        TerminalKeyCode::Home => KeyCode::Home,
        TerminalKeyCode::End => KeyCode::End,
        TerminalKeyCode::PageUp => KeyCode::PageUp,
        TerminalKeyCode::PageDown => KeyCode::PageDown,
        TerminalKeyCode::F(number) => KeyCode::Function(number),
        // Everything else — lock keys, media keys, the menu key, a modifier
        // pressed on its own, `Null` — has no binding and never will.
        _ => return None,
    };
    Some(translated)
}

fn translate_modifiers(modifiers: KeyModifiers) -> Modifiers {
    Modifiers {
        control: modifiers.contains(KeyModifiers::CONTROL),
        alt: modifiers.contains(KeyModifiers::ALT),
        shift: modifiers.contains(KeyModifiers::SHIFT),
        command: modifiers.contains(KeyModifiers::SUPER),
    }
}

fn translate_mouse(mouse: MouseEvent) -> MouseInput {
    MouseInput {
        kind: translate_mouse_kind(mouse.kind),
        column: mouse.column,
        row: mouse.row,
        modifiers: translate_modifiers(mouse.modifiers),
    }
}

fn translate_mouse_kind(kind: MouseEventKind) -> MouseKind {
    match kind {
        MouseEventKind::Down(button) => MouseKind::Down(translate_button(button)),
        MouseEventKind::Up(button) => MouseKind::Up(translate_button(button)),
        MouseEventKind::Drag(button) => MouseKind::Drag(translate_button(button)),
        MouseEventKind::Moved => MouseKind::Moved,
        MouseEventKind::ScrollUp => MouseKind::ScrollUp,
        MouseEventKind::ScrollDown => MouseKind::ScrollDown,
        MouseEventKind::ScrollLeft => MouseKind::ScrollLeft,
        MouseEventKind::ScrollRight => MouseKind::ScrollRight,
    }
}

fn translate_button(button: TerminalMouseButton) -> MouseButton {
    match button {
        TerminalMouseButton::Left => MouseButton::Left,
        TerminalMouseButton::Middle => MouseButton::Middle,
        TerminalMouseButton::Right => MouseButton::Right,
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::KeyEventState;

    use super::*;

    fn press(code: TerminalKeyCode, modifiers: KeyModifiers) -> TerminalEvent {
        TerminalEvent::Key(KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })
    }

    #[test]
    fn a_plain_character() {
        let translated = translate(press(TerminalKeyCode::Char('q'), KeyModifiers::NONE));

        assert_eq!(
            translated,
            Some(Event::Key(KeyPress::plain(KeyCode::Char('q'))))
        );
    }

    #[test]
    fn control_c_keeps_its_modifier() {
        // In raw mode the terminal no longer turns this into a signal, so it
        // has to survive the translation for the keymap to see it.
        let translated = translate(press(TerminalKeyCode::Char('c'), KeyModifiers::CONTROL));

        assert_eq!(
            translated,
            Some(Event::Key(KeyPress::control(KeyCode::Char('c'))))
        );
    }

    #[test]
    fn every_modifier_graft_names() {
        let all =
            KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT | KeyModifiers::SUPER;

        let translated = translate(press(TerminalKeyCode::Left, all));

        assert_eq!(
            translated,
            Some(Event::Key(KeyPress {
                code: KeyCode::Left,
                modifiers: Modifiers {
                    control: true,
                    alt: true,
                    shift: true,
                    command: true,
                },
            }))
        );
    }

    #[test]
    fn a_key_release_is_dropped() {
        // Only reported at all when the kitty protocol is on. Acting on it
        // would fire every binding twice.
        let release = TerminalEvent::Key(KeyEvent {
            code: TerminalKeyCode::Char('q'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: KeyEventState::NONE,
        });

        assert_eq!(translate(release), None);
    }

    #[test]
    fn a_repeat_is_a_press() {
        // Holding `j` must keep scrolling.
        let repeat = TerminalEvent::Key(KeyEvent {
            code: TerminalKeyCode::Char('j'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Repeat,
            state: KeyEventState::NONE,
        });

        assert_eq!(
            translate(repeat),
            Some(Event::Key(KeyPress::plain(KeyCode::Char('j'))))
        );
    }

    #[test]
    fn an_unbindable_key_is_dropped() {
        assert_eq!(
            translate(press(TerminalKeyCode::CapsLock, KeyModifiers::NONE)),
            None
        );
        assert_eq!(
            translate(press(TerminalKeyCode::Null, KeyModifiers::NONE)),
            None
        );
    }

    #[test]
    fn a_resize_keeps_both_numbers_even_when_degenerate() {
        assert_eq!(
            translate(TerminalEvent::Resize(0, 0)),
            Some(Event::Resize {
                columns: 0,
                rows: 0
            })
        );
        assert_eq!(
            translate(TerminalEvent::Resize(120, 40)),
            Some(Event::Resize {
                columns: 120,
                rows: 40
            })
        );
    }

    #[test]
    fn a_click() {
        let click = TerminalEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(TerminalMouseButton::Right),
            column: 7,
            row: 3,
            modifiers: KeyModifiers::SHIFT,
        });

        assert_eq!(
            translate(click),
            Some(Event::Mouse(MouseInput {
                kind: MouseKind::Down(MouseButton::Right),
                column: 7,
                row: 3,
                modifiers: Modifiers {
                    shift: true,
                    ..Modifiers::NONE
                },
            }))
        );
    }

    #[test]
    fn a_scroll() {
        let scroll = TerminalEvent::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });

        assert_eq!(
            translate(scroll),
            Some(Event::Mouse(MouseInput {
                kind: MouseKind::ScrollUp,
                column: 0,
                row: 0,
                modifiers: Modifiers::NONE,
            }))
        );
    }

    #[test]
    fn paste_and_focus_pass_straight_through() {
        assert_eq!(
            translate(TerminalEvent::Paste("two words".to_owned())),
            Some(Event::Paste("two words".to_owned()))
        );
        assert_eq!(
            translate(TerminalEvent::FocusGained),
            Some(Event::FocusGained)
        );
        assert_eq!(translate(TerminalEvent::FocusLost), Some(Event::FocusLost));
    }
}
