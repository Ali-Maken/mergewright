//! A stuck write (m8/13b review fix, M1 items 1–4): the first 10 s of a wait
//! change nothing; then the line offers the stop, and `q` or `Ctrl+C` opens
//! `Quit now?`, whose `y` is the one key that stops such a write half-way.
//! Driven through `update`; the loop's clock is played by calling
//! `waited_long` as the loop does (the count itself is
//! `event_loop::running::quit_clock`, the timeline
//! `event_loop::tests::quit_offer`).

use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_core::job::JobId;

use super::{description, not_cancelled, plain, press, waiting};
use crate::update::events::Command;
use crate::update::quit::{offer_pending, waited_long};
use crate::update::state::{AppState, Quit, Status};

fn ctrl_c() -> KeyPress {
    KeyPress::control(KeyCode::Char('c'))
}

/// Waiting, past `STOP_OFFER_AFTER`.
fn long() -> (AppState, JobId) {
    let (mut state, id) = waiting();
    waited_long(&mut state);
    (state, id)
}

fn long_line(state: &AppState) -> Status {
    Status::Waiting(format!(
        "{} \u{b7} still running after 10 s \u{b7} q stop it now \u{b7} esc stays",
        description(state)
    ))
}

fn lines(state: &AppState) -> Vec<String> {
    state
        .confirm
        .as_ref()
        .expect("a box is up")
        .view
        .message
        .iter()
        .map(|line| line.as_str().to_owned())
        .collect()
}

/// Item 1: before the offer, `q` and `Ctrl+C` while waiting change
/// nothing: no box, the same line.
#[test]
fn before_ten_seconds_q_and_ctrl_c_change_nothing() {
    let (state, id) = waiting();
    assert_eq!(offer_pending(&state), Some(id), "the loop counts this wait");
    for key in [plain('q'), ctrl_c()] {
        let (next, commands) = press(&state, key);
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert!(next.confirm.is_none(), "{key:?}: no box");
        assert_eq!(next.quit, None);
        assert_eq!(next.shown_status(), state.shown_status(), "{key:?}");
    }
}

/// Item 2: at the offer the row reads exactly the long line, and nothing is
/// due after it (fixed text, no ticking count).
#[test]
fn at_ten_seconds_the_row_offers_the_stop_and_nothing_is_due_after() {
    let (state, _) = long();
    assert_eq!(state.shown_status(), &long_line(&state));
    assert_eq!(offer_pending(&state), None, "nothing wakes the loop after");
}

/// With no wait, `waited_long` changes nothing.
#[test]
fn no_wait_is_never_called_long() {
    let (mut state, _) = super::merging();
    let before = state.clone();
    waited_long(&mut state);
    assert_eq!(state.quit_when_done, before.quit_when_done);
    assert_eq!(offer_pending(&state), None);
    let (next, _) = press(&state, plain('q'));
    let open = next
        .confirm
        .as_ref()
        .expect("the waiting box, not the stop box");
    assert_eq!(open.view.title.as_str(), "Quit?");
}

/// Item 3: after the offer, `q` and `Ctrl+C` each open the stop box, with
/// the exact words, cursor on Cancel and `y quit` on the bar.
#[test]
fn after_ten_seconds_q_and_ctrl_c_open_the_stop_box() {
    let (state, _) = long();
    for key in [plain('q'), ctrl_c()] {
        let (next, commands) = press(&state, key);
        assert!(commands.is_empty(), "{key:?}: {commands:?}");
        assert_eq!(next.quit, None);
        let open = next.confirm.as_ref().expect("the stop box");
        assert_eq!(open.view.title.as_str(), "Quit now?");
        assert_eq!(
            lines(&next),
            [
                format!(
                    "{} is still running \u{b7} a git hook may be stuck",
                    description(&state)
                ),
                "quit now stops it half-way \u{b7} the next launch shows where it stopped"
                    .to_owned(),
            ]
        );
        let labels: Vec<&str> = open
            .view
            .choices
            .iter()
            .map(mergewright_term::SanitizedText::as_str)
            .collect();
        assert_eq!(labels, ["Cancel", "Quit"]);
        assert_eq!(open.view.cursor, 0, "the cursor rests on Cancel");
        assert_eq!(
            open.yes(),
            Some(mergewright_ui::graph_screen::ConfirmYes::Quit),
            "the bar leads `y quit`"
        );
    }
}

/// Item 3: Cancel (`⏎` on it) or `Esc` closes the box, and the wait stands
/// with its long line. The next `Esc` drops it; the write runs on.
#[test]
fn cancel_or_esc_keeps_the_long_wait_and_the_next_esc_drops_it() {
    let (state, id) = long();
    let (boxed, _) = press(&state, plain('q'));
    for key in [
        KeyPress::plain(KeyCode::Enter),
        KeyPress::plain(KeyCode::Escape),
    ] {
        let (closed, commands) = press(&boxed, key);
        not_cancelled(&commands);
        assert!(closed.confirm.is_none(), "{key:?} closes the box");
        assert_eq!(closed.quit, None);
        assert_eq!(closed.shown_status(), &long_line(&state), "{key:?}");
        let (dropped, commands) = press(&closed, KeyPress::plain(KeyCode::Escape));
        not_cancelled(&commands);
        assert_eq!(dropped.quit_when_done, None, "the next Esc drops the wait");
        assert_eq!(dropped.operating, Some(id), "the write runs on");
    }
}

/// `Ctrl+C` over the stop box itself does not quit: it passes through the
/// box to `quit::asked`, which opens the same box.
#[test]
fn ctrl_c_over_the_stop_box_does_not_quit() {
    let (state, _) = long();
    let (boxed, _) = press(&state, plain('q'));
    let (next, commands) = press(&boxed, ctrl_c());
    not_cancelled(&commands);
    assert_eq!(next.quit, None);
    assert_eq!(next.confirm, boxed.confirm);
}

/// Item 4: `y` emits `ForceStop(id)` exactly once and quits
/// `StoppedHalfWay`, naming the write.
#[test]
fn y_on_the_stop_box_force_stops_once_and_quits_stopped_half_way() {
    let (state, id) = long();
    let what = description(&state);
    let (boxed, _) = press(&state, plain('q'));
    let (next, commands) = press(&boxed, plain('y'));
    assert_eq!(commands, vec![Command::ForceStop(id)]);
    assert_eq!(next.quit, Some(Quit::StoppedHalfWay { job: id, what }));
    assert!(next.confirm.is_none());
}

/// Nothing else emits `ForceStop`: the whole quit journey up to the stop
/// box — `q`, `y`, `Ctrl+C`, `Esc`, the waiting box's own `y` — never does.
#[test]
fn no_other_path_emits_force_stop() {
    let force = |commands: &[Command]| {
        commands
            .iter()
            .any(|command| matches!(command, Command::ForceStop(_)))
    };
    let (state, _) = super::merging();
    let mut seen = Vec::new();
    let (state, commands) = press(&state, plain('q'));
    seen.extend(commands);
    let (mut state, commands) = press(&state, plain('y'));
    seen.extend(commands);
    for key in [plain('q'), ctrl_c(), plain('y')] {
        let (_, commands) = press(&state, key);
        seen.extend(commands);
    }
    waited_long(&mut state);
    let (boxed, commands) = press(&state, ctrl_c());
    seen.extend(commands);
    let (_, commands) = press(&boxed, KeyPress::plain(KeyCode::Escape));
    seen.extend(commands);
    assert!(!force(&seen), "{seen:?}");
}
