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
use graft::update::{AppEvent, AppState, Size, Status, update};
use graft_core::event::{Event, KeyCode, KeyPress, Modifiers};
use graft_core::on_disk::CURRENT_DIR;

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
        // The opening list (and the id its history walk runs under) is
        // `update::opening`'s, so this harness cannot drift from the real
        // loop (m5-history-and-ghost-preview/05).
        let (state, commands) = graft::update::opening(repo.to_path_buf(), SIZE);
        for command in commands {
            jobs.dispatch(command);
        }
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
    fn pump(&self, state: AppState, what: &str, done: impl Fn(&AppState) -> bool) -> AppState {
        crate::app_harness::pump_events(
            |patience| self.events.recv_timeout(patience),
            |command| self.jobs.dispatch(command),
            state,
            what,
            done,
            crate::app_harness::DEADLINE,
        )
        .unwrap_or_else(|why| panic!("{why}"))
    }

    /// The editor is open and the write has finished, whichever way it went.
    fn settled(&self, state: AppState) -> AppState {
        self.pump(state, "the write to finish", |state| {
            state.operating.is_none()
        })
    }

    fn close(self) {
        let shutdown =
            self.jobs
                .shutdown(Duration::from_secs(10), graft::jobs::EXIT_WRITE_WAIT, &());
        assert_eq!(
            shutdown,
            Shutdown::CLEAN,
            "the job pool did not stop cleanly"
        );
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

/// `Tab` until the subject has focus, then `Enter` — the way ADR-0034 says
/// a commit is made (ease-pass/08: `⏎` from the subject), rather than the
/// `Ctrl+Enter` shortcut a terminal may not report.
fn tab_to_the_subject_and_enter(app: &App, state: AppState) -> AppState {
    let mut state = state;
    for _ in 0..3 {
        if state.commit_editor.as_ref().map(|editor| editor.focus)
            == Some(graft_ui::commit_editor::CommitFocus::Subject)
        {
            return app.key(&state, KeyCode::Enter);
        }
        state = app.key(&state, KeyCode::Tab);
    }
    panic!("`Tab` never reached the subject");
}

/// `c`, a subject, `⏎` from the subject — and the commit git made.
fn make_the_commit(app: &App, repo: &Path, state: &AppState) -> AppState {
    let before = git(repo, &["rev-list", "--count", "HEAD"]);
    let state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    let state = app.typed(&state, "the typed subject");
    let state = tab_to_the_subject_and_enter(app, state);
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
    assert!(state.drafts.commit.is_none(), "and the draft goes with it");
    assert!(
        matches!(state.status, Status::Done { .. }),
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
    let journal = repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl"));
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
fn enter_commits_what_was_typed_and_then_amends_it() {
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
    let state = tab_to_the_subject_and_enter(&app, state);
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
/// with nothing going in neither should let a commit through, and the bar
/// says why (C9: `put a file in first`).
#[test]
fn nothing_staged_makes_no_commit() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let before = git(repo, &["rev-parse", "HEAD"]);

    let state = app.key(&state, KeyCode::Char('c'));
    let state = app.typed(&state, "nothing to say");
    let refused = graft::view::commit_screen_view(&state)
        .expect("the editor is open")
        .refusal;
    assert_eq!(refused, Some(graft_ui::commit_editor::Refusal::NoFile));

    // `Ctrl+Enter` too, in case the focus is what was refused.
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

/// A repository of the run's own with one file staged on a root commit — not
/// the corpus copy `staged_repo` builds on. The corpus commits are dated 2021
/// and the app's commit is dated now, and no fixed clock reads both the same
/// forever; here every commit is made now, so every age reads `now` under
/// the wall clock.
fn fresh_staged_repo() -> tempfile::TempDir {
    let scratch = tempfile::TempDir::with_prefix("graft-subject-enter-")
        .expect("a temp directory for the repository");
    let repo = scratch.path();
    git(
        repo,
        &["init", "-q", "-b", "main", "--object-format=sha1", "."],
    );
    git(repo, &["config", "user.name", "Graft Test"]);
    git(repo, &["config", "user.email", "test@example.invalid"]);
    std::fs::write(repo.join("sample.txt"), "one\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    commit(repo, "add sample");
    std::fs::write(repo.join("sample.txt"), "one\ntwo\n").expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    scratch
}

/// `c`, then its counts and the first file's changes in: a frame drawn
/// after never says `reading…`.
fn opened(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('c'));
    app.pump(state, "the counts and the diff", |state| {
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.counts.is_some() && editor.diff.view.is_some())
            && state.commit_template.is_none()
    })
}

/// The caption of [`capture_fix_subject_enter_commits`]'s files.
fn subject_enter_header(columns: u16, rows: u16) -> String {
    format!(
        "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_fix_subject_enter_commits in
# crates/graft/tests/it/commit_editor_writes.rs (asserted; the commit is
# made by real git; its id and its parent's, both made by the run and new
# every run, are zeroed). One file staged, `c`, `fix: x` typed, then `⏎`
# with the subject focused (m6-conflicts/51).
#
# Judge by eye:
# - Frame 1: the commit screen; header from `◆ api  ~/code/api`, bar from
#   `⏎ commit 1 file  ⇥ more detail`; body empty, `sample.txt` going in.
# - Frame 2: the screen gone; second-last row the done line `✓ Committed
#   1 file to main.  u undo · P push`, last row the bar `u undo the commit,
#   your edits come back · P push · c commit again`; the graph's top row
#   `[◆ main] fix: x`, with nothing uncommitted.
# - Colour is unverified (CLAUDE.md rule 5).
#
=== {columns}x{rows} ===
"
    )
}

/// The write's answer closes the editor; the graph and the changes pane are
/// re-read after it. Waits until a frame shows the commit and nothing is
/// still loading.
fn graph_shows_the_commit(app: &App, state: AppState) -> AppState {
    app.pump(state, "the graph to show the commit", |state| {
        let rows = crate::captures::frame(state, 120, 40);
        rows.iter().any(|row| row.contains("[◆ main] fix: x"))
            && !rows.iter().any(|row| {
                row.contains("Uncommitted changes")
                    || row.contains("reading\u{2026}")
                    || row.contains("sorting\u{2026}")
            })
    })
}

/// `⏎` in the subject makes the commit over real git (m6-conflicts/51, P16):
/// `c`, a subject, `⏎` — no `⇥ ⇥` to reach the button first — and the
/// capture of the frames either side of it.
///
/// Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo nextest run -E
/// 'test(capture_fix_subject_enter_commits)'`.
#[test]
fn capture_fix_subject_enter_commits() {
    let scratch = fresh_staged_repo();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let parent = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();

    let state = opened(&app, &state);
    let typed = app.typed(&state, "fix: x");
    assert_eq!(
        typed.commit_editor.as_ref().map(|editor| editor.focus),
        Some(graft_ui::commit_editor::CommitFocus::Subject),
        "the editor opens on the subject"
    );
    let entered = app.key(&typed, KeyCode::Enter);
    assert!(entered.operating.is_some(), "⏎ started the commit");
    let landed = app.settled(entered);

    assert_eq!(git(repo, &["log", "-1", "--format=%B"]), "fix: x\n\n");
    assert_eq!(git(repo, &["rev-parse", "HEAD~1"]).trim(), parent);
    assert!(landed.commit_editor.is_none(), "the editor closed");
    assert!(
        matches!(landed.status, Status::Done { .. }),
        "{:?}",
        landed.status
    );
    let landed = graph_shows_the_commit(&app, landed);
    // Both ids are the run's own: the root commit and this one are made now.
    let made = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();

    let mut captures = graft_fixtures::Captures::new();
    for (columns, rows) in [(120_u16, 40_u16), (80, 24)] {
        let first = crate::captures::frame_at(&typed, columns, rows, None);
        assert!(
            first
                .iter()
                .any(|row| row.contains("⏎ commit 1 file  ⇥ more detail")),
            "the bar at {columns}x{rows}: {first:#?}"
        );
        let second = crate::captures::frame_at(&landed, columns, rows, None);
        let body = format!(
            "{}== frame 1: `fix: x` typed, subject focused ==\n{}\n\n\
             == frame 2: after `⏎` ==\n{}\n",
            subject_enter_header(columns, rows),
            first.join("\n"),
            second.join("\n"),
        );
        captures.check(
            format!(
                "{}/../../docs/shots/m6/fix-subject-enter-commits-{columns}x{rows}.txt",
                env!("CARGO_MANIFEST_DIR")
            ),
            crate::captures::without_invented_id(
                &crate::captures::without_invented_id(&body, &made),
                &parent,
            ),
        );
    }
    app.close();
    captures.finish();
}

/// Waits for the status re-read after a stage: the user presses `c` once the
/// file shows as staged, and the editor's refusal is drawn from that status.
fn status_shows_it_staged(app: &App, state: AppState, repo: &Path) -> AppState {
    let state = app.settled(state);
    assert_eq!(
        git(repo, &["diff", "--cached", "--name-only"]).trim(),
        "sample.txt",
        "space staged the file; status {:?}",
        state.status
    );
    app.pump(state, "the status re-read after the stage", |state| {
        state.working_tree.as_ref().is_some_and(|status| {
            status
                .entries
                .iter()
                .any(|entry| entry.staged.is_some() && entry.unstaged.is_none())
        })
    })
}

/// The journey "stage one file and commit" (design note
/// `m6-usability-fixes.md`, Journeys), walked over real git and counted:
/// from the uncommitted row, `⏎ j space c <msg> ⏎`. The typed message is
/// one step, as the note counts it. P16 set the ceiling at 6 (it was 9 with
/// `⇥ ⇥ ⇥` to the button).
#[test]
fn stage_one_file_and_commit_is_six_steps() {
    let scratch = staged_repo();
    let repo = scratch.path();
    // `staged_repo` staged its edit; take it back to the worktree so the
    // journey stages it.
    git(repo, &["reset", "-q", "--", "sample.txt"]);
    let before = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();

    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    // Where the journey starts: the uncommitted row (`k` from the top row).
    let state = app.key(&state, KeyCode::Char('k'));
    assert!(
        state.on_working_tree,
        "the journey starts on the uncommitted row"
    );

    // Every step goes through `step`, which counts it; waiting between
    // steps is the app's, not the user's, and is not counted.
    let steps = std::cell::Cell::new(0_u32);
    let step = |state: &AppState, code: KeyCode| {
        steps.set(steps.get() + 1);
        app.key(state, code)
    };
    let state = step(&state, KeyCode::Enter);
    let state = app.pump(state, "the changes pane's file list", |state| {
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    });
    let state = step(&state, KeyCode::Char('j'));
    let state = step(&state, KeyCode::Char(' '));
    let state = status_shows_it_staged(&app, state, repo);
    let state = step(&state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the editor");
    // The message: one step, as the note counts it.
    let state = app.typed(&state, "fix: x");
    steps.set(steps.get() + 1);
    let state = step(&state, KeyCode::Enter);
    assert!(
        state.operating.is_some(),
        "⏎ started no write; refusal {:?}",
        graft::view::commit_screen_view(&state).and_then(|view| view.refusal),
    );
    let state = app.settled(state);

    assert_eq!(git(repo, &["log", "-1", "--format=%s"]).trim(), "fix: x");
    assert_eq!(git(repo, &["rev-parse", "HEAD~1"]).trim(), before);
    assert!(state.commit_editor.is_none(), "the editor closed");
    let steps = steps.get();
    assert!(
        steps <= 6,
        "the journey took {steps} steps; the note allows 6"
    );
    assert_eq!(steps, 6, "the count recorded in the note");
    app.close();
}
