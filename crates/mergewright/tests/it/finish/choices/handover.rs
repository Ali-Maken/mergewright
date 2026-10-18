//! A clean editor handover marks the file edited; a failed one doesn't.
//! Moved out of `choices.rs` to keep that file under its size row.

use super::*;

fn edited(state: &AppState, path: &str) -> bool {
    state
        .choices
        .get(Path::new(path))
        .is_some_and(|choices| choices.edited)
}

fn handed_back(state: &AppState, result: Result<(), String>) -> AppState {
    let (state, _rereads) = update(
        state,
        AppEvent::Produced(
            mergewright_core::job::JobId::new(0),
            Product::HandedOver(result),
        ),
    );
    state
}

/// The terminal handovers that mark a file `edited in your editor`: a
/// clean return from `$EDITOR` (the changes pane's `e`, the resolver's `e`)
/// or the mergetool. The harness cannot hand the terminal over, so each
/// command is answered the way the loop answers it (`event_loop::running`).
#[test]
fn a_clean_handover_marks_the_file_edited() {
    let (scratch, app, state) = stopped_on_gone_and_notes();

    // The changes pane's `e` on `notes.txt`: a failed handover marks nothing.
    let state = crate::resolver::on_the_row(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflictEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Err("vi exited 1, message unchanged".to_owned()));
    assert!(!edited(&state, "notes.txt"), "a failed handover");
    assert_eq!(state.conflict_editing, None, "the path is spent");
    let (state, _) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);

    // The mergetool on `gone.txt`.
    let (state, _rereads) = update(
        &state,
        AppEvent::Produced(
            mergewright_core::job::JobId::new(0),
            Product::Mergetool {
                path: "gone.txt".into(),
                result: Ok(()),
            },
        ),
    );
    assert!(edited(&state, "gone.txt"), "{:?}", state.choices);
    drop(state);
    app.close();

    // The resolver's own `e`, in a fresh session.
    let (app, state) = App::open(scratch.path());
    let state = app.pump(state, "the stopped merge", |state| {
        state.in_progress.is_some() && state.working_tree.is_some()
    });
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let (state, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::OpenExternalEditor { .. })),
        "{commands:?}"
    );
    let state = handed_back(&state, Ok(()));
    assert!(edited(&state, "notes.txt"), "{:?}", state.choices);
    drop(state);
    app.close();
    drop(scratch);
}
