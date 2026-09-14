//! One measured run of Graft's own open → first paint → `G` → `g` path,
//! in-process, over the real job pool and the real `update` (ticket
//! m1-the-graph/27).
//!
//! Shared by the heavy tier's budget tests (`heavy.rs`) and by
//! `examples/budgets.rs`, which runs it by hand against a repository no
//! test may touch (`git/git`, TESTING.md).
//!
//! What "first paint" means here: from just before the repository store is
//! made to the first moment `update` holds a history window with rows —
//! the state the next frame would draw. It leaves out process exec, config
//! loading, terminal detection and the draw itself; those are the terminal
//! half of the budget and belong to the frame-budget ticket (30).

use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use graft::history::{History, Phase};
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::repos::RepoStore;
use graft::update::{AppEvent, AppState, Command, Size, update};
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_git::history::Order;

/// How long one step may take before the run is declared hung. Far above
/// any budget: it turns a hang into a named failure, it is not a threshold.
const PATIENCE: Duration = Duration::from_secs(300);

/// The terminal size the git/git first frame was first timed at (m1/09b).
const SIZE: Size = Size {
    columns: 120,
    rows: 40,
};

/// What one run measured.
#[derive(Debug)]
pub struct Numbers {
    /// Store created → first window with rows.
    pub first_paint: Duration,
    /// `G` pressed → the last row is on screen in final order.
    pub bottom: Duration,
    /// `g` pressed → row 0 is back on screen.
    pub top: Duration,
    /// Rows in the finished layout.
    pub rows: usize,
    /// Whether the first paint was in a provisional order (ADR-0008: no
    /// commit-graph file, so date order until promotion).
    pub provisional: bool,
    /// This process's peak resident set, in bytes, at first paint.
    pub rss_at_paint: u64,
    /// The same, once `G` has landed.
    pub rss_at_bottom: u64,
    /// This process's peak resident set, in bytes, after the whole run.
    pub peak_rss: u64,
    /// `CommitTable::overflow_len()` once the run has settled (m1-the-graph/51b's
    /// 2× check): with no commit-graph file every commit is overflow, and
    /// promoting to strict topological order must resolve the same handles
    /// rather than interning more of them.
    pub overflow_len: usize,
}

/// Opens `repo` as `event_loop::opening_repository` does, and drives it.
pub fn run(repo: &Path) -> Result<Numbers, String> {
    let started = Instant::now();
    let (jobs, events, mut state, store) = open_with_store(repo)?;

    let painted = |h: &History| h.window.as_ref().is_some_and(|w| !w.rows.is_empty());
    state = pump(&jobs, &events, state, painted)?;
    let first_paint = started.elapsed();
    let rss_at_paint = peak()?;
    let provisional = history(&state)?
        .walk
        .as_ref()
        .is_some_and(graft::history::WalkFacts::is_provisional);

    let pressed = Instant::now();
    state = press(&jobs, &state, KeyCode::Char('G'));
    let at_bottom = |h: &History| {
        settled(h)
            && h.phase == Phase::Complete
            && h.window
                .as_ref()
                .is_some_and(|w| w.done && w.end() == w.laid_out)
    };
    state = pump(&jobs, &events, state, at_bottom)?;
    let bottom = pressed.elapsed();
    let rss_at_bottom = peak()?;
    let rows = history(&state)?.rows;

    let pressed = Instant::now();
    state = press(&jobs, &state, KeyCode::Char('g'));
    let at_top = |h: &History| settled(h) && h.window.as_ref().is_some_and(|w| w.first == 0);
    pump(&jobs, &events, state, at_top)?;
    let top = pressed.elapsed();

    let peak_rss = peak()?;
    match jobs.shutdown(Duration::from_secs(10)) {
        Shutdown::Clean => {}
        other @ Shutdown::Stalled { .. } => {
            return Err(format!("the job pool did not stop cleanly: {other:?}"));
        }
    }
    let overflow_len = store.table().map_err(|why| why.to_string())?.overflow_len();
    Ok(Numbers {
        first_paint,
        bottom,
        top,
        rows,
        provisional,
        rss_at_paint,
        rss_at_bottom,
        peak_rss,
        overflow_len,
    })
}

fn peak() -> Result<u64, String> {
    graft_fixtures::peak_rss().map_err(|why| why.to_string())
}

/// A started job pool, the events it reports, and the state before any.
#[allow(
    dead_code,
    reason = "used by many_refs.rs and frames.rs, not visible from budgets.rs's separate compilation of this file"
)]
pub type Opened = (JobRunner<Sender<AppEvent>>, Receiver<AppEvent>, AppState);

/// As [`Opened`], with the store `run` needs afterwards to read
/// `CommitTable::overflow_len()` for the 2× check (m1-the-graph/51b), which
/// nothing else here otherwise keeps a handle to once `JobRunner::start` has
/// its own clone.
type OpenedWithStore = (
    JobRunner<Sender<AppEvent>>,
    Receiver<AppEvent>,
    AppState,
    Arc<RepoStore>,
);

/// Starts the pool and asks for what `event_loop::opening_repository` asks
/// for, in the same order.
///
/// Only `many_refs.rs` and `frames.rs` call this directly (`run`, in this
/// file, uses [`open_with_store`] instead) — that makes it dead code from
/// `examples/budgets.rs`'s own compilation of this file, which pulls in none
/// of the other test modules; kept `pub` because the two callers are real.
#[allow(
    dead_code,
    reason = "used by many_refs.rs and frames.rs, not visible from budgets.rs's separate compilation of this file"
)]
pub fn open(repo: &Path) -> Result<Opened, String> {
    let (jobs, events, state, _store) = open_with_store(repo)?;
    Ok((jobs, events, state))
}

/// As [`open`], but also hands back the store.
fn open_with_store(repo: &Path) -> Result<OpenedWithStore, String> {
    let (sender, events) = mpsc::channel();
    let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
    let jobs = JobRunner::start(sender, 4, Arc::clone(&store)).map_err(|why| why.to_string())?;
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
    Ok((jobs, events, state, store))
}

/// No jump pending and no promotion outstanding: the screen shows final rows.
fn settled(history: &History) -> bool {
    history.jump.is_none() && history.promotion.is_none()
}

fn history(state: &AppState) -> Result<&History, String> {
    state
        .history
        .as_ref()
        .ok_or_else(|| "no history walk was started".to_owned())
}

fn press(jobs: &impl Dispatch, state: &AppState, code: KeyCode) -> AppState {
    let (next, commands) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
    for command in commands {
        jobs.dispatch(command);
    }
    next
}

/// Feeds job events through `update` until `done` holds for the history.
fn pump(
    jobs: &impl Dispatch,
    events: &Receiver<AppEvent>,
    state: AppState,
    done: impl Fn(&History) -> bool,
) -> Result<AppState, String> {
    pump_until(jobs, events, state, |state| {
        state.history.as_ref().is_some_and(&done)
    })
}

/// Feeds job events through `update` until `done` holds for the whole state.
pub fn pump_until(
    jobs: &impl Dispatch,
    events: &Receiver<AppEvent>,
    mut state: AppState,
    done: impl Fn(&AppState) -> bool,
) -> Result<AppState, String> {
    loop {
        if let Some(history) = &state.history
            && let Phase::Failed(why) = &history.phase
        {
            return Err(format!("the walk failed: {why}"));
        }
        if done(&state) {
            return Ok(state);
        }
        let event = events
            .recv_timeout(PATIENCE)
            .map_err(|why| format!("no event for {PATIENCE:?} ({why}); state: {state:?}"))?;
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        if let Some(quit) = &state.quit {
            return Err(format!("Graft quit: {quit:?}"));
        }
    }
}
