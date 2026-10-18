//! The palette over the commit editor (ease-pass/03b, Q8 a): the options
//! are off the commit screen and keyless, so `?` and `⌃P` are their way in.
//! From the editor the palette lists the editor's own rows and nothing
//! else; from anywhere else it lists none of them.

use mergewright_core::event::{Event, KeyCode, KeyPress, Modifiers};
use mergewright_ui::keymap::{ActionId, ContextId, defaults};

use crate::update::commit_editor::tests::ready;
use crate::update::palette::{PaletteTarget, rows};
use crate::update::{AppEvent, AppState, update};
use mergewright_ui::commit_editor::CommitFocus;

fn press(state: &AppState, key: KeyPress) -> AppState {
    update(state, AppEvent::Core(Event::Key(key))).0
}

fn ctrl_p() -> KeyPress {
    KeyPress {
        code: KeyCode::Char('p'),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }
}

/// The commit editor, opened by `c` on a staged file, focus on the subject.
fn editor() -> AppState {
    let open = press(&ready(&["a.rs"]), KeyPress::plain(KeyCode::Char('c')));
    let editor = open.commit_editor.as_ref().expect("`c` opens the editor");
    assert_eq!(editor.focus, CommitFocus::Subject);
    open
}

/// Every action row the open palette shows: its label, its key, its id.
fn action_rows(state: &AppState) -> Vec<(String, String, ActionId)> {
    let palette = state.palette.as_ref().expect("the palette is up");
    rows(state, palette)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .filter_map(|(row, target)| match target {
            PaletteTarget::Action { id, .. } => Some((
                row.label.as_str().to_owned(),
                row.key.map(|key| key.0).unwrap_or_default(),
                id,
            )),
            _ => None,
        })
        .collect()
}

/// The context `id` is bound in.
fn context_of(id: ActionId) -> ContextId {
    defaults()
        .iter()
        .find(|b| b.action == id)
        .unwrap_or_else(|| panic!("{id:?} is in the table"))
        .context
}

/// The open palette, its cursor on the row labelled `label`.
fn cursor_on(mut state: AppState, label: &str) -> AppState {
    let palette = state.palette.as_ref().expect("the palette is up");
    let at = rows(&state, palette)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .position(|(row, _)| row.label.as_str() == label)
        .unwrap_or_else(|| panic!("{label} listed"));
    state.palette.as_mut().expect("up").cursor = at;
    state
}

#[test]
fn ctrl_p_from_the_commit_editor_lists_its_rows_and_only_its_rows() {
    let open = press(&editor(), ctrl_p());
    let rows = action_rows(&open);
    for want in [
        "Amend the last commit",
        "Skip the commit hooks",
        "Allow an empty commit",
    ] {
        assert!(
            rows.iter()
                .any(|(label, key, _)| label == want && key == "—"),
            "{want} spelled `—`: {rows:?}"
        );
    }
    assert!(
        rows.iter()
            .all(|(_, _, id)| context_of(*id) == ContextId::CommitEditor),
        "{rows:?}"
    );
}

#[test]
fn choosing_an_option_row_flips_it_and_keeps_the_editor() {
    let open = cursor_on(press(&editor(), ctrl_p()), "Skip the commit hooks");
    let after = press(&open, KeyPress::plain(KeyCode::Enter));
    assert!(after.palette.is_none(), "the palette closes");
    let editor = after.commit_editor.as_ref().expect("the editor stays");
    assert!(editor.no_verify);
    assert!(!editor.amend && !editor.allow_empty);
    assert_eq!(editor.focus, CommitFocus::Subject, "focus kept");
}

#[test]
fn question_mark_in_the_subject_types_it() {
    let after = press(&editor(), KeyPress::plain(KeyCode::Char('?')));
    assert!(after.palette.is_none());
    let editor = after.commit_editor.as_ref().expect("the editor stays");
    assert_eq!(editor.subject.text(), "?");
}

#[test]
fn question_mark_off_the_text_fields_opens_the_editors_rows() {
    let mut state = editor();
    state.commit_editor.as_mut().expect("open").focus = CommitFocus::Files;
    let open = press(&state, KeyPress::plain(KeyCode::Char('?')));
    let rows = action_rows(&open);
    assert!(
        rows.iter()
            .any(|(label, _, _)| label == "Amend the last commit"),
        "{rows:?}"
    );
}

#[test]
fn from_the_graph_no_commit_editor_row_is_listed() {
    for opener in [ctrl_p(), KeyPress::plain(KeyCode::Char('?'))] {
        let open = press(&ready(&["a.rs"]), opener);
        let rows = action_rows(&open);
        assert!(!rows.is_empty(), "{opener:?} lists rows");
        assert!(
            rows.iter()
                .all(|(_, _, id)| context_of(*id) != ContextId::CommitEditor),
            "{rows:?}"
        );
    }
}

/// Review 03 Minor 5: each keyless option chosen twice from `⌃P` is off
/// again — one flag per id, no row flipping another.
#[test]
fn choosing_each_option_twice_returns_it_to_off() {
    for (label, id) in [
        ("Amend the last commit", "commit.toggle_amend"),
        ("Skip the commit hooks", "commit.toggle_no_verify"),
        ("Allow an empty commit", "commit.toggle_allow_empty"),
    ] {
        let mut state = editor();
        let option = crate::update::commit_editor::CommitOption::of_id(&ActionId(id))
            .expect("one of the three");
        for want in [true, false] {
            let open = cursor_on(press(&state, ctrl_p()), label);
            state = press(&open, KeyPress::plain(KeyCode::Enter));
            let editor = state.commit_editor.as_ref().expect("the editor stays");
            assert_eq!(editor.option(option), want, "{id}");
        }
    }
}

/// Q8 a: `commit.toggle_no_verify` from `?` in the file list names itself
/// in the header.
#[test]
fn skipping_hooks_from_question_mark_in_the_files_shows_in_the_header() {
    let mut state = editor();
    state.commit_editor.as_mut().expect("open").focus = CommitFocus::Files;
    let open = cursor_on(
        press(&state, KeyPress::plain(KeyCode::Char('?'))),
        "Skip the commit hooks",
    );
    let after = press(&open, KeyPress::plain(KeyCode::Enter));
    let header = crate::view::commit_screen_view(&after)
        .expect("the editor stays")
        .header;
    assert_eq!(header.options, ["skipping hooks"]);
}

/// Q8 a: sign-off is a flag the header names, so its row reads as one.
#[test]
fn sign_off_reads_on_off_in_the_keymap() {
    let row = defaults()
        .iter()
        .find(|b| b.action == ActionId("commit.sign_off"))
        .expect("in the table");
    assert_eq!(row.label, "Sign off: on / off");
}
