//! Answering git's credential question by key, over real `git`, the real
//! job pool and a real HTTP remote that checks the password (m4-remotes/06).
//!
//! The promise this proves: a wrong password is a line you can act on, not a
//! hang and not a trip to the shell — and the password itself never leaves
//! the field. Every test records **every** event the loop saw and every
//! status line it drew, and asserts the typed bytes are in none of them, nor
//! in the undo journal on disk.
//!
//! `graft`'s own `git` keeps the user's config, so the work clone blanks
//! `credential.helper`: a keychain helper must neither answer for the test
//! nor store its password.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use graft::jobs::Dispatch;
use graft::update::detail::Loading;
use graft::update::{AppState, Status, update};
use graft_core::event::{KeyCode, KeyPress};
use graft_core::journal::{EntryStatus, JournalEntry};
use graft_fixtures::Shape;
use graft_fixtures::remote::{HttpRemote, LocalRemote};

use crate::app_harness::{App, PATIENCE, git};

const USER: &str = "tester";
const PASSWORD: &str = "correct-horse-battery-staple";
const WRONG: &str = "not-the-password";

/// A work clone whose `origin` is an HTTP remote that checks the password.
struct Served {
    local: LocalRemote,
    http: HttpRemote,
}

impl Served {
    /// `anonymous`: an URL with no user in it, so git asks for the username
    /// first and the password second.
    fn new(anonymous: bool) -> Served {
        let local =
            LocalRemote::new(Shape::Linear).expect("the Linear shape clones into a local remote");
        let http = HttpRemote::serve(local.bare(), USER, PASSWORD).expect("the HTTP remote serves");
        let url = if anonymous {
            http.url_anonymous()
        } else {
            http.url()
        };
        let setup: [&[&str]; 5] = [
            &["remote", "set-url", "origin", &url],
            // Only the test's own answers count.
            &["config", "credential.helper", ""],
            &["config", "user.name", "Graft Test"],
            &["config", "user.email", "test@example.invalid"],
            // A global `core.hooksPath` must not reach this clone.
            &["config", "core.hooksPath", ".git/hooks"],
        ];
        for args in setup {
            local
                .git(local.work(), args)
                .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
        }
        Served { local, http }
    }

    fn work(&self) -> &Path {
        self.local.work()
    }

    fn main_ref(&self) -> String {
        format!("refs/heads/{}", self.local.default_branch())
    }
}

fn graft_binary() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_trib"))
}

fn rev(repo: &Path, name: &str) -> String {
    git(repo, &["rev-parse", name]).trim().to_owned()
}

fn journal(repo: &Path) -> Vec<JournalEntry> {
    graft_ops::journal::Journal::open(&repo.join(".git/tributary/undo-journal.jsonl"))
        .expect("the journal the writes made is readable")
        .entries()
        .to_vec()
}

fn plain(c: char) -> KeyPress {
    KeyPress::plain(KeyCode::Char(c))
}

/// Everything `P` decides from has landed.
fn ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows, refs and remote facts", |state| {
        state.working_tree.is_some()
            && state.refs.is_some()
            && matches!(state.remotes, Loading::Ready(_))
            && state.remote.reading.is_none()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// The push box `P` opened: `→` off `Cancel`, `Enter`.
fn confirm(app: &App, state: &AppState) -> AppState {
    assert!(
        state.confirm.is_some(),
        "P opened no box; status {:?}",
        state.status
    );
    let state = app.key(state, KeyCode::Right);
    app.key(&state, KeyCode::Enter)
}

/// Everything the loop saw, so a test can prove what never appeared in it.
#[derive(Default)]
struct Recorded {
    /// One `{event:?}` per event, the `Progress` texts included.
    events: Vec<String>,
    /// One `{status:?}` per distinct status line drawn.
    statuses: Vec<String>,
    /// git's questions, in the order they were asked.
    prompts: Vec<String>,
}

impl Recorded {
    fn assert_never_mentions(&self, secret: &str, what: &str) {
        for line in self
            .events
            .iter()
            .chain(&self.statuses)
            .chain(&self.prompts)
        {
            assert!(
                !line.contains(secret),
                "{what} reached the loop's own record: {line}"
            );
        }
    }
}

/// Runs the app until `done`, answering each credential question with the
/// next line of `script` — typed character by character, then `Enter`.
///
/// Panics rather than hangs when git asks more often than the test scripted
/// an answer for: an unanswered prompt would wait forever.
fn drive(
    app: &App,
    mut state: AppState,
    what: &str,
    script: &mut VecDeque<&str>,
    seen: &mut Recorded,
    done: impl Fn(&AppState) -> bool,
) -> AppState {
    let mut deadline = crate::deadline::Deadline::of(crate::deadline::DEADLINE);
    loop {
        if done(&state) {
            return state;
        }
        if let Some(prompt) = state.credential.as_ref() {
            seen.prompts.push(prompt.prompt.as_str().to_owned());
            let answer = script
                .pop_front()
                .unwrap_or_else(|| panic!("{what}: git asked again: {}", prompt.prompt.as_str()));
            for character in answer.chars() {
                state = app.press(&state, plain(character));
            }
            state = app.key(&state, KeyCode::Enter);
            assert!(state.credential.is_none(), "Enter closed the prompt");
            continue;
        }
        let event = app
            .events
            .recv_timeout(PATIENCE)
            .unwrap_or_else(|why| panic!("waiting for {what}: {why}"));
        if let Err(why) = deadline.note(what, &event) {
            panic!("{why}");
        }
        seen.events.push(format!("{event:?}"));
        let (next, commands) = update(&state, event);
        state = next;
        if seen.statuses.last() != Some(&format!("{:?}", state.status)) {
            seen.statuses.push(format!("{:?}", state.status));
        }
        for command in commands {
            app.jobs.dispatch(command);
        }
    }
}

/// `P`, `Enter`, a wrong password: the line says authentication failed and
/// offers `P` again. Then `P`, `Enter`, the right one: the bare moves. The
/// journal, the status lines and every progress text hold neither password.
#[test]
fn wrong_then_right_password_pushes() {
    let served = Served::new(false);
    let work = served.work();
    let tip = served
        .local
        .commit_locally("over http")
        .expect("a local commit to push");
    assert_ne!(rev(served.local.bare(), &served.main_ref()), tip);

    let (app, state) = App::open_with_askpass(work, graft_binary());
    let state = ready(&app, state);
    let mut seen = Recorded::default();

    let state = push_answering(&app, &state, "the refused push", WRONG, &mut seen);

    match &state.status {
        Status::Sticky { reason, keys } => {
            assert!(
                reason.starts_with("authentication failed"),
                "the line names the failure: {reason}"
            );
            let keys = keys.as_deref().unwrap_or_default();
            assert!(keys.contains("P to try again"), "{keys}");
        }
        other => panic!("a wrong password must leave a sticky line: {other:?}"),
    }
    assert_eq!(
        served.http.requests(),
        (0, served.http.requests().1),
        "nothing was authorised with the wrong password"
    );
    assert!(
        served.http.requests().1 >= 1,
        "the server refused at least one request"
    );
    assert_ne!(
        rev(served.local.bare(), &served.main_ref()),
        tip,
        "the bare did not move"
    );

    // Again, with the password git wants.
    let state = push_answering(&app, &state, "the push", PASSWORD, &mut seen);

    assert!(
        matches!(&state.status, Status::Done { what: line, .. } if line.starts_with("pushed")),
        "{:?}",
        state.status
    );
    assert_eq!(
        rev(served.local.bare(), &served.main_ref()),
        tip,
        "the bare now has the local tip"
    );
    assert!(
        served.http.requests().0 >= 1,
        "the server authorised at least one request"
    );

    seen.assert_never_mentions(PASSWORD, "the password");
    seen.assert_never_mentions(WRONG, "the wrong password");
    assert_not_in_journal(work, &[PASSWORD, WRONG]);
    drop(state);
    app.close();
}

/// `P`, `Enter`, then `answer` typed into git's one question, to the end of
/// the write.
fn push_answering(
    app: &App,
    state: &AppState,
    what: &str,
    answer: &'static str,
    seen: &mut Recorded,
) -> AppState {
    let state = app.press(state, plain('P'));
    let state = confirm(app, &state);
    let mut script = VecDeque::from(vec![answer]);
    let state = drive(app, state, what, &mut script, seen, |state| {
        state.operating.is_none()
    });
    assert!(
        script.is_empty(),
        "{what}: git never asked for the password"
    );
    state
}

/// None of `secrets` is in the undo journal's bytes.
fn assert_not_in_journal(work: &Path, secrets: &[&str]) {
    let journal_bytes = std::fs::read(work.join(".git/tributary/undo-journal.jsonl"))
        .expect("the journal is readable");
    for secret in secrets {
        assert!(
            !journal_bytes
                .windows(secret.len())
                .any(|window| window == secret.as_bytes()),
            "a password reached the undo journal"
        );
    }
}

/// `Esc` in the prompt stops the whole push: nothing half-authenticated,
/// the entry says cancelled, the bare never heard from us.
#[test]
fn esc_in_prompt_cancels_push() {
    let served = Served::new(false);
    let work = served.work();
    let tip = served
        .local
        .commit_locally("over http")
        .expect("a local commit to push");
    let bare_before = rev(served.local.bare(), &served.main_ref());
    assert_ne!(bare_before, tip);

    let (app, state) = App::open_with_askpass(work, graft_binary());
    let state = ready(&app, state);
    let mut seen = Recorded::default();

    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    // No scripted answer: stop at the question and press `Esc` instead.
    let mut nothing = VecDeque::new();
    let state = drive(
        &app,
        state,
        "git's question",
        &mut nothing,
        &mut seen,
        |state| state.credential.is_some(),
    );
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.credential.is_none(), "Esc closed the prompt");
    let state = app.pump(state, "the cancelled push", |state| {
        state.operating.is_none()
    });

    assert_eq!(
        rev(served.local.bare(), &served.main_ref()),
        bare_before,
        "the bare did not move"
    );
    assert_eq!(
        served.http.requests().0,
        0,
        "nothing was authorised: the push never got a password"
    );
    let last = journal(work).pop().expect("the push was journaled");
    assert!(
        matches!(&last.status, EntryStatus::Aborted { reason } if reason.contains("cancelled")),
        "{:?}",
        last.status
    );
    drop(state);
    app.close();
}

/// An URL with no user in it: git asks for the username, then the password.
/// The first ask is readable, the second masked, and both are answered by
/// key.
#[test]
fn username_then_password() {
    let served = Served::new(true);
    let work = served.work();
    let tip = served
        .local
        .commit_locally("over http")
        .expect("a local commit to push");

    let (app, state) = App::open_with_askpass(work, graft_binary());
    let state = ready(&app, state);
    let mut seen = Recorded::default();

    let state = app.press(&state, plain('P'));
    let state = confirm(&app, &state);
    // The first ask, checked before it is answered.
    let mut nothing = VecDeque::new();
    let state = drive(
        &app,
        state,
        "the username ask",
        &mut nothing,
        &mut seen,
        |state| state.credential.is_some(),
    );
    let asked = state.credential.as_ref().expect("the prompt is open");
    assert!(
        asked.prompt.as_str().contains("sername"),
        "git asks for the username first: {}",
        asked.prompt.as_str()
    );
    assert!(!asked.masked(), "a username is typed in the open");

    let mut script = VecDeque::from(vec![USER, PASSWORD]);
    let state = drive(&app, state, "the push", &mut script, &mut seen, |state| {
        state.operating.is_none()
    });

    assert!(script.is_empty(), "both asks were answered");
    assert_eq!(seen.prompts.len(), 2, "two asks: {:?}", seen.prompts);
    assert!(
        seen.prompts[1].to_lowercase().contains("assword"),
        "the second ask is the password: {:?}",
        seen.prompts
    );
    assert!(
        matches!(&state.status, Status::Done { what: line, .. } if line.starts_with("pushed")),
        "{:?}",
        state.status
    );
    assert_eq!(rev(served.local.bare(), &served.main_ref()), tip);
    seen.assert_never_mentions(PASSWORD, "the password");
    drop(state);
    app.close();
}
