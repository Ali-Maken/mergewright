//! `g_after_idle` (m7-power-tools/12): `G` pressed after the app has sat
//! idle long enough for the history worker to lay history out to the end.
//!
//! The 100k fixture with a commit-graph and 50k refs, at the Unicode tier:
//! ready to scroll, then pumped until the walk says `LaidOut` in its final
//! order and startup's other jobs have answered, then [`GAP`] with no input,
//! then `G`. The latency runs from `G`'s send to the first frame that shows
//! the last row with the cursor on it. The wakes in the gap are counted
//! (`idle_gap_wakes`): with history laid out, nothing should wake.
//!
//! `MWR_FRAME_NO_IDLE_ADVANCE` is set only by `scripts/m7-12-final-perf.sh`
//! for its "before" build, which has the idle advance patched out
//! (`scripts/m7-12-no-idle-advance.patch`): the walk never says `LaidOut`
//! there, so the wait is for startup's jobs only, and nothing is asserted.
//! It reads what `G` cost before the idle advance, on the same harness.

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

use graft::config::Loaded;
use graft::event_loop::{Exit, Wake, pump};
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::session::Session;
use graft::update::detail::Loading;
use graft::update::{AppState, update};
use graft_core::capabilities::RenderingTier;
use graft_core::event::KeyCode;
use graft_fixtures::synthetic::{self, Scale};

use super::paced::{Kind, Paced, Watching};
use super::run::{capabilities, in_memory, ready_to_scroll};
use super::sink::Sink;
use super::{INPUT_TO_FRAME, PATIENCE, Refs};
use crate::frame_trace::{KeyFeed, Post, Posted};
use crate::heavy::MEASURING;

/// The fixture: the frame tests' 100k with a commit-graph (no promotion, so
/// the first `LaidOut` is in the final order).
const SCALE: Scale = Scale::HEAVY.with_commit_graph();

/// The pause with no input between `LaidOut` and `G`.
const GAP: Duration = Duration::from_secs(2);

/// Set by the perf script's "before" build only (see the module docs).
const NO_IDLE_ADVANCE: &str = "MWR_FRAME_NO_IDLE_ADVANCE";

const TIER: RenderingTier = RenderingTier::Unicode;

#[test]
fn g_after_idle() {
    let no_idle_advance = std::env::var_os(NO_IDLE_ADVANCE).is_some();
    let repo = synthetic::borrow(SCALE).unwrap_or_else(|why| panic!("{why}"));
    // Poisoned only when another measuring test panicked while holding it;
    // that test has already failed on its own, and the lock guards no data.
    let turn = MEASURING.lock().unwrap_or_else(PoisonError::into_inner);
    let landing = g_after_gap(&repo, no_idle_advance).unwrap_or_else(|why| panic!("{why}"));
    drop(turn);

    let rows = SCALE.commits() as usize;
    assert!(
        landing.on_last_row,
        "`G`'s frame does not show the last row with the cursor on it: cursor {}, {rows} rows",
        landing.cursor
    );
    let build = if no_idle_advance {
        "no-idle-advance"
    } else {
        "idle-advance"
    };
    println!(
        "frames: g_after_idle build={build} tier={TIER:?} refs={:?} rows={rows} laid_out={} gap_ms={} latency_us={} asked={} wakes_between={} idle_gap_wakes={} woke={:?}",
        Refs::FiftyThousand,
        landing
            .laid_out
            .map_or("-".to_owned(), |rows| rows.to_string()),
        GAP.as_millis(),
        landing.latency.as_micros(),
        u8::from(landing.asked),
        landing.between,
        landing.gap_wakes.len(),
        landing.gap_wakes,
    );
    if no_idle_advance {
        return;
    }
    assert!(
        landing.gap_wakes.is_empty(),
        "{} wakes with no input in the {GAP:?} after `LaidOut`: {:?}",
        landing.gap_wakes.len(),
        landing.gap_wakes
    );
    // ADR-0021: time budgets fail only in release.
    if cfg!(debug_assertions) {
        println!("frames: g_after_idle debug build, budget recorded not asserted");
        return;
    }
    assert!(
        landing.latency <= INPUT_TO_FRAME,
        "`G` after idle took {:?}, over the {INPUT_TO_FRAME:?} input-to-frame budget",
        landing.latency
    );
}

/// What `G` after the gap measured.
struct Landing {
    /// `G`'s send → the frame showing the last row.
    latency: Duration,
    /// `G` sent a `ShowRows` (rows not on hand).
    asked: bool,
    /// Wakes after `G`, up to and including the one whose frame showed its
    /// rows (0: `G`'s own frame did).
    between: usize,
    /// Wakes in the gap that nothing asked for.
    gap_wakes: Vec<&'static str>,
    /// What the walk said it laid out before the gap (`None`: never said).
    laid_out: Option<usize>,
    /// The cursor in that frame, and whether it sat on the last row with
    /// the row on hand.
    cursor: usize,
    on_last_row: bool,
}

fn g_after_gap(repo: &Path, no_idle_advance: bool) -> Result<Landing, String> {
    let rows = SCALE.commits() as usize;
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, _startup) =
        ready_to_scroll(repo, TIER, Refs::FiftyThousand, Post(post.clone()), &events)?;
    let state = settle(&jobs, &events, state, rows, no_idle_advance)?;
    let laid_out = state.history.as_ref().and_then(|history| history.laid_out);

    // His terminals answer DSR (ADR-0054): fenced, each fence answered as
    // it is written, so no frame waits on the silent terminal's 16 ms cap.
    let mut answering = capabilities(TIER);
    answering.answers_dsr = true;
    let mut session = Session::settle(&Loaded::default(), answering);
    let sink = Sink::timed(false).answering(post.clone());
    let mut terminal = in_memory(&sink, &state)?;
    let watching = Watching {
        inner: &jobs,
        asked: Mutex::default(),
    };
    let initial = state.clone();
    let keys = vec![(KeyCode::Char('G'), GAP)];
    let feed = KeyFeed::start(keys.clone(), post)
        .map_err(|why| format!("the input thread did not start: {why}"))?;
    let mut wakes = Paced::new(&events, keys, &watching, sink, feed.log());
    let exit = pump(&mut terminal, &mut wakes, &watching, &mut session, state);
    let stamps = feed.finish();
    let exit = exit.map_err(|why| format!("the loop failed: {why}"))?;
    stamps?;
    if !matches!(exit, Exit::Quit) {
        return Err(format!(
            "the loop ended with {exit:?}, not after `G`'s rows"
        ));
    }
    if let Some(failure) = wakes.failure.take() {
        return Err(failure);
    }
    let landing = landed(&wakes, initial, rows, laid_out)?;

    let shutdown = jobs.shutdown(Duration::from_secs(10), graft::jobs::EXIT_WRITE_WAIT, &());
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}, forced {:?}",
            shutdown.stalled, shutdown.forced
        ));
    }
    Ok(landing)
}

/// Pumps `events` until the walk has said `LaidOut` for every row in its
/// final order (skipped with `no_idle_advance`: that build never says it)
/// and startup's other jobs have answered, so the gap after is the app
/// sitting idle.
fn settle(
    jobs: &JobRunner<Post>,
    events: &Receiver<Posted>,
    mut state: AppState,
    rows: usize,
    no_idle_advance: bool,
) -> Result<AppState, String> {
    loop {
        let laid_out = no_idle_advance
            || state.history.as_ref().is_some_and(|history| {
                history.laid_out == Some(rows)
                    && history
                        .walk
                        .as_ref()
                        .is_some_and(|walk| !walk.is_provisional())
            });
        if laid_out && startup_done(&state) {
            return Ok(state);
        }
        let posted = events.recv_timeout(PATIENCE).map_err(|why| {
            format!(
                "settling: no event for {PATIENCE:?} ({why}); laid out {:?} of {rows}",
                state.history.as_ref().and_then(|history| history.laid_out)
            )
        })?;
        let label = posted.wake.label();
        let Wake::Event(event) = posted.wake else {
            return Err(format!("a wake no job sends: {label}"));
        };
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

/// Startup's jobs other than the walk (the working tree, the radar and its
/// probes) have answered: as `idle_advance::no_wake_after_laid_out` waits.
fn startup_done(state: &AppState) -> bool {
    state.working_tree.is_some()
        && matches!(state.radar.context, Loading::Ready(_))
        && state.radar.built_from.is_some()
        && !matches!(state.radar.duplicates, Loading::Pending(_))
        && !state
            .radar
            .force_push
            .values()
            .any(|verdict| matches!(verdict, Loading::Pending(_)))
}

/// Reads `G`'s landing off `wakes`' stamps: the frame after the first window
/// answering the `ShowRows` `G` sent, or `G`'s own frame when it sent none;
/// that frame's state replayed from `initial` to check it shows the last row.
fn landed<D>(
    wakes: &Paced<'_, D>,
    initial: AppState,
    rows: usize,
    laid_out: Option<usize>,
) -> Result<Landing, String> {
    let (key, sent) = wakes
        .handed
        .iter()
        .enumerate()
        .find_map(|(index, handed)| match handed.kind {
            Kind::Key(0, sent) => Some((index, sent)),
            _ => None,
        })
        .ok_or("`G` was never handed to the loop")?;
    let gap_wakes = wakes.unasked_wakes(0..key);
    let handed = &wakes.handed[key];
    let asked = handed.asked_after > handed.asked_before;
    let shown = if asked {
        let requests = wakes.watching.asked();
        let request = *requests
            .get(handed.asked_after - 1)
            .ok_or("`G`'s request was not recorded")?;
        wakes
            .handed
            .iter()
            .enumerate()
            .skip(key + 1)
            .find_map(|(index, wake)| match wake.kind {
                Kind::Window(answered) if answered == request => Some(index),
                _ => None,
            })
            .ok_or_else(|| format!("`G`'s request {request} was never answered"))?
    } else {
        key
    };
    let drawn = wakes
        .drawn
        .get(shown + 1)
        .ok_or("no frame was drawn after `G`'s rows")?;

    let mut state = initial;
    for event in wakes.replay.iter().take(shown + 1) {
        state = update(&state, event.clone()).0;
    }
    let history = state.history.as_ref().ok_or("no history after `G`")?;
    Ok(Landing {
        latency: drawn.saturating_duration_since(sent),
        asked,
        between: shown - key,
        gap_wakes,
        laid_out,
        cursor: history.cursor,
        on_last_row: history.cursor + 1 == rows && history.selected().is_some(),
    })
}
