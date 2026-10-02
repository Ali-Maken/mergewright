//! The Ghostty-path capture (m7-power-tools/47): `docs/shots/m7/fix-scroll-ghostty.png`.
//!
//! The Pixel-tier frame at release of held `j` (×188, scenario A of
//! `held_key`) on `TeamSpec::MONOREPO` at his 187×50 and 17×37 cell, with
//! Kitty deflate on (Ghostty's path): every strip on screen as Graft sends
//! it — the strip's backdrop with the selection's emphasis image over it —
//! built through `StripCache::composited`, the same picture the two Kitty
//! layers blend to (ADR-0053). Stacked top to bottom, cropped to the graph
//! column and the cursor's row ±10 rows, over a stand-in dark background.
//! Only rows the pane draws are kept: the strips run past the screen, and
//! with the uncommitted row on top the pane draws one history row fewer
//! (`view/strips.rs` `pixel_rows`). `cursor_on_screen` says whether the
//! cursor's row is one of them; the shot does not assert it (47 reports, it
//! does not fix).
//!
//! Colour is unverified (CLAUDE.md rule 5) and Ghostty's own rendering is
//! unverified until the user looks. Run by `scripts/47-scroll-acceptance.sh`
//! step 5; compares against the committed file, rewrites it only under
//! `GRAFT_UPDATE_SHOTS=1` (`graft_fixtures::capture::check_bytes`).

use std::path::Path;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use graft::config::Loaded;
use graft::event_loop::Wake;
use graft::jobs::{Dispatch, JobRunner, Shutdown};
use graft::session::Session;
use graft::update::{AppEvent, AppState, update};
use graft::view::strips::{PixelRows, StripPaint, pixel_rows};
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_fixtures::team::{self, TeamSpec};
use graft_graph::raster::cache::{STRIP_ROWS, StripCache};

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::{capabilities, ready_to_scroll_at};

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
    let shot = shot().unwrap_or_else(|why| panic!("{why}"));
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
    if let Err(drift) = graft_fixtures::capture::check_bytes(&path, &shot.png) {
        panic!("{drift}");
    }
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
    if let Shutdown::Stalled { .. } = jobs.shutdown(Duration::from_secs(10)) {
        return Err("the job pool did not stop cleanly".to_owned());
    }
    built
}

/// Pumps job events into `state` until history has laid out (`LaidOut`) and
/// then none has come for [`QUIET`]; `first` bounds the first wait.
fn answered(
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

/// Every strip on screen, composited and stacked, cropped around the cursor.
fn composed(state: &AppState, want: usize) -> Result<Shot, String> {
    let history = state.history.as_ref().ok_or("no history")?;
    let cursor = history.cursor;
    if cursor != want {
        return Err(format!(
            "the cursor is on row {cursor}, not {want}: the hold did not land"
        ));
    }
    // `view::max_lanes` is crate-private; with the detail pane closed it is
    // `max_lanes_for(screen columns, configured)` (`view/rows.rs`,
    // `view/frame/layout.rs::graph_columns`).
    if state.detail.shown {
        return Err("the detail pane is open: the lane cap would read a narrower graph".to_owned());
    }
    let max_lanes = graft_ui::graph_screen::max_lanes_for(state.size.columns, state.max_lanes);
    let pixels = pixel_rows(state, max_lanes, STRIP_ROWS);
    let session = Session::settle(&Loaded::default(), {
        let mut caps = capabilities(RenderingTier::Pixel);
        caps.kitty_deflate = true;
        caps
    });
    let paint = StripPaint::of(&session);
    let Stack {
        drawn,
        pixels: stacked,
        width,
    } = stack(&pixels, &paint)?;
    let at = drawn
        .iter()
        .position(|row| *row == Some(cursor))
        .ok_or_else(|| format!("the cursor's row {cursor} is in no strip on screen"))?;
    // Strips are aligned, so they run past the screen's top and bottom: the
    // pane draws `graph_pane_rows` rows (the uncommitted row and the history
    // rows under it) from the uncommitted row when it is on top, else from
    // `top` (48).
    let pane_first = if pixels.uncommitted_on_top {
        drawn.iter().position(Option::is_none)
    } else {
        drawn.iter().position(|row| *row == Some(history.top))
    }
    .ok_or("the pane's first row is in no strip")?;
    let pane = pane_first..(pane_first + state.graph_pane_rows()).min(drawn.len());
    let shown = at.saturating_sub(AROUND).max(pane.start)..(at + AROUND + 1).min(pane.end);
    let cursor_on_screen = pane.contains(&at);
    let rows =
        drawn[shown.start].unwrap_or(cursor)..drawn[shown.end - 1].map_or(cursor, |row| row + 1);
    let cell_height = paint.cell_height as usize;
    let stride = width as usize * 4;
    if stacked.len() != drawn.len() * cell_height * stride {
        return Err("the strips' pixels do not match their rows".to_owned());
    }
    let top = shown.start * cell_height * stride;
    let bottom = shown.end * cell_height * stride;
    let height =
        u32::try_from(shown.len() * cell_height).map_err(|why| format!("crop height: {why}"))?;
    let frames = pixels.strips.len();
    let png = on_dark(stacked[top..bottom].to_vec(), width, height)?;
    Ok(Shot {
        png,
        width,
        height,
        cursor,
        pane_rows: shown.start - pane.start..shown.end - pane.start,
        cursor_on_screen,
        rows,
        strips: frames,
    })
}

/// The frame's strips, composited, one under the other.
struct Stack {
    /// Every drawn row, top to bottom: its history row, or `None` for the
    /// uncommitted row (which pushes the rows under it down).
    drawn: Vec<Option<usize>>,
    /// Premultiplied RGBA, `width` pixels a line.
    pixels: Vec<u8>,
    width: u32,
}

fn stack(frame: &PixelRows, paint: &StripPaint) -> Result<Stack, String> {
    let mut cache = StripCache::new(STRIP_ROWS, frame.strips.len().max(1));
    let mut drawn = Vec::new();
    let mut pixels = Vec::new();
    let mut width = None;
    let mut next_row = None;
    for strip in &frame.strips {
        let raster = strip
            .with_layers(frame, paint, |plain, selection| {
                cache.composited(strip.first_row, plain, selection)
            })
            .map_err(|why| format!("composited strip {}: {why:?}", strip.first_row))?;
        let follows = next_row.is_none_or(|next| next == strip.first_row);
        if !follows || *width.get_or_insert(raster.width) != raster.width {
            return Err(format!(
                "strip at row {} (width {}) does not follow row {next_row:?} (width {width:?})",
                strip.first_row, raster.width
            ));
        }
        let uncommitted = strip.uncommitted_at();
        let mut history_row = strip.first_row;
        for k in 0..strip.row_count() {
            if uncommitted == Some(k) {
                drawn.push(None);
            } else {
                drawn.push(Some(history_row));
                history_row += 1;
            }
        }
        next_row = Some(history_row);
        pixels.extend_from_slice(&raster.pixels);
    }
    Ok(Stack {
        drawn,
        pixels,
        width: width.ok_or("no strip on screen")?,
    })
}

/// Premultiplied RGBA over a stand-in dark background (as
/// `graft-graph/tests/it/emphasis_layer.rs::on_dark`), as a PNG.
fn on_dark(pixels: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, String> {
    use tiny_skia::{Color, IntSize, Pixmap, PixmapPaint, Transform};
    let size = IntSize::from_wh(width, height).ok_or("an empty crop has no PNG")?;
    let strip = Pixmap::from_vec(pixels, size).ok_or("the pixel buffer does not match its size")?;
    let mut canvas = Pixmap::new(width, height).ok_or("no canvas for the background")?;
    canvas.fill(Color::from_rgba8(0x1E, 0x1E, 0x2E, 255));
    canvas.draw_pixmap(
        0,
        0,
        strip.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
    canvas
        .encode_png()
        .map_err(|why| format!("encoding a PNG: {why}"))
}
