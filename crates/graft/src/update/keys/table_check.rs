//! `bound_actions` against `graft_ui::keymap::defaults()` (m8/09, m8/10):
//! every action this build binds has a row in the keymap table, so it can be
//! remapped and a collision with it is warned rather than silent.
//!
//! Since m8/10 the list holds actions, not key strings, so there is nothing
//! to split and no word to exempt: the words a bar draws that are not keys
//! (`type`, `release`) are `graft_ui::graph_screen::NOT_A_BINDING`, outside
//! this list. Contexts are checked by pressing (09b's
//! `every_row_is_answered_in_its_context`).

use std::collections::HashSet;

use graft_ui::keymap::defaults;

use super::bound_actions;

#[test]
fn every_bound_action_is_a_row() {
    let rows: HashSet<&str> = defaults().iter().map(|b| b.action.0).collect();
    let missing: Vec<&str> = bound_actions()
        .iter()
        .map(|action| action.0)
        .filter(|name| !rows.contains(name))
        .collect();
    assert!(
        missing.is_empty(),
        "bound actions with no row in graft_ui::keymap::defaults(): {missing:?}"
    );
}

#[test]
fn no_action_is_listed_twice() {
    let mut seen = HashSet::new();
    let twice: Vec<&str> = bound_actions()
        .iter()
        .map(|action| action.0)
        .filter(|name| !seen.insert(*name))
        .collect();
    assert!(twice.is_empty(), "listed twice in bound_actions: {twice:?}");
}
