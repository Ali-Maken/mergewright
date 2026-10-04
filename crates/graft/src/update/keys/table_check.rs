//! `bound_actions` against `graft_ui::keymap::defaults()` (m8/09): every key
//! this build binds has a row in the keymap table, so it can be remapped and
//! a collision with it is warned rather than silent.
//!
//! Keys only, no contexts: `bound_actions` records none. Ticket 09b checks
//! contexts by pressing.

use std::collections::HashSet;

use graft_ui::keymap::defaults;

use super::bound_actions;

/// Spellings in `bound_actions` that are not a key, or not spelled the way
/// `KeyChord::label` spells their key. Each one is a doc question in the
/// ticket's NOTES, not a free pass.
const NOT_A_ROW: &[(&str, &str)] = &[
    (
        "release",
        "the mouse button ending a drag, named as a key for the bar",
    ),
    (
        "type",
        "typing in the picker's field: every printable key, no chord",
    ),
    (
        "tab",
        "the rewrite editor's bar spells Tab `tab`; everywhere else, and its label, `⇥` (ticket 10)",
    ),
];

/// One `bound_actions` key spelling, split into the keys it names: `j/k`,
/// `n N`, `⏎/space`, a bare `/`, and the glyph pairs `<>`, `↑↓`, `alt+↑↓`.
fn keys_in(spelling: &str) -> Vec<String> {
    if spelling == "/" {
        return vec!["/".to_string()]; // the find key itself, not a separator
    }
    let mut out = Vec::new();
    for part in spelling.split(['/', ' ']).filter(|p| !p.is_empty()) {
        match part {
            "<>" | "↑↓" => out.extend(part.chars().map(String::from)),
            "alt+↑↓" => out.extend(["alt+↑".to_string(), "alt+↓".to_string()]),
            _ => out.push(part.to_string()),
        }
    }
    out
}

#[test]
fn every_bound_key_has_a_row() {
    let labels: HashSet<String> = defaults()
        .iter()
        .flat_map(|b| b.default.iter().map(|c| c.label().0))
        .collect();
    let mut missing = Vec::new();
    for (spelling, verb) in bound_actions() {
        if NOT_A_ROW.iter().any(|(s, _)| s == spelling) {
            continue;
        }
        for key in keys_in(spelling) {
            if !labels.contains(&key) {
                missing.push(format!("{key} ({verb})"));
            }
        }
    }
    assert!(
        missing.is_empty(),
        "bound keys with no row in graft_ui::keymap::defaults(): {missing:?}"
    );
}

#[test]
fn every_exemption_is_still_spelled_in_bound_actions() {
    for (spelling, why) in NOT_A_ROW {
        assert!(
            bound_actions().iter().any(|(s, _)| s == spelling),
            "{spelling} ({why}) is no longer in bound_actions: drop the exemption"
        );
    }
}

#[test]
fn the_splitter_reads_every_compound_spelling() {
    assert_eq!(keys_in("j/k"), ["j", "k"]);
    assert_eq!(keys_in("n N"), ["n", "N"]);
    assert_eq!(keys_in("⏎/space"), ["⏎", "space"]);
    assert_eq!(keys_in("<>"), ["<", ">"]);
    assert_eq!(keys_in("alt+↑↓"), ["alt+↑", "alt+↓"]);
    assert_eq!(keys_in("⌃O/⌃T"), ["⌃O", "⌃T"]);
    assert_eq!(keys_in("/"), ["/"]);
}
