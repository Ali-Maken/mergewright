//! `KeyChord`: his text in, the app's spelling out, and the press both ways.

use graft_core::event::{KeyPress, Modifiers};

use super::super::{KeyChord, KeyCode, defaults};

fn parse(text: &str) -> KeyChord {
    KeyChord::parse(text).unwrap_or_else(|e| panic!("{text:?} should parse: {e:?}"))
}

/// The chord as a person writes it in `config.toml`: words, lowercase.
fn canonical(c: &KeyChord) -> String {
    let mut out = String::new();
    if c.ctrl {
        out.push_str("ctrl+");
    }
    if c.alt {
        out.push_str("alt+");
    }
    if c.shift {
        out.push_str("shift+");
    }
    let key = match c.code {
        KeyCode::Char(' ') => "space".to_string(),
        KeyCode::Char(ch) if c.ctrl && ch.is_ascii_uppercase() => {
            format!("shift+{}", ch.to_ascii_lowercase())
        }
        KeyCode::Char(ch) => ch.to_string(),
        KeyCode::Enter => "enter".into(),
        KeyCode::Escape => "esc".into(),
        KeyCode::Tab => "tab".into(),
        KeyCode::BackTab => "shift+tab".into(),
        KeyCode::Backspace => "backspace".into(),
        KeyCode::Delete => "delete".into(),
        KeyCode::Insert => "insert".into(),
        KeyCode::Left => "left".into(),
        KeyCode::Right => "right".into(),
        KeyCode::Up => "up".into(),
        KeyCode::Down => "down".into(),
        KeyCode::Home => "home".into(),
        KeyCode::End => "end".into(),
        KeyCode::PageUp => "pgup".into(),
        KeyCode::PageDown => "pgdn".into(),
        KeyCode::Function(n) => format!("f{n}"),
    };
    out.push_str(&key);
    out
}

#[test]
fn every_default_round_trips_through_its_label_and_its_words() {
    for b in defaults() {
        for c in b.default {
            let label = c.label().0;
            assert_eq!(KeyChord::parse(&label), Ok(*c), "{}: label {label:?}", b.action.0);
            let words = canonical(c);
            assert_eq!(KeyChord::parse(&words), Ok(*c), "{}: text {words:?}", b.action.0);
        }
    }
}

#[test]
fn every_default_round_trips_through_the_press_the_decoder_delivers() {
    for b in defaults() {
        for c in b.default {
            assert_eq!(KeyChord::from_press(&c.to_press()), Some(*c), "{}", b.action.0);
        }
    }
}

#[test]
fn the_ticket_list_of_words_parses() {
    let cases: &[(&str, KeyChord)] = &[
        ("m", KeyChord::key('m')),
        ("M", KeyChord::key('M')),
        ("shift+m", KeyChord::key('M')),
        ("Shift+M", KeyChord::key('M')),
        ("ctrl+p", KeyChord::ctrl('p')),
        ("alt+up", KeyChord::alt(KeyCode::Up)),
        ("shift+tab", KeyChord::plain(KeyCode::BackTab)),
        ("enter", KeyChord::plain(KeyCode::Enter)),
        ("Enter", KeyChord::plain(KeyCode::Enter)),
        ("esc", KeyChord::plain(KeyCode::Escape)),
        ("tab", KeyChord::plain(KeyCode::Tab)),
        ("space", KeyChord::key(' ')),
        ("up", KeyChord::plain(KeyCode::Up)),
        ("down", KeyChord::plain(KeyCode::Down)),
        ("left", KeyChord::plain(KeyCode::Left)),
        ("right", KeyChord::plain(KeyCode::Right)),
        ("pgup", KeyChord::plain(KeyCode::PageUp)),
        ("pageup", KeyChord::plain(KeyCode::PageUp)),
        ("pgdn", KeyChord::plain(KeyCode::PageDown)),
        ("PageDown", KeyChord::plain(KeyCode::PageDown)),
        ("home", KeyChord::plain(KeyCode::Home)),
        ("end", KeyChord::plain(KeyCode::End)),
        ("backspace", KeyChord::plain(KeyCode::Backspace)),
        ("delete", KeyChord::plain(KeyCode::Delete)),
        ("insert", KeyChord::plain(KeyCode::Insert)),
        ("f1", KeyChord::plain(KeyCode::Function(1))),
        ("F12", KeyChord::plain(KeyCode::Function(12))),
        ("+", KeyChord::key('+')),
        ("ctrl++", KeyChord::ctrl('+')),
        ("ctrl+shift+c", KeyChord::ctrl('C')),
        // Every label, as `?` shows it: he may copy a key from there.
        ("⌃P", KeyChord::ctrl('p')),
        ("⌃⇧C", KeyChord::ctrl('C')),
        ("⏎", KeyChord::plain(KeyCode::Enter)),
        ("⇥", KeyChord::plain(KeyCode::Tab)),
        ("⇧⇥", KeyChord::plain(KeyCode::BackTab)),
        ("⌃⏎", KeyChord::ctrl_code(KeyCode::Enter)),
        ("alt+↑", KeyChord::alt(KeyCode::Up)),
        ("PgDn", KeyChord::plain(KeyCode::PageDown)),
    ];
    for (text, want) in cases {
        assert_eq!(parse(text), *want, "{text:?}");
    }
}

#[test]
fn text_that_names_no_key_is_an_error_carrying_the_text() {
    for text in [
        "", "ctrl+", "none", "f13", "f0", "shift+1", "hyper+a", "ab", "ctrl+enterx",
    ] {
        assert_eq!(
            KeyChord::parse(text),
            Err(super::super::ChordError {
                text: text.to_string()
            }),
            "{text:?}"
        );
    }
}

#[test]
fn labels_match_how_the_code_spells_keys_today() {
    let cases: &[(KeyChord, &str)] = &[
        (KeyChord::ctrl('a'), "⌃A"),
        (KeyChord::ctrl('r'), "⌃R"),
        (KeyChord::ctrl_code(KeyCode::Enter), "⌃⏎"),
        (KeyChord::ctrl('C'), "⌃⇧C"),
        (KeyChord::plain(KeyCode::Enter), "⏎"),
        (KeyChord::plain(KeyCode::Escape), "esc"),
        (KeyChord::key(' '), "space"),
        (KeyChord::plain(KeyCode::Tab), "⇥"),
        (KeyChord::plain(KeyCode::BackTab), "⇧⇥"),
        (KeyChord::plain(KeyCode::Up), "↑"),
        (KeyChord::plain(KeyCode::PageUp), "PgUp"),
        (KeyChord::plain(KeyCode::PageDown), "PgDn"),
        (KeyChord::plain(KeyCode::Function(2)), "F2"),
        (KeyChord::alt(KeyCode::Down), "alt+↓"),
        (KeyChord::key('M'), "M"),
    ];
    for (chord, want) in cases {
        assert_eq!(chord.label().0, *want, "{chord:?}");
    }
}

#[test]
fn a_press_with_command_held_is_never_a_chord() {
    let press = KeyPress {
        code: KeyCode::Char('m'),
        modifiers: Modifiers {
            command: true,
            ..Modifiers::NONE
        },
    };
    assert_eq!(KeyChord::from_press(&press), None);
}

#[test]
fn a_shifted_press_lands_in_normal_form() {
    let shift = Modifiers {
        shift: true,
        ..Modifiers::NONE
    };
    let kitty_ctrl_shift = Modifiers {
        shift: true,
        ..Modifiers::CONTROL
    };
    let cases = [
        (KeyCode::Char('m'), shift, KeyChord::key('M')),
        (KeyCode::Char('M'), shift, KeyChord::key('M')),
        (KeyCode::Char('?'), shift, KeyChord::key('?')),
        (KeyCode::Tab, shift, KeyChord::plain(KeyCode::BackTab)),
        (KeyCode::BackTab, shift, KeyChord::plain(KeyCode::BackTab)),
        // Kitty's Ctrl+Shift+C: `Char('C')`, control and shift.
        (KeyCode::Char('C'), kitty_ctrl_shift, KeyChord::ctrl('C')),
        (
            KeyCode::Up,
            shift,
            KeyChord {
                shift: true,
                ..KeyChord::plain(KeyCode::Up)
            },
        ),
    ];
    for (code, modifiers, want) in cases {
        let press = KeyPress { code, modifiers };
        assert_eq!(KeyChord::from_press(&press), Some(want), "{press:?}");
    }
}
