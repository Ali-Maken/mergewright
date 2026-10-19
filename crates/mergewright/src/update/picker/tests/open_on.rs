//! `open_on` (ease-pass 09b): the wanted branch is held until the list
//! lands, then the cursor is put on it; his own move first wins. And
//! when `C` opens the picker at all.

use mergewright_ops::branch_status::{Candidate, CandidateKind};

use super::*;

fn sara() -> Candidate {
    Candidate {
        name: "refs/remotes/origin/sara/login".to_owned(),
        tip: oid(7),
        kind: CandidateKind::Other,
    }
}

/// `open_on`, then the list lands under the asked id.
fn open_on_then_land(candidate: &Candidate, before: &[KeyCode]) -> AppState {
    let mut state = fixture();
    let commands = open_on(&mut state, candidate);
    let id = read_id(&commands);
    assert_eq!(
        picker(&state).list,
        Loading::Pending(id),
        "the list is pending at open"
    );
    let state = before
        .iter()
        .fold(state, |state, code| press(&state, *code).0);
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::SwitchList(Ok(Arc::new(list())))),
    );
    state
}

#[test]
fn the_list_landing_after_open_on_selects_the_branch() {
    let state = open_on_then_land(&sara(), &[]);
    let picker = picker(&state);
    assert_eq!(
        picker.selected_row().map(|row| row.name.clone()),
        Some(name("refs/remotes/origin/sara/login"))
    );
    assert_eq!(picker.wanted, None, "cleared once placed");
}

#[test]
fn a_list_without_the_branch_selects_the_first_row() {
    let gone = Candidate {
        name: "refs/heads/gone".to_owned(),
        ..sara()
    };
    let state = open_on_then_land(&gone, &[]);
    let picker = picker(&state);
    assert_eq!(picker.selected, 0);
    assert_eq!(picker.list, Loading::Ready(Arc::new(list())), "no error");
    assert_eq!(picker.note, None);
}

#[test]
fn a_move_before_the_list_lands_clears_the_wanted_branch() {
    let mut state = fixture();
    let _commands = open_on(&mut state, &sara());
    let (state, _) = press(&state, KeyCode::Char('j'));
    assert_eq!(
        picker(&state).wanted,
        None,
        "`j` (typed: it edits the query) clears it"
    );
    let state = open_on_then_land(&sara(), &[KeyCode::Down]);
    assert_eq!(
        picker(&state).selected,
        0,
        "his position kept, not sara's row"
    );
}

/// m8/09e: was `c_needs_the_graph_focused_and_nothing_over_it` — a pane's
/// focus no longer stops `C` (`update::keys::tests::c_opens_the_picker_from_the_panes`);
/// an overlay still does.
#[test]
fn c_needs_nothing_over_the_graph() {
    let mut state = fixture();
    state.journal.shown = true;
    let (state, _) = press(&state, KeyCode::Char('C'));
    assert!(state.picker.is_none(), "the journal is up");
}

/// The commit editor's `C` stays the editor's (`commit_editor.rs`): the
/// picker is routed ahead of it, and must let it through.
#[test]
fn c_in_the_commit_editor_is_typed_there() {
    let mut state = fixture();
    state.commit_editor = Some(Box::new(crate::update::commit_editor::CommitEditor::new()));
    let (state, commands) = press(&state, KeyCode::Char('C'));
    assert!(state.picker.is_none());
    assert!(
        !commands
            .iter()
            .any(|c| matches!(c, Command::ReadSwitchList(_)))
    );
    assert_eq!(
        state
            .commit_editor
            .as_ref()
            .map(|editor| editor.subject.text().to_owned()),
        Some("C".to_owned())
    );
}
