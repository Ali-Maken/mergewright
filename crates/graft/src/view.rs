//! The end of the loop: `view(state)` (ARCHITECTURE.md).
//!
//! One pure function from [`AppState`] to a frame. It reads the state and
//! draws; it never changes anything, so what is on screen is always a direct
//! function of what the application knows.
//!
//! What it draws is a placeholder — this ticket builds the loop, and the real
//! screens arrive with `ui` in M1. What matters here is the part that outlives
//! the placeholder: the drawing survives a terminal of any size, including the
//! `0×0` one a pty reports before it has been sized, which is exactly what
//! silently broke the graph spike (ADR-0012).

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::widgets::Paragraph;

use crate::update::{AppState, Status};

/// What the placeholder screen says until M1 replaces it.
const PLACEHOLDER: &str = "Graft — press q to quit";

/// Draw the current state.
pub fn view(state: &AppState, frame: &mut Frame) {
    let area = frame.area();
    if area.is_empty() {
        // Nothing to draw into. Not an error — a terminal can legitimately be
        // this size for a moment — so there is nothing to report either.
        return;
    }

    let (body, status) = split_off_status_line(area);
    frame.render_widget(Paragraph::new(PLACEHOLDER), body);
    if let Some(status) = status {
        frame.render_widget(Paragraph::new(status_text(&state.status)), status);
    }
}

/// Give the status line the bottom row, and the rest to the body.
///
/// A terminal one row tall gets no status line rather than a body squeezed to
/// nothing: with only one row to spend, the content is what the user needs.
fn split_off_status_line(area: Rect) -> (Rect, Option<Rect>) {
    if area.height < 2 {
        return (area, None);
    }

    let body = Rect {
        height: area.height - 1,
        ..area
    };
    let status = Rect {
        y: area.y + area.height - 1,
        height: 1,
        ..area
    };
    (body, Some(status))
}

/// What the status line reads.
///
/// `Ready` says so plainly. A failure is shown as the failure text itself —
/// there is no permanent `error:` label, because a field that always looks
/// like an error is a field nobody reads (CLAUDE.md).
fn status_text(status: &Status) -> &str {
    match status {
        Status::Ready => "ready",
        Status::Failed(message) => message,
    }
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::update::Size;

    /// Draw `state` into a terminal of exactly this size and return what the
    /// cells ended up holding.
    fn draw(state: &AppState, columns: u16, rows: u16) -> String {
        let mut terminal =
            Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
        terminal
            .draw(|frame| view(state, frame))
            .expect("TestBackend cannot fail to draw");
        terminal.backend().to_string()
    }

    #[test]
    fn a_normal_terminal() {
        insta::assert_snapshot!(draw(&AppState::default(), 40, 4));
    }

    #[test]
    fn a_failure_replaces_the_status_text() {
        let state = AppState {
            status: Status::Failed("job 3 failed: no such ref".to_owned()),
            ..AppState::default()
        };

        insta::assert_snapshot!(draw(&state, 40, 4));
    }

    #[test]
    fn a_terminal_with_no_cells_at_all() {
        // ADR-0012's degenerate pty: the size a pty reports before anyone has
        // sized it. The point of the test is that drawing returns at all.
        //
        // No snapshot here, because rendering a zero-width `TestBackend` to
        // text is what panics — the draw itself is fine, and an empty buffer
        // is the only thing a snapshot could show anyway.
        let mut terminal =
            Terminal::new(TestBackend::new(0, 0)).expect("TestBackend reports its size");

        terminal
            .draw(|frame| view(&AppState::default(), frame))
            .expect("TestBackend cannot fail to draw");

        assert_eq!(terminal.backend().buffer().area.area(), 0);
    }

    #[test]
    fn a_terminal_two_cells_square() {
        insta::assert_snapshot!(draw(&AppState::default(), 2, 2));
    }

    #[test]
    fn a_terminal_one_row_tall_keeps_the_body() {
        insta::assert_snapshot!(draw(&AppState::default(), 12, 1));
    }

    #[test]
    fn a_terminal_with_width_but_no_height() {
        insta::assert_snapshot!(draw(&AppState::default(), 40, 0));
    }

    #[test]
    fn drawing_does_not_change_the_state() {
        let state = AppState {
            should_quit: false,
            status: Status::Ready,
            size: Size {
                columns: 40,
                rows: 4,
            },
        };
        let before = state.clone();

        let _ = draw(&state, 40, 4);

        assert_eq!(state, before);
    }
}
