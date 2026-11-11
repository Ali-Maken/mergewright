//! The changes pane's writes, over the real job pool and real `git`
//! (m3-working-tree-undo-and-stash/04).
//!
//! Every other test of this work is pure: it checks which `Operation`
//! `update` builds. That is worth nothing if the operation git actually runs
//! does something else, so these two drive the whole path — key → `update` →
//! `Command::Operate` → `jobs::operate` → `mergewright_ops::apply` → `git` — and
//! then ask git itself what happened.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;
use std::process::Command as Shell;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use mergewright::jobs::{Dispatch, JobRunner, Shutdown};
use mergewright::repos::RepoStore;
use mergewright::update::{AppEvent, AppState, Size, Status, update};
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_core::on_disk::DIR;

const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};

/// A tracked file whose two changes are far enough apart to be two hunks at
/// git's default three lines of context.
const BASE: &str = "one\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nsixteen\n";
const EDITED: &str = "ONE\ntwo\nthree\nfour\nfive\nsix\nseven\neight\nnine\nten\n\
eleven\ntwelve\nthirteen\nfourteen\nfifteen\nSIXTEEN\n";

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

/// A scratch repository with one commit, one tracked file edited in two
/// places, and one untracked file.
fn dirty_repo() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::write(repo.join("sample.txt"), BASE).expect("the scratch repo is writable");
    git(repo, &["add", "sample.txt"]);
    git(
        repo,
        &[
            "-c",
            "user.name=Test Maker",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "add sample",
        ],
    );
    std::fs::write(repo.join("sample.txt"), EDITED).expect("the scratch repo is writable");
    std::fs::write(repo.join("scratch.txt"), "throwaway\n").expect("the scratch repo is writable");
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
        let (state, commands) = mergewright::update::opening(repo.to_path_buf(), SIZE);
        for command in commands {
            jobs.dispatch(command);
        }
        (Self { jobs, events }, state)
    }

    fn press(&self, state: &AppState, key: KeyPress) -> AppState {
        let (next, commands) = update(state, AppEvent::Core(Event::Key(key)));
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    fn key(&self, state: &AppState, code: KeyCode) -> AppState {
        self.press(state, KeyPress::plain(code))
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

    fn close(self) {
        let shutdown = self.jobs.shutdown(
            Duration::from_secs(10),
            mergewright::jobs::EXIT_WRITE_WAIT,
            &(),
        );
        assert_eq!(
            shutdown,
            Shutdown::CLEAN,
            "the job pool did not stop cleanly"
        );
    }
}

/// Open, then put the pane on the working tree and wait for its file list.
fn changes_pane(app: &App, state: AppState) -> AppState {
    let state = app.pump(
        state,
        "the first rows and the working-tree status",
        |state| {
            state.working_tree.is_some()
                && state
                    .history
                    .as_ref()
                    .and_then(|history| history.window.as_ref())
                    .is_some_and(|window| !window.rows.is_empty())
        },
    );
    // `k` from the top row selects the uncommitted-changes row above it.
    let state = app.key(&state, KeyCode::Char('k'));
    assert!(
        state.on_working_tree,
        "`k` from the top row selects the uncommitted-changes row"
    );
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the changes pane's file list", |state| {
        matches!(
            state.detail.files,
            mergewright::update::detail::Loading::Ready(_)
        )
    })
}

/// The pane's drawn file rows, as text.
fn rows(state: &AppState) -> Vec<String> {
    match &state.detail.files {
        mergewright::update::detail::Loading::Ready(files) => files
            .rows
            .iter()
            .map(|row| row.name.as_str().to_owned())
            .collect(),
        other => panic!("the file list has not arrived: {other:?}"),
    }
}

/// Presses `j` until the cursor is on the row named `name`.
fn cursor_to(app: &App, state: AppState, name: &str) -> AppState {
    let drawn = rows(&state);
    let want = drawn
        .iter()
        .position(|row| row == name)
        .unwrap_or_else(|| panic!("no row named {name} among {drawn:?}"));
    let mut state = state;
    let mut guard = 0;
    loop {
        let at = match &state.detail.files {
            mergewright::update::detail::Loading::Ready(files) => files.cursor,
            other => panic!("the file list went away: {other:?}"),
        };
        if at == want {
            return state;
        }
        assert!(guard < drawn.len() * 2, "the cursor never reached {name}");
        guard += 1;
        let step = if at < want { 'j' } else { 'k' };
        state = app.key(&state, KeyCode::Char(step));
    }
}

/// Runs the pending write to its answer.
fn settle_write(app: &App, state: AppState) -> AppState {
    assert!(
        state.operating.is_some(),
        "a write was queued; status: {:?}",
        state.status
    );
    let state = app.pump(state, "the write to finish", |state| {
        state.operating.is_none()
    });
    match &state.status {
        Status::Done { .. } => state,
        other => panic!("the write did not succeed: {other:?}"),
    }
}

/// `Space` on a hunk heading in the diff region stages that hunk and leaves
/// the rest of the file alone — the milestone's first checkpoint test.
#[test]
fn space_on_a_hunk_stages_only_that_hunk() {
    let scratch = dirty_repo();
    let repo = scratch.path();
    let (app, state) = App::open(repo);

    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "sample.txt");
    // `Enter` in the file list moves focus to the diff region.
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the file's diff", |state| {
        matches!(
            state.detail.diff,
            mergewright::update::detail::Loading::Ready(_)
        ) && state.detail.diff_raw.is_some()
    });
    // The line cursor opens on body row 0: the first hunk's `@@` heading.
    let state = app.key(&state, KeyCode::Char(' '));
    let state = settle_write(&app, state);
    drop(state);
    app.close();

    let staged = git(repo, &["diff", "--cached", "-U0"]);
    assert!(
        staged.contains("+ONE"),
        "the first hunk is in the index: {staged}"
    );
    assert!(
        !staged.contains("+SIXTEEN"),
        "the second hunk is not: {staged}"
    );
    let left = git(repo, &["diff", "-U0"]);
    assert!(
        left.contains("+SIXTEEN") && !left.contains("+ONE"),
        "the second hunk is still only in the working tree: {left}"
    );
}

/// `d` on an untracked file asks first, and the answer under the cursor is
/// the safe one: `Enter` straight away keeps the file. Moving to `Discard`
/// and confirming deletes it — and leaves a backup and a journal entry
/// behind, which is what makes it undoable.
#[test]
fn discarding_an_untracked_file_asks_first_and_leaves_a_way_back() {
    let scratch = dirty_repo();
    let repo = scratch.path();
    let (app, state) = App::open(repo);

    let state = changes_pane(&app, state);
    let state = cursor_to(&app, state, "scratch.txt");

    // The safe choice is under the cursor: `Enter` now changes nothing.
    let state = app.key(&state, KeyCode::Char('d'));
    let pending = state.confirm.as_ref().expect("the confirm box opened");
    assert_eq!(pending.view.command.as_str(), "git clean -f -- scratch.txt");
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.confirm.is_none(), "the box closed");
    assert!(state.operating.is_none(), "nothing was written");
    assert!(repo.join("scratch.txt").exists(), "the file is still there");

    // `→` then `Enter` is the deliberate second step that deletes it.
    let state = app.key(&state, KeyCode::Char('d'));
    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    let state = settle_write(&app, state);
    drop(state);
    app.close();

    assert!(
        !repo.join("scratch.txt").exists(),
        "the untracked file is gone"
    );
    let backups = git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname)",
            &format!("refs/{DIR}/backup"),
        ],
    );
    assert_eq!(
        backups.lines().count(),
        1,
        "one backup ref holds what was discarded: {backups:?}"
    );
    let journal = std::fs::read_to_string(repo.join(format!(".git/{DIR}/undo-journal.jsonl")))
        .expect("the journal was written");
    assert_eq!(
        journal.lines().count(),
        2,
        "the entry was opened and then finished: {journal}"
    );
}
