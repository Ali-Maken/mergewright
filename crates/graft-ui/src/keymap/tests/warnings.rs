//! Each problem his `[keys]` table can have: produced, ordered, worded.

use super::super::{ActionId, ContextId, KeyChord, KeyWarning, Keymap};
use super::line;

fn warnings(lines: &[(String, Vec<String>)]) -> Vec<KeyWarning> {
    Keymap::with_overrides(lines).1
}

fn words(lines: &[(String, Vec<String>)]) -> Vec<String> {
    warnings(lines).iter().map(KeyWarning::words).collect()
}

#[test]
fn an_unknown_action_is_named() {
    let lines = [line("grpah.merge", &["M"])];
    assert_eq!(warnings(&lines), [KeyWarning::UnknownAction("grpah.merge".into())]);
    assert_eq!(words(&lines), ["grpah.merge is not an action"]);
}

#[test]
fn a_bad_key_leaves_the_action_on_its_defaults() {
    let lines = [line("graph.merge", &["ctrl+"])];
    let (map, got) = Keymap::with_overrides(&lines);
    assert_eq!(
        got,
        [KeyWarning::BadKey {
            action: "graph.merge".into(),
            text: "ctrl+".into()
        }]
    );
    assert_eq!(got[0].words(), "\"ctrl+\" is not a key (graph.merge)");
    assert_eq!(map.chords(&ActionId("graph.merge")), [KeyChord::key('m')]);
}

#[test]
fn none_inside_a_list_is_a_bad_key() {
    let lines = [line("graph.merge", &["M", "none"])];
    assert_eq!(
        warnings(&lines),
        [KeyWarning::BadKey {
            action: "graph.merge".into(),
            text: "none".into()
        }]
    );
}

#[test]
fn ctrl_c_is_reserved_both_ways() {
    let given = [line("graph.merge", &["ctrl+c"])];
    assert_eq!(
        warnings(&given),
        [KeyWarning::Reserved {
            action: "graph.merge".into(),
            key: KeyChord::QUIT_NOW
        }]
    );
    assert_eq!(words(&given), ["⌃C always quits — not given to graph.merge"]);

    let moved = [line("app.quit_now", &["x"])];
    assert_eq!(
        warnings(&moved),
        [KeyWarning::Reserved {
            action: "app.quit_now".into(),
            key: KeyChord::key('x')
        }]
    );
    assert_eq!(words(&moved), ["⌃C always quits — app.quit_now can't move to x"]);
}

#[test]
fn a_text_field_row_is_fixed() {
    let lines = [line("picker.switch", &["s"])];
    let (map, got) = Keymap::with_overrides(&lines);
    assert_eq!(got, [KeyWarning::Fixed(ActionId("picker.switch"))]);
    assert_eq!(
        got[0].words(),
        "picker.switch can't be remapped — keys in a text field stay as they are"
    );
    assert_eq!(map.spell(&ActionId("picker.switch")).0, "⏎");
}

#[test]
fn a_collision_his_override_wins_names_what_lost_it() {
    assert_eq!(
        words(&[line("graph.merge", &["M"])]),
        ["M given to graph.merge — map view has no key"]
    );
    assert_eq!(
        words(&[line("graph.merge", &["u"])]),
        ["u given to graph.merge — undo keeps ⌃Z on the graph"]
    );
}

#[test]
fn a_collision_his_override_loses_says_what_stays() {
    let got = warnings(&[line("app.undo", &["o"])]);
    let resolver = KeyWarning::Collision {
        key: KeyChord::key('o'),
        context: ContextId::Resolver,
        kept: ActionId("resolver.take_ours"),
        lost: ActionId("app.undo"),
    };
    assert!(got.contains(&resolver), "{got:?}");
    assert_eq!(
        resolver.words(),
        "o given to app.undo — in the resolver o stays take ours"
    );
    // The graph's cherry-pick and the start screen's open keep `o` too.
    assert_eq!(got.len(), 3, "{got:?}");
}

#[test]
fn two_overrides_on_one_chord_the_earlier_line_keeps_it() {
    let lines = [line("graph.merge", &["z"]), line("graph.rebase", &["z"])];
    let (map, got) = Keymap::with_overrides(&lines);
    assert_eq!(
        got,
        [KeyWarning::Collision {
            key: KeyChord::key('z'),
            context: ContextId::Graph,
            kept: ActionId("graph.merge"),
            lost: ActionId("graph.rebase"),
        }]
    );
    assert_eq!(
        map.translate(ContextId::Graph, KeyChord::key('z')),
        super::super::Translated::Default(KeyChord::key('m'))
    );
}

#[test]
fn shadowing_that_only_scope_brings_together_is_not_warned() {
    // `w` is the diff's whitespace toggle; the graph keeps it for merge,
    // the diff keeps its own. Neither context holds both.
    assert_eq!(warnings(&[line("graph.merge", &["w"])]), []);
}

#[test]
fn warnings_come_in_his_file_order() {
    let lines = [
        line("graph.merge", &["M"]),
        line("nope.nope", &["x"]),
        line("graph.push", &["ctrl+c"]),
    ];
    let got = warnings(&lines);
    assert!(matches!(got[0], KeyWarning::Collision { .. }), "{got:?}");
    assert!(matches!(got[1], KeyWarning::UnknownAction(_)), "{got:?}");
    assert!(matches!(got[2], KeyWarning::Reserved { .. }), "{got:?}");
}
