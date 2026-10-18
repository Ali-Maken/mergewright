//! 20's mouse box: a click selects, a double-click is `⏎`, rows match the picture.

use super::*;

fn click(row: u16) -> AppEvent {
    use mergewright_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    AppEvent::Core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 60,
        row,
        modifiers: Modifiers::NONE,
    }))
}

#[test]
fn mouse_double_click_is_enter() {
    let mut state = blamed_at(22);
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    // Row 0 is the title; screen row 5 is the fifth line (top 0).
    let (once, _) = update(&state, click(5));
    assert_eq!(cursor_line(&once), 5);
    assert!(once.trail.on_screen().is_some(), "one click only selects");
    let (twice, _) = update(&once, click(5));
    assert!(twice.trail.hidden(), "a double-click is `⏎`");
    assert_eq!(
        twice.line_landing.as_ref().map(|landing| landing.spec),
        Some(DetailSpec::Commit(mo()))
    );
}

/// Review Minor 8: the click's row sum (`header_rows`, `keys.rs`) against
/// the picture itself, at two sizes, with and without the ignore line.
#[test]
fn a_click_on_each_drawn_body_row_puts_the_cursor_there() {
    for (columns, rows) in [(120, 40), (80, 24)] {
        for ignore_file in [false, true] {
            let mut state = place_on_screen(pane_at(22), false, ignore_file);
            state.size = crate::update::Size { columns, rows };
            state.harness.clock = crate::session::Clock::Fixed(1_000);
            if let Some(blame) = state.trail.top_mut() {
                blame.cursor = 0;
            }
            let frame = crate::view::tests::draw(&state, columns, rows);
            let body: Vec<u16> = (0..rows)
                .filter(|&row| line_on(&frame, row).is_some())
                .collect();
            assert!(body.len() >= 10, "{columns}×{rows}: body rows {body:?}");
            for row in body {
                let (clicked, _) = update(&state, click(row));
                let drawn = crate::view::tests::draw(&clicked, columns, rows);
                let at = format!("{columns}×{rows} ignore {ignore_file}, row {row}");
                assert_eq!(
                    Some(cursor_line(&clicked)),
                    line_on(&drawn, row),
                    "{at}: the cursor is the clicked row's line"
                );
                let shown = drawn.lines().nth(usize::from(row)).expect("the row");
                assert!(shown.contains('▸'), "{at}: ▸ on it:\n{drawn}");
            }
        }
    }
}

/// The file line drawn on screen row `row` (`line N` / `retry N`).
fn line_on(frame: &str, row: u16) -> Option<u32> {
    let text = frame.lines().nth(usize::from(row))?;
    let (_, after) = text
        .split_once("line ")
        .or_else(|| text.split_once("retry "))?;
    let digits: String = after.chars().take_while(char::is_ascii_digit).collect();
    digits.parse().ok()
}

#[test]
fn a_click_in_blame_never_reaches_the_graph() {
    let state = blamed_at(22);
    let cursor = |state: &AppState| state.history.as_ref().map(|history| history.cursor);
    for row in [0, 1, 5, 20, 39] {
        let (next, _) = update(&state, click(row));
        assert_eq!(cursor(&next), cursor(&state), "row {row} moved the graph");
        assert_eq!(next.detail, state.detail, "row {row} reached the pane");
    }
}
