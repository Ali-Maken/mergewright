//! The two editors' bars: the rewrite editor and the branch picker.

use super::super::keys::{
    BACK, DROP, FIELD_ESC, FIXUP, MOVE_ROW, NEXT_PANE, PICK, PICKER_ROWS, PICKER_SHOW,
    PICKER_SWITCH, REWORD, REWRITE_CHANGES, REWRITE_EDIT, REWRITE_START, SQUASH, TYPE,
};
use super::super::{Hint, hint};

/// The rewrite editor's bar (note R1, keymap §2.7): `y rewrite` leads while
/// it would start, then `tab after` below 120 (the one way to the AFTER
/// side there), then the row keys in R1's order.
pub(super) fn rewrite_editor_list(narrow: bool, can_start: bool) -> Vec<Hint> {
    let mut list = Vec::new();
    if can_start {
        list.push(hint(REWRITE_START, "rewrite"));
    }
    if narrow {
        list.push(hint(NEXT_PANE, "after"));
    }
    list.extend([
        hint(REWRITE_CHANGES, "changes"),
        hint(PICK, "pick"),
        hint(REWORD, "reword"),
        hint(REWRITE_EDIT, "edit"),
        hint(SQUASH, "squash"),
        hint(FIXUP, "fixup"),
        hint(DROP, "drop"),
        hint(MOVE_ROW, "move"),
        hint(BACK, "cancel"),
    ]);
    list
}

/// The branch picker's bar.
/// Keymap §2.16: the branch picker is a text field, so every letter
/// types; the bar names what the non-letter keys do
/// (`mergewright::update::picker::shown_key`). `esc close` is third, not
/// last as the ticket spelled it: the first three are the floor drawn
/// at every width (ADR-0037 §4), none of these verbs has a short rung
/// in the user's table, and with `⇥` third the floor is 52 cells — a
/// 40-column bar would draw nothing. With `esc` third it is 39, and
/// the way out stays in words (ADR-0044 §4's reason).
pub(super) fn picker_list() -> Vec<Hint> {
    picker_list_with(PICKER_SWITCH_VERB)
}

/// [`picker_list`] with the cursor on HEAD's branch: nothing to switch
/// to, so `⏎` does what `⇥` does — closes the picker, the graph cursor on
/// HEAD (m8 usability Q2, keymap §2.16) — and says it in `⇥`'s words, not
/// `esc`'s `close` (review m8 g1, minor 1). Every other clause and the
/// order are unchanged.
pub(super) fn picker_on_current_list() -> Vec<Hint> {
    picker_list_with(PICKER_SHOW_VERB)
}

const PICKER_SWITCH_VERB: &str = "switch to it";
const PICKER_SHOW_VERB: &str = "show it on the graph";

fn picker_list_with(enter: &'static str) -> Vec<Hint> {
    vec![
        hint(TYPE, "to find"),
        hint(PICKER_SWITCH, enter),
        hint(FIELD_ESC, "close"),
        hint(PICKER_SHOW, PICKER_SHOW_VERB),
        hint(PICKER_ROWS, "row"),
    ]
}
