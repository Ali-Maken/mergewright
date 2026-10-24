//! The Ghostty-path capture (m7-power-tools/47): `docs/shots/m7/fix-scroll-ghostty.png`.
//!
//! The Pixel-tier frame at release of held `j` (×188, scenario A of
//! `held_key`) on `TeamSpec::MONOREPO` at his 187×50 and 17×37 cell, with
//! Kitty deflate on (Ghostty's path): every strip on screen as Mergewright sends
//! it — the strip's backdrop with the selection's emphasis image over it —
//! built through `StripCache::composited`, the same picture the two Kitty
//! layers blend to (ADR-0053). Stacked top to bottom, cropped to the graph
//! column and the cursor's line ±10 lines, over a stand-in dark background.
//! Only the lines the pane draws are kept, laid out as the real frame places
//! them (`event_loop::placements`, the call `Graphics::drawn` makes): the
//! uncommitted row on top, and under the cursor's row the line its second
//! line takes, drawn as the continuation band. So the cursor's row is never
//! the last line, and the crop's lines are the text grid's, one for one
//! (`scripts/47-scroll-acceptance.sh` step 5).
//!
//! Colour is unverified (CLAUDE.md rule 5) and Ghostty's own rendering is
//! unverified until the user looks. Run by `scripts/47-scroll-acceptance.sh`
//! step 5; compares against the committed file, rewrites it only under
//! `MWR_UPDATE_SHOTS=1` (`mergewright_fixtures::capture::check_bytes`).

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use mergewright::event_loop::{Wake, placements};
use mergewright::jobs::{Dispatch, JobRunner, Shutdown};
use mergewright::update::{AppEvent, AppState, update};
use mergewright::view::strips::{PixelRows, pixel_rows};
use mergewright_core::capabilities::RenderingTier;
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_fixtures::team::{self, TeamSpec};
use mergewright_graph::raster::cache::STRIP_ROWS;

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::ready_to_scroll_at;

/// His terminal, `(columns, rows)` (as `held_key::SIZE`).
const SIZE: (u16, u16) = (187, 50);
/// The hold: `held_key::HELD`.
const HELD: usize = 188;
/// Rows kept above and below the cursor's.
const AROUND: usize = 10;
/// Waiting for laid-out history; a hang guard, not a threshold.
const PATIENCE: Duration = Duration::from_secs(120);
/// Job answers are in once none has come for this long.
const QUIET: Duration = Duration::from_millis(300);
/// The shot's cap (CLAUDE.md "Captures stay small").
const MAX_BYTES: usize = 200 * 1024;

#[test]
fn fix_scroll_ghostty_png() {
    let shot = shared().as_ref().unwrap_or_else(|why| panic!("{why}"));
    println!(
        "SUMMARY ghostty_png bytes={} width_px={} height_px={} cursor_row={} rows={}..{} pane_rows={}..{} cursor_on_screen={} strips={} deflate=on",
        shot.png.len(),
        shot.width,
        shot.height,
        shot.cursor,
        shot.rows.start,
        shot.rows.end,
        shot.pane_rows.start,
        shot.pane_rows.end,
        shot.cursor_on_screen,
        shot.strips
    );
    assert!(
        shot.png.len() <= MAX_BYTES,
        "fix-scroll-ghostty.png is {} bytes, over 200 KB: take fewer rows, never resample",
        shot.png.len()
    );
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/m7/fix-scroll-ghostty.png");
    if let Err(drift) = mergewright_fixtures::capture::check_bytes(&path, &shot.png) {
        panic!("{drift}");
    }
}

/// The red-first test of ease-pass/22: the PNG's lines are the lines the real
/// frame places for the same state (`placements`), the cursor row's second
/// line included, and the cursor row is not the last line it shows.
#[test]
fn the_shot_shows_the_lines_the_real_frame_places() {
    let shot = shared().as_ref().unwrap_or_else(|why| panic!("{why}"));
    let real: Vec<Option<Line>> =
        shot.real[shot.first_line..shot.first_line + shot.lines.len()].to_vec();
    let shown: Vec<Option<Line>> = shot.lines.iter().copied().map(Some).collect();
    assert_eq!(shown, real, "PNG lines from pane line {}", shot.first_line);
    assert!(
        shot.lines.contains(&Line::Second),
        "the cursor row's second line is in the shot: {:?}",
        shot.lines
    );
}

/// The capture and what it shows.
struct Shot {
    png: Vec<u8>,
    width: u32,
    height: u32,
    /// The cursor's history row.
    cursor: usize,
    /// History rows in the crop.
    rows: std::ops::Range<usize>,
    /// The same rows counted from the graph pane's first drawn row (the
    /// uncommitted row, when it is on top): where the text grid's are.
    pane_rows: std::ops::Range<usize>,
    /// Whether the pane draws the cursor's row (rather than a strip row
    /// past the screen's bottom).
    cursor_on_screen: bool,
    /// Strips stacked before cropping.
    strips: usize,
    /// What each line of the PNG shows, top to bottom.
    lines: Vec<Line>,
    /// The pane line the first of them is.
    first_line: usize,
    /// Every line of the pane as the real frame places it.
    real: Vec<Option<Line>>,
}

/// The shot is taken once for both tests (it lays out a 17 700-commit repo).
fn shared() -> &'static Result<Shot, String> {
    static SHOT: std::sync::OnceLock<Result<Shot, String>> = std::sync::OnceLock::new();
    SHOT.get_or_init(shot)
}

/// One line of the graph column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Line {
    /// A history row.
    Row(usize),
    /// The uncommitted row.
    Uncommitted,
    /// The cursor row's second line: the band the real frame leaves under it.
    Second,
}

/// The pane's lines as the real frame places them: `placements`, the same
/// call `Graphics::drawn` makes, read off its rows.
fn real_lines(state: &AppState, pixels: &PixelRows) -> Vec<Option<Line>> {
    let height = state.graph_pane_rows();
    let mut lines = vec![None; height];
    for placed in placements(state, pixels, u16::try_from(height).unwrap_or(u16::MAX)) {
        let strip = &pixels.strips[placed.strip];
        let uncommitted = strip.uncommitted_at();
        for (shown, at) in &placed.room.parts {
            for (k, row) in shown.clone().enumerate() {
                let line = usize::from(*at) + k;
                let history_row =
                    strip.first_row + row - usize::from(uncommitted.is_some_and(|u| u < row));
                if let Some(slot) = lines.get_mut(line) {
                    *slot = Some(if uncommitted == Some(row) {
                        Line::Uncommitted
                    } else {
                        Line::Row(history_row)
                    });
                }
            }
        }
        if let Some((_, line)) = placed.room.band
            && let Some(slot) = lines.get_mut(usize::from(line))
        {
            *slot = Some(Line::Second);
        }
    }
    lines
}

fn shot() -> Result<Shot, String> {
    let repo = team::borrow(&TeamSpec::MONOREPO).map_err(|why| format!("fixture: {why:?}"))?;
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, _startup) = ready_to_scroll_at(
        &repo,
        RenderingTier::Pixel,
        Refs::Fixture,
        Post(post),
        &events,
        SIZE,
    )?;
    let mut state = answered(&jobs, state, &events, PATIENCE)?;
    let from = state.history.as_ref().map_or(0, |history| history.cursor);
    for _ in 0..HELD {
        let key = AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('j'))));
        let (next, commands) = update(&state, key);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
    // The thread and window answers the hold asked for.
    let state = answered(&jobs, state, &events, QUIET)?;
    let built = composed(&state, from + HELD);
    let shutdown = jobs.shutdown(
        Duration::from_secs(10),
        mergewright::jobs::EXIT_WRITE_WAIT,
        &(),
    );
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}, forced {:?}",
            shutdown.stalled, shutdown.forced
        ));
    }
    built
}

/// Pumps job events into `state` until history has laid out (`LaidOut`) and
/// then none has come for [`QUIET`]; `first` bounds the first wait.
pub(crate) fn answered(
    jobs: &JobRunner<Post>,
    mut state: AppState,
    events: &Receiver<Posted>,
    first: Duration,
) -> Result<AppState, String> {
    let laid_out = |state: &AppState| state.history.as_ref().is_some_and(|h| h.laid_out.is_some());
    loop {
        let wait = if laid_out(&state) {
            QUIET.min(first)
        } else {
            first
        };
        let posted = match events.recv_timeout(wait) {
            Ok(posted) => posted,
            Err(mpsc::RecvTimeoutError::Timeout) if laid_out(&state) => return Ok(state),
            Err(why) => return Err(format!("waiting for job answers: {why}")),
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

/// The cursor's row, once the hold landed and the lane cap reads as drawn.
fn checked(state: &AppState, want: usize) -> Result<usize, String> {
    let cursor = state.history.as_ref().ok_or("no history")?.cursor;
    if cursor != want {
        return Err(format!(
            "the cursor is on row {cursor}, not {want}: the hold did not land"
        ));
    }
    // As `view::max_lanes` reads it: with the detail pane closed it is
    // `max_lanes_for(screen columns, configured)` (`view/rows/window.rs`,
    // `view/frame/layout.rs::graph_columns`).
    if state.detail.shown {
        return Err("the detail pane is open: the lane cap would read a narrower graph".to_owned());
    }
    Ok(cursor)
}

/// Every strip on screen, composited, laid out as the real frame places them
/// (`placements`: the uncommitted row, the cursor row's second line with its
/// continuation band), cropped around the cursor.
fn composed(state: &AppState, want: usize) -> Result<Shot, String> {
    let cursor = checked(state, want)?;
    let max_lanes =
        mergewright_ui::graph_screen::max_lanes_for(state.size.columns, state.max_lanes);
    let pixels = pixel_rows(state, max_lanes, STRIP_ROWS);
    let paint = kitty_paint();
    let Stack {
        drawn,
        pixels: stacked,
        width,
    } = stack(&pixels, &paint)?;
    let cell_height = paint.cell_height as usize;
    let stride = width as usize * 4;
    if stacked.len() != drawn.len() * cell_height * stride {
        return Err("the strips' pixels do not match their rows".to_owned());
    }
    let real = real_lines(state, &pixels);
    let at = real
        .iter()
        .position(|line| *line == Some(Line::Row(cursor)))
        .ok_or_else(|| format!("the cursor's row {cursor} is on no line of the pane"))?;
    let cursor_on_screen = true;
    let first = at.saturating_sub(AROUND);
    let end = (at + AROUND + 1).min(real.len());
    let kept = &real[first..end];
    let band = continuation(state, &pixels, &paint)?;
    if band.as_ref().is_some_and(|(w, _)| *w != width) {
        return Err("the continuation band is not as wide as the strips".to_owned());
    }
    let (canvas, rows) = crop(
        kept,
        first,
        &stacked,
        &drawn,
        band.as_ref(),
        (width, cell_height),
    )?;
    let height =
        u32::try_from(kept.len() * cell_height).map_err(|why| format!("crop height: {why}"))?;
    let frames = pixels.strips.len();
    let png = on_dark(canvas, width, height)?;
    Ok(Shot {
        png,
        width,
        height,
        cursor,
        pane_rows: first..end,
        cursor_on_screen,
        rows: rows.0..rows.1,
        strips: frames,
        lines: kept.iter().filter_map(|line| *line).collect(),
        first_line: first,
        real,
    })
}

mod pixels;
pub(crate) use pixels::on_dark;
use pixels::{Stack, continuation, crop, kitty_paint, stack};
