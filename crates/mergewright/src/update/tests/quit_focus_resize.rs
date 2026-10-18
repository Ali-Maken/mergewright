//! `update` tests: quit focus resize. Moved out of `update/tests.rs` (m8/21b-5, move-only).

use super::*;

#[test]
fn q_quits() {
    let (next, commands) = update(&AppState::default(), key(KeyCode::Char('q')));

    assert_eq!(next.quit, Some(Quit::Requested));
    assert!(commands.is_empty());
}

#[test]
fn control_c_quits() {
    let event = core(Event::Key(KeyPress::control(KeyCode::Char('c'))));

    let (next, _) = update(&AppState::default(), event);

    assert_eq!(next.quit, Some(Quit::Requested));
}

#[test]
fn control_q_is_not_quit() {
    // Only the bare `q` is bound. A modified one belongs to whatever
    // binding claims it later, and must not fall through to quitting.
    let event = core(Event::Key(KeyPress::control(KeyCode::Char('q'))));

    let (next, _) = update(&AppState::default(), event);

    assert_eq!(next.quit, None);
}

#[test]
fn a_bare_c_is_not_quit() {
    let (next, _) = update(&AppState::default(), key(KeyCode::Char('c')));

    assert_eq!(next.quit, None);
}

/// The state a focus report means something in: a repository is open.
fn opened_at(now_seconds: i64) -> AppState {
    AppState {
        repo_path: PathBuf::from("/code/api"),
        repository: Some(Box::new(facts("/code/api", "main"))),
        harness: Harness {
            clock: crate::session::Clock::Fixed(now_seconds),
            ..Harness::default()
        },
        ..AppState::default()
    }
}

#[test]
fn coming_back_to_the_window_re_reads_the_working_tree_and_nothing_else() {
    // Design note change 7: what another window did to the repository is
    // what the user expects to see on coming back. Not the history walk —
    // restarting it would move the cursor under their hands.
    // The launch journal read rides along on the first `update` of an open
    // repository, whatever the event; what matters is the working-tree read
    // and that no history walk starts.
    let (next, commands) = update(&opened_at(10), core(Event::FocusGained));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadHistory(..) | Command::ShowRows(..))),
        "{commands:?}"
    );
    assert_eq!(next.last_focus_refresh, Some(10_000));
}

#[test]
fn a_second_focus_report_within_half_a_second_is_ignored() {
    // Terminals report focus in pairs, and tmux reports it per pane switch.
    let mut state = opened_at(10);
    state.last_focus_refresh = Some(10_000 - 100);
    let (next, commands) = update(&state, core(Event::FocusGained));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert_eq!(next.last_focus_refresh, Some(10_000 - 100), "untouched");

    state.last_focus_refresh = Some(10_000 - 600);
    let (_, commands) = update(&state, core(Event::FocusGained));
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "600 ms later: {commands:?}"
    );
}

#[test]
fn a_focus_report_before_the_repository_opened_asks_for_nothing() {
    let before = AppState::default();
    let (next, commands) = update(&before, core(Event::FocusGained));
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(next, before);
}

#[test]
fn unbound_events_change_nothing() {
    let before = AppState::default();
    let unbound = [
        key(KeyCode::Char('z')),
        key(KeyCode::Function(7)),
        core(Event::Paste("hello".to_owned())),
        core(Event::FocusGained),
        core(Event::FocusLost),
        core(Event::JobCancelled(JobId::new(1))),
    ];

    for event in unbound {
        let (next, commands) = update(&before, event.clone());

        assert_eq!(next, before, "{event:?} changed the state");
        assert!(commands.is_empty(), "{event:?} asked for work");
    }
}

#[test]
fn resize_is_remembered_including_a_degenerate_one() {
    let (next, _) = update(
        &AppState::default(),
        core(Event::Resize {
            columns: 0,
            rows: 0,
        }),
    );

    assert_eq!(
        next.size,
        Size {
            columns: 0,
            rows: 0
        }
    );

    let (next, _) = update(
        &next,
        core(Event::Resize {
            columns: 120,
            rows: 40,
        }),
    );

    assert_eq!(
        next.size,
        Size {
            columns: 120,
            rows: 40
        }
    );
}
