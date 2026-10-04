//! `Keymap::{spell_keys, spell_every}`: how every hint, menu and footer
//! spells a key (ticket m8/10, decisions 1–2).

use super::super::{ActionId, Keymap};
use super::line;

const DOWN: ActionId = ActionId("app.down");
const UP: ActionId = ActionId("app.up");
const MERGE: ActionId = ActionId("graph.merge");
const MAP: ActionId = ActionId("graph.map");
const TOGGLE_DIFF: ActionId = ActionId("finish.toggle_diff");

#[test]
fn spell_keys_joins_each_first_chord_with_a_slash() {
    let map = Keymap::default();
    assert_eq!(map.spell_keys(&[DOWN, UP]).as_deref(), Some("j/k"));
    assert_eq!(map.spell_keys(&[MERGE]).as_deref(), Some("m"));
}

#[test]
fn spell_keys_leaves_an_unbound_action_out() {
    let (map, warnings) = Keymap::with_overrides(&[line("graph.merge", &["none"])]);
    assert_eq!(warnings, []);
    assert_eq!(map.spell_keys(&[MERGE]), None);
    assert_eq!(map.spell_keys(&[DOWN, MERGE]).as_deref(), Some("j"));
    assert_eq!(map.spell_keys(&[]), None);
}

#[test]
fn spell_every_spells_every_chord_or_none() {
    let map = Keymap::default();
    assert_eq!(map.spell_every(&TOGGLE_DIFF).as_deref(), Some("⏎/space"));
    assert_eq!(map.spell_every(&DOWN).as_deref(), Some("j/↓"));
    let (map, _) = Keymap::with_overrides(&[line("finish.toggle_diff", &["none"])]);
    assert_eq!(map.spell_every(&TOGGLE_DIFF), None);
    assert_eq!(map.spell_every(&ActionId("no.such")), None);
}

/// `graph.merge = "M"` takes `M` from the Map view (an override beats a
/// default in one context): the Map's `M` no longer does anything, so it is
/// spelled nowhere — a hint naming it would be a lie.
#[test]
fn a_chord_lost_in_a_collision_is_not_spelled() {
    let (map, warnings) = Keymap::with_overrides(&[line("graph.merge", &["M"])]);
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert_eq!(map.spell_keys(&[MERGE]).as_deref(), Some("M"));
    assert_eq!(map.spell_keys(&[MAP]), None);
    assert_eq!(map.spell_every(&MAP), None);
    assert_eq!(map.spell(&MAP).0, "—");
}

#[test]
fn a_keymap_clones_and_compares_by_its_chords() {
    let (moved, _) = Keymap::with_overrides(&[line("graph.merge", &["M"])]);
    assert_eq!(moved.clone(), moved);
    assert_ne!(moved, Keymap::default());
    assert_eq!(*Keymap::shipped(), Keymap::default());
}
