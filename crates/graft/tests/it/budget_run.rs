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

use graft::history::{History, Phase, View, Views, WalkSource, seeds_for};
use graft::jobs::{Dispatch, JobEvents, JobRunner, Shutdown};
use graft::repos::RepoStore;
use graft::update::{AppEvent, AppState, Size, update};
use graft_core::commit_handle::CommitHandle;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_git::history::{Order, Walk};
use graft_graph::Included;
use graft_graph::lane::LaneIdentity;
use graft_graph::layout::{
    CommitSource, EmittedOrder, LayoutCursor, StreamConfig, StreamingLayout, Work,
};
use graft_graph::row::RowShape;

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
    /// This process's physical footprint once `G` has landed
    /// (m1-the-graph/57). `Err` only means this platform cannot read it
    /// (`graft_fixtures::footprint`, macOS only) — not a failure of the run.
    pub footprint_at_bottom: Result<graft_fixtures::Footprint, graft_fixtures::MemoryError>,
    /// The same reading taken once more at the very end of the run, whose
    /// `peak` is this run's own lifetime maximum footprint.
    pub footprint_at_end: Result<graft_fixtures::Footprint, graft_fixtures::MemoryError>,
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
    let footprint_at_bottom = graft_fixtures::footprint();
    let rows = history(&state)?.rows;

    let pressed = Instant::now();
    state = press(&jobs, &state, KeyCode::Char('g'));
    let at_top = |h: &History| settled(h) && h.window.as_ref().is_some_and(|w| w.first == 0);
    pump(&jobs, &events, state, at_top)?;
    let top = pressed.elapsed();

    let peak_rss = peak()?;
    let footprint_at_end = graft_fixtures::footprint();
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
        footprint_at_bottom,
        footprint_at_end,
    })
}

fn peak() -> Result<u64, String> {
    graft_fixtures::peak_rss().map_err(|why| why.to_string())
}

/// Set by [`gated_in_child`] for `heavy::child_gated_run`: the repository to
/// run the gated path on.
///
/// Used by `heavy.rs`'s slope guard; not visible from
/// `examples/budgets.rs`'s separate compilation of this file (as
/// [`Opened`] above), hence the `dead_code` allow on every item down to
/// [`parse_child_peak`].
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub const CHILD_REPO_ENV: &str = "GRAFT_CHILD_REPO";

/// The child's one line of output carries this (cargo's own harness may
/// share the line with a `test <name> ... ` prefix, so it is not
/// necessarily at the start — see [`gated_in_child`]).
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub const CHILD_MARK: &str = "child_peak";

/// One gated run's peaks, read back from a fresh process (m1-the-graph/57).
///
/// `getrusage`'s peak and `footprint`'s peak are both lifetime maxima of the
/// *process* they run in, not of the work inside it (see
/// `graft_fixtures::memory`'s module docs). A slope guard wants one
/// fixture's own peak, never a peak an earlier fixture already set in the
/// same test binary, so every fixture in a slope guard runs the gated path
/// in a process of its own and this is what that process reports back.
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub struct ChildPeak {
    /// The child's peak RSS (`getrusage`) over its whole gated run. The
    /// only field the slope guard asserts on.
    pub peak_rss: u64,
    /// The child's peak physical footprint over its whole gated run.
    #[allow(
        dead_code,
        reason = "reported by budgets --gate's own instrumentation, not yet by a heavy-tier assertion"
    )]
    pub footprint_peak: u64,
    /// The child's physical footprint the moment `G` landed.
    #[allow(
        dead_code,
        reason = "reported by budgets --gate's own instrumentation, not yet by a heavy-tier assertion"
    )]
    pub footprint_after_g: u64,
    /// The child's first-paint duration.
    #[allow(
        dead_code,
        reason = "carried for a future first-paint-in-a-child guard; not asserted here"
    )]
    pub first_paint: Duration,
}

/// Why [`gated_in_child`] could not read a [`ChildPeak`].
#[derive(Debug)]
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub enum ChildError {
    /// The child process could not even be started, or this binary could
    /// not find its own path to re-exec.
    Spawn(std::io::Error),
    /// The child exited without success. Carries the tail of its stderr, so
    /// the failure is visible rather than turned into a default reading.
    Exit {
        status: std::process::ExitStatus,
        stderr_tail: String,
    },
    /// The child exited cleanly but printed no [`CHILD_MARK`] line, or the
    /// line it printed did not parse.
    NoReport,
}

impl std::fmt::Display for ChildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChildError::Spawn(why) => write!(f, "could not run the measuring child: {why}"),
            ChildError::Exit {
                status,
                stderr_tail,
            } => write!(f, "the measuring child exited {status}: {stderr_tail}"),
            ChildError::NoReport => {
                write!(f, "the measuring child printed no {CHILD_MARK:?} line")
            }
        }
    }
}

impl std::error::Error for ChildError {}

/// Runs the gated path (open → first paint → `G` → `g`) on `repo` in a
/// fresh copy of this test binary — `heavy::child_gated_run`, selected by
/// [`CHILD_REPO_ENV`] — so the peaks reported are that run's own, not
/// whatever this process's lifetime maximum already was.
#[allow(
    dead_code,
    reason = "used by heavy.rs's slope guard, not visible from examples/budgets.rs's separate compilation of this file"
)]
pub fn gated_in_child(repo: &Path) -> Result<ChildPeak, ChildError> {
    let exe = std::env::current_exe().map_err(ChildError::Spawn)?;
    let output = std::process::Command::new(exe)
        .args([
            "heavy::child_gated_run",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(CHILD_REPO_ENV, repo)
        .output()
        .map_err(ChildError::Spawn)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stderr_tail: String = stderr.lines().rev().take(20).collect::<Vec<_>>().join("\n");
        return Err(ChildError::Exit {
            status: output.status,
            stderr_tail,
        });
    }
    // The mark is not necessarily at the start of the line: cargo's own test
    // harness prints `test heavy::child_gated_run ... ` with no newline
    // before the test body's own `println!` runs, so the two share a line.
    // `contains`, not `starts_with`, is what `result_line` (this file's
    // predecessor for the other budget tests) already did for the same
    // reason.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout
        .lines()
        .find(|line| line.contains(CHILD_MARK))
        .ok_or(ChildError::NoReport)?;
    parse_child_peak(line).ok_or(ChildError::NoReport)
}

/// Parses one `child_peak rss=<bytes> footprint_peak=<bytes>
/// footprint_after_g=<bytes> first_paint_ms=<n>` line. `None` for any
/// missing or unparseable field — [`gated_in_child`] turns that into
/// [`ChildError::NoReport`] rather than defaulting a field.
fn parse_child_peak(line: &str) -> Option<ChildPeak> {
    let field = |name: &str| -> Option<u64> {
        line.split_whitespace()
            .find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
            .and_then(|value| value.parse().ok())
    };
    Some(ChildPeak {
        peak_rss: field("rss")?,
        footprint_peak: field("footprint_peak")?,
        footprint_after_g: field("footprint_after_g")?,
        first_paint: Duration::from_millis(field("first_paint_ms")?),
    })
}

/// `EmittedOrder`'s approximate bytes once every commit is laid out, and the
/// commit count it was measured over (m1-the-graph/51c, 51's last box).
pub fn order_bytes(repo: &Path) -> Result<(usize, usize), String> {
    direct_layout(repo, |layout| {
        layout.set_viewport(0..usize::MAX);
        advance_to_the_end(layout)?;
        Ok((layout.resident().order_bytes, layout.rows_emitted()))
    })
}

/// What a cold `G` read with the viewport at the top (m1-the-graph/52b).
#[derive(Debug)]
pub struct ColdJump {
    /// The layout's work counters once every row is laid out.
    pub work: Work,
    /// Rows laid out.
    pub rows: usize,
    /// `2 × chunk_rows + buffer_rows`: the most `work.hydrated` may be.
    pub hydrated_bound: usize,
    /// The most `work.merge_reads` may be: the distinct merge lanes open at
    /// the first row of every materialised chunk after the first.
    pub merge_bound: usize,
}

/// Lays out all of `repo` with the viewport on the first screen, as a cold
/// `G` does before the screen moves, and reads the layout's work counters.
pub fn cold_jump(repo: &Path) -> Result<ColdJump, String> {
    direct_layout(repo, |layout| {
        let screen = usize::from(SIZE.rows);
        layout.set_viewport(0..screen);
        advance_to_the_end(layout)?;
        let work = layout.work();
        let rows = layout.rows_emitted();
        let config = layout.config();

        // The chunks `advance` materialised: those meeting the viewport plus
        // `buffer_rows`. Their shapes, replayed from the top by links only.
        let reach = (screen + config.buffer_rows).min(rows);
        let chunks = reach.div_ceil(config.chunk_rows);
        let window = (chunks * config.chunk_rows).min(rows);
        let handles = (0..window)
            .map(|row| {
                let id = layout.id_at(row).ok_or(format!("no commit at row {row}"))?;
                layout
                    .source()
                    .handle_of(&id)
                    .ok_or(format!("{id} is not interned"))
            })
            .collect::<Result<Vec<CommitHandle>, String>>()?;
        let links = layout
            .source_mut()
            .links(&handles)
            .map_err(|why| why.to_string())?;
        let source = layout.source();
        let mut cursor = LayoutCursor::new(source.seeds().clone(), source);
        let shapes = cursor.extend(&links, source, &mut EmittedOrder::new());
        let merge_bound = (1..chunks)
            .map(|chunk| merge_lanes_open(&shapes[chunk * config.chunk_rows]))
            .sum();
        Ok(ColdJump {
            work,
            rows,
            hydrated_bound: 2 * config.chunk_rows + config.buffer_rows,
            merge_bound,
        })
    })
}

/// One far jump after a cold `G` (m1-the-graph/56): what `rows` costs when
/// the row asked for sits in a passed-through stretch of history.
#[allow(
    dead_code,
    reason = "measured by examples/budgets.rs only; no test asserts on it yet (the 1M jump is a review-time number)"
)]
pub struct FarJump {
    /// The row jumped to.
    pub row: usize,
    /// Rows laid out in total.
    pub rows: usize,
    /// Wall time of the one `rows` call.
    pub took: Duration,
    /// Rows replayed from the nearest checkpoint to serve it.
    pub rows_replayed: usize,
    /// Chunks rebuilt to serve it.
    pub chunks_rebuilt: usize,
    /// Commits read to assemble the screen.
    pub hydrated: usize,
    /// Rows between kept checkpoints at the time of the jump
    /// (m1-the-graph/60: the grid's stride, doubled each time
    /// `max_checkpoints` was reached).
    pub checkpoint_stride: usize,
}

/// Lays out all of `repo` as [`cold_jump`] does, then asks for one screen
/// at `at(rows)` — a `Find` deep into history — and reads what the layout
/// had to replay to serve it. The checkpoints thin to an even grid rather
/// than evicting least-recently-used, so a jump anywhere in history replays
/// at most one stride's worth of links (m1-the-graph/60).
#[allow(
    dead_code,
    reason = "measured by examples/budgets.rs only; no test asserts on it yet (the 1M jump is a review-time number)"
)]
pub fn far_jump(repo: &Path, at: impl FnOnce(usize) -> usize) -> Result<FarJump, String> {
    direct_layout(repo, |layout| {
        let screen = usize::from(SIZE.rows);
        layout.set_viewport(0..screen);
        advance_to_the_end(layout)?;
        let rows = layout.rows_emitted();
        let row = at(rows).min(rows.saturating_sub(screen));
        let before = layout.work();
        let started = Instant::now();
        let served = layout
            .rows(row..(row + screen).min(rows))
            .map_err(|why| why.to_string())?;
        let took = started.elapsed();
        if served.len() != (row + screen).min(rows) - row {
            return Err(format!(
                "asked {screen} rows at {row}, got {}",
                served.len()
            ));
        }
        let after = layout.work();
        Ok(FarJump {
            row,
            rows,
            took,
            rows_replayed: after.rows_replayed - before.rows_replayed,
            chunks_rebuilt: after.chunks_rebuilt - before.chunks_rebuilt,
            hydrated: after.hydrated - before.hydrated,
            checkpoint_stride: layout.checkpoint_stride(),
        })
    })
}

/// Distinct `Merge` identities drawn on `shape`.
fn merge_lanes_open(shape: &RowShape) -> usize {
    let mut open: Vec<CommitHandle> = shape
        .lanes
        .iter()
        .flatten()
        .filter_map(|identity| match *identity {
            LaneIdentity::Merge { merge, .. } => Some(merge),
            LaneIdentity::Known(_) => None,
        })
        .collect();
    open.sort_unstable();
    open.dedup();
    open.len()
}

fn advance_to_the_end<S: CommitSource>(layout: &mut StreamingLayout<S>) -> Result<(), String>
where
    S::Error: std::fmt::Display,
{
    let stop = || false;
    while !layout.is_done() {
        layout
            .advance(usize::MAX, &stop)
            .map_err(|why| why.to_string())?;
    }
    Ok(())
}

/// Opens `repo`'s all-parents layout the way `history::stream` does and
/// hands it to `drive`.
///
/// `history::stream`'s own `Views` never leaves it — `serve` reports only
/// `HistoryUpdate`, no resident-set or work numbers — so this repeats its
/// setup (`Walk::all_refs`, `WalkSource`, `Views::new`) to drive a layout
/// directly, rather than adding a new update variant through the job pool
/// for measurements.
fn direct_layout<T>(
    repo: &Path,
    drive: impl FnOnce(&mut StreamingLayout<Included<WalkSource<'_>>>) -> Result<T, String>,
) -> Result<T, String> {
    let store = RepoStore::new(repo.to_path_buf(), None);
    let branches = store.branches().map_err(|why| why.to_string())?;
    let shared = store.shared().map_err(|why| why.to_string())?;
    let handle = shared.handle();
    let refs = handle.references().map_err(|why| why.to_string())?;
    let seeds = seeds_for(&refs, &branches);
    let table = store.table().map_err(|why| why.to_string())?;
    let walk = Walk::all_refs(&handle, Order::Topological, Arc::clone(&table))
        .map_err(|why| why.to_string())?;

    let mut started = Some(walk);
    let handle_ref = &handle;
    let open = move |promoted: Option<&Arc<[CommitHandle]>>| {
        if let Some(order) = promoted {
            return Ok(WalkSource::in_order(
                handle_ref,
                Arc::clone(&table),
                Arc::clone(order),
            ));
        }
        let walk = match started.take() {
            Some(walk) => walk,
            None => Walk::all_refs(handle_ref, Order::Topological, Arc::clone(&table))?,
        };
        Ok(WalkSource::new(handle_ref, walk))
    };
    let mut views = Views::new(seeds, StreamConfig::default(), Box::new(open));
    let layout = views
        .layout(&View::default())
        .map_err(|why| why.to_string())?;
    drive(layout)
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

/// As [`open`], with the jobs reporting into `events`, whatever its shape:
/// `frames` passes the loop's own `Sender<Wake>`, so job results and keys
/// share one channel, as in the app (m7-power-tools/04b).
#[allow(
    dead_code,
    reason = "used by frames.rs, not visible from budgets.rs's separate compilation of this file"
)]
pub fn open_into<S: JobEvents>(repo: &Path, events: S) -> Result<(JobRunner<S>, AppState), String> {
    let (jobs, state, _store) = start(repo, events)?;
    Ok((jobs, state))
}

/// As [`open`], but also hands back the store.
fn open_with_store(repo: &Path) -> Result<OpenedWithStore, String> {
    let (sender, events) = mpsc::channel();
    let (jobs, state, store) = start(repo, sender)?;
    Ok((jobs, events, state, store))
}

/// Starts the pool reporting into `events` and asks for the opening.
fn start<S: JobEvents>(
    repo: &Path,
    events: S,
) -> Result<(JobRunner<S>, AppState, Arc<RepoStore>), String> {
    let store = Arc::new(RepoStore::new(repo.to_path_buf(), None));
    let jobs = JobRunner::start(events, 4, Arc::clone(&store)).map_err(|why| why.to_string())?;
    let (state, commands) = graft::update::opening(repo.to_path_buf(), SIZE);
    for command in commands {
        jobs.dispatch(command);
    }
    Ok((jobs, state, store))
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
    state: AppState,
    done: impl Fn(&AppState) -> bool,
) -> Result<AppState, String> {
    pump_until_from(jobs, |patience| events.recv_timeout(patience), state, done)
}

/// As [`pump_until`], taking each event from `next` (given how long it may
/// wait): for a harness whose jobs report into a channel of its own shape,
/// as `frames` does into the loop's `Wake` channel (m7-power-tools/04b).
pub fn pump_until_from<E: std::fmt::Display>(
    jobs: &impl Dispatch,
    mut next: impl FnMut(Duration) -> Result<AppEvent, E>,
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
        let event = next(PATIENCE)
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
