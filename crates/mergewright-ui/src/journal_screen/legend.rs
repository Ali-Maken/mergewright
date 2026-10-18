//! The legend line: what each glyph means (J1), and the `⇢` clause when it
//! fits (usability Q1).

use unicode_width::UnicodeWidthStr;

use crate::keymap::{ActionId, Keymap};

use super::cut_at_word;

/// The product word the legend names (ADR-0057 §10), from the names table.
const APP: &str = mergewright_core::on_disk::Generation::CURRENT.title();

/// What each glyph means, on screen for as long as the rows are (J1), with
/// `app.undo`'s key spelled from `keys` (`u`, or `u/⌃Z`'s first — the bar's
/// one key per action). Unbound, the legend says it in words.
#[must_use]
pub fn legend(keys: &Keymap) -> String {
    let (can, never) = match keys.spell_keys(&[ActionId("app.undo")]) {
        Some(undo) => (
            format!("{undo} can undo it"),
            format!("{undo} never touches it"),
        ),
        None => (
            "undo can take it back".to_owned(),
            "undo never touches it".to_owned(),
        ),
    };
    format!("● done in {APP}: {can} · $ done outside {APP}: {never} · ✗ a commit no branch holds")
}

/// The `⇢` row's clause, last in the legend (usability Q1).
const REPLACED_CLAUSE: &str = " · ⇢ undo brings it back";

/// The legend as drawn `width` cells wide: with the `⇢` clause last when the
/// whole of it fits, else today's [`legend`] cut at a word as before — so
/// the clause is the first thing to go and never shows in half.
pub(super) fn legend_at(keys: &Keymap, width: usize) -> String {
    let legend = legend(keys);
    let whole = format!("{legend}{REPLACED_CLAUSE}");
    if whole.width() <= width {
        whole
    } else {
        cut_at_word(&legend, width)
    }
}
