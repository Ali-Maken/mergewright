//! `update::undo` tests: the answers. Moved out of the inline module (m8/21b-5,
//! move-only).

use super::*;

/// m7/30o R3: every launch says what it did, so no caller counts
/// commands to find out.
#[test]
fn start_undo_says_what_it_did() {
    // A write in flight: refused, in the one wording every refused
    // launch uses (V1: it used to be silent).
    let mut busy = AppState {
        operating: Some(JobId::new(4)),
        ..AppState::default()
    };
    let mut commands = Vec::new();
    assert_eq!(
        start_undo(&mut busy, &mut commands, false, &mut ids()),
        UndoStart::Refused
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        busy.status,
        Status::Failed("wait for the write to finish".to_owned())
    );

    // An operation stopped: refused, with its words.
    let mut stopped = AppState {
        in_progress: Some(mergewright_git::state::InProgressDetail {
            kind: mergewright_git::state::InProgress::Rebase,
            step: None,
            head_name: None,
            onto: None,
            theirs: Ok(None),
        }),
        ..AppState::default()
    };
    assert_eq!(
        start_undo(&mut stopped, &mut commands, true, &mut ids()),
        UndoStart::Refused
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert!(
        matches!(&stopped.status, Status::Sticky { reason, .. } if reason == "finish or abort the rebase first"),
        "{:?}",
        stopped.status
    );

    // After a push: the box, and nothing runs.
    let mut pushed = after_a_push();
    assert_eq!(
        start_undo(&mut pushed, &mut commands, false, &mut ids()),
        UndoStart::Asked
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert!(pushed.confirm.is_some(), "the undo-after-push box");
    assert_eq!(pushed.operating, None);

    // Otherwise: one undo.
    let mut free = AppState::default();
    assert_eq!(
        start_undo(&mut free, &mut commands, false, &mut ids()),
        UndoStart::Launched
    );
    assert_eq!(
        commands,
        vec![Command::Undo(
            JobId::new(1),
            false,
            std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone())
        )]
    );
}

#[test]
fn a_finished_undo_says_what_it_undid_and_re_reads_the_working_tree() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Done(applied("undo: stage src/a.rs", Refresh::WorkingTree)),
        &mut ids(),
    );
    assert_eq!(state.status, Status::done("undo: stage src/a.rs"));
    assert!(state.operating.is_none());
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadWorkingTree(_))),
        "{commands:?}"
    );
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadJournal(_))),
        "the journal has a new entry of its own: {commands:?}"
    );
}

#[test]
fn an_undo_that_moved_refs_re_reads_everything() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Done(applied("undo: commit \"fix\"", Refresh::Everything)),
        &mut ids(),
    );
    assert!(commands.contains(&Command::OpenRepository), "{commands:?}");
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::ReadRefs(_))),
        "{commands:?}"
    );
}

#[test]
fn an_answer_for_another_job_is_dropped() {
    let mut state = AppState {
        operating: Some(JobId::new(9)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Done(applied("undo: stage a", Refresh::Everything)),
        &mut ids(),
    );
    assert!(commands.is_empty());
    assert_eq!(state.operating, Some(JobId::new(9)));
    assert_eq!(state.status, Status::Ready);
}

#[test]
fn nothing_to_undo_is_an_answer_not_a_failure() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Nothing {
            reason: "nothing to undo".to_owned(),
            redo: None,
        },
        &mut ids(),
    );
    assert_eq!(state.status, Status::done("nothing to undo"));
    assert!(commands.is_empty());
}

/// m6-conflicts/77: with a redo waiting, "nothing to undo" names the
/// key that goes the other way instead of ending on a dead end.
#[test]
fn nothing_to_undo_names_the_redo_waiting() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    undone(
        &mut state,
        JobId::new(2),
        Undone::Nothing {
            reason: "nothing to undo".to_owned(),
            redo: Some("checkout fix".to_owned()),
        },
        &mut ids(),
    );
    assert_eq!(
        state.status,
        Status::done_with_keys("nothing to undo", "U redoes checkout fix")
    );
}

/// ease-pass 28 A: the refresh a failed undo asks for finishes, and the
/// failure is still on screen, still marked as one.
#[test]
fn a_failed_undo_stays_marked_after_its_refresh() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Failed {
            error: "refs/heads/main is not where it was".to_owned(),
            refresh: Some(Refresh::WorkingTree),
        },
        &mut ids(),
    );
    assert!(!commands.is_empty(), "the refresh was asked for");
    let want = Status::sticky_failure_keys("refs/heads/main is not where it was".to_owned(), None);
    assert_eq!(state.status, want);
    for n in 1..=4 {
        let (next, _) = crate::update::update(
            &state,
            crate::update::AppEvent::Core(mergewright_core::event::Event::JobFinished(JobId::new(
                n,
            ))),
        );
        state = next;
    }
    assert_eq!(state.status, want, "the refresh must not wipe it");
}

#[test]
fn drift_opens_a_box_that_quotes_what_moved_and_answers_nothing_yet() {
    let mut state = AppState {
        operating: Some(JobId::new(2)),
        ..AppState::default()
    };
    let commands = undone(
        &mut state,
        JobId::new(2),
        Undone::Drift {
            description: "stage src/a.rs".to_owned(),
            drift: drift(),
            redo: false,
        },
        &mut ids(),
    );
    assert!(commands.is_empty(), "nothing runs until the user says so");
    let pending = state.confirm.as_ref().expect("the drift box opened");
    assert_eq!(pending.view.selected(), 0, "the safe choice first");
    assert_eq!(pending.view.choices[0].as_str(), "Stop");
    assert_eq!(pending.view.choices[1].as_str(), "Undo");
    assert!(
        pending
            .view
            .message
            .iter()
            .any(|line| line.as_str().contains(&drift().explain())),
        "the box says what ops said: {:?}",
        pending.view.message
    );
    assert_eq!(pending.then, ConfirmThen::Undo { redo: false });
}

#[test]
fn a_long_drift_explanation_is_wrapped_rather_than_clipped() {
    // The box is 64 columns at most, and a `…` where the drift's own
    // words should be is the one thing it must not do.
    let drift = mergewright_ops::undo::Drift {
        index: Some(index_drift()),
        worktree: true,
        unknown: vec!["HEAD", "the working tree"],
        ..drift()
    };
    let mut state = AppState::default();
    let id = JobId::new(1);
    state.operating = Some(id);
    undone(
        &mut state,
        id,
        Undone::Drift {
            description: "commit \"Add the OAuth provider and its tests\"".to_owned(),
            drift,
            redo: false,
        },
        &mut ids(),
    );
    let view = &state.confirm.as_ref().expect("the guard opened").view;
    assert!(view.message.len() > 2, "{:?}", view.message);
    for line in &view.message {
        assert!(line.chars().count() <= 62, "{line}");
    }
    let joined = view
        .message
        .iter()
        .map(mergewright_term::SanitizedText::as_str)
        .collect::<Vec<_>>()
        .join(" ");
    assert!(joined.contains("the working tree"), "{joined}");
}

#[test]
fn enter_on_stop_changes_nothing_and_enter_on_undo_forces() {
    let box_open = |cursor_on_undo: bool| {
        let mut view = drift_box("stage src/a.rs", &drift(), false);
        if cursor_on_undo {
            view.view.toggle();
        }
        AppState {
            confirm: Some(view),
            ..AppState::default()
        }
    };

    let mut stopped = box_open(false);
    let mut commands = Vec::new();
    assert!(super::super::super::confirm::confirm_key(
        &mut stopped,
        &mut commands,
        KeyPress::plain(KeyCode::Enter),
        &mut ids(),
    ));
    assert!(commands.is_empty(), "Stop does nothing: {commands:?}");
    assert!(stopped.operating.is_none());

    let mut forced = box_open(true);
    let mut commands = Vec::new();
    assert!(super::super::super::confirm::confirm_key(
        &mut forced,
        &mut commands,
        KeyPress::plain(KeyCode::Enter),
        &mut ids(),
    ));
    assert_eq!(
        commands,
        vec![Command::Undo(
            JobId::new(1),
            true,
            std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone())
        )]
    );
}

#[test]
fn a_redo_drift_box_re_runs_the_redo() {
    let mut state = AppState {
        confirm: Some(drift_box("undo: commit \"fix\"", &drift(), true)),
        ..AppState::default()
    };
    state
        .confirm
        .as_mut()
        .expect("the box is open")
        .view
        .toggle();
    let mut commands = Vec::new();
    assert!(super::super::super::confirm::confirm_key(
        &mut state,
        &mut commands,
        KeyPress::plain(KeyCode::Enter),
        &mut ids(),
    ));
    assert_eq!(
        commands,
        vec![Command::Redo(
            JobId::new(1),
            true,
            std::sync::Arc::new(mergewright_ui::keymap::Keymap::shipped().clone())
        )]
    );
}
