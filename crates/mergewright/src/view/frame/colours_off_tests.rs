//! `#` (ease-pass 07, G14): every cell of the frame loses its colour, the
//! glyphs stay, `#` again brings it back, and a screen change keeps it.

use mergewright_core::event::{Event, KeyCode, KeyPress};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;

use crate::session::for_tests::session;
use crate::update::{AppState, Screen, Size};
use crate::view::tests::history_of;
use crate::view::view;

fn press(state: &AppState, code: KeyCode) -> AppState {
    crate::update::update(
        state,
        crate::update::AppEvent::Core(Event::Key(KeyPress::plain(code))),
    )
    .0
}

fn frame(state: &AppState) -> Buffer {
    let mut terminal =
        Terminal::new(TestBackend::new(80, 12)).expect("TestBackend reports its size");
    terminal
        .draw(|frame| view(&session(), state, frame))
        .expect("TestBackend cannot fail to draw");
    terminal.backend().buffer().clone()
}

fn coloured_cells(buffer: &Buffer) -> usize {
    buffer
        .content()
        .iter()
        .filter(|cell| cell.fg != Color::Reset || cell.bg != Color::Reset)
        .count()
}

fn glyphs(buffer: &Buffer) -> String {
    buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect()
}

fn graph_state() -> AppState {
    AppState {
        history: Some(history_of(5, 1)),
        size: Size {
            columns: 80,
            rows: 12,
        },
        ..AppState::default()
    }
}

#[test]
fn hash_takes_every_colour_off_the_frame_and_hash_again_puts_it_back() {
    let state = graph_state();
    let before = frame(&state);
    assert!(coloured_cells(&before) > 0, "the fixture draws in colour");

    let off = press(&state, KeyCode::Char('#'));
    assert!(off.look.colours_off);
    let drawn = frame(&off);
    assert_eq!(coloured_cells(&drawn), 0, "no cell keeps a colour");
    assert_eq!(glyphs(&drawn), glyphs(&before), "the shapes stay");

    let on = press(&off, KeyCode::Char('#'));
    assert!(!on.look.colours_off);
    assert_eq!(frame(&on), before, "`#` again draws as before");
}

#[test]
fn colours_stay_off_across_a_screen_change_and_back() {
    let off = press(&graph_state(), KeyCode::Char('#'));
    let status = press(&off, KeyCode::Function(2));
    assert_eq!(status.screen, Screen::Status);
    assert_eq!(coloured_cells(&frame(&status)), 0, "the status screen too");
    let back = press(&status, KeyCode::Function(2));
    assert_eq!(back.screen, Screen::Graph);
    assert!(back.look.colours_off);
    assert_eq!(coloured_cells(&frame(&back)), 0);
}
