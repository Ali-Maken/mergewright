//! m8-polish-and-ship/05, the gate before wiring (rule 1), as amendment 1
//! of the Part 2 ruling sets it: do content-keyed emphasis pieces (runs,
//! arcs, nodes; ADR-0053 amendment) localise on his walk?
//!
//! MONOREPO, 50 `j` from the top, one frame per key as the paced loop draws
//! them (`docs/perf/m8-trace-followup.txt`: motions p50=1), at his Ghostty
//! shape (222×70, 17×37 cell) and his `held_key` shape (187×50). Per frame
//! it counts the pixel area rasterised and sent:
//! - **today**: each viewport strip whose emphasis key changed sends its
//!   whole emphasis image (`StripCache::paint_key`, one slot per strip);
//! - **pieces**: each piece whose key no earlier frame of the walk had
//!   (the terminal holds every key sent so far; no byte budget at this
//!   size).
//!
//! The ruling's gate, both shapes: over the walk's keys, pieces' area
//! ≤ 25 % of today's (`pieces_localise_on_his_walk`); in every frame,
//! pieces' area ≤ today's (`no_frame_rasterises_more`). Either misses →
//! stop, no wiring. Heavy tier (`heavy-tests`).

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use mergewright::config::Loaded;
use mergewright::event_loop::Wake;
use mergewright::jobs::{Dispatch, JobRunner};
use mergewright::session::Session;
use mergewright::update::{AppEvent, AppState, update};
use mergewright::view::max_lanes;
use mergewright::view::strips::{StripPaint, pixel_rows};
use mergewright_core::capabilities::RenderingTier;
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_fixtures::team::{self, TeamSpec};
use mergewright_graph::raster::cache::{ByteBudget, STRIP_ROWS, StripCache};
use mergewright_graph::raster::{
    PlainStrip, Selection, emphasis_pieces, rasterise_emphasis, rasterise_piece,
};

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::{capabilities, ready_to_scroll_at};

/// His Ghostty run (`docs/perf/m8-trace-2026-10-08-ghostty.txt:1`) and
/// his held-key run (`held_key`, 187×50).
const SHAPES: [(u16, u16); 2] = [(222, 70), (187, 50)];
/// The script's walk (`scripts/m8-trace-followup.sh`).
const KEYS: usize = 50;
/// The ruling's gate over the walk.
const GATE_PERCENT: u64 = 25;

#[test]
fn pieces_localise_on_his_walk() {
    for size in SHAPES {
        let frames = walk(size);
        // The first frame sends everything either way: per key only.
        let (today, pieces) = frames[1..].iter().fold((0, 0), |(today, pieces), frame| {
            (today + frame.today, pieces + frame.pieces)
        });
        let sent: usize = frames[1..].iter().map(|frame| frame.sent).sum();
        let changed: usize = frames[1..].iter().map(|frame| frame.changed).sum();
        let percent = (pieces * 100).checked_div(today).unwrap_or(0);
        println!(
            "SUMMARY piece_gate size={}x{} keys={KEYS} today_area_px={today} pieces_area_px={pieces} percent={percent} today_strips_changed={changed} pieces_sent={sent} today_px_per_key={} pieces_px_per_key={}",
            size.0,
            size.1,
            today / KEYS as u64,
            pieces / KEYS as u64,
        );
        assert!(
            today > 0,
            "{size:?}: the walk changed no emphasis: not his shape"
        );
        assert!(
            pieces * 100 <= today * GATE_PERCENT,
            "blocked: pieces don't localise at {size:?} ({percent} % of today's area)"
        );
    }
}

#[test]
fn no_frame_rasterises_more() {
    for size in SHAPES {
        let frames = walk(size);
        let over: Vec<_> = frames
            .iter()
            .enumerate()
            .filter(|(_, frame)| frame.pieces > frame.today)
            .map(|(index, frame)| (index, frame.today, frame.pieces))
            .collect();
        let most = frames.iter().map(|frame| frame.pieces).max().unwrap_or(0);
        let first = &frames[0];
        println!(
            "SUMMARY piece_gate_frames size={}x{} frames={} over={} first_today_px={} first_pieces_px={} most_pieces_px={most}",
            size.0,
            size.1,
            frames.len(),
            over.len(),
            first.today,
            first.pieces,
        );
        assert!(
            over.is_empty(),
            "blocked: pieces don't localise at {size:?}: frames (index, today px, pieces px) {over:?}"
        );
    }
}

/// One frame of the walk: today's area rasterised, the pieces' area
/// rasterised, pieces sent, strips whose emphasis key changed.
#[derive(Default)]
struct Frame {
    today: u64,
    pieces: u64,
    sent: usize,
    changed: usize,
}

/// The walk at `size`: frame 0 is the first draw, then one per `j`.
fn walk(size: (u16, u16)) -> Vec<Frame> {
    let repo = team::borrow(&TeamSpec::MONOREPO).unwrap_or_else(|why| panic!("fixture: {why:?}"));
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, _startup) = ready_to_scroll_at(
        &repo,
        RenderingTier::Pixel,
        Refs::Fixture,
        Post(post),
        &events,
        size,
    )
    .unwrap_or_else(|why| panic!("{why}"));
    let mut state = quiet(&jobs, state, &events);
    let session = Session::settle(&Loaded::default(), capabilities(RenderingTier::Pixel));
    let paint = StripPaint::of(&session);
    let cache = StripCache::with_budget(ByteBudget(usize::MAX), 64);
    let mut seen = Seen::default();
    let mut frames = Vec::with_capacity(KEYS + 1);
    for key in 0..=KEYS {
        let pixels = pixel_rows(&state, max_lanes(&state), STRIP_ROWS);
        let Some(history) = state.history.as_ref() else {
            panic!("{size:?}: no history after key {key}");
        };
        let (top, height) = (history.top, state.graph_rows());
        let mut frame = Frame::default();
        for strip in &pixels.strips {
            if strip.first_row >= top + height || strip.first_row + strip.row_count() <= top {
                continue;
            }
            let got = strip.with_layers(&pixels, &paint, |plain, selection| {
                areas(&cache, &mut seen, strip.first_row, plain, selection)
            });
            let (today, pieces, sent, changed) = got.unwrap_or_else(|why| panic!("{why}"));
            frame.today += today;
            frame.pieces += pieces;
            frame.sent += sent;
            frame.changed += usize::from(changed);
        }
        frames.push(frame);
        if key < KEYS {
            state = press(&jobs, &state, &events, KeyCode::Char('j'));
        }
    }
    frames
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
        |frame: &mergewright_core::raster::RasterFrame| u64::from(frame.width) * u64::from(frame.height);
    let today = if changed {
        rasterise_emphasis(plain, selection)
            .map_err(|why| format!("emphasis: {why:?}"))?
            .map_or(0, |top| area(&top.frame))
    } else {
        0
    };
    let (mut sent_area, mut sent) = (0, 0);
    for piece in emphasis_pieces(plain, selection) {
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
