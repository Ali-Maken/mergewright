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
}

/// Opens `repo` as `event_loop::opening_repository` does, and drives it.
pub fn run(repo: &Path) -> Result<Numbers, String> {
    let started = Instant::now();
    let (jobs, events, mut state) = open(repo)?;

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
    Ok(Numbers {
        first_paint,
        bottom,
        top,
        rows,
        provisional,
        rss_at_paint,
        rss_at_bottom,
        peak_rss,
    })
}

fn peak() -> Result<u64, String> {
    graft_fixtures::peak_rss().map_err(|why| why.to_string())
}

/// A started job pool, the events it reports, and the state before any.
type Opened = (JobRunner<Sender<AppEvent>>, Receiver<AppEvent>, AppState);

/// Starts the pool and asks for what `event_loop::opening_repository` asks
/// for, in the same order.
fn open(repo: &Path) -> Result<Opened, String> {
    let (sender, events) = mpsc::channel();
    let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
    let jobs = JobRunner::start(sender, 4, store).map_err(|why| why.to_string())?;
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
    Ok((jobs, events, state))
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
    mut state: AppState,
    done: impl Fn(&History) -> bool,
) -> Result<AppState, String> {
    loop {
        if let Some(history) = &state.history {
            if let Phase::Failed(why) = &history.phase {
                return Err(format!("the walk failed: {why}"));
            }
            if done(history) {
                return Ok(state);
            }
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
