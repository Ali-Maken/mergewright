//! Writing a `docs/shots/` capture: one frame of the real `view::view`, and
//! the file it goes into. Shared, because a capture written two slightly
//! different ways is two pictures of two apps.
#![allow(
    clippy::expect_used,
    reason = "a capture that cannot be drawn or written has nothing to show; the panic names why"
)]

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;

/// Every row of the frame `view::view` draws for `state` at this size, drawn
/// against the wall clock.
///
/// A capture must not use this: its relative dates would read a different
/// age every run. Capture writers call [`frame_at`] with
/// `Some(Clock::Fixed(…))`.
pub fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    frame_at(state, columns, rows, None)
}

/// [`frame`], with the instant its relative dates are drawn against. `None`
/// is the wall clock; a capture that must not age passes
/// `Some(Clock::Fixed(…))`.
pub fn frame_at(state: &AppState, columns: u16, rows: u16, clock: Option<Clock>) -> Vec<String> {
    let session = graft::session::Session::settle(
        &graft::config::Loaded::default(),
        crate::hint_bar::capabilities(),
    );
    let mut state = state.clone();
    state.size = graft::update::Size { columns, rows };
    if let Some(clock) = clock {
        state.harness.clock = clock;
    }
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("test backend opens");
    terminal
        .draw(|frame| graft::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

/// Writes one capture into `docs/shots/` and says where it went.
pub fn write_shot(name: &str, body: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/shots")
        .join(name);
    std::fs::write(&path, body).expect("docs/shots is writable");
    println!("wrote {} ({} bytes)", path.display(), body.len());
}
