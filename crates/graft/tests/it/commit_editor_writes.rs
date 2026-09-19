//! The commit editor's writes, over the real job pool and real `git`
//! (m3-working-tree-undo-and-stash/05).
//!
//! Everything else about the editor is pure: which `Operation` a key builds,
//! what the refusal says. That is worth nothing if the commit git actually
//! makes is a different one, so this drives the whole path — `c`, typing,
//! `Tab`, `Enter` → `Command::Operate` → `jobs::operate` → `graft_ops::apply`
//! → `git` — and then asks git itself what is in the repository.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;
use std::process::Command as Shell;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::repos::RepoStore;
use graft::update::{AppEvent, AppState, Command, Size, Status, update};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_git::history::Order;

/// Long enough that a slow machine never fails here, short enough that a
/// hang is a named failure rather than a stuck suite.
const PATIENCE: Duration = Duration::from_secs(120);

const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// Commit as a named author, so the fixture does not depend on whatever
/// `user.name` the machine running the suite happens to have.
fn commit(repo: &Path, message: &str) {
    git(
        repo,
        &[
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            message,
        ],
    );
}

/// A scratch repository with one commit and one file staged on top of it.
fn staged_repo() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), "one\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    commit(repo, "add sample");
    // Graft's own commit needs an identity too, and the write does not go
    // through a shell where `-c` could be passed.
    git(repo, &["config", "user.name", "Graft Test"]);
    git(repo, &["config", "user.email", "test@example.invalid"]);

    std::fs::write(repo.join("sample.txt"), "one\ntwo\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    scratch
}

struct App {
    jobs: JobRunner<Sender<AppEvent>>,
    events: Receiver<AppEvent>,
}

impl App {
    fn open(repo: &Path) -> (Self, AppState) {
        let (sender, events) = mpsc::channel();
        let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
        let jobs = JobRunner::start(sender, 2, store).expect("the job pool starts");
        for command in [
            Command::OpenRepository,
            Command::CheckSystemGit,
            Command::ReadHistory(Order::Topological),
            Command::ReadRefs,
            Command::ReadWorkingTree,
        ] {
            jobs.dispatch(command);
        }
        let state = AppState {
            repo_path: repo.to_path_buf(),
            size: SIZE,
            ..AppState::default()
        };
        (Self { jobs, events }, state)
    }

    fn key(&self, state: &AppState, code: KeyCode) -> AppState {
        self.press(state, KeyPress::plain(code))
    }

    fn press(&self, state: &AppState, key: KeyPress) -> AppState {
        let (next, commands) = update(state, AppEvent::Core(Event::Key(key)));
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    fn typed(&self, state: &AppState, text: &str) -> AppState {
        let mut state = state.clone();
        for character in text.chars() {
            state = self.key(&state, KeyCode::Char(character));
        }
        state
    }

    /// Feeds job answers through `update` until `done` holds.
    fn pump(&self, mut state: AppState, what: &str, done: impl Fn(&AppState) -> bool) -> AppState {
        loop {
            if done(&state) {
                return state;
            }
            let event = self
                .events
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|why| panic!("waiting for {what}: {why}"));
            let (next, commands) = update(&state, event);
            state = next;
            for command in commands {
                self.jobs.dispatch(command);
            }
        }
    }

    /// The editor is open and the write has finished, whichever way it went.
    fn settled(&self, state: AppState) -> AppState {
        self.pump(state, "the write to finish", |state| {
            state.operating.is_none()
        })
    }

    fn close(self) {
        match self.jobs.shutdown(Duration::from_secs(10)) {
            Shutdown::Clean => {}
            other @ Shutdown::Stalled { .. } => panic!("the job pool did not stop: {other:?}"),
        }
    }
}

/// Open, and wait until the first rows and the working-tree status are in.
fn ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the status", |state| {
        state.working_tree.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// `refs/heads/main`'s id under `field` in the journal's JSONL. Read by hand
/// rather than with a JSON parser: this asserts one fact about one line, and
/// a dependency for that would outlive its use.
fn ref_in(entries: &str, field: &str) -> String {
    let at = entries
        .find(&format!(r#""{field}":{{"refs/heads/main":""#))
        .unwrap_or_else(|| panic!("no {field} for main in {entries}"));
    let rest = &entries[at + field.len() + r#""":{"refs/heads/main":""#.len()..];
    rest.chars().take_while(|c| *c != '"').collect()
}

/// `Tab` until `[ Commit ]` has focus, then `Enter` — the way ADR-0034 says
/// a commit is made, rather than the `Ctrl+Enter` shortcut a terminal may
/// not report.
fn press_the_button(app: &App, state: AppState) -> AppState {
    let mut state = state;
    for _ in 0..4 {
        if state.commit_editor.as_ref().map(|editor| editor.focus)
            == Some(graft_ui::commit_editor::Field::CommitButton)
        {
            return app.key(&state, KeyCode::Enter);
        }
        state = app.key(&state, KeyCode::Tab);
    }
    panic!("`Tab` never reached the button");
}

/// `c`, a subject, `Tab` to the button, `Enter` — and the commit git made.
fn make_the_commit(app: &App, repo: &Path, state: &AppState) -> AppState {
    let before = git(repo, &["rev-list", "--count", "HEAD"]);
    let state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    let state = app.typed(&state, "the typed subject");
    let state = press_the_button(app, state);
    let state = app.settled(state);

    assert_eq!(
        git(repo, &["log", "-1", "--format=%s"]).trim(),
        "the typed subject",
        "status: {:?}",
        state.status,
    );
    assert!(
        state.commit_editor.is_none(),
        "the editor closes on a commit that landed",
    );
    assert!(state.commit_draft.is_none(), "and the draft goes with it");
    assert!(
        matches!(state.status, Status::Done(_)),
        "{:?}",
        state.status,
    );

    assert_eq!(
        git(repo, &["rev-list", "--count", "HEAD"])
            .trim()
            .parse::<u32>()
            .expect("a count"),
        before.trim().parse::<u32>().expect("a count") + 1,
    );

    // The journal is `graft_ops`', and the undo group's business; what
    // matters here is that the write went through it, and that the entry
    // records the branch actually moving — which is what an undo will read.
    let journal = repo.join(".git/graft/undo-journal.jsonl");
    let entries = std::fs::read_to_string(&journal).expect("the commit was journalled");
    assert!(entries.contains(r#""kind":"commit""#), "{entries}");
    assert!(entries.contains(r#""status":"committed""#), "{entries}");
    let before_main = ref_in(&entries, "refs_before");
    let after_main = ref_in(&entries, "refs_after");
    assert_ne!(before_main, after_main, "main moved: {entries}");

    state
}

/// The whole path, twice: a commit, and then an amend of it.
///
/// One test rather than two because the amend needs a commit of Graft's own
/// to amend — splitting them would mean building the first commit twice.
#[test]
fn the_button_commits_what_was_typed_and_then_amends_it() {
    let scratch = staged_repo();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let state = make_the_commit(&app, repo, &state);
    let commits = git(repo, &["rev-list", "--count", "HEAD"]);

    // `a` asks for `HEAD`'s message first and the editor opens on the answer.
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.pump(state, "HEAD's message", |state| {
        state.commit_editor.is_some()
    });
    {
        let editor = state.commit_editor.as_ref().expect("the amend editor");
        assert!(editor.amend);
        assert_eq!(editor.subject.text(), "the typed subject");
    }

    // Clear the prefilled subject and type a new one.
    let mut state = state;
    for _ in 0.."the typed subject".len() {
        state = app.key(&state, KeyCode::Backspace);
    }
    let state = app.typed(&state, "the reworded subject");
    let state = press_the_button(&app, state);
    let state = app.settled(state);

    assert_eq!(
        git(repo, &["log", "-1", "--format=%s"]).trim(),
        "the reworded subject",
        "status: {:?}",
        state.status,
    );
    assert_eq!(
        git(repo, &["rev-list", "--count", "HEAD"]).trim(),
        commits.trim(),
        "an amend replaces the commit, it does not add one",
    );
    app.close();
}

/// The refusal is a preview, and `Operation::check` is the decision — but
/// with nothing staged neither should let a commit through, and the user
/// should be told which it is.
#[test]
fn nothing_staged_makes_no_commit() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let before = git(repo, &["rev-parse", "HEAD"]);

    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.typed(&state, "nothing to say");
    let refused = graft::view::commit_editor_view(&state)
        .expect("the editor is open")
        .refusal
        .map(|reason| reason.as_str().to_owned());
    assert_eq!(refused.as_deref(), Some("nothing staged"));

    // `Ctrl+Enter` too, in case the button's focus is what was refused.
    let state = app.press(
        &state,
        KeyPress {
            code: KeyCode::Enter,
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        },
    );
    assert!(state.operating.is_none(), "no write was started");
    assert_eq!(git(repo, &["rev-parse", "HEAD"]), before);
    assert!(state.commit_editor.is_some(), "the editor stays open");
    app.close();
}
