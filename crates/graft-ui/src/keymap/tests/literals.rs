//! Ticket m8/09d, D5: the five literals 10c allow-listed now spell from a
//! row. Each row is the planner's reading of the design; this pins it to
//! what the shipped keymap resolves the default chord to where the words
//! are drawn.

use super::super::resolve::Over;
use super::super::{ContextId, KeyChord, KeyCode, Keymap, Translated, defaults};

/// `(where the words show, the chord they name, the row it must be)`.
const FORMER_LITERALS: &[(ContextId, KeyChord, &str)] = &[
    // `this is the compare base · x clears it` (detail header): `x` acts
    // from the graph and from the pane (Diff falls through to Graph).
    (ContextId::Graph, KeyChord::key('x'), "graph.compare"),
    (ContextId::Diff, KeyChord::key('x'), "graph.compare"),
    // `… N more lines · L load all` (diff footer).
    (ContextId::Diff, KeyChord::key('L'), "diff.load_all"),
    // The `edit` stop's banner words: the graph has the keyboard.
    (ContextId::Graph, KeyChord::key(' '), "graph.fold"),
    (ContextId::Graph, KeyChord::key('a'), "graph.amend"),
    (ContextId::Graph, KeyChord::key('c'), "graph.commit"),
    (
        ContextId::Graph,
        KeyChord::plain(KeyCode::Tab),
        "app.next_pane",
    ),
    // `CHANGES of <sha> · ⏎ back to AFTER` (rewrite editor, rows).
    (
        ContextId::Rewrite,
        KeyChord::plain(KeyCode::Enter),
        "rewrite.toggle_changes",
    ),
    // `x base` (the compare-base badge on the graph).
    (ContextId::Graph, KeyChord::key('x'), "graph.compare"),
];

#[test]
fn each_former_literal_names_the_row_that_answers_it() {
    let shipped = Keymap::shipped();
    for (ctx, chord, want) in FORMER_LITERALS {
        let row = shipped
            .resolve(*ctx, *chord, Over::Effective)
            .unwrap_or_else(|| panic!("{ctx:?} {chord:?}: no row"));
        assert_eq!(defaults()[row].action.0, *want, "{ctx:?} {chord:?}");
        // The default does what it says: handed to the panes as pressed.
        assert_eq!(shipped.translate(*ctx, *chord), Translated::Pass);
    }
}
