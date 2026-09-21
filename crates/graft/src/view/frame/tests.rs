//! `view::frame`'s own tests. Moved out of an inline `mod tests` that had
//! grown the file past the 400-line rule (PROMPT.md §6.2; M5 groups
//! 12/13 review, M2), the same shape `update::context_menu::verbs` uses.

use graft_core::job::JobId;
use graft_term::sanitize_line;
use graft_ui::text_input::TextField;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::session::for_tests::session;
use crate::update::confirm::{ConfirmThen, PendingConfirm};
use crate::update::credential::{CredentialPrompt, PromptField};

const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 80,
    height: 24,
};

fn menu_rect() -> Rect {
    Rect {
        x: 0,
        y: 4,
        width: 33,
        height: 15,
    }
}

/// With no detail pane the band runs to the right edge of the screen, so
/// nothing the graph drew survives beside the popup
/// (m5-history-and-ghost-preview/12).
#[test]
fn the_band_runs_to_the_end_of_the_whole_area_when_there_is_no_split() {
    let band = overlay_band(SCREEN, menu_rect(), Some(SCREEN));
    assert_eq!(band.x, 0);
    assert_eq!(band.width, 80, "out to the screen's edge");
    assert_eq!(band.y, 4);
    assert_eq!(band.height, 15, "only the popup's own rows");
}

/// With the detail pane open the band stops at the graph's edge: clearing
/// past it would erase the pane the user is reading.
#[test]
fn the_band_stops_at_the_pane_edge_so_the_detail_pane_survives() {
    let graph = Rect {
        x: 0,
        y: 0,
        width: 48,
        height: 24,
    };
    let band = overlay_band(SCREEN, menu_rect(), Some(graph));
    assert_eq!(band.width, 48, "the graph's width, not the screen's");
    assert_eq!(
        band.x + band.width,
        48,
        "the detail pane's first column is untouched"
    );
}

/// A pane that somehow ends before the popup does still gets cleared
/// under the whole popup — the band is never narrower than what it is
/// there to cover.
#[test]
fn the_band_is_never_narrower_than_the_popup_itself() {
    let narrow = Rect {
        x: 0,
        y: 0,
        width: 10,
        height: 24,
    };
    assert_eq!(overlay_band(SCREEN, menu_rect(), Some(narrow)).width, 33);
    assert_eq!(overlay_band(SCREEN, menu_rect(), None).width, 33);
}

/// One glyph that appears nowhere in any overlay, so a cell still
/// holding it is a cell the overlay never wrote.
const DIRT: char = '#';

/// Fills every cell with [`DIRT`], lets `draw` put an overlay on top,
/// and returns what each row of the terminal ended up holding. This is
/// the graph rows underneath, stood in for by something unmistakable.
fn over_a_dirty_screen(
    columns: u16,
    rows: u16,
    draw: impl FnOnce(&mut Frame<'_>, Rect),
) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(columns, rows)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| {
            let area = frame.area();
            let dirt: String = std::iter::repeat_n(DIRT, usize::from(area.width)).collect();
            for y in area.top()..area.bottom() {
                frame
                    .buffer_mut()
                    .set_string(area.x, y, &dirt, Style::default());
            }
            draw(frame, area);
        })
        .expect("TestBackend cannot fail to draw");
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

/// Asserts that nothing the overlay was handed still shows what was
/// under it.
fn nothing_shows_through(drawn: &[String], rect: Rect, what: &str) {
    for y in rect.top()..rect.bottom() {
        let row: String = drawn[usize::from(y)]
            .chars()
            .skip(usize::from(rect.x))
            .take(usize::from(rect.width))
            .collect();
        assert!(
            !row.contains(DIRT),
            "{what}: row {y} of the overlay's own rectangle still shows \
             what was underneath it — {row:?}\n{}",
            drawn.join("\n")
        );
    }
}

fn a_confirm() -> PendingConfirm {
    PendingConfirm {
        view: graft_ui::confirm::ConfirmView::new(
            sanitize_line("Discard changes?"),
            vec![sanitize_line("3 files lose their edits.")],
            sanitize_line("git restore ."),
        ),
        then: ConfirmThen::Undo { redo: false },
    }
}

fn a_credential() -> CredentialPrompt {
    CredentialPrompt {
        title: sanitize_line("Credentials — example.com"),
        prompt: sanitize_line("Username for 'https://example.com':"),
        field: PromptField::Open(TextField::new("")),
        job: JobId::new(1),
    }
}

/// The confirm box owns the rectangle the caller tints for it — at every
/// size, its degraded sizes included. Below two usable rows the widget
/// draws **nothing**, and without the clear the tint simply recolours
/// the graph row that was there (m5-history-and-ghost-preview/13).
#[test]
fn the_confirm_box_never_shows_what_was_under_it() {
    let state = AppState {
        confirm: Some(a_confirm()),
        ..AppState::default()
    };
    let session = session();
    let view = &state.confirm.as_ref().expect("a box is open").view;
    for (columns, rows) in [(80_u16, 24_u16), (64, 9), (40, 4), (20, 3), (80, 2), (3, 2)] {
        let area = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: rows,
        };
        let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
            draw_confirm(&state, frame, area, &session.theme, &[]);
        });
        let rect = graft_ui::confirm::confirm_area(area, view).intersection(area);
        nothing_shows_through(&drawn, rect, &format!("confirm at {columns}x{rows}"));
    }
}

/// The same for git's credential question. Below seven rows the widget
/// draws one *centred* warning and returns, so without the clear the
/// tinted row keeps the graph at both ends of it.
#[test]
fn the_credential_prompt_never_shows_what_was_under_it() {
    let state = AppState {
        credential: Some(a_credential()),
        ..AppState::default()
    };
    let session = session();
    for (columns, rows) in [
        (80_u16, 24_u16),
        (80, 6),
        (60, 4),
        (39, 8),
        (30, 2),
        (80, 1),
    ] {
        let area = Rect {
            x: 0,
            y: 0,
            width: columns,
            height: rows,
        };
        let drawn = over_a_dirty_screen(columns, rows, |frame, area| {
            draw_credential(&state, frame, area, &session);
        });
        let rect = graft_ui::credential_prompt::credential_area(area).intersection(area);
        nothing_shows_through(&drawn, rect, &format!("credential at {columns}x{rows}"));
    }
}
