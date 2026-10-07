//! Held `j` at his scale (m7-power-tools/42): the team fixture shaped like
//! his work repository (`TeamSpec::MONOREPO`), at his terminal's 187×50,
//! through the real loop into a terminal that drains at a fixed rate
//! ([`DrainSink`]) and blocks the writer when full, as a pty does.
//!
//! Scenario A: `j` every 16 ms for 3 s, then quiet. Scenario B: the same
//! hold, 50 ms, then `J`. Every time is taken from the last key's
//! **arrival** (its send through [`KeyFeed`]): a key release never reaches
//! the loop (`graft-core/src/event.rs`: no release variant), and the last
//! repeat comes before the release, so this is at least as strict as
//! "after I let go". Names say "release" in the user's word.
//!
//! Red at HEAD by design (ticket 42 "Red at HEAD is the point"): 43 turns
//! the Unicode tier green, 46 the pixel tier. Counts fail in every build;
//! times only in release (ADR-0021). Run by `scripts/fix-scroll-held-key.sh`.

include!("held_key_bound.rs");

use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use graft::config::Loaded;
use graft::event_loop::{Exit, Wake, pump};
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::session::Session;
use graft::update::{AppEvent, AppState, update};
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_fixtures::team::{self, TeamSpec};
use graft_graph::Ancestry;
use graft_ui::detail::DetailFocus;

use crate::drain_sink::DrainSink;
use crate::frame_trace::{KeyFeed, Post, Posted};
use crate::frames::Refs;
use crate::frames::paced::{Handed, Kind, Paced, Watching};
use crate::frames::run::{capabilities, in_memory, ready_to_scroll_at};
use crate::frames::sink::Sink;

use graft_core::capabilities::PixelProtocol;
use graft_term::kitty::EMPHASIS_ID_BIT;

/// His terminal (`docs/field-notes.md:57`), `(columns, rows)`.
const SIZE: (u16, u16) = (187, 50);
/// The held key's repeat: macOS's fastest is 15 ms.
const REPEAT: Duration = Duration::from_millis(16);
/// 3 s of repeats at [`REPEAT`].
const HELD: usize = 188;
/// Scenario B: from the last `j` to `J`.
const THEN_J: Duration = Duration::from_millis(50);
/// The pipe: a pty-sized buffer (the real macOS pty's, measured by the
/// script, is recorded beside it in `docs/perf/fix-scroll-held-key.txt`).
const PIPE: usize = 16 * 1024;
/// Drain rates: stress models of a terminal busy parsing, chosen so HEAD
/// overruns them and a fixed build has room (ticket 42 "Drain model").
const UNICODE_RATE: u64 = 500_000;
const PIXEL_RATE: u64 = 40_000_000;
/// A terminal that never falls behind (m8 G-pace J1: "at 40 MB/s and
/// free"): every byte drained as it is written.
const FREE_RATE: u64 = crate::drain_sink::FREE;
/// Done: no byte moved for this long.
const QUIET: Duration = Duration::from_secs(1);
/// A hang guard, not a threshold.
const PATIENCE: Duration = Duration::from_secs(120);
/// The cell width `capabilities` reports (`cell=17x37` in the summary).
const CELL_WIDTH: u32 = 17;
/// "At once" (Nielsen's 0.1 s).
const AT_ONCE_MS: u128 = 100;

#[test]
fn held_j_stops_on_release_unicode() {
    held(RenderingTier::Unicode, false, Encoding::Raw);
}

#[test]
fn held_j_stops_on_release_pixel() {
    held(RenderingTier::Pixel, false, Encoding::Raw);
}

#[test]
fn held_j_stops_on_release_pixel_deflate() {
    held(RenderingTier::Pixel, false, Encoding::Deflate);
}

#[test]
fn held_j_stops_on_release_pixel_iterm2() {
    held(RenderingTier::Pixel, false, Encoding::Iterm2);
}

/// G-pace J1 (m8-polish-and-ship/02): the paced loop at the other drains.
#[test]
fn held_j_stops_on_release_unicode_40mb() {
    held_at(RenderingTier::Unicode, false, Encoding::Raw, PIXEL_RATE);
}

#[test]
fn held_j_stops_on_release_unicode_free() {
    held_at(RenderingTier::Unicode, false, Encoding::Raw, FREE_RATE);
}

#[test]
fn held_j_stops_on_release_pixel_free() {
    held_at(RenderingTier::Pixel, false, Encoding::Raw, FREE_RATE);
}

#[test]
fn held_j_stops_on_release_pixel_deflate_free() {
    held_at(RenderingTier::Pixel, false, Encoding::Deflate, FREE_RATE);
}

/// G-settle (m8-polish-and-ship/03, review C1/Minor 4): the path 03
/// changed — the pane shown, following the cursor through MONOREPO's
/// merge-rich `main`, so its reads and the evil probe are held mid-burst
/// and run once at rest. J1 here is the graph's tail: the frames up to the
/// rest's own (the pane catching up), which is reported apart, unjudged.
#[test]
fn held_j_with_the_pane_shown_stops_on_release_unicode_40mb() {
    held_in(
        RenderingTier::Unicode,
        false,
        Encoding::Raw,
        PIXEL_RATE,
        true,
    );
}

#[test]
fn held_j_with_the_pane_shown_stops_on_release_pixel_deflate() {
    held_in(
        RenderingTier::Pixel,
        false,
        Encoding::Deflate,
        PIXEL_RATE,
        true,
    );
}

#[test]
fn j_then_journal_opens_at_once_unicode() {
    held(RenderingTier::Unicode, true, Encoding::Raw);
}

#[test]
fn j_then_journal_opens_at_once_pixel() {
    held(RenderingTier::Pixel, true, Encoding::Raw);
}

#[test]
fn j_then_journal_opens_at_once_pixel_deflate() {
    held(RenderingTier::Pixel, true, Encoding::Deflate);
}

impl Encoding {
    /// The harness's terminal at `tier`, sending images this way: a field
    /// set on the value, never a `Capabilities` literal (42 Decided 4).
    fn capabilities(self, tier: RenderingTier) -> graft_core::capabilities::Capabilities {
        let mut caps = capabilities(tier);
        match self {
            Encoding::Raw => {}
            Encoding::Deflate => caps.kitty_deflate = true,
            Encoding::Iterm2 => caps.pixel_protocol = Some(PixelProtocol::Iterm2),
        }
        caps
    }
}

/// How the pixel tier's images go out (46 Decided 7): set on the
/// capabilities value by parameter, never by env (42 Decided 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Encoding {
    /// Kitty, raw RGBA: `kitty_deflate` off.
    Raw,
    /// Kitty, zlib (`o=z`): `kitty_deflate` on.
    Deflate,
    /// iTerm2 inline images: one composited band per strip.
    Iterm2,
}

fn keys(journal: bool) -> Vec<(KeyCode, Duration)> {
    let mut keys = vec![(KeyCode::Char('j'), REPEAT); HELD];
    if journal {
        keys.push((KeyCode::Char('J'), THEN_J));
    }
    keys
}

/// What one hold measured.
#[derive(Debug, Default)]
struct Reading {
    keys: usize,
    frames_while_held: usize,
    frames_after_release: usize,
    tail_ms: u128,
    bytes_per_frame_held: u64,
    strip_transmits_held: usize,
    emphasis_transmits_held: usize,
    rows_scrolled: usize,
    /// Frames from the loop's own rest on (G-settle: the pane catching up),
    /// and last `j` → their last byte drained. Reported, not judged.
    rest_frames: usize,
    rest_ms: Option<u128>,
    /// Scenario B only.
    j_ms: Option<u128>,
    j_done_ms: Option<u128>,
    frames_before_j: Option<usize>,
    /// Every byte the run wrote, per held `j` (cost line, not judged).
    bytes_per_key: u64,
    /// Emphasis images' width in cells, 99th percentile over the run
    /// (`s=` / cell width); `None` with none sent.
    emphasis_cells_p99: Option<u32>,
    deflated_transmits: usize,
    peak_rss_mb: u64,
    /// Premises: frames judged, frames with ≥ 12 lanes, frames whose lit
    /// columns changed from the frame before.
    premise_frames: usize,
    wide_frames: usize,
    lit_changed: usize,
}

fn held(tier: RenderingTier, journal: bool, encoding: Encoding) {
    let rate = match tier {
        RenderingTier::Pixel => PIXEL_RATE,
        _ => UNICODE_RATE,
    };
    held_at(tier, journal, encoding, rate);
}

fn held_at(tier: RenderingTier, journal: bool, encoding: Encoding, rate: u64) {
    held_in(tier, journal, encoding, rate, false);
}

/// `pane`: the detail pane open and the graph focused before the hold.
fn held_in(tier: RenderingTier, journal: bool, encoding: Encoding, rate: u64, pane: bool) {
    let reading = run(tier, journal, rate, encoding, pane).unwrap_or_else(|why| panic!("{why}"));
    let scenario = match (journal, tier, encoding) {
        (false, RenderingTier::Pixel, Encoding::Deflate) => "A-deflate",
        (false, RenderingTier::Pixel, Encoding::Iterm2) => "A-iterm2",
        (true, RenderingTier::Pixel, Encoding::Deflate) => "B-deflate",
        (true, RenderingTier::Pixel, Encoding::Iterm2) => "B-iterm2",
        (false, ..) => "A",
        (true, ..) => "B",
    };
    let scenario = if pane {
        format!("{scenario}-pane")
    } else {
        scenario.to_owned()
    };
    let scenario = scenario.as_str();
    let line = summary(tier, scenario, rate, &reading);
    println!("{line}");

    // Premises: the fixture still has his shape. Fail the test, not the verdict.
    assert!(
        reading.premise_frames > 0
            && reading.wide_frames * 2 >= reading.premise_frames
            && reading.lit_changed * 2 >= reading.premise_frames,
        "PREMISE failed (the fixture is not his shape): {line}"
    );
    let failed = verdict(tier, scenario, &reading);
    assert!(
        failed.is_empty(),
        "held_key verdict failed {failed:?}: {line}"
    );
}

/// The `SUMMARY held_key` line.
fn summary(tier: RenderingTier, scenario: &str, rate: u64, reading: &Reading) -> String {
    let mut line = format!(
        "SUMMARY held_key tier={tier:?} scenario={scenario} size={}x{} cell=17x37 drain={rate}B/s pipe={PIPE} keys={} frames_while_held={} frames_after_release={} tail_ms={} bytes_per_frame_held={} strip_transmits_held={} emphasis_transmits_held={} rows_scrolled={}",
        SIZE.0,
        SIZE.1,
        reading.keys,
        reading.frames_while_held,
        reading.frames_after_release,
        reading.tail_ms,
        reading.bytes_per_frame_held,
        reading.strip_transmits_held,
        reading.emphasis_transmits_held,
        reading.rows_scrolled,
    );
    if let (Some(j), Some(done), Some(before)) =
        (reading.j_ms, reading.j_done_ms, reading.frames_before_j)
    {
        line += &format!(" j_ms={j} j_done_ms={done} frames_before_J={before}");
    }
    if let Some(rest_ms) = reading.rest_ms {
        line += &format!(" rest_frames={} rest_ms={rest_ms}", reading.rest_frames);
    }
    if tier == RenderingTier::Pixel {
        // Frames whose fold differs from the frame before's: not readable
        // through this harness (see `verdict`).
        line += &format!(
            " fold_changes_held=unread bytes_per_key={} emphasis_cells_p99={} deflated_transmits={}",
            reading.bytes_per_key,
            reading
                .emphasis_cells_p99
                .map_or_else(|| "none".to_owned(), |cells| cells.to_string()),
            reading.deflated_transmits,
        );
    }
    line += &format!(
        " peak_rss_mb={} premise_wide={}/{} premise_lit_changed={}/{}",
        reading.peak_rss_mb,
        reading.wide_frames,
        reading.premise_frames,
        reading.lit_changed,
        reading.premise_frames
    );
    line
}

/// Prints one `VERDICT held_key` line per item; returns the failed ones.
/// Counts in every build, times in release only (ADR-0021).
fn verdict(tier: RenderingTier, scenario: &str, reading: &Reading) -> Vec<String> {
    let mut failed = Vec::new();
    let mut check = |name: &str, holds: bool| {
        println!(
            "VERDICT held_key tier={tier:?} scenario={scenario} {name}: {}",
            if holds { "ok" } else { "FAILED" }
        );
        if !holds {
            failed.push(name.to_owned());
        }
    };
    check("frames_after_release<=2", reading.frames_after_release <= 2);
    if let Some(before) = reading.frames_before_j {
        check("frames_before_J<=1", before <= 1);
    }
    if tier == RenderingTier::Pixel {
        check(
            "bytes_per_frame_held<=512KiB",
            reading.bytes_per_frame_held <= 512 * 1024,
        );
        // The strip bound is `ceil(rows_scrolled/32) + 2 + fold_changes_held
        // × strips_per_frame` (46c Decided 4): on a graph wider than the
        // lane cap the fold keeps the selection's thread lanes (ADR-0025
        // Decision 2), so a `j` may change the layout and re-send every
        // strip. This harness cannot read the drawn fold — `Paced` hands
        // out events and offsets, `PixelRows` has no kept-column set and
        // the cap (`view::max_lanes`) is crate-private — so the count is
        // printed, not judged. The invariant is guarded in the fast tier:
        // `pixel_scroll::a_capped_graph_resends_strips_only_when_the_fold_changes`.
        println!(
            "VERDICT held_key tier={tier:?} scenario={scenario} strip_transmits_held={}: unjudged (fold_changes_held unread)",
            reading.strip_transmits_held
        );
    }
    if cfg!(debug_assertions) {
        println!("held_key: debug build, times printed not asserted (ADR-0021)");
    } else {
        check("tail_ms<=100", reading.tail_ms <= AT_ONCE_MS);
        if let Some(j) = reading.j_ms {
            check("j_ms<=100", j <= AT_ONCE_MS);
        }
    }
    failed
}

fn run(
    tier: RenderingTier,
    journal: bool,
    rate: u64,
    encoding: Encoding,
    pane: bool,
) -> Result<Reading, String> {
    let repo = team::borrow(&TeamSpec::MONOREPO).map_err(|why| format!("fixture: {why:?}"))?;
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, mut state, _startup) = ready_to_scroll_at(
        &repo,
        tier,
        Refs::Fixture,
        Post(post.clone()),
        &events,
        SIZE,
    )?;
    if pane {
        state = open_pane(&jobs, state)?;
    }
    let state = settle(&jobs, state, &events)?;

    let mut session = Session::settle(&Loaded::default(), answering(encoding, tier));
    let drain = DrainSink::new(PIPE, rate);
    drain.answering_fences(answer_on(post.clone()));
    let sink = Sink::draining(&drain);
    let mut terminal = in_memory(&sink, &state)?;
    let watching = Watching {
        inner: &jobs,
        asked: Mutex::default(),
    };
    let initial = state.clone();
    let script = keys(journal);
    let feed = KeyFeed::start(script.clone(), post)
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(&events, script.clone(), &watching, sink.clone(), feed.log());
    let exit = pump(&mut terminal, &mut wakes, &watching, &mut session, state);
    let stamps = feed.finish();
    let exit = exit.map_err(|why| format!("the loop failed: {why}"))?;
    let stamps = stamps?;
    if !matches!(exit, Exit::Quit) {
        return Err(format!("the loop ended with {exit:?}"));
    }
    if let Some(failure) = wakes.failure.take() {
        return Err(failure);
    }
    // Every frame is in the pipe; wait for the terminal to swallow it.
    let deadline = Instant::now() + PATIENCE;
    while !drain.quiet_for(QUIET) {
        if Instant::now() > deadline {
            return Err(format!("the drain never went quiet in {PATIENCE:?}"));
        }
        thread::sleep(Duration::from_millis(10));
    }
    keys_were_the_script(&script, &stamps, journal)?;

    let reading = read(&wakes, &drain, initial, &stamps, journal);
    let shutdown = jobs.shutdown(Duration::from_secs(10), graft::jobs::EXIT_WRITE_WAIT, &());
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}, forced {:?}",
            shutdown.stalled, shutdown.forced
        ));
    }
    reading
}

/// 47's walk count: the keys sent are exactly the script.
fn keys_were_the_script(
    script: &[(KeyCode, Duration)],
    stamps: &[crate::frame_trace::Sent],
    journal: bool,
) -> Result<(), String> {
    let sent: Vec<KeyCode> = script.iter().map(|(code, _)| *code).collect();
    let mut want = vec![KeyCode::Char('j'); HELD];
    if journal {
        want.push(KeyCode::Char('J'));
    }
    if sent != want || stamps.len() != want.len() {
        return Err(format!(
            "key list: sent {} of {} keys, script {sent:?}",
            stamps.len(),
            want.len()
        ));
    }
    Ok(())
}

/// `⏎` opens the pane on the files; `Tab` `Tab` (files → diff → graph)
/// gives `j` back to the graph. The caller's `settle` lands the reads.
fn open_pane(jobs: &impl Dispatch, mut state: AppState) -> Result<AppState, String> {
    for code in [KeyCode::Enter, KeyCode::Tab, KeyCode::Tab] {
        let (next, commands) = update(&state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
    if state.detail.shown && state.detail.focus == DetailFocus::Graph {
        Ok(state)
    } else {
        Err("the pane did not open with the graph focused".to_owned())
    }
}

/// The harness's terminal answers fences as it reads them (ADR-0054), so
/// `held_key` measures the paced loop: at most one frame in the pipe.
fn answering(encoding: Encoding, tier: RenderingTier) -> graft_core::capabilities::Capabilities {
    let mut capabilities = encoding.capabilities(tier);
    capabilities.answers_dsr = true;
    capabilities
}

/// The drain's answer to a fence: a `Wake::Fence` on the loop's channel.
fn answer_on(post: mpsc::Sender<Posted>) -> impl FnMut() + Send + 'static {
    move || {
        let answer = Posted {
            at: Instant::now(),
            wake: Wake::Fence,
        };
        // A failed send means the loop has ended and dropped its end:
        // nothing waits for this answer any more.
        if post.send(answer).is_err() {}
    }
}

/// Pumps job events into `state` until history has laid out to the end
/// unasked (`LaidOut`), then until no event has come for 300 ms.
fn settle(
    jobs: &JobRunner<Post>,
    mut state: AppState,
    events: &Receiver<Posted>,
) -> Result<AppState, String> {
    let laid_out = |state: &AppState| state.history.as_ref().is_some_and(|h| h.laid_out.is_some());
    loop {
        let wait = if laid_out(&state) {
            Duration::from_millis(300)
        } else {
            PATIENCE
        };
        let posted = match events.recv_timeout(wait) {
            Ok(posted) => posted,
            Err(mpsc::RecvTimeoutError::Timeout) if laid_out(&state) => return Ok(state),
            Err(why) => return Err(format!("waiting for LaidOut: {why}")),
        };
        let Wake::Event(event) = posted.wake else {
            return Err(format!("a wake no job sends: {}", posted.wake.label()));
        };
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
}

fn ms(from: Instant, to: Instant) -> u128 {
    to.saturating_duration_since(from).as_millis()
}

/// The readings, from the loop's stamps and the drain's progress.
///
/// Frame `k` (the frame for `handed[k]`) is the bytes `written[k]` to
/// `written[k + 1]`: `written[i]` is taken at the loop's first look after
/// drawing the frame before it, images included (they are written inside
/// `draw`, before `Drawn`).
fn read<D>(
    wakes: &Paced<'_, D>,
    drain: &DrainSink,
    initial: AppState,
    stamps: &[crate::frame_trace::Sent],
    journal: bool,
) -> Result<Reading, String> {
    let handed = &wakes.handed;
    let written: Vec<u64> = wakes.written.iter().map(|&w| w as u64).collect();
    let end_of = |k: usize| written.get(k + 1).copied();
    let last_j = stamps.get(HELD - 1).ok_or("no stamp for the last j")?.at;

    let mut reading = Reading {
        keys: stamps.len(),
        peak_rss_mb: graft_fixtures::peak_rss().map_or(0, |bytes| bytes >> 20),
        ..Reading::default()
    };

    // Frames and their bytes while held / after the last j.
    let mut held_bytes = 0;
    let mut held_end = written.first().copied().unwrap_or(0);
    for (k, frame) in handed.iter().enumerate() {
        let Some(drawn_at) = frame.drawn_at() else {
            continue;
        };
        if drawn_at <= last_j {
            reading.frames_while_held += 1;
            let end = end_of(k).ok_or("a held frame with no end offset")?;
            held_bytes += end - written[k];
            held_end = end;
        }
    }
    // After release: in B only the frames before `J`'s (42 §Planner correction).
    let js_frame = if journal {
        Some(js_frame_index(handed)?)
    } else {
        None
    };
    // A (G-settle): the graph's frames end where the loop's own rest begins;
    // `written[r]` is stamped at the rest's look, the graph's last byte.
    let rest = (!journal).then(|| rest_after(handed, last_j)).flatten();
    let graph_last = rest.map_or_else(|| written.last().copied().unwrap_or(0), |r| written[r]);
    let draws: Vec<_> = handed[..rest.unwrap_or(handed.len())]
        .iter()
        .map(Handed::drawn_at)
        .collect();
    let (after, tail_end) = after_release(&draws, last_j, js_frame);
    reading.frames_after_release = after;
    if let Some(r) = rest {
        read_rest(handed, r, drain, &written, last_j, &mut reading)?;
    }
    reading.bytes_per_frame_held = held_bytes / reading.frames_while_held.max(1) as u64;
    count_transmits(
        drain,
        held_end,
        written.last().copied().unwrap_or(0),
        &mut reading,
    );

    // Tail: last j sent → the drain passing the last counted frame's last
    // byte (A: the last frame; B with none counted: `J`'s first byte).
    let tail_byte = match tail_end {
        TailEnd::LastByte => graph_last,
        TailEnd::Frame(k) => end_of(k).ok_or("a counted frame with no end offset")?,
        TailEnd::JsFirstByte => written[js_frame.ok_or("no J frame")?] + 1,
    };
    let drained = drain
        .drained_at(tail_byte)
        .ok_or("the drain never passed the tail's last byte")?;
    reading.tail_ms = ms(last_j, drained);

    if journal {
        journal_frame(wakes, drain, &written, stamps, &mut reading)?;
    }
    replay(wakes, initial, &mut reading);
    Ok(reading)
}

/// The loop's own rest after the last `j` (G-settle): where the graph's
/// frames end and the pane's catch-up begins.
fn rest_after(handed: &[Handed], last_j: Instant) -> Option<usize> {
    handed.iter().position(|frame| {
        matches!(frame.kind, Kind::Rest) && frame.drawn_at().is_some_and(|at| at > last_j)
    })
}

/// The rest's frames, from `handed[rest]` on — the pane catching up — and
/// last `j` → their last byte drained. Reported apart, not judged.
fn read_rest(
    handed: &[Handed],
    rest: usize,
    drain: &DrainSink,
    written: &[u64],
    last_j: Instant,
    reading: &mut Reading,
) -> Result<(), String> {
    reading.rest_frames = handed[rest..]
        .iter()
        .filter(|frame| frame.drawn_at().is_some())
        .count();
    let last = written.last().copied().unwrap_or(0);
    let drained = drain
        .drained_at(last)
        .ok_or("the drain never passed the rest's last byte")?;
    reading.rest_ms = Some(ms(last_j, drained));
    Ok(())
}

/// Kitty transmits while held, by layer, and the run's cost lines (46):
/// recorded, not judged. `total` is every byte the run wrote.
fn count_transmits(drain: &DrainSink, held_end: u64, total: u64, reading: &mut Reading) {
    let transmits = drain.kitty_transmits();
    for transmit in &transmits {
        if transmit.offset >= held_end {
            break;
        }
        if transmit.image_id & EMPHASIS_ID_BIT != 0 {
            reading.emphasis_transmits_held += 1;
        } else {
            reading.strip_transmits_held += 1;
        }
    }
    // Cost lines (46): recorded, not judged.
    reading.bytes_per_key = total / HELD as u64;
    reading.deflated_transmits = transmits.iter().filter(|t| t.deflated).count();
    let mut widths: Vec<u32> = transmits
        .iter()
        .filter(|t| t.image_id & EMPHASIS_ID_BIT != 0)
        .filter_map(|t| t.width_px)
        .map(|px| px / CELL_WIDTH)
        .collect();
    widths.sort_unstable();
    reading.emphasis_cells_p99 =
        (!widths.is_empty()).then(|| widths[(widths.len() * 99 / 100).min(widths.len() - 1)]);
}

/// Scenario B: the index of `J`'s frame in `handed`.
fn js_frame_index(handed: &[Handed]) -> Result<usize, String> {
    handed
        .iter()
        .position(|frame| matches!(frame.kind, Kind::Key(key, _) if key == HELD))
        .ok_or_else(|| "J was never handed to the loop".to_string())
}

/// Scenario B: `J`'s frame — when its first and last byte drained, and the
/// frames drawn between `J`'s send and it.
fn journal_frame<D>(
    wakes: &Paced<'_, D>,
    drain: &DrainSink,
    written: &[u64],
    stamps: &[crate::frame_trace::Sent],
    reading: &mut Reading,
) -> Result<(), String> {
    let handed = &wakes.handed;
    let j_sent = stamps.get(HELD).ok_or("no stamp for J")?.at;
    let k = js_frame_index(handed)?;
    let start = written[k] + 1;
    let end = *written.get(k + 1).ok_or("J's frame has no end offset")?;
    let first = drain.drained_at(start).ok_or("J's frame never drained")?;
    let done = drain.drained_at(end).ok_or("J's frame never drained")?;
    reading.j_ms = Some(ms(j_sent, first));
    reading.j_done_ms = Some(ms(j_sent, done));
    reading.frames_before_j = Some(
        handed[..k]
            .iter()
            .filter(|frame| frame.drawn_at().is_some_and(|at| at > j_sent))
            .count(),
    );
    Ok(())
}

/// Replays the loop's events from `initial`: rows scrolled by the held `j`s,
/// and the premises over their frames (lanes drawn; the selection's lit
/// columns on screen, per `Ancestry::lit`, changing frame to frame).
fn replay<D>(wakes: &Paced<'_, D>, initial: AppState, reading: &mut Reading) {
    let handed = &wakes.handed;
    let mut state = initial;
    let start_cursor = state.history.as_ref().map_or(0, |h| h.cursor);
    let mut held_cursor = start_cursor;
    let mut previous_lit: Option<Vec<BTreeSet<usize>>> = None;
    for (frame, event) in handed.iter().zip(&wakes.replay) {
        state = update(&state, event.clone()).0;
        let Kind::Key(key, _) = frame.kind else {
            continue;
        };
        if key >= HELD {
            continue;
        }
        let Some(history) = state.history.as_ref() else {
            continue;
        };
        held_cursor = history.cursor;
        let height = state.graph_rows();
        let visible = history.visible(height);
        let (Some(window), Some(selected)) = (history.window.as_ref(), history.selected()) else {
            continue;
        };
        reading.premise_frames += 1;
        let lanes = visible.iter().map(|row| row.lanes.len()).max().unwrap_or(0);
        if lanes >= 12 {
            reading.wide_frames += 1;
        }
        let first = history.top.saturating_sub(window.first);
        let lit: Vec<BTreeSet<usize>> = match Ancestry::of(&window.rows, selected.commit.id) {
            Some(ancestry) => (first..first + visible.len())
                .map(|row| ancestry.lit(row).clone())
                .collect(),
            None => Vec::new(),
        };
        if previous_lit.as_ref().is_some_and(|before| *before != lit) {
            reading.lit_changed += 1;
        }
        previous_lit = Some(lit);
    }
    reading.rows_scrolled = held_cursor.saturating_sub(start_cursor);
}

/// Set by `heavy::budgets_hold_at_pixel_*` for [`pixel_rss_child`]: the
/// drain rate, bytes per second (m8 G-pixel-mem).
pub(crate) const PIXEL_RSS_CHILD_RATE: &str = "GRAFT_PIXEL_RSS_CHILD_RATE";

/// The child's result line starts with this.
pub(crate) const PIXEL_RSS_MARK: &str = "pixel_rss:";

/// Not a test of its own: the half of `heavy::budgets_hold_at_pixel_*` that
/// runs in a fresh process, whose `getrusage` peak is this hold's alone.
/// Scenario A at Pixel, Kitty with deflate (as Ghostty probes), at the rate
/// in [`PIXEL_RSS_CHILD_RATE`]. Times are not judged here: `held_key`'s own
/// tests do that. Without the variable set — any ordinary run of the tier —
/// it returns at once.
#[test]
fn pixel_rss_child() {
    let Some(rate) = std::env::var_os(PIXEL_RSS_CHILD_RATE) else {
        return;
    };
    let rate: u64 = rate
        .to_str()
        .and_then(|rate| rate.parse().ok())
        .unwrap_or_else(|| panic!("{PIXEL_RSS_CHILD_RATE}={rate:?} is not a rate"));
    let reading = run(RenderingTier::Pixel, false, rate, Encoding::Deflate, false)
        .unwrap_or_else(|why| panic!("{why}"));
    let peak = graft_fixtures::peak_rss().unwrap_or_else(|why| panic!("peak RSS: {why}"));
    println!(
        "{PIXEL_RSS_MARK} peak_rss={peak} keys={} rows_scrolled={} strip_transmits_held={} deflated_transmits={}",
        reading.keys,
        reading.rows_scrolled,
        reading.strip_transmits_held,
        reading.deflated_transmits,
    );
}
