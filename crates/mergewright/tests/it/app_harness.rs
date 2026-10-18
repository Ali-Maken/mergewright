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
#[cfg(test)]
use std::time::Instant;

use mergewright::jobs::{Dispatch, JobRunner, Shutdown};
use mergewright::repos::RepoStore;
use mergewright::update::{AppEvent, AppState, Command, Size, Status, update};
use mergewright_core::event::{
    Event, KeyCode, KeyPress, Modifiers, MouseButton, MouseInput, MouseKind,
};

pub use crate::deadline::{DEADLINE, Deadline};

/// Long enough that a slow machine never fails here, short enough that a
/// hang is a named failure rather than a stuck suite.
pub const PATIENCE: Duration = Duration::from_secs(120);

/// Feeds events from `next` through `update`, dispatching each command, until
/// `done` holds — or fails when one event takes longer than [`PATIENCE`] or
/// the whole pump longer than `limit`. The loop `App::pump` runs, apart from
/// its source so a stub stream can prove the deadline.
pub fn pump_events<E: std::fmt::Display>(
    mut next: impl FnMut(Duration) -> Result<AppEvent, E>,
    mut dispatch: impl FnMut(Command),
    mut state: AppState,
    what: &str,
    done: impl Fn(&AppState) -> bool,
    limit: Duration,
) -> Result<AppState, String> {
    let mut deadline = Deadline::of(limit);
    loop {
        if done(&state) {
            return Ok(state);
        }
        let event = next(PATIENCE).map_err(|why| format!("waiting for {what}: {why}"))?;
        if std::env::var_os("MWR_TEST_TRACE").is_some() {
            eprintln!("[{what}] {event:?} -> operating {:?}", state.operating);
        }
        deadline.note(what, &event)?;
        let (next_state, commands) = update(&state, event);
        state = next_state;
        for command in commands {
            dispatch(command);
        }
    }
}

pub const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};

/// A key press with the commands it produced handed back instead of
/// dispatched. `y` needs this: `CopyToClipboard` is terminal output the
/// event loop performs, and `jobs::runner` refuses it with `unreachable!`,
/// so `App::press` cannot carry it.
///
/// Free-standing because it dispatches nothing, and so needs no job pool.
pub fn press_for_commands(state: &AppState, key: KeyPress) -> (AppState, Vec<Command>) {
    update(state, AppEvent::Core(Event::Key(key)))
}

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

/// Give `repo` the identity [`git_as_graft`] uses, in its own config, so
/// what **the app** writes there (a stash, a merge, a ghost's author) is
/// stamped the same on every machine.
///
/// The in-process app runs git with the test process's `HOME`, so without
/// this its commits carry the developer's global identity: captures drew
/// `MA` on the Mac and `TE` in the Linux container, and the ids changed
/// with them (m8-polish-and-ship/30).
pub fn pin_identity(repo: &Path) {
    git(repo, &["config", "user.name", "Graft Test"]);
    git(repo, &["config", "user.email", "test@example.invalid"]);
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
    /// credential question names `CARGO_BIN_EXE_mwr` here.
    pub fn open_with_askpass(repo: &Path, helper: std::path::PathBuf) -> (Self, AppState) {
        Self::open_with(repo, Some(helper))
    }

    /// Open over a pool of exactly `workers` threads, for a test whose point
    /// is what happens when they run out (m5-history-and-ghost-preview/05).
    pub fn open_with_workers(repo: &Path, workers: usize) -> (Self, AppState) {
        Self::open_sized(repo, None, workers, mergewright::session::Clock::Wall)
    }

    /// Open with every journal entry this run writes stamped from `clock`,
    /// and every age the frame draws measured against it (m8/19d-b2).
    ///
    /// For a capture — or a test — of a repository the app writes **during**
    /// the run: the entry's own `at_unix_ms` is drawn, so left on the wall
    /// clock it reads differently every time
    /// (m5-history-and-ghost-preview/25, `mergewright::session::Clock`).
    pub fn open_with_clock(repo: &Path, clock: mergewright::session::Clock) -> (Self, AppState) {
        Self::open_sized(repo, None, 2, clock)
    }

    fn open_with(repo: &Path, askpass_helper: Option<std::path::PathBuf>) -> (Self, AppState) {
        Self::open_sized(repo, askpass_helper, 2, mergewright::session::Clock::Wall)
    }

    fn open_sized(
        repo: &Path,
        askpass_helper: Option<std::path::PathBuf>,
        workers: usize,
        clock: mergewright::session::Clock,
    ) -> (Self, AppState) {
        let (sender, events) = mpsc::channel();
        let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
        let jobs = JobRunner::start(sender, workers, store).expect("the job pool starts");
        let jobs = jobs.with_clock(clock);
        let jobs = match askpass_helper {
            Some(helper) => jobs.with_askpass_helper(helper),
            None => jobs,
        };
        // The opening list (and the id its history walk runs under) is
        // `update::opening`'s, so this harness cannot drift from the real
        // loop (m5-history-and-ghost-preview/05).
        let (mut state, commands) = mergewright::update::opening(repo.to_path_buf(), SIZE);
        // One clock for the frame and the journal stamps, as `MWR_NOW`
        // gives the binary (m8/19d-b2).
        state.harness.clock = clock;
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

    /// A mouse button going down at a screen cell, through the real event
    /// the terminal sends. Pressed through `update` like every other input:
    /// a right-click built as a state change instead of an event is exactly
    /// what missed m5-history-and-ghost-preview/18.
    pub fn mouse_down(
        &self,
        state: &AppState,
        button: MouseButton,
        column: u16,
        row: u16,
    ) -> AppState {
        self.mouse(state, MouseKind::Down(button), column, row)
    }

    /// Any mouse event at a screen cell, through the real event the terminal
    /// sends. The drag gesture needs `Drag` and `Up` as well as `Down`
    /// (m5-history-and-ghost-preview/10).
    pub fn mouse(&self, state: &AppState, kind: MouseKind, column: u16, row: u16) -> AppState {
        let (next, commands) = update(
            state,
            AppEvent::Core(Event::Mouse(MouseInput {
                kind,
                column,
                row,
                modifiers: Modifiers::default(),
            })),
        );
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    /// [`mouse_down`](Self::mouse_down) with the right button — the click
    /// that opens the context menu.
    pub fn right_click(&self, state: &AppState, column: u16, row: u16) -> AppState {
        self.mouse_down(state, MouseButton::Right, column, row)
    }

    /// The terminal got focus back — he edited in his editor and came back —
    /// through the real event the event loop sends: Mergewright re-reads what may
    /// have changed under it (m7-power-tools/07d's edit-stop walk).
    pub fn focus_gained(&self, state: &AppState) -> AppState {
        let (next, commands) = update(state, AppEvent::Core(Event::FocusGained));
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    /// The terminal changed size, through the real event the event loop
    /// sends. A test that needs a viewport the fixture is taller than says
    /// so here rather than building a hundred real commits.
    pub fn resize(&self, state: &AppState, columns: u16, rows: u16) -> AppState {
        let (next, commands) = update(state, AppEvent::Core(Event::Resize { columns, rows }));
        for command in commands {
            self.jobs.dispatch(command);
        }
        next
    }

    /// Feeds job answers through `update` until `done` holds.
    pub fn pump(&self, state: AppState, what: &str, done: impl Fn(&AppState) -> bool) -> AppState {
        pump_events(
            |patience| self.events.recv_timeout(patience),
            |command| self.jobs.dispatch(command),
            state,
            what,
            done,
            DEADLINE,
        )
        .unwrap_or_else(|why| panic!("{why}"))
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
            Status::Done { .. } => state,
            other => panic!(
                "{what} did not succeed: {other:?}; confirm box: {:?}",
                state.confirm.as_ref().map(|pending| &pending.view)
            ),
        }
    }

    pub fn close(self) {
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
        matches!(
            state.detail.files,
            mergewright::update::detail::Loading::Ready(_)
        )
    })
}

/// The pane's drawn file rows, as text.
pub fn rows(state: &AppState) -> Vec<String> {
    match &state.detail.files {
        mergewright::update::detail::Loading::Ready(files) => files
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

/// The fast tier's total deadline fits inside the tier (PROMPT.md 7.1: the
/// whole fast tier runs in under 3 minutes) and never trips a pump that is
/// merely waiting out its per-event [`PATIENCE`] (review m7-18f-32b-06f-33
/// Major 2).
#[test]
fn deadline_fits_inside_the_fast_tier() {
    assert!(DEADLINE <= Duration::from_secs(180), "{DEADLINE:?}");
    assert!(DEADLINE >= PATIENCE, "{DEADLINE:?} < {PATIENCE:?}");
}

/// A stream that never settles — a key, forever — makes the pump fail within
/// its deadline, naming what it waited for and the events it saw.
#[test]
fn pump_fails_on_a_stream_that_never_settles() {
    let started = Instant::now();
    let result = pump_events(
        |_| {
            Ok::<_, String>(AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char(
                'j',
            )))))
        },
        |command| panic!("a plain `j` on an empty state asks for nothing: {command:?}"),
        AppState::default(),
        "a state that never comes",
        |_| false,
        Duration::from_millis(200),
    );
    let Err(why) = result else {
        panic!("a stream that never settles must fail");
    };
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "{:?}",
        started.elapsed()
    );
    assert!(
        why.contains("waiting for a state that never comes"),
        "{why}"
    );
    assert!(why.contains("last 10 events"), "{why}");
    assert!(why.contains("Char('j')"), "{why}");
}
