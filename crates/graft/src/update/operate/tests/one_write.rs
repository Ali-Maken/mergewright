//! One write at a time, where the slot is taken (m6-conflicts/78, Decided
//! 3): `claim_write` refuses while a write runs, and every launch caller the
//! scout listed as reachable with a write running words the refusal and
//! launches nothing.
//!
//! The name prompt's row — every kind, over a real `f` — is
//! `name_prompt::tests::in_flight::a_write_in_flight_refuses_inside_the_prompt`.
//! The confirm box and the Ghost Preview are set up here with the write
//! already running: every opener refuses while one runs, so a key cannot
//! get there today, but the launch no longer relies on that.

use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_core::job::JobId;
use graft_core::journal::EntryId;
use graft_ops::operation::{Operation, Target};
use graft_term::sanitize_line;
use graft_ui::confirm::ConfirmView;

use crate::update::confirm::{ConfirmThen, PendingConfirm, confirm_key};
use crate::update::events::Command;
use crate::update::operate::{claim_write, in_flight};
use crate::update::preview::{PreviewSubject, preview_key, preview_refusal};
use crate::update::remote::fetch_key;
use crate::update::state::{AppState, Status};

fn ids() -> impl FnMut() -> JobId {
    let mut next = 100;
    move || {
        next += 1;
        JobId::new(next)
    }
}

fn key(code: KeyCode) -> KeyPress {
    KeyPress {
        code,
        modifiers: Modifiers::NONE,
    }
}

/// `state` with `f` running: a real launch, so `operating` and the running
/// write's description are what a fetch leaves.
fn fetching(mut state: AppState) -> AppState {
    let mut commands = Vec::new();
    assert!(fetch_key(&mut state, &mut commands, &mut ids()));
    assert!(state.operating.is_some(), "the fetch launched");
    state
}

/// The one wording, read from the running write.
fn wait_words(state: &AppState) -> String {
    in_flight(state).expect("a write is running").words()
}

/// Nothing was run or planned.
fn launched_nothing(commands: &[Command]) -> bool {
    !commands.iter().any(|command| {
        matches!(
            command,
            Command::Operate(..) | Command::RevertTo(..) | Command::PlanPreview(..)
        )
    })
}

fn discard_box() -> PendingConfirm {
    PendingConfirm {
        view: ConfirmView::new(
            sanitize_line("Discard changes to a.rs?"),
            vec![sanitize_line("consequence")],
            sanitize_line("git restore --worktree -- a.rs"),
        ),
        then: ConfirmThen::Operate(Operation::Discard(Target::Paths(vec!["a.rs".into()]))),
    }
}

#[test]
fn one_write_at_a_time() {
    // The slot itself: taken, it refuses and writes nothing.
    let running = fetching(AppState::default());
    let mut claimed = running.clone();
    let refused = claim_write(&mut claimed, JobId::new(999));
    assert_eq!(
        refused.map_err(|busy| busy.words()),
        Err(wait_words(&running))
    );
    assert_eq!(claimed, running, "a refused claim changes nothing");

    // The confirm box's `Operate` (`confirm.rs` act): the acting choice
    // closes the box and says why on the row it gave back. (A discard box
    // answers `→ ⏎`, not `y`: `PendingConfirm::yes`.)
    let mut state = running.clone();
    state.confirm = Some(discard_box());
    let mut commands = Vec::new();
    let mut next = ids();
    assert!(confirm_key(
        &mut state,
        &mut commands,
        key(KeyCode::Right),
        &mut next
    ));
    assert!(confirm_key(
        &mut state,
        &mut commands,
        key(KeyCode::Enter),
        &mut next
    ));
    assert!(state.confirm.is_none(), "confirm box: closed on the key");
    assert_eq!(
        state.status,
        Status::Failed(wait_words(&running)),
        "confirm box"
    );
    assert_eq!(
        state.operating, running.operating,
        "confirm box: the fetch keeps it"
    );
    assert!(launched_nothing(&commands), "confirm box: {commands:?}");

    // The Ghost Preview's `Confirm`, both subjects (`confirm_preview`).
    for subject in ["operation", "revert-to"] {
        let (mut state, _) =
            crate::update::preview::tests::opened_with(crate::update::preview::tests::a_plan());
        if subject == "revert-to" {
            let preview = state.preview.as_mut().expect("a preview is open");
            preview.subject = PreviewSubject::RevertTo(EntryId::new(1, [0; 10]));
        }
        let state = fetching(state);
        let running = state.operating;
        let words = wait_words(&state);
        let mut state = state;
        let mut commands = Vec::new();
        let mut next = ids();
        preview_key(&mut state, &mut commands, key(KeyCode::Right), &mut next);
        preview_key(&mut state, &mut commands, key(KeyCode::Enter), &mut next);
        assert!(state.preview.is_none(), "{subject}: closed on the key");
        assert_eq!(state.status, Status::Failed(words), "{subject}");
        assert_eq!(state.operating, running, "{subject}: the fetch keeps it");
        assert!(launched_nothing(&commands), "{subject}: {commands:?}");
    }
}

/// `f`, the Ghost Preview's refusal and `InFlight::words` say "a write is
/// running" one way (scout Q3 found two copies).
#[test]
fn in_flight_words_are_one_wording() {
    let mut state = fetching(AppState::default());
    let words = wait_words(&state);
    assert!(
        words.starts_with("wait for fetch") && words.ends_with(" to finish"),
        "{words}"
    );
    assert_eq!(preview_refusal(&state).as_deref(), Some(words.as_str()));
    let mut commands = Vec::new();
    assert!(fetch_key(&mut state, &mut commands, &mut ids()));
    assert_eq!(state.status, Status::Failed(words));
    assert!(commands.is_empty(), "the second `f` ran nothing");
}
