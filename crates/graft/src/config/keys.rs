//! `[keys]`: the one switch at its top level and the remap sub-tables under it
//! (`[keys.<context>] <verb> = "<chord>" | ["<chord>", …]`, ADR-0055).
//!
//! serde reads `suspend_on_ctrl_z` and ignores the sub-tables; this reads the
//! sub-tables off the raw table, so the typed parse and the remaps come from
//! the same text. What the remaps *mean* — whether an action exists, whether a
//! chord parses, what collides — is `graft_ui::keymap`'s to say, not this
//! file's: here a line is only refused when its value is not text.

use graft_ui::keymap::{KeySwitches, Keymap};

use super::load::{ConfigWarning, Loaded};

/// The one key straight under `[keys]` that is not a sub-table.
const SUSPEND: &str = "suspend_on_ctrl_z";

/// What `[keys]` held besides `suspend_on_ctrl_z`: the remap lines, as
/// `("<ctx>.<verb>", chords)`, and a warning per entry it could not read.
///
/// **Order:** the `toml` crate is built without `preserve_order`, so its
/// table is sorted — the lines come out sorted by context, then verb, not in
/// file order. The keymap ranks a collision by line, so of two lines claiming
/// one key in one context the one whose id sorts later wins.
pub(super) fn remaps(
    keys: Option<&toml::Value>,
) -> (Vec<(String, Vec<String>)>, Vec<ConfigWarning>) {
    let mut lines = Vec::new();
    let mut warnings = Vec::new();
    let Some(table) = keys.and_then(toml::Value::as_table) else {
        return (lines, warnings);
    };
    for (name, value) in table {
        if name == SUSPEND {
            continue;
        }
        let Some(context) = value.as_table() else {
            warnings.push(ConfigWarning::UnknownKey {
                path: format!("keys.{name}"),
            });
            continue;
        };
        for (verb, chords) in context {
            match chord_texts(chords) {
                Some(texts) => lines.push((format!("{name}.{verb}"), texts)),
                None => warnings.push(ConfigWarning::NotAKey {
                    path: format!("keys.{name}.{verb}"),
                }),
            }
        }
    }
    (lines, warnings)
}

/// `"M"` → `["M"]`; `["M", "ctrl+m"]` → both; anything else (a number, a
/// table, a list holding one) → `None`.
fn chord_texts(value: &toml::Value) -> Option<Vec<String>> {
    match value {
        toml::Value::String(text) => Some(vec![text.clone()]),
        toml::Value::Array(items) => items
            .iter()
            .map(|item| item.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    }
}

/// The process's one keymap and every config problem in words: the file's
/// own warnings first, in their usual words, then each the keymap raised.
#[must_use]
pub(crate) fn keymap_from(loaded: &Loaded) -> (Keymap, Vec<String>) {
    // The switch decides the chords before his lines: with it on, `⌃Z` is
    // `app.suspend` and undo keeps `u` (ticket m8/09c, D2).
    let switches = KeySwitches {
        suspend_on_ctrl_z: loaded.config.keys.suspend_on_ctrl_z,
    };
    let (keymap, warned) = Keymap::with_overrides(&loaded.config.keys.remaps, switches);
    let problems = loaded
        .warnings
        .iter()
        .map(ToString::to_string)
        .chain(warned.iter().map(graft_ui::keymap::KeyWarning::words))
        .collect();
    (keymap, problems)
}
