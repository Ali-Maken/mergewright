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

//!
//! The harness's parts: [`run`] opens the fixture and pumps one scroll,
//! [`paced`] hands the loop its wakes and stamps them, [`readings`] turns
//! the stamps into latencies and trace lines, [`sink`] is the in-memory
//! terminal, [`verdict`] judges and prints, [`controls`] holds the perf
//! script's controls. This file holds the scroll tests and their script.

mod controls;
mod journal_find;
mod paced;
mod readings;
mod run;
mod sink;
mod verdict;

use std::sync::PoisonError;
use std::time::Duration;

use graft_core::capabilities::RenderingTier;
use graft_core::event::KeyCode;
use graft_fixtures::synthetic::{self, Scale};

use crate::frame_trace::{FrameTrace, Gate, TRACE_VAR};
use crate::heavy::MEASURING;
use controls::Control;
use run::scroll;
use verdict::{judge, print_details};

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
    // Printed by `judge`, after every other assertion (review 04 Major 1).
    let summary = format!(
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
    judge(tier, refs, pane, &run, &gate, &summary);
}
