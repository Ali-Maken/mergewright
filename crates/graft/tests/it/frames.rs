//! Input to frame on the 100k-commit fixture (PROMPT.md §1.2 "scroll at 60
//! fps", §6.3 "input to frame: under 16 ms"; DESIGN.md §3.8), asserted at
//! the Unicode and pixel tiers, and again with 50k refs on the badges
//! (PROMPT.md §6.3 edge cases). Ticket m1-the-graph/30.
//!
//! The real loop (`event_loop::pump`) runs over the real job pool and an
//! in-memory terminal. Keys arrive on a 60 Hz clock, the way a held key or
//! a fast hand scrolls, sent by an input thread into the loop's wake channel
//! as the app's input forwarder sends them ([`KeyFeed`], m7-power-tools/04b).
//! Each key's latency is measured from its send. The thread keeps the
//! script's clock, so a loop that falls behind shows up as keys queued in
//! the channel, counted in their latency, not hidden. The thread's own
//! sleep, and the OS timer slack on it, lands before the send, where a real
//! key's arrival is (m7-power-tools/04: up to 10 ms after a 600 ms pause,
//! with no Graft code running). A key's frame is the first frame that shows the
//! rows the key asked for: when a key sends a `ShowRows`, the frame drawn
//! after the job's answer, not the frame drawn while the rows were still
//! out.
//!
//! What it leaves out: the terminal's own parsing and painting, and write
//! back-pressure on a real pty. The in-memory terminal only copies what it
//! is sent; counting Kitty commands in it happens after the scroll, outside
//! every timed window (see [`Sink`]). Heavy tier only (`heavy-tests`, ADR-0018).
//!
//! The budget fails only in a release build (ADR-0021): PROMPT.md's 16 ms is
//! a claim about the shipped binary. A debug build runs the same scroll and
//! prints every late key, recorded and not failing. The rows-on-hand check
//! (no key drawn before its viewport rows arrived) follows the same rule:
//! it depends on how fast the layout worker runs.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use graft::config::Loaded;
use graft::event_loop::{Exit, Phase, Wake, Wakes, pump};
use graft::history::{HistoryUpdate, Phase as WalkPhase};
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::refs::{RefBadges, badges};
use graft::session::Session;
use graft::update::{AppEvent, AppState, Command, Progress, update};
use graft_core::capabilities::{
    Capabilities, CellSize, CellSizeSource, ColorDepth, PixelProtocol, ProbeOutcome, RenderingTier,
    Rgb, Supported, Theme as TerminalTheme, ThemeHint, ThemeSource, TmuxStatus,
};
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_fixtures::synthetic::{self, Scale};
use graft_git::history::AheadBehind;
use graft_git::repo::HeadState;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::{Terminal, TerminalOptions, Viewport};

use crate::budget_run;
use crate::frame_trace::{FrameTrace, Gate, KeyFeed, Post, Posted, SentLog, TRACE_VAR, WriteClock};
use crate::heavy::MEASURING;

/// PROMPT.md §6.3: input to frame under 16 ms.
const INPUT_TO_FRAME: Duration = Duration::from_millis(16);

/// 60 Hz: the key clock, and the frame rate PROMPT.md §1.2 asks to sustain.
const TICK: Duration = Duration::from_micros(16_667);

/// The pause before each idle-wake key: longer than the half second after
/// which the first key missed in release (m1-the-graph/30c), as when the user
/// reads the screen before pressing the next key.
const IDLE: Duration = Duration::from_millis(600);

/// How long the end of a run waits for rows still out. Far above any
/// budget: it turns a hang into a named failure, it is not a threshold.
const PATIENCE: Duration = Duration::from_secs(60);

/// PROMPT.md §6.3: "repos with 50k+ refs".
const REFS: usize = 50_000;

#[test]
fn scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier() {
    check(RenderingTier::Unicode, Refs::Fixture, Pane::Closed);
}

#[test]
fn scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier() {
    check(RenderingTier::Pixel, Refs::Fixture, Pane::Closed);
}

#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_unicode_tier() {
    check(RenderingTier::Unicode, Refs::FiftyThousand, Pane::Closed);
}

/// M2 review M9: M2 put the detail pane, its diff and the Mistake Radar on
/// the frame path, and none of the frame tests looked at them. This is the
/// 50,000-ref scroll with the pane open on the selected commit, so every
/// frame draws the header, the radar's lines and the file list beside the
/// graph.
#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_with_the_detail_pane_open() {
    check(RenderingTier::Unicode, Refs::FiftyThousand, Pane::Open);
}

/// M2 review M9, round 1b: round 1a's pane-open test never actually scrolled
/// the diff body — `Enter` leaves focus on Files, and the scroll's `j`/
/// `PageDown`/`PageUp` moved the file cursor instead. `Tab` before the
/// scroll moves focus to Diff, so this is the case round 1a's test missed.
#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_scrolling_the_diff() {
    check(
        RenderingTier::Unicode,
        Refs::FiftyThousand,
        Pane::DiffScroll,
    );
}

/// M2 review M9, round 1b: no frame test covered the Map view (`M`) at all.
/// Run on the same 50k-ref fixture as the pane-open test, since the Map
/// view's cost is walking every tracked branch (`crate::map`) — the same
/// ref count C1 made cheap for the graph and the radar.
#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_with_the_map_view_open() {
    check(RenderingTier::Unicode, Refs::FiftyThousand, Pane::Map);
}

#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_pixel_tier() {
    check(RenderingTier::Pixel, Refs::FiftyThousand, Pane::Closed);
}

/// ADR-0026 deferred the 1M `frames::` scroll numbers to the M1 perf run
/// (m1-the-graph/64). The fixture is a ~300 MiB repository these tests build
/// or borrow on demand: too expensive for the standard heavy-tier command
/// (`cargo test --features heavy-tests -- --test-threads=2`, no filter, runs
/// every compiled test) or the release perf run's `frames::` filter
/// (`TESTING.md`) to sweep up by accident. `scripts/memory-curve.sh`'s own
/// 1M cells are manual the same way, but that mechanism is a shell script
/// never wired into `cargo test`, which does not fit a `#[test]` fn. So
/// these compile only behind the `heavy-1m` feature (ADR-0018 amendment,
/// 2026-09-17, m1-the-graph/80): unset, the standard heavy-tier command
/// never sees them; `--features heavy-1m` (implies `heavy-tests`) runs them,
/// per TESTING.md's manual command.
#[cfg(feature = "heavy-1m")]
#[test]
fn scrolling_the_1m_commit_graph_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier() {
    check_scale(
        RenderingTier::Unicode,
        Refs::Fixture,
        ONE_MILLION_CG,
        Pane::Closed,
    );
}

#[cfg(feature = "heavy-1m")]
#[test]
fn scrolling_the_1m_commit_graph_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier() {
    check_scale(
        RenderingTier::Pixel,
        Refs::Fixture,
        ONE_MILLION_CG,
        Pane::Closed,
    );
}

/// The 1M small-commit fixture with a commit-graph (ticket asks for "1M
/// small cg"; `CommitSize::Small` is `Scale`'s default).
#[cfg(feature = "heavy-1m")]
const ONE_MILLION_CG: Scale = Scale::of(1_000_000).with_commit_graph();

/// Which badges the scroll draws.
#[derive(Clone, Copy, Debug)]
enum Refs {
    /// The fixture's own refs, as the refs job read them.
    Fixture,
    /// [`REFS`] refs in place of them (see [`fifty_thousand`]).
    FiftyThousand,
}

/// Whether the detail pane is open while the scroll runs.
#[derive(Clone, Copy, Debug)]
enum Pane {
    Closed,
    /// `Enter` before the scroll: every frame after it draws the pane.
    Open,
    /// `Enter` then `Tab`: focus lands on the Diff region, so the scroll
    /// below moves `body_top` through the diff body every frame instead of
    /// the file list (review M9 round 1b: nothing measured diff scrolling).
    DiffScroll,
    /// `M` before the scroll: the Map view over the branch list, on the
    /// same 50k-ref fixture `Refs::FiftyThousand` builds (review M9 round
    /// 1b: nothing measured a Map-view frame).
    Map,
}

/// Which run this is: `GRAFT_FRAME_CONTROL`, set by `scripts/m7-perf-key.sh`
/// (m7-power-tools/04); unset is the plain run. Each control takes one
/// suspect away from the slow keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Control {
    Plain,
    /// A (`a`): the idle keys follow a tick, not an [`IDLE`] pause.
    NoIdle,
    /// B (`b`): a thread spins on one core through the whole scroll, so the
    /// machine never sits idle.
    Busy,
    /// C (`c`): no idle prefetch. The loop is told a wake is waiting before
    /// it readies any strip.
    NoPrefetch,
}

impl Control {
    fn from_env() -> Result<Self, String> {
        match std::env::var("GRAFT_FRAME_CONTROL").as_deref() {
            Err(std::env::VarError::NotPresent) | Ok("" | "plain") => Ok(Self::Plain),
            Ok("a") => Ok(Self::NoIdle),
            Ok("b") => Ok(Self::Busy),
            Ok("c") => Ok(Self::NoPrefetch),
            other => Err(format!(
                "GRAFT_FRAME_CONTROL: {other:?} is not plain, a, b or c"
            )),
        }
    }
}

/// A thread spinning on one core until dropped (control B).
struct Spinner {
    stop: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Spinner {
    fn start() -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::spawn(move || {
            while !flag.load(Ordering::Relaxed) {
                std::hint::spin_loop();
            }
        });
        Self {
            stop,
            thread: Some(thread),
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            // The spinner only loads a flag, so it cannot panic, and a drop
            // has nowhere to report a panic anyway.
            drop(thread.join());
        }
    }
}

/// The scroll: down a line at a time, down a page at a time far past the
/// first layout chunks, then back up a page at a time.
fn script(pane: Pane) -> Vec<KeyCode> {
    let mut keys = match pane {
        Pane::Closed => Vec::new(),
        Pane::Open => vec![KeyCode::Enter],
        Pane::DiffScroll => vec![KeyCode::Enter, KeyCode::Tab],
        Pane::Map => vec![KeyCode::Char('M')],
    };
    keys.extend(vec![KeyCode::Char('j'); 120]);
    keys.extend(vec![KeyCode::PageDown; 240]);
    keys.extend(vec![KeyCode::PageUp; 60]);
    keys
}

/// After the scroll, keys that each follow an [`IDLE`] pause. Measured and
/// printed on their own, never asserted (user, 2026-09-13): most of their
/// cost is the CPU waking up (ADR-0021), and printing them keeps a real
/// regression there visible.
fn idle_wakes() -> Vec<KeyCode> {
    vec![
        KeyCode::Char('j'),
        KeyCode::PageDown,
        KeyCode::Char('k'),
        KeyCode::PageUp,
        KeyCode::Char('j'),
    ]
}

/// Every key the run presses, with the pause before it. Control A takes
/// the idle pauses away.
fn keys(pane: Pane, control: Control) -> Vec<(KeyCode, Duration)> {
    let pause = if control == Control::NoIdle {
        TICK
    } else {
        IDLE
    };
    let scroll = script(pane).into_iter().map(|key| (key, TICK));
    let idle = idle_wakes().into_iter().map(|key| (key, pause));
    scroll.chain(idle).collect()
}

fn check(tier: RenderingTier, refs: Refs, pane: Pane) {
    check_scale(tier, refs, Scale::HEAVY.with_commit_graph(), pane);
}

/// [`check`], generalised to any fixture scale (m1-the-graph/64: the same
/// harness at 1M). Output lines are identical either way.
fn check_scale(tier: RenderingTier, refs: Refs, scale: Scale, pane: Pane) {
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let control = Control::from_env().unwrap_or_else(|why| panic!("{why}"));
    let trace = FrameTrace::from_var(std::env::var_os(TRACE_VAR));
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let run = scroll(&repo, tier, refs, pane, control, trace.is_some())
        .unwrap_or_else(|why| panic!("{why}"));
    drop(turn);

    assert_eq!(
        run.latencies.len(),
        keys(pane, control).len(),
        "every key got a frame"
    );
    if let Some(trace) = &trace {
        let label = format!(
            "tier={tier:?} refs={refs:?} pane={pane:?} control={control:?} run={}",
            std::env::var("GRAFT_FRAME_RUN").unwrap_or_else(|_unset| "-".to_owned())
        );
        let lines: Vec<String> = run
            .trace
            .iter()
            .map(|line| format!("trace {label} {line}"))
            .collect();
        trace
            .append(&lines)
            .unwrap_or_else(|why| panic!("{TRACE_VAR}: {why}"));
    }
    if trace.is_some() {
        for line in &run.startup {
            println!("frame_trace: tier={tier:?} refs={refs:?} pane={pane:?} {line}");
        }
    }
    let (scrolled, idle) = run.latencies.split_at(script(pane).len());
    let gate = Gate::of(scrolled, INPUT_TO_FRAME);
    println!(
        "frames: tier={tier:?} refs={refs:?} pane={pane:?} control={control:?} keys={} {} kitty_commands={} bytes={}",
        scrolled.len(),
        gate.line(),
        run.kitty_commands,
        run.bytes,
    );

    print_details(tier, refs, &run, idle);
    match tier {
        RenderingTier::Pixel => assert!(run.kitty_commands > 0, "the pixel tier drew no images"),
        _ => assert_eq!(run.kitty_commands, 0, "a text tier sent images"),
    }
    judge(tier, refs, pane, &run, &gate);
}

/// Release enforces the input-to-frame budget on the p99, debug records it
/// (ADR-0021). Every key over the budget is printed either way, and the
/// max is on the summary line (m7-power-tools/04: one outlier in 420 keys
/// no longer fails the run).
fn judge(tier: RenderingTier, refs: Refs, pane: Pane, run: &Run, gate: &Gate) {
    let total = run.latencies.len() - idle_wakes().len();
    let late: Vec<_> = gate
        .over
        .iter()
        .map(|&key| {
            format!(
                "key {key}: {:?} ({})",
                run.latencies[key], run.breakdown[key]
            )
        })
        .collect();
    if cfg!(debug_assertions) {
        // ADR-0021: debug records the budget, release enforces it. Rows on
        // hand depend on pace too: whether layout stays ahead of a held
        // PageDown at 60 Hz is a question of how fast the worker runs, and
        // debug code runs it 2-5x slower.
        println!(
            "frames: tier={tier:?} refs={refs:?} pane={pane:?} debug build, budget recorded not asserted: {} of {} keys at or over {INPUT_TO_FRAME:?}: {}",
            late.len(),
            total,
            late.join(", ")
        );
        println!(
            "frames: tier={tier:?} refs={refs:?} pane={pane:?} debug build, rows on hand recorded not asserted: {} of {} keys drawn before their rows arrived: {}",
            run.waited.len(),
            run.latencies.len(),
            run.waited.join(", ")
        );
        return;
    }
    // PROMPT.md's third principle: a key draws what is on hand, never waits;
    // in release, what is on hand must already be the whole viewport.
    assert!(
        run.waited.is_empty(),
        "{} keys were drawn before their rows arrived (layout fell behind the keys): {}",
        run.waited.len(),
        run.waited.join(", ")
    );
    println!(
        "frames: tier={tier:?} refs={refs:?} pane={pane:?} keys over {INPUT_TO_FRAME:?} (recorded; the gate is the p99): {} of {}: {}",
        late.len(),
        total,
        late.join(", ")
    );
    assert!(
        gate.holds(INPUT_TO_FRAME),
        "p99 {:?} is over the {INPUT_TO_FRAME:?} input-to-frame budget at {tier:?} with {refs:?} refs ({} of {} keys over): {}",
        gate.p99,
        late.len(),
        total,
        late.join(", ")
    );
}

/// Prints what each kind of wake cost and the idle-wake keys: recorded,
/// not asserted.
fn print_details(tier: RenderingTier, refs: Refs, run: &Run, idle: &[Duration]) {
    for (trigger, frames, sum) in &run.by_trigger {
        println!(
            "frames: tier={tier:?} refs={refs:?} trigger={trigger} frames={frames} totals: bytes={} transmits={} transmit_bytes={} placements={} deletes={} frees={}",
            sum.bytes, sum.transmits, sum.transmit_bytes, sum.placements, sum.deletes, sum.frees,
        );
    }

    let mut prefetch = run.prefetch.clone();
    prefetch.sort();
    let over = |limit: Duration| prefetch.iter().filter(|spent| **spent >= limit).count();
    println!(
        "frames: tier={tier:?} refs={refs:?} idle_prefetch_us (per frame; recorded, not asserted): frames={} p50={} p99={} max={} total={} over_1ms={} over_2ms={}",
        prefetch.len(),
        prefetch
            .get(prefetch.len() / 2)
            .map_or(0, Duration::as_micros),
        prefetch
            .get((prefetch.len().saturating_sub(1)) * 99 / 100)
            .map_or(0, Duration::as_micros),
        prefetch.last().map_or(0, Duration::as_micros),
        prefetch.iter().sum::<Duration>().as_micros(),
        over(Duration::from_millis(1)),
        over(Duration::from_millis(2)),
    );

    let idle: Vec<_> = idle_wakes()
        .iter()
        .zip(idle)
        .map(|(key, latency)| format!("{key:?}={}", latency.as_micros()))
        .collect();
    println!(
        "frames: tier={tier:?} refs={refs:?} idle_wake_us (first key after {IDLE:?} idle; recorded, not asserted): {}",
        idle.join(" ")
    );
}

/// Prints what one `update`'s copy of the state costs: a clone of `state`
/// and dropping the state it replaces, which every event pays (`update`
/// clones, the loop drops the old one). Recorded, not asserted
/// (m1-the-graph/69 profiled the 50k-ref frames with it).
fn print_state_clone(tier: RenderingTier, refs: Refs, state: &AppState) {
    const LAPS: u32 = 64;
    let mut laps: Vec<Duration> = (0..LAPS)
        .map(|_| {
            let started = Instant::now();
            drop(state.clone());
            started.elapsed()
        })
        .collect();
    laps.sort();
    println!(
        "frames: tier={tier:?} refs={refs:?} state_clone_and_drop_us (recorded, not asserted): laps={LAPS} p50={} max={}",
        laps[laps.len() / 2].as_micros(),
        laps.last().map_or(0, Duration::as_micros),
    );
}

/// What one scroll measured.
struct Run {
    /// Per key, in [`keys`] order: due time → the key's own frame.
    latencies: Vec<Duration>,
    /// Per key, in [`keys`] order: where its latency went.
    breakdown: Vec<String>,
    /// Keys whose own frame lacked the cursor's row or part of the viewport.
    waited: Vec<String>,
    /// Kitty graphics commands written (`ESC _ G`): images at the pixel tier.
    kitty_commands: usize,
    /// Every byte the loop wrote.
    bytes: usize,
    /// Per frame: the idle prefetch after it (the loop's first look for a
    /// wake to its `next_wake`).
    prefetch: Vec<Duration>,
    /// Per trigger (key name, `Window`, `Other`): frames, and what they wrote.
    by_trigger: Vec<(String, usize, Counts)>,
    /// Per key: its phases, as `GRAFT_FRAME_TRACE` lines (see [`Paced::trace`]).
    trace: Vec<String>,
    /// Every non-key wake from the first window of rows to key 1's frame,
    /// and the burst's length (see [`Paced::startup`]).
    startup: Vec<String>,
}

fn scroll(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    pane: Pane,
    control: Control,
    traced: bool,
) -> Result<Run, String> {
    // One channel for job results and keys, as in the app
    // (`impl JobEvents for Sender<Wake>`, event_loop/mod.rs).
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, before_loop) =
        ready_to_scroll(repo, tier, refs, Post(post.clone()), &events)?;

    let mut session = Session::settle(&Loaded::default(), capabilities(tier));
    let sink = Sink::timed(traced);
    let mut terminal = in_memory(&sink, &state)?;

    let watching = Watching {
        inner: &jobs,
        asked: Mutex::default(),
    };
    let initial = state.clone();
    let feed = KeyFeed::start(keys(pane, control), post)
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(
        &events,
        keys(pane, control),
        &watching,
        sink.clone(),
        feed.log(),
    );
    wakes.no_prefetch = control == Control::NoPrefetch;
    let spinner = (control == Control::Busy).then(Spinner::start);
    let exit = pump(&mut terminal, &mut wakes, &watching, &mut session, state);
    drop(spinner);
    // Before any early return: a loop that stopped mid-script still leaves
    // the thread sending into a live channel, so this waits at most the
    // script's remaining pauses.
    let stamps = feed.finish();
    let exit = exit.map_err(|why| format!("the loop failed: {why}"))?;
    stamps?;
    if !matches!(exit, Exit::Quit) {
        return Err(format!(
            "the loop ended with {exit:?}, not at the end of the script"
        ));
    }
    if let Some(failure) = wakes.failure.take() {
        return Err(failure);
    }
    let latencies = wakes.latencies()?;
    let prefetch = std::mem::take(&mut wakes.prefetch);
    // The scroll is over and every latency stamped: only now read what the
    // terminal was sent.
    let (written, counted) = sink.count(&wakes.written);
    let breakdown = wakes.breakdown(&counted);
    let waited = wakes.waited(initial);
    let by_trigger = wakes.written_by_trigger(&counted);
    let trace = wakes.trace(script(pane).len());
    let startup = wakes.startup(&before_loop);

    match jobs.shutdown(Duration::from_secs(10)) {
        Shutdown::Clean => {}
        other @ Shutdown::Stalled { .. } => {
            return Err(format!("the job pool did not stop cleanly: {other:?}"));
        }
    }
    Ok(Run {
        latencies,
        prefetch,
        breakdown,
        waited,
        kitty_commands: written.kitty_commands,
        bytes: written.bytes,
        by_trigger,
        trace,
        startup,
    })
}

/// One non-key wake near startup: what it was, when it was sent and taken,
/// and what the loop spent on it (`draw` is `None` before the loop starts:
/// [`ready_to_scroll`] takes events in without drawing).
#[derive(Clone, Copy)]
struct StartupWake {
    label: &'static str,
    sent: Instant,
    recv: Instant,
    update: Option<Duration>,
    draw: Option<Duration>,
}

/// Opens `repo` with its jobs reporting into `post`, and pumps `events`
/// until the screen is ready to scroll: rows on hand and the refs job
/// answered, so no stray refs answer lands mid-scroll.
///
/// Every event it takes is stamped ([`StartupWake`]) for the startup trace.
fn ready_to_scroll(
    repo: &Path,
    tier: RenderingTier,
    refs: Refs,
    post: Post,
    events: &Receiver<Posted>,
) -> Result<(JobRunner<Post>, AppState, Vec<StartupWake>), String> {
    let (jobs, mut state) = budget_run::open_into(repo, post)?;
    state.settled_tier = Some(tier);
    let ready = |state: &AppState| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .is_some_and(|h| h.window.as_ref().is_some_and(|w| !w.rows.is_empty()))
    };
    // `budget_run::pump_until_from`'s loop, with each event timed.
    let mut taken = Vec::new();
    loop {
        if let Some(history) = &state.history
            && let WalkPhase::Failed(why) = &history.phase
        {
            return Err(format!("the walk failed: {why}"));
        }
        if ready(&state) {
            break;
        }
        let posted = events
            .recv_timeout(PATIENCE)
            .map_err(|why| format!("no event for {PATIENCE:?} ({why}); state: {state:?}"))?;
        let recv = Instant::now();
        let label = posted.wake.label();
        let Wake::Event(event) = posted.wake else {
            return Err(format!("a wake no job sends: {label}"));
        };
        let started = Instant::now();
        let (next, commands) = update(&state, event);
        let update_took = started.elapsed();
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
        taken.push(StartupWake {
            label,
            sent: posted.at,
            recv,
            update: Some(update_took),
            draw: None,
        });
        if let Some(quit) = &state.quit {
            return Err(format!("Graft quit: {quit:?}"));
        }
    }
    if let Refs::FiftyThousand = refs {
        state.refs = Some(Arc::new(fifty_thousand(&state)?));
    }
    print_state_clone(tier, refs, &state);
    Ok((jobs, state, taken))
}

/// A terminal of `state`'s size writing into `sink`.
fn in_memory(sink: &Sink, state: &AppState) -> Result<Terminal<CrosstermBackend<Sink>>, String> {
    Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, state.size.columns, state.size.rows)),
        },
    )
    .map_err(|why| format!("in-memory terminal: {why}"))
}

/// [`REFS`] refs, as `refs::read` would hand them over: half local branches
/// each tracking a remote-tracking ref, half those remote-tracking refs.
/// Nearly all point outside the first screen, as in a real repository with
/// that many; the rows on hand carry a few each, so badges are drawn and
/// looked up on every frame.
fn fifty_thousand(state: &AppState) -> Result<RefBadges, String> {
    let on_screen: Vec<Oid> = state
        .history
        .as_ref()
        .and_then(|h| h.window.as_ref())
        .map(|w| w.rows.iter().map(|row| row.commit.id).collect())
        .unwrap_or_default();
    if on_screen.is_empty() {
        return Err("no rows on hand to put badges on".to_owned());
    }
    let elsewhere = |index: usize| {
        let mut bytes = [0xEEu8; Oid::SHA1_BYTES];
        bytes[..8].copy_from_slice(&(index as u64).to_be_bytes());
        Oid::from_bytes(&bytes).map_err(|why| why.to_string())
    };
    let name = |text: String| RefName::new(text).map_err(|why| why.to_string());

    let branches = REFS / 2;
    let mut refs = Vec::with_capacity(REFS);
    let mut upstreams = HashMap::with_capacity(branches);
    for index in 0..branches {
        // One branch in 100 sits on a row on hand.
        let id = if index % 100 == 0 {
            on_screen[index / 100 % on_screen.len()]
        } else {
            elsewhere(index)?
        };
        let local = name(format!("refs/heads/topic/{index:05}"))?;
        let remote = name(format!("refs/remotes/origin/topic/{index:05}"))?;
        refs.push((local.clone(), id));
        refs.push((remote.clone(), id));
        upstreams.insert(local, remote);
    }
    let head = HeadState::Branch(refs[0].0.clone());

    let started = Instant::now();
    let built = badges(&refs, &head, &upstreams, &[], |_, _| {
        Ok(AheadBehind {
            ahead: 0,
            behind: 0,
        })
    })?;
    let building = started.elapsed();

    let started = Instant::now();
    let mut found = 0;
    for index in 0..branches {
        found += built.on(refs[index * 2].1).len();
    }
    let lookups = started.elapsed();
    println!(
        "refs: {REFS} refs, badges() {building:?}, {branches} lookups {lookups:?} ({} ns each), {found} labels found",
        lookups.as_nanos() / branches as u128
    );
    Ok(built)
}

/// A truecolour terminal that answered every probe, drawing at `tier`; at
/// the pixel tier over Kitty, with the user's measured cell (ADR-0012).
fn capabilities(tier: RenderingTier) -> Capabilities {
    let pixel = tier == RenderingTier::Pixel;
    Capabilities {
        tier,
        pixel_protocol: pixel.then_some(PixelProtocol::Kitty),
        kitty_graphics: if pixel { Supported::Yes } else { Supported::No },
        sixel: Supported::No,
        iterm2_images: false,
        kitty_keyboard: Supported::No,
        synchronized_output: Supported::Yes,
        cell_size: CellSize {
            width: 17,
            height: 37,
            source: CellSizeSource::Reported,
        },
        theme: ThemeHint {
            theme: TerminalTheme::Dark,
            source: ThemeSource::Background(Rgb {
                red: 0x10,
                green: 0x10,
                blue: 0x14,
            }),
        },
        color_depth: ColorDepth::TrueColor,
        no_color: false,
        is_tty: true,
        over_ssh: false,
        tmux: TmuxStatus::Outside,
        probe: ProbeOutcome::Answered,
        probe_budget: Duration::from_millis(250),
    }
}

/// The terminal: keeps every byte it is sent, and only copies them.
///
/// A write is a copy into a log, the way a write to a pty is a copy into the
/// kernel's buffer; parsing is the terminal's own work, which this test
/// leaves out. Frames note only how many bytes had been sent
/// ([`Sink::sent`]); [`Sink::count`] parses the log once, after the scroll,
/// so no timed window pays for the test's own bookkeeping (m1-the-graph/30f:
/// parsing each byte inside `write` added ~3.6 ms to every pixel page key).
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Log>>);

/// The bytes sent, in chunks so that growing the log never moves what is
/// already in it (a reallocation would copy megabytes inside one frame).
#[derive(Default)]
struct Log {
    chunks: Vec<Vec<u8>>,
    len: usize,
    /// Time inside `write`, read only when tracing.
    clock: WriteClock,
}

/// A new chunk's capacity, unless one write is bigger.
const CHUNK: usize = 4 << 20;

impl Sink {
    /// A sink that times its writes when `timed` (tracing is on).
    fn timed(timed: bool) -> Self {
        Self(Arc::new(Mutex::new(Log {
            clock: WriteClock::new(timed),
            ..Log::default()
        })))
    }

    /// Time spent inside `write` so far; zero unless timed.
    fn write_spent(&self) -> Duration {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clock
            .spent()
    }

    /// Bytes sent so far.
    fn sent(&self) -> usize {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).len
    }

    /// Parses everything sent: the totals, and the running [`Counts`] at each
    /// of `marks` (byte offsets from [`Sink::sent`], in order).
    fn count(&self, marks: &[usize]) -> (Written, Vec<Counts>) {
        let log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let mut written = Written::default();
        let mut at = Vec::with_capacity(marks.len());
        let mut marks = marks.iter().peekable();
        let mut offset = 0;
        let mut note = |written: &Written, offset: usize, at: &mut Vec<Counts>| {
            while marks.next_if(|mark| **mark <= offset).is_some() {
                at.push(written.counts);
            }
        };
        for &byte in log.chunks.iter().flatten() {
            note(&written, offset, &mut at);
            written.take(byte);
            offset += 1;
        }
        note(&written, offset, &mut at);
        (written, at)
    }
}

#[derive(Default)]
struct Written {
    bytes: usize,
    kitty_commands: usize,
    counts: Counts,
    parse: Parse,
    control: Vec<u8>,
    /// The kind of the command being read, for its payload bytes.
    current: Option<CommandKind>,
}

/// Running totals of what was written, by kind.
#[derive(Clone, Copy, Default, Debug)]
struct Counts {
    bytes: usize,
    /// `a=t` commands: one per transmitted strip.
    transmits: usize,
    /// Bytes of transmit commands, continuation chunks included.
    transmit_bytes: usize,
    /// `a=p` commands.
    placements: usize,
    /// `a=d,d=i`: placements removed.
    deletes: usize,
    /// `a=d,d=I`: uploads freed.
    frees: usize,
}

impl Counts {
    fn minus(self, before: Counts) -> Counts {
        Counts {
            bytes: self.bytes - before.bytes,
            transmits: self.transmits - before.transmits,
            transmit_bytes: self.transmit_bytes - before.transmit_bytes,
            placements: self.placements - before.placements,
            deletes: self.deletes - before.deletes,
            frees: self.frees - before.frees,
        }
    }

    fn plus(self, other: Counts) -> Counts {
        Counts {
            bytes: self.bytes + other.bytes,
            transmits: self.transmits + other.transmits,
            transmit_bytes: self.transmit_bytes + other.transmit_bytes,
            placements: self.placements + other.placements,
            deletes: self.deletes + other.deletes,
            frees: self.frees + other.frees,
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum CommandKind {
    Transmit,
    Other,
}

/// Where the byte stream is, relative to `ESC _ G <control> [; payload] ESC \`.
#[derive(Default, Clone, Copy, PartialEq)]
enum Parse {
    #[default]
    Outside,
    Esc,
    EscUnderscore,
    Control,
    ControlEsc,
    Payload,
    PayloadEsc,
}

impl Written {
    fn take(&mut self, byte: u8) {
        self.bytes += 1;
        self.counts.bytes += 1;
        if matches!(self.current, Some(CommandKind::Transmit)) {
            self.counts.transmit_bytes += 1;
        }
        self.parse = match (self.parse, byte) {
            (Parse::Outside | Parse::Esc, 0x1b) => Parse::Esc,
            (Parse::Esc, b'_') => Parse::EscUnderscore,
            (Parse::EscUnderscore, b'G') => {
                self.kitty_commands += 1;
                self.control.clear();
                Parse::Control
            }
            (Parse::Outside | Parse::Esc | Parse::EscUnderscore, _) => Parse::Outside,
            (Parse::Control, b';') => {
                self.classify();
                Parse::Payload
            }
            (Parse::Control, 0x1b) => {
                self.classify();
                Parse::ControlEsc
            }
            (Parse::Control, _) => {
                self.control.push(byte);
                Parse::Control
            }
            (Parse::Payload, 0x1b) => Parse::PayloadEsc,
            (Parse::ControlEsc | Parse::PayloadEsc, b'\\') => {
                self.current = None;
                Parse::Outside
            }
            (Parse::Payload | Parse::ControlEsc | Parse::PayloadEsc, _) => Parse::Payload,
        };
    }

    /// The control keys are in: count the command by kind. A continuation
    /// chunk (`m=` only) belongs to the transmit before it.
    fn classify(&mut self) {
        let control = String::from_utf8_lossy(&self.control).into_owned();
        let keys: Vec<&str> = control.split(',').collect();
        let has = |key: &str| keys.contains(&key);
        if has("a=t") {
            self.counts.transmits += 1;
            self.current = Some(CommandKind::Transmit);
        } else if keys.iter().all(|key| key.starts_with("m=")) {
            self.current = Some(CommandKind::Transmit);
        } else if has("a=p") {
            self.counts.placements += 1;
            self.current = Some(CommandKind::Other);
        } else if has("d=i") {
            self.counts.deletes += 1;
            self.current = Some(CommandKind::Other);
        } else if has("d=I") {
            self.counts.frees += 1;
            self.current = Some(CommandKind::Other);
        } else {
            self.current = Some(CommandKind::Other);
        }
    }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut log = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        let Log { chunks, len, clock } = &mut *log;
        clock.time(|| {
            let room = chunks
                .last()
                .map_or(0, |chunk| chunk.capacity() - chunk.len());
            if room < buf.len() {
                chunks.push(Vec::with_capacity(CHUNK.max(buf.len())));
            }
            if let Some(chunk) = chunks.last_mut() {
                chunk.extend_from_slice(buf);
            }
            *len += buf.len();
        });
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The job pool, noting the id of every `ShowRows` the loop sends.
struct Watching<'a, D> {
    inner: &'a D,
    asked: Mutex<Vec<u64>>,
}

impl<D: Dispatch> Dispatch for Watching<'_, D> {
    fn dispatch(&self, command: Command) {
        if let Command::ShowRows(_, request) = &command {
            self.asked
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(request.id);
        }
        self.inner.dispatch(command);
    }
}

impl<D> Watching<'_, D> {
    /// How many `ShowRows` the loop has sent, and the newest id.
    fn asked_so_far(&self) -> (usize, Option<u64>) {
        let asked = self.asked.lock().unwrap_or_else(PoisonError::into_inner);
        (asked.len(), asked.last().copied())
    }

    fn asked(&self) -> Vec<u64> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// What one wake was.
#[derive(Clone, Copy, Debug)]
enum Kind {
    /// The key at this index of the script, sent by the input thread at
    /// this instant: where its latency starts.
    Key(usize, Instant),
    /// A window of rows answering this request.
    Window(u64),
    /// Any other job event.
    Other,
}

/// One wake handed to the loop.
struct Handed {
    kind: Kind,
    /// [`Wake::label`]: what it was, without its payload.
    label: &'static str,
    /// Just before it was sent (the input thread's or a job's send).
    sent: Instant,
    /// When the loop took it: its frame runs from here to the next stamp.
    at: Instant,
    /// A key: the script's tick for it. Due → sent is the input thread's own
    /// wake from its sleep, before the send and outside the latency.
    due: Option<Instant>,
    /// When each [`Phase`] of its frame ended, in `Phase` order.
    marks: [Option<Instant>; 4],
    /// `ShowRows` sent before its frame, and by the end of it.
    asked_before: usize,
    asked_after: usize,
    /// A `ShowRows` was still unanswered when it was handed: the history
    /// job thread was busy while this frame ran.
    rows_out: bool,
    /// Time inside the terminal's `write` before its frame, and by its end.
    wrote_before: Duration,
    wrote_after: Duration,
}

/// Where `phase` sits in [`Handed::marks`].
fn slot(phase: Phase) -> usize {
    match phase {
        Phase::Drained => 0,
        Phase::Updated => 1,
        Phase::CarriedOut => 2,
        Phase::Drawn => 3,
    }
}

/// Wakes for [`pump`]: one channel carrying the script's keys, sent by the
/// [`KeyFeed`] thread on a 60 Hz clock, and job events as they arrive, read
/// the way `ChannelWakes` reads the app's (`event_loop/mod.rs`: `recv` to
/// wait, `try_recv` to look). Every call means the loop has drawn the frame
/// for the wake before it, so each call is stamped.
struct Paced<'a, D> {
    events: &'a Receiver<Posted>,
    keys: Vec<(KeyCode, Duration)>,
    /// Keys received so far: the index of the next one.
    next_key: usize,
    /// When the input thread sent each key.
    sent: SentLog,
    watching: &'a Watching<'a, D>,
    handed: Vec<Handed>,
    /// `replay[i]`: the event `handed[i]` carried, to replay through `update`.
    replay: Vec<AppEvent>,
    /// `drawn[i]`: the frame after `handed[i - 1]` was on the terminal
    /// (`drawn[0]` is the first frame, before any wake).
    drawn: Vec<Instant>,
    /// `written[i]`: bytes the terminal had been sent when `drawn[i]` was
    /// stamped; parsed into [`Counts`] after the scroll ([`Sink::count`]).
    written: Vec<usize>,
    sink: Sink,
    failure: Option<String>,
    /// A wake `waiting` took off the channel to see it; handed out first.
    seen: Option<Posted>,
    /// When the loop first looked for a wake since the last one was handed:
    /// its frame was on the terminal then, before any idle prefetch began.
    looked: Option<Instant>,
    /// Per look: from the loop's first look to its `next_wake` — the idle
    /// prefetch after that frame.
    prefetch: Vec<Duration>,
    /// Control C: say a wake is waiting before any strip is readied.
    no_prefetch: bool,
    /// The newest window of rows handed to the loop.
    answered: Option<u64>,
}

impl Handed {
    fn kind_name(&self) -> String {
        match self.kind {
            Kind::Key(key, _) => format!("key {key}"),
            Kind::Window(request) => format!("window {request}"),
            Kind::Other => "other event".to_owned(),
        }
    }
}

impl<'a, D> Paced<'a, D> {
    fn new(
        events: &'a Receiver<Posted>,
        keys: Vec<(KeyCode, Duration)>,
        watching: &'a Watching<'a, D>,
        sink: Sink,
        sent: SentLog,
    ) -> Self {
        Self {
            events,
            keys,
            next_key: 0,
            sent,
            watching,
            handed: Vec::new(),
            replay: Vec::new(),
            drawn: Vec::new(),
            written: Vec::new(),
            sink,
            failure: None,
            seen: None,
            looked: None,
            prefetch: Vec::new(),
            no_prefetch: false,
            answered: None,
        }
    }

    /// Stamp the frame just drawn, once: at the loop's first look for a wake.
    fn stamp(&mut self) {
        if self.looked.is_none() {
            self.looked = Some(Instant::now());
            self.drawn.push(Instant::now());
            self.written.push(self.sink.sent());
        }
    }

    fn hand(&mut self, kind: Kind, due: Option<Instant>, sent: Instant, event: AppEvent) -> Wake {
        let (asked, newest) = self.watching.asked_so_far();
        if let Kind::Window(id) = kind {
            self.answered = self.answered.max(Some(id));
        }
        let wake = Wake::Event(event);
        self.handed.push(Handed {
            kind,
            label: wake.label(),
            sent,
            at: Instant::now(),
            due,
            marks: [None; 4],
            asked_before: asked,
            asked_after: asked,
            rows_out: newest > self.answered,
            wrote_before: self.sink.write_spent(),
            wrote_after: Duration::ZERO,
        });
        if let Wake::Event(event) = &wake {
            self.replay.push(event.clone());
        }
        wake
    }

    fn event(&mut self, event: AppEvent, sent: Instant) -> Wake {
        let kind = match &event {
            AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) => {
                Kind::Window(window.request)
            }
            _ => Kind::Other,
        };
        self.hand(kind, None, sent, event)
    }

    /// Hands `wake` to the loop: the next script key, or a job event.
    fn take(&mut self, posted: Posted) -> Option<Wake> {
        let event = match posted.wake {
            Wake::Event(AppEvent::Core(Event::Key(press))) => press,
            Wake::Event(event) => return Some(self.event(event, posted.at)),
            other => {
                self.failure = Some(format!("a wake the harness never sends: {other:?}"));
                return None;
            }
        };
        let key = self.next_key;
        let (Some((code, _)), Some(sent)) = (self.keys.get(key), self.sent.get(key)) else {
            self.failure = Some(format!("key {key} arrived with no script entry or stamp"));
            return None;
        };
        if event != KeyPress::plain(*code) {
            self.failure = Some(format!("key {key} arrived as {event:?}, not {code:?}"));
            return None;
        }
        self.next_key += 1;
        let press = AppEvent::Core(Event::Key(event));
        Some(self.hand(Kind::Key(key, sent.at), Some(sent.due), sent.at, press))
    }

    /// What the frame after each wake wrote, summed by what woke it: the
    /// script's key, a window of rows, or another job event.
    /// `counted[i]` is `written[i]` parsed.
    fn written_by_trigger(&self, counted: &[Counts]) -> Vec<(String, usize, Counts)> {
        let keys = &self.keys;
        let mut groups: Vec<(String, usize, Counts)> = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let (Some(before), Some(after)) = (counted.get(index), counted.get(index + 1)) else {
                continue;
            };
            let name = match handed.kind {
                Kind::Key(key, _) => format!("{:?}", keys[key].0),
                Kind::Window(_) => "Window".to_owned(),
                Kind::Other => "Other".to_owned(),
            };
            let frame = after.minus(*before);
            match groups.iter_mut().find(|(group, ..)| *group == name) {
                Some((_, frames, sum)) => {
                    *frames += 1;
                    *sum = sum.plus(frame);
                }
                None => groups.push((name, 1, frame)),
            }
        }
        groups
    }

    /// Each key's latency: its send → its own frame, the one drawn right
    /// after it. A key draws from the rows on hand ([`Paced::waited`] checks
    /// they were there); rows it asked for arrive later as their own frame.
    fn latencies(&self) -> Result<Vec<Duration>, String> {
        let mut latencies = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(_, sent) = handed.kind else {
                continue;
            };
            let drawn = self
                .drawn
                .get(index + 1)
                .ok_or_else(|| format!("no frame was drawn after wake {index}"))?;
            latencies.push(drawn.saturating_duration_since(sent));
        }
        Ok(latencies)
    }

    /// Per key, in [`keys`] order, where its latency went: time queued
    /// behind the wake before it (and what that wake was), then the key's
    /// own frame and what it wrote. Printed for late keys.
    /// `counted[i]` is `written[i]` parsed.
    fn breakdown(&self, counted: &[Counts]) -> Vec<String> {
        let mut lines = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(_, sent) = handed.kind else {
                continue;
            };
            let (Some(drawn), Some(before), Some(after)) = (
                self.drawn.get(index + 1),
                counted.get(index),
                counted.get(index + 1),
            ) else {
                lines.push("no frame drawn".to_owned());
                continue;
            };
            let previous = index.checked_sub(1).and_then(|at| {
                let wake = self.handed.get(at)?;
                let ended = self.drawn.get(at + 1)?;
                Some(format!(
                    "{} frame {:?}",
                    wake.kind_name(),
                    ended.saturating_duration_since(wake.at)
                ))
            });
            let frame = after.minus(*before);
            lines.push(format!(
                "queued {:?} behind {}; own frame {:?}, bytes={} transmits={} placements={}",
                handed.at.saturating_duration_since(sent),
                previous.unwrap_or_else(|| "nothing".to_owned()),
                drawn.saturating_duration_since(handed.at),
                frame.bytes,
                frame.transmits,
                frame.placements,
            ));
        }
        lines
    }

    /// Per key: one `GRAFT_FRAME_TRACE` line of its phases, in µs.
    /// `latency`: send → the key's frame. `oversleep`: the input thread's
    /// own wake past the due tick, before the send, so outside `latency`
    /// (kept so the OS timer slack stays visible). `queued`: send → handed
    /// to the loop (the channel hand-over, plus any frame ahead). Then the
    /// loop's phases: `drain`, `update`, `carry` (dispatch), `draw` (view,
    /// render, and `write`, the part inside the terminal's `write`), and
    /// `tail`, `Drawn` → the stamp the latency ends at. `asked`: the key
    /// sent a `ShowRows`; `rows_out`: one was unanswered when it was handed.
    /// Keys at or past `scrolled` are the idle keys.
    fn trace(&self, scrolled: usize) -> Vec<String> {
        let micros = |from: Option<Instant>, to: Option<Instant>| match (from, to) {
            (Some(from), Some(to)) => to.saturating_duration_since(from).as_micros().to_string(),
            _ => "-".to_owned(),
        };
        let mut lines = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(key, sent) = handed.kind else {
                continue;
            };
            let drawn = self.drawn.get(index + 1).copied();
            let [drained, updated, carried, painted] = handed.marks;
            let behind = index.checked_sub(1).and_then(|at| {
                let wake = self.handed.get(at)?;
                let ended = self.drawn.get(at + 1)?;
                Some(format!(
                    "{}:{}",
                    wake.kind_name().replace(' ', "_"),
                    ended.saturating_duration_since(wake.at).as_micros()
                ))
            });
            lines.push(format!(
                "key={key} code={:?} kind={} latency_us={} oversleep_us={} queued_us={} drain_us={} update_us={} carry_us={} draw_us={} write_us={} tail_us={} asked={} rows_out={} behind={}",
                self.keys[key].0,
                if key >= scrolled { "idle" } else { "scroll" },
                micros(Some(sent), drawn),
                micros(handed.due, Some(sent)),
                micros(Some(sent), Some(handed.at)),
                micros(Some(handed.at), drained),
                micros(drained, updated),
                micros(updated, carried),
                micros(carried, painted),
                handed
                    .wrote_after
                    .saturating_sub(handed.wrote_before)
                    .as_micros(),
                micros(painted, drawn),
                u8::from(handed.asked_after > handed.asked_before),
                u8::from(handed.rows_out),
                behind.unwrap_or_else(|| "nothing".to_owned()),
            ));
        }
        lines
    }

    /// The startup trace (m7-power-tools/12): every non-key wake from the
    /// first window of rows to key 1's frame — `before_loop` (taken in by
    /// [`ready_to_scroll`], not drawn) then the loop's own — one line each:
    /// `sent`/`recv` in µs after the first window was sent, then what the
    /// loop spent on it. Then one line: `burst_after_first_paint_us`, the
    /// first window's send to the last of these wakes' send (in the app the
    /// first window is painted as it lands, so a key pressed inside this span
    /// queues behind the rest of the burst), and `loop_start_us`, when this
    /// harness's loop drew its first frame (key 0 is sent then).
    fn startup(&self, before_loop: &[StartupWake]) -> Vec<String> {
        let key_one = self
            .handed
            .iter()
            .position(|handed| matches!(handed.kind, Kind::Key(1, _)))
            .unwrap_or(self.handed.len());
        let span = |from: Option<Instant>, to: Option<Instant>| match (from, to) {
            (Some(from), Some(to)) => Some(to.saturating_duration_since(from)),
            _ => None,
        };
        let in_loop = self.handed[..key_one]
            .iter()
            .filter(|handed| !matches!(handed.kind, Kind::Key(..)))
            .map(|handed| {
                let [drained, updated, carried, drawn] = handed.marks;
                StartupWake {
                    label: handed.label,
                    sent: handed.sent,
                    recv: handed.at,
                    update: span(drained, updated),
                    draw: span(carried, drawn),
                }
            });
        let mut wakes: Vec<StartupWake> = Vec::new();
        wakes.extend(before_loop.iter().copied());
        wakes.extend(in_loop);
        let Some(first) = wakes
            .iter()
            .position(|wake| wake.label == "Progress::History::Window")
        else {
            return vec!["startup no window of rows was taken".to_owned()];
        };
        let wakes = &wakes[first..];
        let origin = wakes[0].sent;
        let micros = |at: Instant| at.saturating_duration_since(origin).as_micros();
        let took =
            |spent: Option<Duration>| spent.map_or("-".to_owned(), |d| d.as_micros().to_string());
        let mut lines: Vec<String> = wakes
            .iter()
            .enumerate()
            .map(|(index, wake)| {
                format!(
                    "startup i={index} kind={} sent_us={} recv_us={} update_us={} draw_us={}",
                    wake.label,
                    micros(wake.sent),
                    micros(wake.recv),
                    took(wake.update),
                    took(wake.draw),
                )
            })
            .collect();
        let last_sent = wakes.iter().map(|wake| wake.sent).max().unwrap_or(origin);
        lines.push(format!(
            "startup burst_after_first_paint_us={} wakes={} loop_start_us={}",
            micros(last_sent),
            wakes.len(),
            self.drawn
                .first()
                .map_or("-".to_owned(), |at| micros(*at).to_string()),
        ));
        lines
    }

    /// The keys whose own frame was drawn before their rows arrived: replays every
    /// wake through `update` from the state the loop started with, and after
    /// each key checks that the cursor's row and the whole viewport were
    /// already on hand. `update` is pure, so this is the state each key's
    /// frame was drawn from. The text tiers draw exactly those rows; the
    /// pixel tier's strips widen them (`History::drawn`), not checked here.
    fn waited(&self, mut state: AppState) -> Vec<String> {
        let keys = &self.keys;
        let mut waited = Vec::new();
        for (handed, event) in self.handed.iter().zip(&self.replay) {
            state = update(&state, event.clone()).0;
            let Kind::Key(key, _) = handed.kind else {
                continue;
            };
            let height = state.graph_rows();
            let Some(history) = state.history.as_ref() else {
                waited.push(format!("key {key}: no history on screen"));
                continue;
            };
            let on_screen = height.min(history.rows.saturating_sub(history.top));
            if history.selected().is_none() || history.visible(height).len() < on_screen {
                waited.push(format!(
                    "key {key} ({:?}): cursor {}, top {}, {} of {on_screen} rows on hand",
                    keys[key].0,
                    history.cursor,
                    history.top,
                    history.visible(height).len()
                ));
            }
        }
        waited
    }
}

impl<D> Wakes for Paced<'_, D> {
    fn next_wake(&mut self) -> Option<Wake> {
        self.stamp();
        if let Some(looked) = self.looked.take() {
            self.prefetch.push(looked.elapsed());
        }
        if let Some(wake) = self.seen.take() {
            return self.take(wake);
        }
        if self.next_key < self.keys.len() {
            // As the app's loop waits: blocked in `recv` until a job or
            // the input thread sends.
            return match self.events.recv() {
                Ok(wake) => self.take(wake),
                Err(_hung_up) => {
                    self.failure =
                        Some("the job pool and the input thread hung up mid-scroll".to_owned());
                    None
                }
            };
        }
        // The script is over: wait only for rows still out.
        let newest_asked = self.watching.asked().iter().max().copied();
        let newest_answered = self
            .handed
            .iter()
            .filter_map(|handed| match handed.kind {
                Kind::Window(id) => Some(id),
                _ => None,
            })
            .max();
        if newest_asked.is_none() || newest_answered >= newest_asked {
            return None;
        }
        match self.events.recv_timeout(PATIENCE) {
            Ok(wake) => self.take(wake),
            Err(why) => {
                self.failure = Some(format!("rows still out after {PATIENCE:?}: {why}"));
                None
            }
        }
    }

    /// A key or an event has come: the loop stops readying strips, so a
    /// key's latency counts the one strip it waited behind.
    fn waiting(&mut self) -> bool {
        self.looked() || self.no_prefetch
    }

    fn mark(&mut self, phase: Phase) {
        let now = Instant::now();
        let after = (phase == Phase::Drawn)
            .then(|| (self.watching.asked_so_far().0, self.sink.write_spent()));
        if let Some(handed) = self.handed.last_mut() {
            handed.marks[slot(phase)] = Some(now);
            if let Some((asked, wrote)) = after {
                handed.asked_after = asked;
                handed.wrote_after = wrote;
            }
        }
    }
}

impl<D> Paced<'_, D> {
    fn looked(&mut self) -> bool {
        self.stamp();
        if self.seen.is_some() {
            return true;
        }
        // The script is over: nothing left to measure, so nothing to ready.
        if self.next_key >= self.keys.len() {
            return true;
        }
        // As `ChannelWakes::waiting` looks (event_loop/mod.rs).
        match self.events.try_recv() {
            Ok(wake) => {
                self.seen = Some(wake);
                true
            }
            Err(TryRecvError::Empty) => false,
            // `next_wake` records the hang-up.
            Err(TryRecvError::Disconnected) => true,
        }
    }
}

/// Control D (m7-power-tools/04): the machine alone, no Graft code at all.
/// Each lap sleeps [`IDLE`] the way the harness did before 04b's
/// [`KeyFeed`] (`recv_timeout` on a quiet channel), then times a fixed piece of CPU and memory work at once
/// (`after_idle`) and again straight after (`warm`), and times a wake
/// handed over from another thread after the same pause (`handoff`, the
/// real loop's shape: the input thread sends, the loop's `recv` returns).
/// With and without a core held busy (control B). `tick_oversleep`: the
/// same timed wait at the scroll's one-tick length. Recorded, not asserted.
#[test]
fn the_machine_alone_after_idle_and_warm() {
    const LAPS: usize = 8;
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    // 4 MiB: bigger than a core's L1/L2 share, like a frame's working set.
    let buffer: Vec<u64> = (0..(4 << 20) / 8).collect();
    let work = || {
        let started = Instant::now();
        let mut sum = 0u64;
        for pass in 0..4u64 {
            for value in &buffer {
                sum = sum.wrapping_mul(31).wrapping_add(value ^ pass);
            }
        }
        std::hint::black_box(sum);
        started.elapsed()
    };
    for busy in [false, true] {
        let spinner = busy.then(Spinner::start);
        let (mut oversleep, mut after_idle, mut warm, mut handoff) =
            (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for _ in 0..LAPS {
            oversleep.push(timed_wait(IDLE));
            after_idle.push(work());
            warm.push(work());
            handoff.push(handoff_after(IDLE));
        }
        // The same timed wait at the scroll's pace: one tick, 30 times.
        let tick_oversleep: Vec<Duration> = (0..30).map(|_| timed_wait(TICK)).collect();
        drop(spinner);
        let list = |laps: &[Duration]| {
            laps.iter()
                .map(|lap| lap.as_micros().to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        println!(
            "machine: busy={busy} oversleep_us={} tick_oversleep_us={} work_after_idle_us={} work_warm_us={} handoff_us={}",
            list(&oversleep),
            list(&tick_oversleep),
            list(&after_idle),
            list(&warm),
            list(&handoff),
        );
    }
    drop(turn);
}

/// Waits `wait` the way [`Paced`] does, on a channel nobody sends to, and
/// returns how late the thread got back.
fn timed_wait(wait: Duration) -> Duration {
    let (_quiet, never) = mpsc::channel::<()>();
    let due = Instant::now() + wait;
    match never.recv_timeout(wait) {
        Err(RecvTimeoutError::Timeout) => {}
        other => panic!("a quiet channel answered: {other:?}"),
    }
    Instant::now().saturating_duration_since(due)
}

/// Another thread sleeps `pause` then sends; returns send → `recv` back.
fn handoff_after(pause: Duration) -> Duration {
    let (post, inbox) = mpsc::channel::<Instant>();
    let sender = thread::spawn(move || {
        thread::sleep(pause);
        post.send(Instant::now())
    });
    let sent = inbox
        .recv()
        .unwrap_or_else(|why| panic!("the sender hung up: {why}"));
    let handoff = sent.elapsed();
    sender
        .join()
        .unwrap_or_else(|_| panic!("the sender panicked"))
        .unwrap_or_else(|why| panic!("the send failed: {why}"));
    handoff
}

/// The measurement change must not change what is counted: parsing the log
/// after the run gives, at every frame mark, exactly what parsing each write
/// as it arrived gave (the Sink before m1-the-graph/30f). Commands split
/// across writes, a continuation chunk, and a write bigger than a chunk.
#[test]
fn the_sink_counts_after_the_run_what_parsing_each_write_counted_during_it() {
    let transmit_payload = "A".repeat(CHUNK + 10);
    let stream: Vec<String> = vec![
        "plain text \x1b[31m red".to_owned(),
        format!("\x1b_Ga=t,f=32,i=1,m=1;{transmit_payload}\x1b\\"),
        "\x1b_Gm=0;QUJD\x1b\\".to_owned(),
        "\x1b_Ga=p,i=1,p=1\x1b\\\x1b_Ga=d,d=i,i=1\x1b\\".to_owned(),
        "\x1b_Ga=d,d=I,i=1\x1b\\\x1b_Gq=2\x1b\\ tail".to_owned(),
    ];
    let bytes: Vec<u8> = stream.concat().into_bytes();
    // Split everywhere interesting: inside ESC _ G, inside the control keys,
    // inside the payload, between ESC and '\'.
    let cuts = [0, 3, 13, 14, 15, 22, 40, CHUNK, CHUNK + 30, CHUNK + 33];
    let cuts: Vec<usize> = cuts
        .into_iter()
        .chain((CHUNK + 40..bytes.len()).step_by(7))
        .chain([bytes.len()])
        .collect();

    let mut sink = Sink::default();
    let mut during = Written::default();
    let mut expected = vec![during.counts];
    let mut marks = vec![0];
    for pair in cuts.windows(2) {
        let piece = &bytes[pair[0]..pair[1]];
        sink.write_all(piece).unwrap_or_else(|why| panic!("{why}"));
        for &byte in piece {
            during.take(byte);
        }
        expected.push(during.counts);
        marks.push(sink.sent());
    }

    let (after, counted) = sink.count(&marks);
    assert_eq!(sink.sent(), bytes.len(), "every byte kept");
    assert_eq!(after.bytes, during.bytes);
    assert_eq!(after.kitty_commands, during.kitty_commands);
    assert_eq!(after.kitty_commands, 6);
    let (transmits, placements, deletes, frees) = (1, 1, 1, 1);
    let last = counted.last().copied().unwrap_or_default();
    assert_eq!(
        (last.transmits, last.placements, last.deletes, last.frees),
        (transmits, placements, deletes, frees)
    );
    assert_eq!(format!("{counted:?}"), format!("{expected:?}"));
}
