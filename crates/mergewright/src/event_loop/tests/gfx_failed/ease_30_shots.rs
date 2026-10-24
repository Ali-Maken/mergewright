//! The captures `docs/shots/ease-pass/30-gfx-failed-{before,after}.txt`:
//! the same 100x30 Kitty session after an uncured error reply. Both are
//! drawn by the view from the state the loop reaches: "before" is what main
//! drew (status `Failed`, pixel tier kept, nothing in the column), "after"
//! the state `AppEvent::PixelsFailed` leaves.

use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::GraphicsStatus;
use ratatui::backend::TestBackend;

use super::*;
use crate::update::Size;
use crate::view::view_with_graphics;

fn grid(session: &Session, state: &AppState, status: &GraphicsStatus) -> String {
    let (columns, rows) = (state.size.columns, state.size.rows);
    let mut terminal =
        ratatui::Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend has a size");
    terminal
        .draw(|frame| {
            view_with_graphics(session, state, status, frame);
        })
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer().clone();
    (0..rows)
        .map(|y| {
            let line: String = (0..columns).map(|x| buffer[(x, y)].symbol()).collect();
            line.trim_end().to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn header(what: &str, right: &str) -> String {
    format!(
        "\
# TestBackend capture (not a pty), no colour recorded. Fixture:
# capture_gfx_failed_fallback in crates/mergewright/src/event_loop/tests/gfx_failed/ease_30_shots.rs
# (asserted). 100x30, Kitty pixel tier, after an uncured error reply (EINVAL:bad).
# {what}
# Right: {right}
# Cells under an image are blank in a capture: pixels are not recorded. Colour
# is unverified: a capture cannot show it.
#
"
    )
}

#[test]
fn capture_gfx_failed_fallback() {
    let (session, mut state) = at_pixel_tier();
    state.size = Size {
        columns: 100,
        rows: 30,
    };
    let failed = GraphicsStatus::Failed(sanitize_line("EINVAL:bad"));
    let before = grid(&session, &state, &failed);

    let (dropped, commands) = update(&state, AppEvent::PixelsFailed(sanitize_line("EINVAL:bad")));
    assert!(commands.is_empty());
    let after = grid(&session, &dropped, &GraphicsStatus::Ok);

    assert!(before.contains("gfx: EINVAL:bad"), "{before}");
    assert!(!before.contains('\u{25cf}'), "before: an empty column");
    assert!(after.contains('\u{25cf}'), "after: nodes drawn as text");
    assert!(!after.contains("gfx"), "{after}");
    assert!(
        after.contains("renderer: unicode \u{b7} pixels failed: EINVAL:bad (F3 cycles)"),
        "{after}"
    );

    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/shots/ease-pass/");
    mergewright_fixtures::capture(
        format!("{root}30-gfx-failed-before.txt"),
        format!(
            "{}{before}\n",
            header(
                "BEFORE (main): the graph column is empty, the header says `gfx: EINVAL:bad`.",
                "header `gfx: EINVAL:bad`; no lanes, nodes or badges at the left."
            )
        ),
    );
    mergewright_fixtures::capture(
        format!("{root}30-gfx-failed-after.txt"),
        format!(
            "{}{after}\n",
            header(
                "AFTER: the graph drawn as text from its first column, no `gfx` field.",
                "no `gfx` in the header; status row `renderer: unicode · pixels failed: EINVAL:bad (F3 cycles)`."
            )
        ),
    );
}
