use super::*;

/// `u`, `U` and `Ctrl+Z`, in that order.
fn undo_keys() -> [AppEvent; 3] {
    [
        key(KeyCode::Char('u')),
        key(KeyCode::Char('U')),
        AppEvent::Core(Event::Key(KeyPress {
            code: KeyCode::Char('z'),
            modifiers: Modifiers::CONTROL,
        })),
    ]
}

/// Feeds the three keys and fails if any of them asked for an undo or a
/// redo, or put the app into a write.
fn press_undo_keys(state: &AppState) -> AppState {
    let mut state = state.clone();
    for event in undo_keys() {
        let (next, commands) = update(&state, event);
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Undo(..))),
            "a text field's key asked for an undo: {commands:?}"
        );
        assert!(next.operating.is_none(), "a write started from a letter");
        state = next;
    }
    state
}

#[test]
fn typing_u_into_the_stash_message_writes_letters_not_an_undo() {
    let state = AppState::default();
    let (state, _) = update(&state, key(KeyCode::Char('S')));
    let (state, _) = update(&state, key(KeyCode::Char('s')));
    assert!(state.stash.prompt.is_some(), "`s` opened the save prompt");

    let state = press_undo_keys(&state);

    let prompt = state.stash.prompt.as_ref().expect("the prompt is open");
    assert_eq!(
        prompt.message.text(),
        "uU",
        "`u` and `U` are letters here; `Ctrl+Z` is not text and is swallowed"
    );
}

#[test]
fn typing_u_into_the_commit_subject_writes_letters_not_an_undo() {
    let state = AppState {
        commit_editor: Some(Box::new(commit_editor::CommitEditor::new())),
        ..AppState::default()
    };
    let state = press_undo_keys(&state);

    let editor = state.commit_editor.as_ref().expect("the editor is open");
    assert_eq!(editor.subject.text(), "uU");
}

#[test]
fn typing_u_into_the_commit_body_writes_letters_not_an_undo() {
    let mut editor = commit_editor::CommitEditor::new();
    editor.focus = mergewright_ui::commit_editor::CommitFocus::Body;
    let state = AppState {
        commit_editor: Some(Box::new(editor)),
        ..AppState::default()
    };
    let state = press_undo_keys(&state);

    let editor = state.commit_editor.as_ref().expect("the editor is open");
    assert_eq!(editor.body.text(), "uU");
    assert_eq!(editor.subject.text(), "", "the body had focus, not this");
}

#[test]
fn typing_u_into_the_search_prompt_writes_letters_not_an_undo() {
    // `/` only opens over a walk that exists.
    let (state, _) = merge_on_screen(JobId::new(1), 0);
    let (state, _) = update(&state, key(KeyCode::Char('/')));
    assert!(state.search_prompt.is_some(), "`/` opened the prompt");

    let state = press_undo_keys(&state);

    assert_eq!(
        state.search_prompt.as_deref(),
        Some("uU"),
        "the query takes the letters"
    );
}

/// The predicate itself, so a text field added later is caught by name
/// rather than by whichever handler happens to run first.
#[test]
fn every_text_field_the_app_has_is_named_by_text_field_open() {
    assert!(!AppState::default().text_field_open(), "none open");

    let searching = AppState {
        search_prompt: Some(String::new()),
        ..AppState::default()
    };
    assert!(searching.text_field_open(), "the search prompt");

    let committing = AppState {
        commit_editor: Some(Box::new(commit_editor::CommitEditor::new())),
        ..AppState::default()
    };
    assert!(committing.text_field_open(), "the commit editor");

    let mut stashing = AppState::default();
    stashing.stash.prompt = Some(stash::StashPrompt::default());
    assert!(stashing.text_field_open(), "the stash save prompt");

    // The rewrite editor's rows are not a field; its `r` field is.
    let mut rewriting = AppState::default();
    crate::update::rewrite_editor::tests::open_an_editor(&mut rewriting);
    assert!(!rewriting.text_field_open(), "the rewrite editor's rows");
    crate::update::rewrite_editor::tests::open_an_editor_field(&mut rewriting);
    assert!(rewriting.text_field_open(), "the rewrite editor's r field");
}
