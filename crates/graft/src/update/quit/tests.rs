//! `q` asks first while a write runs or git is stopped mid-operation
//! (m8-polish-and-ship/13, ruling 4). Driven through `update`, so every
//! handler between the key and `graft_key` is in the path.

use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::job::JobId;
use graft_ops::operation::{FetchParams, Operation};

use crate::update::events::{AppEvent, Command};
use crate::update::state::{AppState, Quit};

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    crate::update::update(state, AppEvent::Core(Event::Key(key)))
}

fn plain(character: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(character))
}

/// A fetch launched and still running.
fn fetching() -> AppState {
    let mut state = AppState::default();
    let mut commands = Vec::new();
    let mut next = 40;
    crate::update::operate::launch(
        &mut state,
        &mut commands,
        Operation::Fetch(FetchParams {
            remote: Some("origin".to_owned()),
            prune: false,
        }),
        &mut || {
            next += 1;
            JobId::new(next)
        },
    )
    .expect("nothing else runs");
    state
}

fn box_line(state: &AppState) -> String {
    let open = state.confirm.as_ref().expect("a box is up");
    open.view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn q_with_nothing_running_quits() {
    let (next, commands) = press(&AppState::default(), plain('q'));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.confirm.is_none());
    assert!(commands.is_empty());
}

#[test]
fn q_during_a_fetch_opens_the_box() {
    let state = fetching();
    let (next, commands) = press(&state, plain('q'));
    assert_eq!(next.quit, None, "not yet: the box asks");
    assert!(commands.is_empty(), "nothing stopped yet: {commands:?}");
    assert_eq!(
        box_line(&next),
        "fetch origin is still running \u{b7} quit stops it"
    );
    let open = next.confirm.as_ref().expect("the box");
    let labels: Vec<&str> = open
        .view
        .choices
        .iter()
        .map(graft_term::SanitizedText::as_str)
        .collect();
    assert_eq!(labels, ["Cancel", "Quit"]);
}

/// `[ Quit ]` (here by `→ ⏎`) cancels the job, then quits: the loop
/// carries out the commands before it reads `quit` (`event_loop::running`).
#[test]
fn quit_from_the_box_cancels_the_fetch_and_quits() {
    let state = fetching();
    let id = state.operating.expect("running");
    let (state, _) = press(&state, plain('q'));
    let (state, _) = press(&state, KeyPress::plain(KeyCode::Right));
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert_eq!(commands, [Command::CancelJob(id)]);
}

/// `y` takes `[ Quit ]` from wherever the cursor is (ADR-0044 §3): it
/// cancels the fetch and quits, and the bar offers `y quit`.
#[test]
fn y_quits_from_the_box_and_cancels_the_fetch() {
    let state = fetching();
    let id = state.operating.expect("running");
    let (state, _) = press(&state, plain('q'));
    let open = state.confirm.as_ref().expect("the box");
    assert_eq!(
        open.yes(),
        Some(graft_ui::graph_screen::ConfirmYes::Quit),
        "the bar draws `y quit`"
    );
    let (next, commands) = press(&state, plain('y'));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert_eq!(commands, [Command::CancelJob(id)]);
}

#[test]
fn cancel_or_esc_keeps_the_app_and_the_fetch() {
    let state = fetching();
    let (boxed, _) = press(&state, plain('q'));
    for key in [KeyCode::Escape, KeyCode::Enter] {
        let (next, commands) = press(&boxed, KeyPress::plain(key));
        assert_eq!(next.quit, None, "{key:?}");
        assert!(next.confirm.is_none(), "{key:?} closed the box");
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert_eq!(next.operating, state.operating, "the fetch runs on");
    }
}

/// The fetch finished while the box was up: `[ Quit ]` just quits.
#[test]
fn quit_after_the_write_ended_cancels_nothing() {
    let state = fetching();
    let (mut state, _) = press(&state, plain('q'));
    state.operating = None;
    let (state, _) = press(&state, KeyPress::plain(KeyCode::Right));
    let (next, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(commands.is_empty(), "{commands:?}");
}

/// `Ctrl+C` never asks (keymap §2.1).
#[test]
fn ctrl_c_during_a_fetch_quits_without_a_box() {
    let (next, _) = press(&fetching(), KeyPress::control(KeyCode::Char('c')));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(next.confirm.is_none());
}

/// **The pending ruling's one case** (module doc): a stopped rebase boxes
/// as DESIGN.md §2.2 reads today. Flip `stopped_box` and this test with it.
#[test]
fn q_with_a_stopped_rebase_boxes_pending_the_ruling() {
    let state = AppState {
        in_progress: Some(graft_git::state::InProgressDetail {
            kind: graft_git::state::InProgress::Rebase,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        ..AppState::default()
    };
    let (next, commands) = press(&state, plain('q'));
    assert_eq!(next.quit, None);
    assert!(commands.is_empty());
    assert_eq!(
        box_line(&next),
        "rebase is stopped \u{b7} next launch offers continue or abort"
    );
    let (next, _) = press(&next, KeyPress::plain(KeyCode::Right));
    let (next, commands) = press(&next, KeyPress::plain(KeyCode::Enter));
    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(commands.is_empty(), "nothing to stop: {commands:?}");
}
