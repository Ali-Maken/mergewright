//! ease-pass/20: the pinned uncommitted row, before and after it became its own
//! image: `docs/shots/ease-pass/20-pinned-row-{before,after}.png`.
//!
//! `TeamSpec::MONOREPO` at his 187x50 and 17x37 cell, `j` x60, the uncommitted
//! row pinned on top with `HEAD` scrolled off. The top six pane rows of the
//! graph column, every strip composited as `ghostty_shot` stacks them, over a
//! stand-in dark background. Colour is unverified (CLAUDE.md rule 5); judged by
//! the user's eye, not asserted. Compares against the committed file, rewrites
//! it only under `MWR_UPDATE_SHOTS=1`.

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use mergewright::config::Loaded;
use mergewright::jobs::Dispatch;
use mergewright::session::Session;
use mergewright::update::{AppEvent, update};
use mergewright::view::strips::{StripPaint, pixel_rows};
use mergewright_core::capabilities::RenderingTier;
use mergewright_core::event::{Event, KeyCode, KeyPress};
use mergewright_fixtures::team::{self, TeamSpec};
use mergewright_graph::raster::cache::STRIP_ROWS;

use crate::frame_trace::{Post, Posted};
use crate::frames::Refs;
use crate::frames::run::{capabilities, ready_to_scroll_at};
use crate::ghostty_shot::{answered, on_dark, stack};

const SIZE: (u16, u16) = (187, 50);
const PRESSES: usize = 60;
/// Pane rows kept from the top: the pinned row and five under it.
const KEPT: usize = 6;
const PATIENCE: Duration = Duration::from_secs(120);
const QUIET: Duration = Duration::from_millis(300);
/// The file this build is compared against (`before` was taken at 3123840e).
const FILE: &str = "20-pinned-row-before.png";

#[test]
fn pinned_row_png() {
    let repo = team::borrow(&TeamSpec::MONOREPO).expect("fixture");
    let (post, events) = mpsc::channel::<Posted>();
    let (jobs, state, _startup) = ready_to_scroll_at(
        &repo,
        RenderingTier::Pixel,
        Refs::Fixture,
        Post(post),
        &events,
        SIZE,
    )
    .expect("ready");
    let mut state = answered(&jobs, state, &events, PATIENCE).expect("laid out");
    for _ in 0..PRESSES {
        let key = AppEvent::Core(Event::Key(KeyPress::plain(KeyCode::Char('j'))));
        let (next, commands) = update(&state, key);
        state = next;
        for command in commands {
            jobs.dispatch(command);
        }
    }
    let state = answered(&jobs, state, &events, QUIET).expect("answers");
    let max_lanes =
        mergewright_ui::graph_screen::max_lanes_for(state.size.columns, state.max_lanes);
    let pixels = pixel_rows(&state, max_lanes, STRIP_ROWS);
    assert!(
        pixels.uncommitted_on_top && !state.uncommitted_row_attached(),
        "the frame must have the uncommitted row pinned with HEAD scrolled off"
    );
    let session = Session::settle(&Loaded::default(), {
        let mut caps = capabilities(RenderingTier::Pixel);
        caps.kitty_deflate = true;
        caps
    });
    let paint = StripPaint::of(&session);
    let stacked = stack(&pixels, &paint).expect("stack");
    let first = stacked
        .drawn
        .iter()
        .position(Option::is_none)
        .expect("the uncommitted row is in the stack");
    let cell_height = paint.cell_height as usize;
    let stride = stacked.width as usize * 4;
    let top = first * cell_height * stride;
    let bottom = (first + KEPT) * cell_height * stride;
    let height = u32::try_from(KEPT * cell_height).expect("height");
    let png = on_dark(stacked.pixels[top..bottom].to_vec(), stacked.width, height).expect("png");
    println!("SUMMARY pinned_row_png bytes={} under={:?}", png.len(), stacked.drawn.get(first + 1));
    assert!(png.len() <= 200 * 1024, "{} bytes: over the 200 KB cap", png.len());
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/shots/ease-pass").join(FILE);
    let checked = mergewright_fixtures::capture::check_bytes(&path, &png);
    jobs.shutdown(Duration::from_secs(10), mergewright::jobs::EXIT_WRITE_WAIT, &());
    if let Err(drift) = checked {
        panic!("{drift}");
    }
}
