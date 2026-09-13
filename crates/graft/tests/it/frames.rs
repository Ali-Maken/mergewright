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
//! back-pressure on a real pty. Heavy tier only (`heavy-tests`, ADR-0018).
//!
//! The budget fails only in a release build (ADR-0021): PROMPT.md's 16 ms is
//! a claim about the shipped binary. A debug build runs the same scroll and
//! prints every late key, recorded and not failing.

use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use graft::config::Loaded;
use graft::event_loop::{Exit, Wake, Wakes, pump};
use graft::history::HistoryUpdate;
use graft::jobs::{Dispatch, Shutdown};
use graft::refs::{RefBadges, badges};
use graft::session::Session;
use graft::update::{AppEvent, AppState, Command, Progress};
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

/// How long the end of a run waits for rows still out. Far above any
/// budget: it turns a hang into a named failure, it is not a threshold.
const PATIENCE: Duration = Duration::from_secs(60);

/// PROMPT.md §6.3: "repos with 50k+ refs".
const REFS: usize = 50_000;

#[test]
fn scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_unicode_tier() {
    check(RenderingTier::Unicode, Refs::Fixture);
}

#[test]
fn scrolling_the_100k_fixture_keeps_input_to_frame_under_16_ms_at_the_pixel_tier() {
    check(RenderingTier::Pixel, Refs::Fixture);
}

#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_unicode_tier() {
    check(RenderingTier::Unicode, Refs::FiftyThousand);
}

#[test]
fn fifty_thousand_refs_keep_input_to_frame_under_16_ms_at_the_pixel_tier() {
    check(RenderingTier::Pixel, Refs::FiftyThousand);
}

/// Which badges the scroll draws.
#[derive(Clone, Copy, Debug)]
enum Refs {
    /// The fixture's own refs, as the refs job read them.
    Fixture,
    /// [`REFS`] refs in place of them (see [`fifty_thousand`]).
    FiftyThousand,
}

/// The scroll: down a line at a time, down a page at a time far past the
/// first layout chunks, then back up a page at a time.
fn script() -> Vec<KeyCode> {
    let mut keys = vec![KeyCode::Char('j'); 120];
    keys.extend(vec![KeyCode::PageDown; 240]);
    keys.extend(vec![KeyCode::PageUp; 60]);
    keys
}

fn check(tier: RenderingTier, refs: Refs) {
    let repo =
        synthetic::borrow(Scale::HEAVY.with_commit_graph()).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let run = scroll(&repo, tier, refs).unwrap_or_else(|why| panic!("{why}"));
    drop(turn);

    let mut sorted = run.latencies.clone();
    sorted.sort();
    // Rank rounded down; a few hundred samples.
    let at = |percent: usize| sorted[(sorted.len() - 1) * percent / 100];
    let slowest = sorted.last().copied().unwrap_or_default();
    println!(
        "frames: tier={tier:?} refs={refs:?} keys={} p50_us={} p99_us={} max_us={} kitty_commands={} bytes={}",
        sorted.len(),
        at(50).as_micros(),
        at(99).as_micros(),
        slowest.as_micros(),
        run.kitty_commands,
        run.bytes,
    );

    for (trigger, frames, sum) in &run.by_trigger {
        println!(
            "frames: tier={tier:?} refs={refs:?} trigger={trigger} frames={frames} totals: bytes={} transmits={} transmit_bytes={} placements={} deletes={} frees={}",
            sum.bytes, sum.transmits, sum.transmit_bytes, sum.placements, sum.deletes, sum.frees,
        );
    }

    assert_eq!(sorted.len(), script().len(), "every key got a frame");
    match tier {
        RenderingTier::Pixel => assert!(run.kitty_commands > 0, "the pixel tier drew no images"),
        _ => assert_eq!(run.kitty_commands, 0, "a text tier sent images"),
    }
    let late: Vec<_> = run
        .latencies
        .iter()
        .enumerate()
        .filter(|(_, latency)| **latency >= INPUT_TO_FRAME)
        .map(|(key, latency)| format!("key {key}: {latency:?}"))
        .collect();
    if cfg!(debug_assertions) {
        // ADR-0021: debug records the budget, release enforces it.
        println!(
            "frames: tier={tier:?} refs={refs:?} debug build, budget recorded not asserted: {} of {} keys at or over {INPUT_TO_FRAME:?}: {}",
            late.len(),
            sorted.len(),
            late.join(", ")
        );
        return;
    }
    assert!(
        late.is_empty(),
        "{} of {} keys missed the {INPUT_TO_FRAME:?} input-to-frame budget at {tier:?} with {refs:?} refs: {}",
        late.len(),
        sorted.len(),
        late.join(", ")
    );
}

/// What one scroll measured.
struct Run {
    /// Per key, in script order: due tick → the frame showing its rows.
    latencies: Vec<Duration>,
    /// Kitty graphics commands written (`ESC _ G`): images at the pixel tier.
    kitty_commands: usize,
    /// Every byte the loop wrote.
    bytes: usize,
    /// Per trigger (key name, `Window`, `Other`): frames, and what they wrote.
    by_trigger: Vec<(String, usize, Counts)>,
}

fn scroll(repo: &Path, tier: RenderingTier, refs: Refs) -> Result<Run, String> {
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
        state.refs = Some(fifty_thousand(&state)?);
    }

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
    let mut wakes = Paced::new(&events, script(), &watching, sink.clone());
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
    let by_trigger = wakes.written_by_trigger();

    match jobs.shutdown(Duration::from_secs(10)) {
        Shutdown::Clean => {}
        other @ Shutdown::Stalled { .. } => {
            return Err(format!("the job pool did not stop cleanly: {other:?}"));
        }
    }
    let written = sink.0.lock().unwrap_or_else(PoisonError::into_inner);
    Ok(Run {
        latencies,
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

/// The terminal: counts what was written, by Kitty command kind, and keeps
/// none of it.
#[derive(Clone, Default)]
struct Sink(Arc<Mutex<Written>>);

impl Sink {
    fn snapshot(&self) -> Counts {
        self.0.lock().unwrap_or_else(PoisonError::into_inner).counts
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
        let mut written = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        written.bytes += buf.len();
        written.counts.bytes += buf.len();
        for &byte in buf {
            written.take(byte);
        }
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
    /// `ShowRows` requests sent before this wake was handed over.
    asked_before: usize,
}

/// Wakes for [`pump`]: the script's keys on a 60 Hz clock, job events as
/// they arrive between ticks. Every call means the loop has drawn the frame
/// for the wake before it, so each call is stamped.
struct Paced<'a, D> {
    events: &'a Receiver<AppEvent>,
    keys: Vec<KeyCode>,
    next_key: usize,
    due: Instant,
    watching: &'a Watching<'a, D>,
    handed: Vec<Handed>,
    /// `drawn[i]`: the frame after `handed[i - 1]` was on the terminal
    /// (`drawn[0]` is the first frame, before any wake).
    drawn: Vec<Instant>,
    /// `written[i]`: what the terminal had been sent when `drawn[i]` was stamped.
    written: Vec<Counts>,
    sink: Sink,
    failure: Option<String>,
}

impl<'a, D> Paced<'a, D> {
    fn new(
        events: &'a Receiver<AppEvent>,
        keys: Vec<KeyCode>,
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
            drawn: Vec::new(),
            written: Vec::new(),
            sink,
            failure: None,
        }
    }

    fn hand(&mut self, kind: Kind, wake: Wake) -> Wake {
        self.handed.push(Handed {
            kind,
            asked_before: self.watching.asked().len(),
        });
        wake
    }

    fn event(&mut self, event: AppEvent) -> Wake {
        let kind = match &event {
            AppEvent::Progress(_, Progress::History(HistoryUpdate::Window(window))) => {
                Kind::Window(window.request)
            }
            _ => Kind::Other,
        };
        self.hand(kind, Wake::Event(event))
    }

    /// What the frame after each wake wrote, summed by what woke it: the
    /// script's key, a window of rows, or another job event.
    fn written_by_trigger(&self) -> Vec<(String, usize, Counts)> {
        let keys = script();
        let mut groups: Vec<(String, usize, Counts)> = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let (Some(before), Some(after)) =
                (self.written.get(index), self.written.get(index + 1))
            else {
                continue;
            };
            let name = match handed.kind {
                Kind::Key(key, _) => format!("{:?}", keys[key]),
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

    /// Each key's latency: due tick → the frame drawn after it, or after the
    /// window answering the newest request it sent.
    fn latencies(&self) -> Result<Vec<Duration>, String> {
        let asked = self.watching.asked();
        let mut latencies = Vec::new();
        for (index, handed) in self.handed.iter().enumerate() {
            let Kind::Key(key, due) = handed.kind else {
                continue;
            };
            let asked_after = self
                .handed
                .get(index + 1)
                .map_or(asked.len(), |next| next.asked_before);
            let shown = match asked[handed.asked_before..asked_after].iter().max() {
                None => index,
                Some(&request) => self.handed[index + 1..]
                    .iter()
                    .position(|later| matches!(later.kind, Kind::Window(id) if id >= request))
                    .map(|offset| index + 1 + offset)
                    .ok_or_else(|| format!("key {key}'s request {request} was never answered"))?,
            };
            let drawn = self
                .drawn
                .get(shown + 1)
                .ok_or_else(|| format!("no frame was drawn after wake {shown}"))?;
            latencies.push(drawn.saturating_duration_since(due));
        }
        Ok(latencies)
    }
}

impl<D> Wakes for Paced<'_, D> {
    fn next_wake(&mut self) -> Option<Wake> {
        self.drawn.push(Instant::now());
        self.written.push(self.sink.snapshot());
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
            self.due += TICK;
            let press = KeyPress::plain(self.keys[key]);
            return Some(self.hand(
                Kind::Key(key, due),
                Wake::Event(AppEvent::Core(Event::Key(press))),
            ));
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
}
