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
        opened_by: None,
    });
    // The seed is pinned here: `update` is pure, and every test that
    // looks at the drawn row needs to know where the pulse started.
    credential_needed(&mut state, JOB, prompt, false);
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

/// The open field's text — only ever valid for a `PromptField::Open`
/// (a username ask); a secret field has no `text()` to read (that is the
/// point of `SecretField`), so tests that exercise the masked case use
/// [`typed_len`] and [`typed_bytes`] instead.
fn typed(state: &AppState) -> &str {
    match &state.credential.as_ref().expect("the prompt is open").field {
        PromptField::Open(field) => field.text(),
        PromptField::Secret(_) => {
            panic!("typed() is for the open field; use typed_len/typed_bytes")
        }
    }
}

/// How many characters are in the open prompt's field, whichever kind.
fn typed_len(state: &AppState) -> usize {
    match &state.credential.as_ref().expect("the prompt is open").field {
        PromptField::Open(field) => field.len(),
        PromptField::Secret(field) => field.len(),
    }
}

/// Consumes `state`'s prompt and reads back what a `Secret` field holds —
/// only ever done in a test, to prove editing behaves like `TextField`'s
/// did; the real path (`Answer::new`) never turns it back into a `String`.
fn typed_bytes(state: AppState) -> String {
    match state.credential.expect("the prompt is open").field {
        PromptField::Secret(field) => {
            String::from_utf8(field.into_bytes().to_vec()).expect("typed text is valid UTF-8")
        }
        PromptField::Open(field) => field.text().to_owned(),
    }
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
    // The open field, not the secret one: this test is about which keys
    // reach the prompt, not about masking, and `typed()` only reads back
    // an `Open` field (a `Secret` one has no `text()` to compare).
    let mut state = asking("Username for 'https://github.com':");
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
    assert_eq!(typed_len(&state), 7);

    let (after, commands) = press(
        &state,
        KeyPress {
            code: KeyCode::Char('u'),
            modifiers: Modifiers::CONTROL,
        },
    );

    assert_eq!(typed_len(&after), 0);
    assert!(commands.is_empty(), "{commands:?}");
    assert!(after.credential.is_some(), "the prompt stays open");
    assert_eq!(
        typed_bytes(after),
        "",
        "the buffer itself was replaced, not just marked empty"
    );
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
            Answer::new(zeroize::Zeroizing::new(b"hunter2".to_vec()))
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
        vec![Command::AnswerCredential(
            JOB,
            Answer::new(zeroize::Zeroizing::new(Vec::new()))
        )],
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
    // The secret field: there is no `text()` to read back mid-test, so
    // every intermediate check is by length; the final content is checked
    // once, by consuming the state into `typed_bytes`.
    let mut state = asking("Password for 'http://u@h':");
    type_in(&mut state, "hunter3");
    let (mut state, _) = press(&state, KeyPress::plain(KeyCode::Backspace));
    assert_eq!(typed_len(&state), 6, "hunter");
    for code in [KeyCode::Home, KeyCode::Right, KeyCode::Delete] {
        let (next, _) = press(&state, KeyPress::plain(code));
        state = next;
    }
    assert_eq!(typed_len(&state), 5, "Home, →, Delete: hnter");

    let (state, commands) = update(
        &state,
        AppEvent::Core(Event::Paste("2wo\u{1b}rd".to_owned())),
    );
    assert_eq!(
        typed_len(&state),
        10,
        "a paste lands at the cursor, its control characters refused"
    );
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(
        typed_bytes(state),
        "h2wordnter",
        "the exact content, proven once by consuming into_bytes"
    );
}

/// `{:?}` on `SecretField`, `FieldView`, `CredentialPrompt` and `AppState`
/// (with a prompt open) never contains the typed bytes — the acceptance's
/// "no `Debug` leak" tick, at every layer that carries the field.
/// `graft_ui::text_input::tests::debug_redacts_a_secret_fields_text` and
/// `secret_field_debug_and_eq_never_compare_content` cover `FieldView` and
/// `SecretField` directly; this covers them wired into the app.
#[test]
fn secret_never_in_debug() {
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

    // The field, on its own — `PromptField`'s derived `Debug` calls through
    // to `SecretField`'s own, which prints a constant. Not the length
    // either: this used to read `Secret(SecretField(7))` (m5-01 review,
    // Major 1), and ADR-0038 §2 bars the length from `Debug` as firmly as
    // from the drawn row.
    let field_printed = format!("{:?}", state.credential.as_ref().expect("open").field);
    assert!(!field_printed.contains("hunter2"), "{field_printed}");
    assert!(
        !field_printed.chars().any(char::is_numeric),
        "a digit here is the typed length: {field_printed}"
    );
    // The same print whatever was typed, so there is nothing to difference.
    let mut longer = asking("Password for 'http://u@h':");
    type_in(&mut longer, "a-much-longer-passphrase");
    assert_eq!(
        format!("{:?}", longer.credential.as_ref().expect("open").field),
        field_printed
    );
    assert!(
        !format!("{longer:?}").contains("a-much-longer-passphrase"),
        "the typed passphrase reached a Debug print"
    );

    // The view built for drawing — `FieldView`'s `Debug` redacts unless
    // `secret` is `None`, but this field carries no real text either way
    // (`credential_view` never puts one there for a secret field).
    let view = crate::view::credential_view(&state).expect("prompt is open");
    let view_printed = format!("{:?}", view.field);
    assert!(!view_printed.contains("hunter2"), "{view_printed}");

    // The command carrying the answer away is no different.
    let (_, commands) = press(&state, KeyPress::plain(KeyCode::Enter));
    let printed = format!("{commands:?}");
    assert!(!printed.contains("hunter2"), "{printed}");
}

#[test]
fn a_second_ask_replaces_the_first_and_a_stale_one_is_ignored() {
    let mut state = asking("Username for 'https://github.com':");
    type_in(&mut state, "ali");
    assert!(
        !state.credential.as_ref().expect("open").masked(),
        "a username is typed in the open"
    );

    // git asks for the password next, under the same job.
    credential_needed(
        &mut state,
        JOB,
        "Password for 'https://ali@github.com':",
        false,
    );
    let prompt = state.credential.as_ref().expect("open");
    assert!(prompt.masked(), "the second ask is a password");
    assert_eq!(typed_len(&state), 0, "the username is not left in it");

    // An ask from a job the screen is not waiting on opens nothing.
    let mut other = AppState {
        operating: Some(JobId::new(9)),
        ..AppState::default()
    };
    credential_needed(&mut other, JOB, "Password for 'https://x':", false);
    assert!(other.credential.is_none());

    // Nor does one for a write the user has already asked to stop.
    let mut stopping = AppState {
        operating: Some(JOB),
        ..AppState::default()
    };
    stopping.remote.cancelling = Some(JOB);
    credential_needed(&mut stopping, JOB, "Password for 'https://x':", false);
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
