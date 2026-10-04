//! The keymap's tests: the table's shape, the chord text both ways, the
//! warnings, and the translate sweep that keeps the edge honest
//! (ADR-0055 Consequences).

mod chords;
mod table;
mod translate;
mod warnings;

use super::{KeyChord, KeyCode};

/// Every chord no default uses, for the sweep: `alt` with each printable
/// ASCII character, `alt` with F1–F12, then `ctrl+alt` with each letter.
fn free_chords() -> Vec<KeyChord> {
    let mut out: Vec<KeyChord> = ('!'..='~')
        .map(|c| KeyChord::alt(KeyCode::Char(c)))
        .collect();
    out.extend((1..=12).map(|n| KeyChord::alt(KeyCode::Function(n))));
    out.extend(('a'..='z').map(|c| KeyChord {
        ctrl: true,
        ..KeyChord::alt(KeyCode::Char(c))
    }));
    out
}

/// `(id, [chords])` as his config would give them.
fn line(id: &str, chords: &[&str]) -> (String, Vec<String>) {
    (
        id.to_string(),
        chords.iter().map(|c| (*c).to_string()).collect(),
    )
}
