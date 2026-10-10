//! m8-polish-and-ship/05 Part 2, the gate before wiring (rule 1): do
//! per-column emphasis pieces localise on his walk?
//!
//! His Ghostty shape (222×70, 17×37 cell) on MONOREPO, 50 `j` from the
//! top, one frame per key as the paced loop draws them
//! (`docs/perf/m8-trace-followup.txt`: motions p50=1). Per key it counts
//! the pixel area the terminal would be sent:
//! - **today**: each viewport strip whose emphasis key changed sends its
//!   whole emphasis image (`StripCache::paint_key`, one slot per strip);
//! - **pieces**: each piece whose key the terminal does not already hold
//!   (every key sent so far; no byte budget at this size).
//!
//! The ruling's gate: pieces' area ≥ 50 % of today's → stop, no wiring.
//! Heavy tier (`heavy-tests`).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use graft::config::Loaded;
use graft::event_loop::Wake;
use graft::jobs::{Dispatch, JobRunner};
use graft::session::Session;
use graft::update::{AppEvent, AppState, update};
use graft::view::max_lanes;
use graft::view::strips::{StripPaint, pixel_rows};
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_fixtures::team::{self, TeamSpec};
use graft_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache};
use graft_graph::raster::{
    PlainStrip, Selection, emphasis_pieces, rasterise_emphasis, rasterise_piece,
};

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::{capabilities, ready_to_scroll_at};

/// His Ghostty run (`docs/perf/m8-trace-2026-10-08-ghostty.txt:1`).
const SIZE: (u16, u16) = (222, 70);
/// The script's walk (`scripts/m8-trace-followup.sh`).
const KEYS: usize = 50;
/// The ruling's gate.
const GATE_PERCENT: u64 = 50;

#[test]
fn pieces_localise_on_his_walk() {
    let repo = team::borrow(&TeamSpec::MONOREPO).unwrap_or_else(|why| panic!("fixture: {why:?}"));
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, _startup) = ready_to_scroll_at(
        &repo,
        RenderingTier::Pixel,
        Refs::Fixture,
        Post(post),
        &events,
        SIZE,
    )
    .unwrap_or_else(|why| panic!("{why}"));
    let mut state = quiet(&jobs, state, &events);
    let session = Session::settle(&Loaded::default(), capabilities(RenderingTier::Pixel));
    let paint = StripPaint::of(&session);
    let cache = StripCache::with_budget(ByteBudget(usize::MAX), 64);
    let mut seen = Seen::default();
    let (mut today, mut pieces, mut piece_count, mut strips_changed) = (0u64, 0u64, 0usize, 0usize);
    for key in 0..=KEYS {
        let pixels = pixel_rows(&state, max_lanes(&state), STRIP_ROWS);
        let history = state.history.as_ref().expect("history");
        let (top, height) = (history.top, state.graph_rows());
        for strip in &pixels.strips {
            if strip.first_row >= top + height || strip.first_row + strip.row_count() <= top {
                continue;
            }
            let got = strip.with_layers(&pixels, &paint, |plain, selection| {
                areas(&cache, &mut seen, strip.first_row, plain, selection)
            });
            let (area_today, area_pieces, sent, changed) =
                got.unwrap_or_else(|why| panic!("{why}"));
            // The first frame sends everything either way: per key only.
            if key > 0 {
                today += area_today;
                pieces += area_pieces;
                piece_count += sent;
                strips_changed += usize::from(changed);
            }
        }
        if key < KEYS {
            state = press(&jobs, &state, &events, KeyCode::Char('j'));
        }
    }
    let percent = (pieces * 100).checked_div(today).unwrap_or(0);
    println!(
        "SUMMARY piece_gate size={}x{} keys={KEYS} today_area_px={today} pieces_area_px={pieces} percent={percent} today_strips_changed={strips_changed} pieces_sent={piece_count} today_px_per_key={} pieces_px_per_key={}",
        SIZE.0,
        SIZE.1,
        today / KEYS as u64,
        pieces / KEYS as u64,
    );
    assert!(today > 0, "the walk changed no emphasis: not his shape");
    assert!(
        percent < GATE_PERCENT,
        "blocked: pieces don't localise ({percent} % of today's area)"
    );
}

/// What the terminal holds so far: today's emphasis key per strip slot, and
/// every piece key sent.
#[derive(Default)]
struct Seen {
    slots: HashMap<usize, u64>,
    held: HashSet<u64>,
}

/// One strip this key: (today's area sent, the pieces' area sent, pieces
/// sent, whether today's emphasis key changed).
fn areas(
    cache: &StripCache,
    seen: &mut Seen,
    first: usize,
    plain: &PlainStrip<'_>,
    selection: &Selection<'_>,
) -> Result<(u64, u64, usize, bool), String> {
    let paint_key = cache
        .paint_key(first, plain, selection)
        .map_err(|why| format!("paint key: {why:?}"))?;
    let changed = seen.slots.insert(first, paint_key) != Some(paint_key);
    let area =
        |frame: &graft_core::raster::RasterFrame| u64::from(frame.width) * u64::from(frame.height);
    let today = if changed {
        rasterise_emphasis(plain, selection)
            .map_err(|why| format!("emphasis: {why:?}"))?
            .map_or(0, |top| area(&top.frame))
    } else {
        0
    };
    let strip_key = cache
        .backdrop_key(first, plain, selection.backdrop)
        .map_err(|why| format!("backdrop key: {why:?}"))?;
    let (mut sent_area, mut sent) = (0, 0);
    for piece in emphasis_pieces(plain, selection, strip_key) {
        if seen.held.insert(piece.key) {
            sent += 1;
            sent_area += rasterise_piece(plain, selection, piece)
                .map_err(|why| format!("piece: {why:?}"))?
                .map_or(0, |top| area(&top.frame));
        }
    }
    Ok((today, sent_area, sent, changed))
}

/// One key through `update`, its commands dispatched, then quiet.
fn press(
    jobs: &JobRunner<Post>,
    state: &AppState,
    events: &Receiver<Posted>,
    code: KeyCode,
) -> AppState {
    let (next, commands) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
    for command in commands {
        jobs.dispatch(command);
    }
    quiet(jobs, next, events)
}

/// Job events into `state` until none has come for 200 ms.
fn quiet(jobs: &JobRunner<Post>, mut state: AppState, events: &Receiver<Posted>) -> AppState {
    loop {
        let posted = match events.recv_timeout(Duration::from_millis(200)) {
            Ok(posted) => posted,
            Err(mpsc::RecvTimeoutError::Timeout) => return state,
            Err(why) => panic!("job events: {why}"),
        };
        let Wake::Event(event) = posted.wake else {
            panic!("a wake no job sends: {}", posted.wake.label());
        };
        let (next, commands) = update(&state, event);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
}
