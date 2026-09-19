//! The credential prompt's keys and rules, over scripted state. What a
//! real password does to a real push is `tests/it/credential_by_key.rs`.

use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};

use super::*;
use crate::update::events::AppEvent;
use crate::update::remote::Running;
use crate::update::state::Status;
use crate::update::update;

const JOB: JobId = JobId::new(4);

/// A push running under [`JOB`], with git's question open on screen.
fn asking(prompt: &str) -> AppState {
    let mut state = AppState {
        operating: Some(JOB),
        status: Status::Busy("push main → origin/main · esc cancels".to_owned()),
        ..AppState::default()
    };
    state.remote.running = Some(Running {
        id: JOB,
        description: "push main → origin/main".to_owned(),
        op: None,
        stoppable: crate::update::remote::Stoppable::Yes,
    });
    credential_needed(&mut state, JOB, prompt);
    assert!(state.credential.is_some(), "the prompt opened");
    state
}

fn press(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

fn type_in(state: &mut AppState, text: &str) -> Vec<Command> {
    let mut all = Vec::new();
    for character in text.chars() {
        let (next, commands) = press(state, KeyPress::plain(KeyCode::Char(character)));
        *state = next;
        all.extend(commands);
    }
    all
}

fn typed(state: &AppState) -> &str {
    state
        .credential
        .as_ref()
        .expect("the prompt is open")
        .field
        .text()
}

#[test]
fn is_masked_prompt_table() {
    for masked in [
        "Password for 'http://user@127.0.0.1:8443':",
        "Enter passphrase for key '/home/u/.ssh/id_ed25519':",
        "password:",
    ] {
        assert!(is_masked_prompt(masked), "{masked:?} must be masked");
    }
    for plain in [
        "Username for 'https://github.com':",
        "Are you sure you want to continue connecting (yes/no/[fingerprint])?",
    ] {
        assert!(!is_masked_prompt(plain), "{plain:?} must be readable");
    }
}

#[test]
fn the_title_names_the_host_when_the_prompt_has_a_url() {
    assert_eq!(
        host_of("Password for 'http://user@127.0.0.1:8443':").as_deref(),
        Some("127.0.0.1:8443")
    );
    assert_eq!(
        host_of("Username for 'https://github.com':").as_deref(),
        Some("github.com")
    );
    assert_eq!(host_of("Enter passphrase for key '/k/id_ed25519':"), None);

    let state = asking("Password for 'http://user@127.0.0.1:8443':");
    let prompt = state.credential.as_ref().expect("open");
    assert_eq!(
        prompt.title.as_str(),
        "Credentials \u{2014} 127.0.0.1:8443",
        "the title names the host, not the user"
    );
    let plain = asking("Enter passphrase for key '/k/id_ed25519':");
    assert_eq!(
        plain.credential.expect("open").title.as_str(),
        "Credentials"
    );
}

#[test]
fn gits_words_are_cleaned_before_they_are_kept() {
    // Remote-reachable text: a crafted URL in git's prompt must not carry
    // an escape sequence to the screen.
    let state = asking("Password for 'http://u@h/\u{1b}[31mred\r\n':");
    let prompt = state.credential.expect("open");
    assert!(
        !prompt.prompt.as_str().contains('\u{1b}'),
        "{:?}",
        prompt.prompt.as_str()
    );
    assert!(!prompt.prompt.as_str().contains('\n'));
}

#[test]
fn prompt_is_modal_and_swallows_globals() {
    let mut state = asking("Password for 'http://u@h':");
    assert!(
        state.text_field_open(),
        "the prompt is a text field: no global binding may fire under it"
    );

    // The m3/06 Critical 1 regression, through `update()`: the undo keys,
    // the remote keys and the stash key are letters here and nothing else.
    let commands = type_in(&mut state, "uUfpP");
    assert_eq!(typed(&state), "uUfpP");
    assert!(commands.is_empty(), "{commands:?}");
    assert!(state.operating == Some(JOB) && state.confirm.is_none());

    let (after, commands) = press(
        &state,
        KeyPress {
            code: KeyCode::Char('z'),
            modifiers: Modifiers::CONTROL,
        },
    );
    assert_eq!(typed(&after), "uUfpP", "Ctrl+Z is not text and not undo");
    assert!(commands.is_empty(), "{commands:?}");

    // `q` does not quit, `S` does not open the stash, `/` does not search.
    let mut state = after;
    let commands = type_in(&mut state, "qS/");
    assert_eq!(typed(&state), "uUfpPqS/");
    assert!(state.quit.is_none() && !state.stash.shown && state.search_prompt.is_none());
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn ctrl_u_clears_the_field() {
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunter2");
    assert_eq!(typed(&state), "hunter2");

    let (after, commands) = press(
        &state,
        KeyPress {
            code: KeyCode::Char('u'),
            modifiers: Modifiers::CONTROL,
        },
    );

    assert_eq!(typed(&after), "");
    assert!(commands.is_empty(), "{commands:?}");
    assert!(after.credential.is_some(), "the prompt stays open");
}

#[test]
fn enter_answers_and_clears() {
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunter2");

    let (after, commands) = press(&state, KeyPress::plain(KeyCode::Enter));

    assert_eq!(
        commands,
        vec![Command::AnswerCredential(
            JOB,
            Answer::new(b"hunter2".to_vec())
        )],
        "the typed bytes go to the job that asked"
    );
    assert!(after.credential.is_none(), "the prompt closed");
    assert!(
        after.operating == Some(JOB),
        "the push is still running: git has the answer now"
    );
    // The field went with the prompt; nothing on the state holds the text.
    assert!(!format!("{after:?}").contains("hunter2"));
}

#[test]
fn an_empty_answer_is_still_an_answer() {
    let state = asking("Username for 'https://github.com':");

    let (_after, commands) = press(&state, KeyPress::plain(KeyCode::Enter));

    assert_eq!(
        commands,
        vec![Command::AnswerCredential(JOB, Answer::new(Vec::new()))],
        "git decides what an empty answer means"
    );
}

/// `Esc` in the prompt stops the whole write, and stops it **before** it
/// refuses the ask. The other order lets git see the refusal and die on its
/// own, which is classified `AuthFailed` — the line would then blame the
/// password for what the user chose (m4-remotes/06 review, Minor m1).
#[test]
fn esc_cancels_the_operation() {
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunt");

    let (after, commands) = press(&state, KeyPress::plain(KeyCode::Escape));

    assert_eq!(
        commands,
        vec![Command::CancelJob(JOB), Command::CancelCredential(JOB)],
        "the write stops first, then the ask is answered `cancelled`"
    );
    assert!(after.credential.is_none());
    assert_eq!(after.remote.cancelling, Some(JOB));
    assert_eq!(
        after.status,
        Status::Busy("push main → origin/main · cancelling".to_owned())
    );
    assert!(!format!("{after:?}").contains("hunt"));
}

#[test]
fn editing_keys_work_and_paste_lands_in_the_field() {
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunter3");
    let (mut state, _) = press(&state, KeyPress::plain(KeyCode::Backspace));
    assert_eq!(typed(&state), "hunter");
    for code in [KeyCode::Home, KeyCode::Right, KeyCode::Delete] {
        let (next, _) = press(&state, KeyPress::plain(code));
        state = next;
    }
    assert_eq!(typed(&state), "hnter", "Home, →, Delete");

    let (state, commands) = update(
        &state,
        AppEvent::Core(Event::Paste("2wo\u{1b}rd".to_owned())),
    );
    assert_eq!(
        typed(&state),
        "h2wordnter",
        "a paste lands at the cursor, its control characters refused"
    );
    assert!(commands.is_empty(), "{commands:?}");
}

#[test]
fn debug_redacts_secret() {
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunter2");

    let printed = format!("{state:?}");

    assert!(
        !printed.contains("hunter2"),
        "the typed password reached a Debug print: {printed}"
    );
    assert!(
        printed.contains("CredentialPrompt"),
        "the prompt is still visible in a debug dump: {printed}"
    );
    // The command carrying it away is no different.
    let (_, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    let printed = format!("{commands:?}");
    assert!(!printed.contains("hunter2"), "{printed}");
}

#[test]
fn a_second_ask_replaces_the_first_and_a_stale_one_is_ignored() {
    let mut state = asking("Username for 'https://github.com':");
    type_in(&mut state, "ali");
    assert!(
        !state.credential.as_ref().expect("open").masked,
        "a username is typed in the open"
    );

    // git asks for the password next, under the same job.
    credential_needed(&mut state, JOB, "Password for 'https://ali@github.com':");
    let prompt = state.credential.as_ref().expect("open");
    assert!(prompt.masked, "the second ask is a password");
    assert_eq!(prompt.field.text(), "", "the username is not left in it");

    // An ask from a job the screen is not waiting on opens nothing.
    let mut other = AppState {
        operating: Some(JobId::new(9)),
        ..AppState::default()
    };
    credential_needed(&mut other, JOB, "Password for 'https://x':");
    assert!(other.credential.is_none());

    // Nor does one for a write the user has already asked to stop.
    let mut stopping = AppState {
        operating: Some(JOB),
        ..AppState::default()
    };
    stopping.remote.cancelling = Some(JOB);
    credential_needed(&mut stopping, JOB, "Password for 'https://x':");
    assert!(stopping.credential.is_none());
}

#[test]
fn a_prompt_whose_write_has_ended_closes_itself() {
    let state = asking("Password for 'http://u@h':");

    // The job ended between the ask and the next event.
    let mut ended = state.clone();
    ended.operating = None;
    close_if_orphaned(&mut ended);

    assert!(
        ended.credential.is_none(),
        "a prompt with nothing to answer would answer into nothing"
    );
}
