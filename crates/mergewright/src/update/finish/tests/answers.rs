//! The `--continue` the review started, answered (moved out of
//! `finish/tests.rs`, ease-pass/12, same names).

use super::*;

/// The answer routes, over plain data: the real-git ones are in
/// `tests/it/finish.rs`.
pub(super) fn committing() -> (AppState, JobId) {
    let state = key(&opened(), KeyCode::Char('y'));
    let id = state.operating.expect("the commit runs");
    (state, id)
}

/// m8/13b review fix M2: `Esc` drops the quit wait on the review as it does
/// on the graph — the commit still running, no `CancelJob`.
#[test]
fn esc_on_the_summary_drops_the_quit_wait() {
    let (state, id) = committing();
    let (state, _) = press(&state, control(KeyCode::Char('c')));
    assert!(state.confirm.is_some(), "Ctrl+C boxes over the commit");
    let (state, commands) = press(&state, plain(KeyCode::Char('y')));
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.quit_when_done.is_some(), "the wait is set");
    let (state, commands) = press(&state, plain(KeyCode::Escape));
    assert_eq!(state.quit_when_done, None, "Esc drops the wait");
    assert_eq!(state.operating, Some(id), "the commit still runs");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::CancelJob(_))),
        "{commands:?}"
    );
    assert!(finish(&state).committing.is_some(), "the review stays");
}

#[test]
fn a_refusal_keeps_the_summary_open_with_the_words() {
    let (state, id) = committing();
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::Resumed(Box::new(crate::jobs::operate::Operated::Failed {
                error: "the commit-msg hook said no".to_owned(),
                refresh: None,
                remote: None,
                refused: None,
                edits: None,
            })),
        ),
    );
    let open = finish(&state);
    assert_eq!(open.committing, None);
    assert_eq!(
        open.notice,
        Some(("the commit-msg hook said no".to_owned(), true))
    );
    assert_eq!(state.status, Status::Ready);
    assert!(state.operating.is_none());
}
