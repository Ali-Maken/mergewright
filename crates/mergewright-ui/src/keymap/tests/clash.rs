//! Clashes under the three modal overlays (m8/09f Major 2): a line in the
//! Map's section never takes a key from a row that does nothing there.

use super::super::{ActionId, ContextId, KeyChord, KeySwitches, KeyWarning, Keymap};
use super::line;

fn no_collision(warnings: &[KeyWarning]) -> bool {
    !warnings
        .iter()
        .any(|w| matches!(w, KeyWarning::Collision { .. }))
}

/// `[keys.map] seek = "c"`: no problem — nothing at the Map reaches
/// `graph.commit` — and the graph bar still spells `c commit` (D7 drops
/// nothing).
#[test]
fn a_map_line_on_a_graph_chord_loses_nothing_on_the_graph() {
    let (map, warnings) =
        Keymap::with_overrides(&[line("map.seek", &["c"])], KeySwitches::default());
    assert!(no_collision(&warnings), "{warnings:?}");
    let c = KeyChord::key('c');
    assert_eq!(map.acting(ContextId::Map, c), Some(ActionId("map.seek")));
    assert_eq!(
        map.acting(ContextId::Graph, c),
        Some(ActionId("graph.commit"))
    );
    assert_eq!(
        map.key_verb(&[ActionId("graph.commit")], "commit")
            .as_deref(),
        Some("c commit")
    );
}

/// `[keys.map] seek = "q"`: the Map answers `q` with nothing (it stands
/// `app.quit` down there), so taking it loses nothing, and `q quit` is
/// still spelled everywhere quit acts.
#[test]
fn a_map_line_on_a_key_the_map_stands_down_loses_nothing() {
    let (map, warnings) =
        Keymap::with_overrides(&[line("map.seek", &["q"])], KeySwitches::default());
    assert!(no_collision(&warnings), "{warnings:?}");
    assert_eq!(map.spell(&ActionId("app.quit")).0, "q");
}
