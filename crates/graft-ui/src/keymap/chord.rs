//! One key with its modifiers, as the keymap names it: read from his config
//! (`parse`), spelled for the screen (`label`), and converted to and from the
//! press the input decoder delivers.
//!
//! **Normal form.** A `Char` carries its own case and `shift` is `false`
//! (`M`, never `shift+m`); `Shift+Tab` is `BackTab`. So one key has one
//! `KeyChord`, and two chords compare equal exactly when they are the same
//! key.

use graft_core::event::{KeyCode, KeyPress, Modifiers};

/// A key and the modifiers held with it, in normal form (module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct KeyChord {
    /// Which key.
    pub code: KeyCode,
    /// Control held.
    pub ctrl: bool,
    /// Alt / Option held.
    pub alt: bool,
    /// Shift held — only ever on a non-character key other than Tab.
    pub shift: bool,
}

/// Text from his config that names no key.
#[derive(Debug, PartialEq, Eq)]
pub struct ChordError {
    /// The text as he wrote it.
    pub text: String,
}

/// A key spelled the way every surface spells it (`⌃P`, `m`, `⏎`), or `—`
/// for an action with no key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyLabel(pub String);

impl KeyChord {
    /// A plain character key: `KeyChord::key('m')`.
    #[must_use]
    pub const fn key(c: char) -> Self {
        Self::plain(KeyCode::Char(c))
    }

    /// A key with no modifiers.
    #[must_use]
    pub const fn plain(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: false,
            shift: false,
        }
    }

    /// Control with a character: `KeyChord::ctrl('a')` is `⌃A`.
    #[must_use]
    pub const fn ctrl(c: char) -> Self {
        Self {
            code: KeyCode::Char(c),
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    /// Control with a non-character key: `⌃⏎`.
    #[must_use]
    pub const fn ctrl_code(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: true,
            alt: false,
            shift: false,
        }
    }

    /// Alt with a key: `alt+↑`.
    #[must_use]
    pub const fn alt(code: KeyCode) -> Self {
        Self {
            code,
            ctrl: false,
            alt: true,
            shift: false,
        }
    }

    /// `Ctrl+C`: always quits, never remapped, never given away.
    pub const QUIT_NOW: Self = Self::ctrl('c');

    /// Read one chord from his config, or from any [`KeyChord::label`].
    ///
    /// Names are case-insensitive (`Enter`, `PGUP`); a single character is
    /// case-sensitive (`m` and `M` are two keys). `none` is not a chord: the
    /// override reader handles it before calling this.
    ///
    /// # Errors
    /// [`ChordError`] carrying the text when it names no key.
    pub fn parse(text: &str) -> Result<Self, ChordError> {
        parse_chord(text).ok_or_else(|| ChordError {
            text: text.to_string(),
        })
    }

    /// The chord as the hint bar, the `.` menu and `?` spell it today.
    #[must_use]
    pub fn label(&self) -> KeyLabel {
        let mut out = String::new();
        if self.alt {
            out.push_str("alt+");
        }
        if self.ctrl {
            out.push('⌃');
        }
        match self.code {
            KeyCode::Char(c) if self.ctrl && c.is_ascii_lowercase() => {
                out.push(c.to_ascii_uppercase());
            }
            KeyCode::Char(c) if self.ctrl && c.is_ascii_uppercase() => {
                out.push('⇧');
                out.push(c);
            }
            code => {
                if self.shift {
                    out.push('⇧');
                }
                out.push_str(&code_label(code));
            }
        }
        KeyLabel(out)
    }

    /// The chord a press is, or `None` when Command / Super is held: no
    /// binding uses it, and the terminal or the OS owns those keys.
    #[must_use]
    pub fn from_press(press: &KeyPress) -> Option<Self> {
        let m = press.modifiers;
        if m.command {
            return None;
        }
        let (code, shift) = match press.code {
            KeyCode::Char(c) if m.shift && c.is_ascii_lowercase() => {
                (KeyCode::Char(c.to_ascii_uppercase()), false)
            }
            KeyCode::Char(c) => (KeyCode::Char(c), false),
            KeyCode::Tab if m.shift => (KeyCode::BackTab, false),
            KeyCode::BackTab => (KeyCode::BackTab, false),
            code => (code, m.shift),
        };
        Some(Self {
            code,
            ctrl: m.control,
            alt: m.alt,
            shift,
        })
    }

    /// The press a dispatcher matches for this chord: the shape the input
    /// decoder delivers it in, so a translated default acts exactly as
    /// pressing that default does.
    #[must_use]
    pub fn to_press(&self) -> KeyPress {
        let shift = self.shift || self.code == KeyCode::BackTab;
        KeyPress {
            code: self.code,
            modifiers: Modifiers {
                control: self.ctrl,
                alt: self.alt,
                shift: shift || ctrl_upper(self),
                command: false,
            },
        }
    }
}

/// Control with an uppercase letter is `Ctrl+Shift+<letter>`; the decoder
/// reports shift alongside it.
fn ctrl_upper(chord: &KeyChord) -> bool {
    chord.ctrl && matches!(chord.code, KeyCode::Char(c) if c.is_ascii_uppercase())
}

/// The key alone, without modifiers, as the code spells it.
fn code_label(code: KeyCode) -> String {
    let word = match code {
        KeyCode::Char(' ') => "space",
        KeyCode::Char(c) => return c.to_string(),
        KeyCode::Enter => "⏎",
        KeyCode::Escape => "esc",
        KeyCode::Tab => "⇥",
        KeyCode::BackTab => "⇧⇥",
        KeyCode::Backspace => "backspace",
        KeyCode::Delete => "delete",
        KeyCode::Insert => "insert",
        KeyCode::Left => "←",
        KeyCode::Right => "→",
        KeyCode::Up => "↑",
        KeyCode::Down => "↓",
        KeyCode::Home => "home",
        KeyCode::End => "end",
        KeyCode::PageUp => "PgUp",
        KeyCode::PageDown => "PgDn",
        KeyCode::Function(n) => return format!("F{n}"),
    };
    word.to_string()
}

/// A named key (case-insensitive), or a glyph [`code_label`] draws.
fn named(word: &str) -> Option<KeyCode> {
    let code = match word.to_ascii_lowercase().as_str() {
        "enter" | "return" | "⏎" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Escape,
        "tab" | "⇥" => KeyCode::Tab,
        "⇧⇥" | "backtab" => KeyCode::BackTab,
        "space" => KeyCode::Char(' '),
        "up" | "↑" => KeyCode::Up,
        "down" | "↓" => KeyCode::Down,
        "left" | "←" => KeyCode::Left,
        "right" | "→" => KeyCode::Right,
        "pgup" | "pageup" => KeyCode::PageUp,
        "pgdn" | "pagedown" => KeyCode::PageDown,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" => KeyCode::Insert,
        f if f.len() > 1 && f.starts_with('f') => {
            let n: u8 = f[1..].parse().ok()?;
            if !(1..=12).contains(&n) {
                return None;
            }
            KeyCode::Function(n)
        }
        _ => return None,
    };
    Some(code)
}

/// Modifiers met so far while reading a chord.
#[derive(Default)]
struct Held {
    ctrl: bool,
    alt: bool,
    shift: bool,
    /// `⌃` written as a glyph: the label's `⌃A` means control with `a`.
    glyph_ctrl: bool,
}

fn parse_chord(text: &str) -> Option<KeyChord> {
    let text = text.trim();
    let mut held = Held::default();
    let mut rest = text;
    // Word modifiers: `ctrl+`, `alt+`, `shift+`, in any order. A lone `+`
    // (or a trailing `++`) is the plus key itself.
    while let Some((head, tail)) = rest.split_once('+') {
        if tail.is_empty() && head.is_empty() {
            break; // `+` itself
        }
        match head.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => held.ctrl = true,
            "alt" | "option" => held.alt = true,
            "shift" => held.shift = true,
            _ => break,
        }
        rest = tail;
    }
    // Glyph modifiers, as `label` writes them: `⌃`, then `⇧`.
    if let Some(tail) = rest.strip_prefix('⌃') {
        held.ctrl = true;
        held.glyph_ctrl = true;
        rest = tail;
    }
    if rest != "⇧⇥" {
        if let Some(tail) = rest.strip_prefix('⇧') {
            held.shift = true;
            rest = tail;
        }
    }
    if rest.is_empty() {
        return None;
    }
    let code = key_of(rest, &held)?;
    normalise(code, &held)
}

/// The key named by what is left once the modifiers are read.
fn key_of(rest: &str, held: &Held) -> Option<KeyCode> {
    if let Some(glyph_or_name) = named(rest) {
        return Some(glyph_or_name); // `⏎`, `↑`, `enter`, …
    }
    let mut chars = rest.chars();
    let (Some(c), None) = (chars.next(), chars.next()) else {
        return None;
    };
    // The label spells control with a letter in capitals (`⌃A`), the way
    // terminals and menus do; `⌃⇧A` is the shifted one.
    if held.glyph_ctrl && c.is_ascii_uppercase() && !held.shift {
        return Some(KeyCode::Char(c.to_ascii_lowercase()));
    }
    Some(KeyCode::Char(c))
}

/// Fold shift into the key where the normal form says so.
fn normalise(code: KeyCode, held: &Held) -> Option<KeyChord> {
    let (code, shift) = match code {
        KeyCode::Char(c) if held.shift => {
            // Shift applies to a letter; on anything else the character
            // already says which key it is, and the layout decides it.
            if !c.is_ascii_alphabetic() {
                return None;
            }
            (KeyCode::Char(c.to_ascii_uppercase()), false)
        }
        KeyCode::Tab if held.shift => (KeyCode::BackTab, false),
        KeyCode::BackTab => (KeyCode::BackTab, false),
        KeyCode::Char(c) => (KeyCode::Char(c), false),
        code => (code, held.shift),
    };
    Some(KeyChord {
        code,
        ctrl: held.ctrl,
        alt: held.alt,
        shift,
    })
}
