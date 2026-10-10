//! The emphasis-pieces capture (m8-polish-and-ship 05 Part 2):
//! `docs/shots/m8-05-emphasis-pieces.png`.
//!
//! `TeamSpec::MONOREPO` at his Ghostty shape (222×70, 17×37 cell), the
//! cursor moved down with `j` onto the first merge row past the start, then
//! the same with Focus on (`F`). Each panel is what Kitty is sent: every
//! strip's backdrop with each of its emphasis pieces composited over it in
//! column order (`emphasis_pieces` + `rasterise_piece`, ADR-0053 amendment
//! 2026-10-10), cropped to the cursor's row ±10, over a stand-in dark
//! background. Top: Focus off. Bottom: Focus on.
//!
//! Colour is **unverified** (CLAUDE.md rule 5), and so is how a terminal
//! stacks pieces placed at one z (here: column order). Compares against the
//! committed file; rewrites it only under `TRIB_UPDATE_SHOTS=1`.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use graft::config::Loaded;
use graft::jobs::{Dispatch, Shutdown};
use graft::session::Session;
use graft::update::{AppEvent, AppState, update};
use graft::view::strips::{StripPaint, pixel_rows};
use graft_core::capabilities::RenderingTier;
use graft_core::event::{Event, KeyCode, KeyPress};
use graft_fixtures::team::{self, TeamSpec};
use graft_graph::raster::cache::STRIP_ROWS;
use graft_graph::raster::{composite, emphasis_pieces, rasterise_backdrop, rasterise_piece};

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::{capabilities, ready_to_scroll_at};
use crate::ghostty_shot::{answered, on_dark};

/// His Ghostty run's shape (`docs/perf/m8-trace-2026-10-08-ghostty.txt`).
const SIZE: (u16, u16) = (222, 70);
/// Rows kept above and below the cursor's.
const AROUND: usize = 10;
/// Waiting for laid-out history; a hang guard, not a threshold.
const PATIENCE: Duration = Duration::from_secs(120);
const QUIET: Duration = Duration::from_millis(300);
/// The shot's cap (CLAUDE.md "Captures stay small").
const MAX_BYTES: usize = 200 * 1024;
/// Dark rows between the two panels.
const GAP_PX: usize = 8;

#[test]
fn emphasis_pieces_png() {
    let (png, merge_row, pieces) = shot().unwrap_or_else(|why| panic!("{why}"));
    println!(
        "SUMMARY pieces_png bytes={} merge_row={merge_row} pieces_off={} pieces_focus={}",
        png.len(),
        pieces.0,
        pieces.1
    );
    assert!(png.len() <= MAX_BYTES, "{} bytes, over 200 KB", png.len());
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/m8-05-emphasis-pieces.png");
    if let Err(drift) = graft_fixtures::capture::check_bytes(&path, &png) {
        panic!("{drift}");
    }
}

/// The PNG, the merge row the cursor sits on, and how many pieces each
/// panel composited.
type Shot = (Vec<u8>, usize, (usize, usize));

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
    let press = |state: &AppState, code: KeyCode| {
        let (next, commands) = update(state, AppEvent::Core(Event::Key(KeyPress::plain(code))));
        for command in commands {
            jobs.dispatch(command);
        }
        next
    };
    // `j` until the cursor sits on a merge, at least one row down.
    for _ in 0..200 {
        state = press(&state, KeyCode::Char('j'));
        if cursor_on_merge(&state) {
            break;
        }
    }
    if !cursor_on_merge(&state) {
        return Err("no merge within 200 rows of the start".to_owned());
    }
    let state = answered(&jobs, state, &events, QUIET)?;
    let merge_row = state.history.as_ref().map_or(0, |history| history.cursor);
    let (off, off_pieces) = panel(&state)?;
    let focused = press(&state, KeyCode::Char('F'));
    let focused = answered(&jobs, focused, &events, QUIET)?;
    let (on, on_pieces) = panel(&focused)?;
    let shutdown = jobs.shutdown(Duration::from_secs(10), graft::jobs::EXIT_WRITE_WAIT, &());
    if shutdown != Shutdown::CLEAN {
        return Err(format!(
            "the job pool did not stop cleanly: stalled {}",
            shutdown.stalled
        ));
    }
    if off.width != on.width {
        return Err(format!(
            "panels differ in width: {} vs {}",
            off.width, on.width
        ));
    }
    let same = off.pixels == on.pixels;
    println!(
        "SUMMARY pieces_png focus_on={} focus_changes_pixels={}",
        focused.focus, !same
    );
    let stride = off.width as usize * 4;
    let mut pixels = off.pixels;
    pixels.resize(pixels.len() + GAP_PX * stride, 0);
    pixels.extend_from_slice(&on.pixels);
    let height = u32::try_from(pixels.len() / stride).map_err(|why| why.to_string())?;
    let png = on_dark(pixels, off.width, height)?;
    Ok((png, merge_row, (off_pieces, on_pieces)))
}

fn cursor_on_merge(state: &AppState) -> bool {
    let Some(history) = state.history.as_ref() else {
        return false;
    };
    let Some(window) = history.window.as_ref() else {
        return false;
    };
    history
        .cursor
        .checked_sub(window.first)
        .and_then(|at| window.rows.get(at))
        .is_some_and(|row| row.commit.parents.len() > 1)
}

/// One panel: premultiplied RGBA, `width` pixels a line.
struct Panel {
    pixels: Vec<u8>,
    width: u32,
}

/// Every strip on screen as Kitty is sent it (backdrop, then each piece
/// over it), stacked, cropped to the cursor's row ±[`AROUND`]; and the
/// number of pieces composited.
fn panel(state: &AppState) -> Result<(Panel, usize), String> {
    let history = state.history.as_ref().ok_or("no history")?;
    if state.detail.shown {
        return Err("the detail pane is open: the lane cap would read a narrower graph".to_owned());
    }
    let pixels = pixel_rows(state, graft::view::max_lanes(state), STRIP_ROWS);
    let session = Session::settle(&Loaded::default(), {
        let mut caps = capabilities(RenderingTier::Pixel);
        caps.kitty_deflate = true;
        caps
    });
    let paint = StripPaint::of(&session);
    let (mut drawn, mut stacked, mut width, mut count) = (Vec::new(), Vec::new(), None, 0);
    for strip in &pixels.strips {
        let frame = strip.with_layers(&pixels, &paint, |plain, selection| {
            // The strip key only names the pieces; nothing here is held.
            let mut frame = rasterise_backdrop(plain, selection.backdrop)?;
            for piece in emphasis_pieces(plain, selection) {
                if let Some(top) = rasterise_piece(plain, selection, piece)? {
                    frame = composite(&frame, &top);
                    count += 1;
                }
            }
            Ok::<_, graft_graph::raster::RasterError>(frame)
        });
        let frame = frame.map_err(|why| format!("strip {}: {why:?}", strip.first_row))?;
        if *width.get_or_insert(frame.width) != frame.width {
            return Err(format!(
                "strip {} is {} px wide",
                strip.first_row, frame.width
            ));
        }
        let uncommitted = strip.uncommitted_at();
        let mut row = strip.first_row;
        for k in 0..strip.row_count() {
            if uncommitted == Some(k) {
                drawn.push(None);
            } else {
                drawn.push(Some(row));
                row += 1;
            }
        }
        stacked.extend_from_slice(&frame.pixels);
    }
    let width = width.ok_or("no strip on screen")?;
    let at = drawn
        .iter()
        .position(|row| *row == Some(history.cursor))
        .ok_or("the cursor's row is in no strip")?;
    let shown = at.saturating_sub(AROUND)..(at + AROUND + 1).min(drawn.len());
    let line = paint.cell_height as usize * width as usize * 4;
    let pixels = stacked[shown.start * line..shown.end * line].to_vec();
    Ok((Panel { pixels, width }, count))
}
