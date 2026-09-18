//! Input to frame on the 100k-commit fixture (PROMPT.md §1.2 "scroll at 60
//! fps", §6.3 "input to frame: under 16 ms"; DESIGN.md §3.8), asserted at
//! the Unicode and pixel tiers, and again with 50k refs on the badges
//! (PROMPT.md §6.3 edge cases). Ticket m1-the-graph/30.
//!
//! The real loop (`event_loop::pump`) runs over the real job pool and an
//! in-memory terminal. Keys arrive on a 60 Hz clock, the way a held key or
//! a fast hand scrolls, and each key's latency is measured from the tick it
//! was *due* on — so a loop that falls behind shows up as growing lateness,
//! not hides behind a queue. A key's frame is the first frame that shows the
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
use std::sync::mpsc::{Receiver, RecvTimeoutError, TryRecvError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use graft::config::Loaded;
use graft::event_loop::{Exit, Wake, Wakes, pump};
use graft::history::HistoryUpdate;
use graft::jobs::{Dispatch, Shutdown};
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
}

/// The scroll: down a line at a time, down a page at a time far past the
/// first layout chunks, then back up a page at a time.
fn script(pane: Pane) -> Vec<KeyCode> {
    let mut keys = match pane {
        Pane::Closed => Vec::new(),
        Pane::Open => vec![KeyCode::Enter],
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

/// Every key the run presses, with the pause before it.
fn keys(pane: Pane) -> Vec<(KeyCode, Duration)> {
    let scroll = script(pane).into_iter().map(|key| (key, TICK));
    let idle = idle_wakes().into_iter().map(|key| (key, IDLE));
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
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let run = scroll(&repo, tier, refs, pane).unwrap_or_else(|why| panic!("{why}"));
    drop(turn);

    assert_eq!(
        run.latencies.len(),
        keys(pane).len(),
        "every key got a frame"
    );
    let (scrolled, idle) = run.latencies.split_at(script(pane).len());
    let mut sorted = scrolled.to_vec();
    sorted.sort();
    // Rank rounded down; a few hundred samples.
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    let slowest = sorted.last().copied().unwrap_or_default();
    println!(
        "frames: tier={tier:?} refs={refs:?} pane={pane:?} keys={} p50_us={} p99_us={} max_us={} kitty_commands={} bytes={}",
        sorted.len(),
        at(50).as_micros(),
        at(99).as_micros(),
        slowest.as_micros(),
        run.kitty_commands,
        run.bytes,
    );

    print_details(tier, refs, &run, idle);
    match tier {
        RenderingTier::Pixel => assert!(run.kitty_commands > 0, "the pixel tier drew no images"),
        _ => assert_eq!(run.kitty_commands, 0, "a text tier sent images"),
    }
    judge(tier, refs, pane, &run, scrolled, sorted.len());
}

/// Release enforces the input-to-frame budget, debug records it
/// (ADR-0021). `total` is how many keys the scroll pressed.
fn judge(
    tier: RenderingTier,
    refs: Refs,
    pane: Pane,
    run: &Run,
    scrolled: &[Duration],
    total: usize,
) {
    let late: Vec<_> = scrolled
        .iter()
        .enumerate()
        .filter(|(_, latency)| **latency >= INPUT_TO_FRAME)
        .map(|(key, latency)| format!("key {key}: {latency:?} ({})", run.breakdown[key]))
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
    assert!(
        late.is_empty(),
        "{} of {} keys missed the {INPUT_TO_FRAME:?} input-to-frame budget at {tier:?} with {refs:?} refs: {}",
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
}

fn scroll(repo: &Path, tier: RenderingTier, refs: Refs, pane: Pane) -> Result<Run, String> {
    let (jobs, events, mut state) = budget_run::open(repo)?;
    state.settled_tier = Some(tier);
    // The screen ready to scroll: rows on hand and the refs job answered, so
    // no stray refs answer lands mid-scroll.
    let ready = |state: &AppState| {
        state.refs.is_some()
            && state
                .history
                .as_ref()
                .is_some_and(|h| h.window.as_ref().is_some_and(|w| !w.rows.is_empty()))
    };
    state = budget_run::pump_until(&jobs, &events, state, ready)?;
    if let Refs::FiftyThousand = refs {
        state.refs = Some(Arc::new(fifty_thousand(&state)?));
    }
    print_state_clone(tier, refs, &state);

    let mut session = Session::settle(&Loaded::default(), capabilities(tier));
    let sink = Sink::default();
    let mut terminal = Terminal::with_options(
        CrosstermBackend::new(sink.clone()),
        TerminalOptions {
            viewport: Viewport::Fixed(Rect::new(0, 0, state.size.columns, state.size.rows)),
        },
    )
    .map_err(|why| format!("in-memory terminal: {why}"))?;

    let watching = Watching {
        inner: &jobs,
        asked: Mutex::default(),
    };
    let initial = state.clone();
    let mut wakes = Paced::new(&events, keys(pane), &watching, sink.clone());
    let exit = pump(&mut terminal, &mut wakes, &watching, &mut session, state)
        .map_err(|why| format!("the loop failed: {why}"))?;
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
    })
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
    let built = badges(&refs, &head, &upstreams, |_, _| {
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
}

/// A new chunk's capacity, unless one write is bigger.
const CHUNK: usize = 4 << 20;

impl Sink {
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
        let room = log
            .chunks
            .last()
            .map_or(0, |chunk| chunk.capacity() - chunk.len());
        if room < buf.len() {
            log.chunks.push(Vec::with_capacity(CHUNK.max(buf.len())));
        }
        if let Some(chunk) = log.chunks.last_mut() {
            chunk.extend_from_slice(buf);
        }
        log.len += buf.len();
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
    /// The key at this index of the script, due at this tick.
    Key(usize, Instant),
    /// A window of rows answering this request.
    Window(u64),
    /// Any other job event.
    Other,
}

/// One wake handed to the loop.
struct Handed {
    kind: Kind,
    /// When the loop took it: its frame runs from here to the next stamp.
    at: Instant,
}

/// Wakes for [`pump`]: the script's keys on a 60 Hz clock, job events as
/// they arrive between ticks. Every call means the loop has drawn the frame
/// for the wake before it, so each call is stamped.
struct Paced<'a, D> {
    events: &'a Receiver<AppEvent>,
    keys: Vec<(KeyCode, Duration)>,
    next_key: usize,
    due: Instant,
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
    /// An event `waiting` took off the channel to see it; handed out first.
    seen: Option<AppEvent>,
    /// When the loop first looked for a wake since the last one was handed:
    /// its frame was on the terminal then, before any idle prefetch began.
    looked: Option<Instant>,
    /// Per look: from the loop's first look to its `next_wake` — the idle
    /// prefetch after that frame.
    prefetch: Vec<Duration>,
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
        events: &'a Receiver<AppEvent>,
        keys: Vec<(KeyCode, Duration)>,
        watching: &'a Watching<'a, D>,
        sink: Sink,
    ) -> Self {
        Self {
            events,
            keys,
            next_key: 0,
            due: Instant::now(),
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

    fn hand(&mut self, kind: Kind, event: AppEvent) -> Wake {
        self.handed.push(Handed {
            kind,
            at: Instant::now(),
        });
        self.replay.push(event.clone());
        Wake::Event(event)
    }

    fn event(&mut self, event: AppEvent) -> Wake {
        let kind = match &event {
            AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) => {
                Kind::Window(window.request)
            }
            _ => Kind::Other,
        };
        self.hand(kind, event)
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

    /// Each key's latency: due time → its own frame, the one drawn right
    /// after it. A key draws from the rows on hand ([`Paced::waited`] checks
    /// they were there); rows it asked for arrive later as their own frame.
    fn latencies(&self) -> Result<Vec<Duration>, String> {
        let mut latencies = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(_, due) = handed.kind else {
                continue;
            };
            let drawn = self
                .drawn
                .get(index + 1)
                .ok_or_else(|| format!("no frame was drawn after wake {index}"))?;
            latencies.push(drawn.saturating_duration_since(due));
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
            let Kind::Key(_, due) = handed.kind else {
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
                handed.at.saturating_duration_since(due),
                previous.unwrap_or_else(|| "nothing".to_owned()),
                drawn.saturating_duration_since(handed.at),
                frame.bytes,
                frame.transmits,
                frame.placements,
            ));
        }
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
        if let Some(event) = self.seen.take() {
            return Some(self.event(event));
        }
        if self.next_key < self.keys.len() {
            let wait = self.due.saturating_duration_since(Instant::now());
            if !wait.is_zero() {
                match self.events.recv_timeout(wait) {
                    Ok(event) => return Some(self.event(event)),
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => {
                        self.failure = Some("the job pool hung up mid-scroll".to_owned());
                        return None;
                    }
                }
            }
            let key = self.next_key;
            let due = self.due;
            self.next_key += 1;
            if let Some((_, pause)) = self.keys.get(self.next_key) {
                self.due += *pause;
            }
            let press = KeyPress::plain(self.keys[key].0);
            return Some(self.hand(Kind::Key(key, due), AppEvent::Core(Event::Key(press))));
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
            Ok(event) => Some(self.event(event)),
            Err(why) => {
                self.failure = Some(format!("rows still out after {PATIENCE:?}: {why}"));
                None
            }
        }
    }

    /// The next key is due, or an event has come: the loop stops readying
    /// strips, so a key's latency counts the one strip it waited behind.
    fn waiting(&mut self) -> bool {
        self.looked()
    }
}

impl<D> Paced<'_, D> {
    fn looked(&mut self) -> bool {
        self.stamp();
        if self.seen.is_some() {
            return true;
        }
        // The script is over: nothing left to measure, so nothing to ready.
        if self.next_key >= self.keys.len() || Instant::now() >= self.due {
            return true;
        }
        match self.events.try_recv() {
            Ok(event) => {
                self.seen = Some(event);
                true
            }
            Err(TryRecvError::Empty) => false,
            // `next_wake` records the hang-up.
            Err(TryRecvError::Disconnected) => true,
        }
    }
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
