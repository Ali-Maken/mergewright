//! Every text field open, for the R4 checks (split out by m8/09d, Minor 8).

use super::*;

/// Every text field `text_field_open` names, each open and empty: the
/// search prompt, blame's find, J's find, the stash prompt, the commit
/// editor, the credential box, the name prompt, the picker and the rewrite
/// editor's `r` field. (The merge review's subject left with ease-pass/12.)
pub(in super::super) fn every_field() -> Vec<(&'static str, AppState)> {
    let typed = |state: AppState, c: char| update(&state, key(KeyCode::Char(c))).0;
    let mut rewrite = on_a_branch_row();
    super::rewrite_editor::tests::open_an_editor_field(&mut rewrite);
    let fields = vec![
        ("search", typed(on_a_branch_row(), '/')),
        (
            "blame find",
            typed(fixture(ContextId::Blame).expect("blame"), '/'),
        ),
        (
            "J find",
            typed(fixture(ContextId::Journal).expect("J"), '/'),
        ),
        (
            "stash prompt",
            typed(fixture(ContextId::Stash).expect("stash"), 's'),
        ),
        (
            "commit editor",
            fixture(ContextId::CommitEditor).expect("editor"),
        ),
        (
            "credential",
            fixture(ContextId::Credential).expect("credential"),
        ),
        (
            "name prompt",
            box_over_the_graph(walk(), &HintContext::NamePrompt { toggle: true }),
        ),
        ("picker", fixture(ContextId::Picker).expect("picker")),
        ("rewrite field", rewrite),
    ];
    fields
        .into_iter()
        .map(|(name, state)| (name, sized_to_draw(state)))
        .collect()
}
