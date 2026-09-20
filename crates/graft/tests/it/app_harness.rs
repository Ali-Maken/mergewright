//! Driving the whole application over the real job pool and real `git`.
//!
//! `changes_pane_writes` and `commit_editor_writes` each grew their own copy
//! of this before there were three of them; this is the shared one the newer
//! tests use (m3-working-tree-undo-and-stash/06).
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
use graft_core::event::{Event, KeyCode, KeyPress};

/// Long enough that a slow machine never fails here, short enough that a
/// hang is a named failure rather than a stuck suite.
pub const PATIENCE: Duration = Duration::from_secs(120);

pub const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};

/// Run `git` in `repo` and return its stdout, panicking with git's own
/// stderr when it fails.
pub fn git(repo: &Path, args: &[&str]) -> String {
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

/// `git` as an identity the machine's own config cannot change.
pub fn git_as_graft(repo: &Path, args: &[&str]) -> String {
    let mut full = vec![
        "-c",
        "user.name=Graft Test",
        "-c",
        "user.email=test@example.invalid",
    ];
    full.extend_from_slice(args);
    git(repo, &full)
}

pub struct App {
    pub jobs: JobRunner<Sender<AppEvent>>,
    pub events: Receiver<AppEvent>,
}

impl App {
    /// Open a repository the way the real loop does, and hand back the
    /// starting state.
    pub fn open(repo: &Path) -> (Self, AppState) {
        Self::open_with(repo, None)
    }

    /// Open with `graft` itself as git's askpass helper (m4-remotes/06).
    ///
    /// The runner defaults to `current_exe`, which under `cargo test` is the
    /// test binary and has no askpass mode; a test that means to answer a
    /// credential question names `CARGO_BIN_EXE_graft` here.
    pub fn open_with_askpass(repo: &Path, helper: std::path::PathBuf) -> (Self, AppState) {
        Self::open_with(repo, Some(helper))
    }

    /// Open over a pool of exactly `workers` threads, for a test whose point
    /// is what happens when they run out (m5-history-and-ghost-preview/05).
    pub fn open_with_workers(repo: &Path, workers: usize) -> (Self, AppState) {
        Self::open_sized(repo, None, workers)
    }

    fn open_with(repo: &Path, askpass_helper: Option<std::path::PathBuf>) -> (Self, AppState) {
        Self::open_sized(repo, askpass_helper, 2)
    }

    fn open_sized(
        repo: &Path,
        askpass_helper: Option<std::path::PathBuf>,
        workers: usize,
    ) -> (Self, AppState) {
        let (sender, events) = mpsc::channel();
        let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
        let jobs = JobRunner::start(sender, workers, store).expect("the job pool starts");
        let jobs = match askpass_helper {
            Some(helper) => jobs.with_askpass_helper(helper),
            None => jobs,
        };
        // The opening list (and the id its history walk runs under) is
        // `update::opening`'s, so this harness cannot drift from the real
        // loop (m5-history-and-ghost-preview/05).
        let (state, commands) = graft::update::opening(repo.to_path_buf(), SIZE);
        for command in commands {
            jobs.dispatch(command);
        }
        (Self { jobs, events }, state)
    }

    pub fn press(&self, state: &AppState, key: KeyPress) -> AppState {
        let (next, commands) = update(state, AppEvent::Core(Event::Key(key)));
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    pub fn key(&self, state: &AppState, code: KeyCode) -> AppState {
        self.press(state, KeyPress::plain(code))
    }

    /// Feeds job answers through `update` until `done` holds.
    pub fn pump(
        &self,
        mut state: AppState,
        what: &str,
        done: impl Fn(&AppState) -> bool,
    ) -> AppState {
        loop {
            if done(&state) {
                return state;
            }
            let event = self
                .events
                .recv_timeout(PATIENCE)
                .unwrap_or_else(|why| panic!("waiting for {what}: {why}"));
            if std::env::var_os("GRAFT_TEST_TRACE").is_some() {
                eprintln!("[{what}] {event:?} -> operating {:?}", state.operating);
            }
            let (next, commands) = update(&state, event);
            state = next;
            for command in commands {
                self.jobs.dispatch(command);
            }
        }
    }

    /// Runs the pending write — a stage, a commit, an undo — to its answer,
    /// and insists it succeeded.
    pub fn settle_write(&self, state: AppState, what: &str) -> AppState {
        assert!(
            state.operating.is_some(),
            "{what}: a write was queued; status: {:?}",
            state.status
        );
        let state = self.pump(state, what, |state| state.operating.is_none());
        match &state.status {
            Status::Done(_) => state,
            other => panic!(
                "{what} did not succeed: {other:?}; confirm box: {:?}",
                state.confirm.as_ref().map(|pending| &pending.view)
            ),
        }
    }

    pub fn close(self) {
        match self.jobs.shutdown(Duration::from_secs(10)) {
            Shutdown::Clean => {}
            other @ Shutdown::Stalled { .. } => panic!("the job pool did not stop: {other:?}"),
        }
    }
}

/// Open, then put the detail pane on the working tree and wait for its file
/// list — the changes pane, as the user reaches it.
pub fn changes_pane(app: &App, state: AppState) -> AppState {
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
        matches!(state.detail.files, graft::update::detail::Loading::Ready(_))
    })
}

/// The pane's drawn file rows, as text.
pub fn rows(state: &AppState) -> Vec<String> {
    match &state.detail.files {
        graft::update::detail::Loading::Ready(files) => files
            .rows
            .iter()
            .map(|row| row.name.as_str().to_owned())
            .collect(),
        other => panic!("the file list has not arrived: {other:?}"),
    }
}

/// Presses `j`/`k` until the cursor is on the row named `name`.
pub fn cursor_to(app: &App, state: AppState, name: &str) -> AppState {
    let drawn = rows(&state);
    let want = drawn
        .iter()
        .position(|row| row == name)
        .unwrap_or_else(|| panic!("no row named {name} among {drawn:?}"));
    let mut state = state;
    let mut guard = 0;
    loop {
        let at = match &state.detail.files {
            graft::update::detail::Loading::Ready(files) => files.cursor,
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
