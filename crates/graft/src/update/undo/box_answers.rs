//! m6-conflicts/69, Decided 4: **a job answer that opens a box ends its own
//! `Busy`** in the same update. The two answer sites that open one are the
//! undo/redo drift box (`undone`) and the time machine's drift box
//! (`reverted_to`); every other `confirm = Some` is a key press, with no
//! `Busy` of its own to end.

use graft_core::event::KeyCode;
use graft_core::journal::EntryId;

use super::*;
use crate::jobs::undo::RevertedTo;

fn ids() -> impl FnMut() -> JobId {
    let mut next = 0;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn drift() -> graft_ops::undo::Drift {
    graft_ops::undo::Drift {
        worktree: true,
        ..graft_ops::undo::Drift::default()
    }
}

/// The job `undo_key` launched, from the command it sent.
fn launched(commands: &[Command]) -> JobId {
    match commands {
        [Command::Undo(id, ..) | Command::Redo(id, ..)] => *id,
        other => panic!("one undo or redo job: {other:?}"),
    }
}

/// **The failing test first.** `u`, then the drift answer: the box is open
/// and `undo · esc cancels` is gone with the job it described.
#[test]
fn the_drift_box_ends_the_undo_busy_line() {
    let mut state = AppState::default();
    let id = launched(&undo_key(&mut state, false, &mut ids()));
    assert!(
        matches!(state.status, Status::Busy(_)),
        "{:?}",
        state.status
    );
    undone(
        &mut state,
        id,
        Undone::Drift {
            description: "stage src/a.rs".to_owned(),
            drift: drift(),
            redo: false,
        },
        &mut ids(),
    );
    assert!(state.confirm.is_some(), "the drift box opened");
    assert_eq!(state.status, Status::Ready);
}

/// One row per answer site that opens a box: `u` and `U` over `undone`,
/// and the time machine over `reverted_to`. Each started with its own
/// `Busy`, and each answer ends it.
#[test]
fn a_box_opened_by_an_answer_ends_its_own_busy() {
    for redo in [false, true] {
        let mut state = AppState::default();
        let id = launched(&undo_key(&mut state, redo, &mut ids()));
        assert!(matches!(state.status, Status::Busy(_)));
        undone(
            &mut state,
            id,
            Undone::Drift {
                description: "stage src/a.rs".to_owned(),
                drift: drift(),
                redo,
            },
            &mut ids(),
        );
        assert!(state.confirm.is_some(), "redo {redo}: the box");
        assert_eq!(state.status, Status::Ready, "redo {redo}");
    }

    // The time machine: its first attempt's `Busy` is the preview's
    // (`preview.rs`, `PreviewSubject::RevertTo`), worded the same way as
    // `revert_to_forced`'s; the answer is the same function either way.
    let mut state = AppState::default();
    let mut commands = Vec::new();
    revert_to_forced(
        &mut state,
        &mut commands,
        EntryId::new(1, [0; 10]),
        "commit one",
        &mut ids(),
    );
    let [Command::Unstoppable(..), Command::RevertTo(id, ..)] = commands.as_slice() else {
        panic!("{commands:?}")
    };
    assert!(matches!(state.status, Status::Busy(_)));
    reverted_to(
        &mut state,
        *id,
        RevertedTo::Drift {
            description: "commit one".to_owned(),
            total: 1,
            drift: drift(),
            target: EntryId::new(1, [0; 10]),
        },
        &mut ids(),
    );
    assert!(state.confirm.is_some(), "the revert drift box");
    assert_eq!(state.status, Status::Ready);
}

/// The credential prompt is not an answer: its job is still running and
/// waiting, so its `Busy` stays in state.
#[test]
fn a_credential_prompt_keeps_its_jobs_busy() {
    let id = JobId::new(5);
    let busy = Status::Busy("fetch origin".to_owned());
    let mut state = AppState {
        operating: Some(id),
        status: busy.clone(),
        ..AppState::default()
    };
    crate::update::credential::credential_needed(
        &mut state,
        id,
        "Password for 'https://host': ",
        false,
    );
    assert!(state.credential.is_some(), "the prompt opened");
    assert_eq!(state.status, busy);
    // Answering it leaves the job running, and its line with it.
    let state = crate::update::update(
        &state,
        crate::update::AppEvent::Core(graft_core::event::Event::Key(
            graft_core::event::KeyPress::plain(KeyCode::Char('x')),
        )),
    )
    .0;
    assert_eq!(state.status, busy);
}
