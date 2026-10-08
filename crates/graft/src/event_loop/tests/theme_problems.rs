//! `[theme.*]` problems at startup, through `Arrival::first` and
//! `seed_keys` as `run_repo` does (m8-polish-and-ship/16).

use std::sync::Arc;

use graft_core::event::{Event, KeyCode, KeyPress};

use super::super::startup::Arrival;
use crate::config::tests::loaded_from_a_file;
use crate::update::{AppEvent, AppState, Size, update};

/// The dark theme's built-in `bg` as `#RRGGBB`, read from the theme — no
/// literal.
fn dark_bg_hex() -> String {
    let bg = crate::config::theme_tests::built_in_dark_bg();
    format!("#{:02X}{:02X}{:02X}", bg.red, bg.green, bg.blue)
}

fn arrived(file: &str, no_color: bool) -> AppState {
    let arrival = Arrival::first(&loaded_from_a_file(file), no_color);
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: Size {
            columns: 100,
            rows: 16,
        },
        ..AppState::default()
    };
    state.seed_keys(Arc::clone(&arrival.keymap), arrival.config_problems);
    state
}

fn draw(state: &AppState) -> String {
    crate::view::tests::draw_pinned(state, state.size.columns, state.size.rows)
}

fn f2(state: &AppState) -> AppState {
    let press = KeyPress::plain(KeyCode::Function(2));
    update(state, AppEvent::Core(Event::Key(press))).0
}

#[test]
fn selection_on_bg_is_one_problem_on_the_bottom_row_and_on_f2() {
    let state = arrived(
        &format!("[theme.dark]\nselection = \"{}\"\n", dark_bg_hex()),
        false,
    );
    assert_eq!(
        state.config_problems,
        ["[theme.dark] selection on bg 1.0:1 — under 3.0"]
    );
    let graph = draw(&state);
    assert!(
        graph.contains("config: 1 problem · ? shows them"),
        "{graph}"
    );
    let status = draw(&f2(&state));
    assert!(
        status.contains("[theme.dark] selection on bg 1.0:1 — under 3.0"),
        "{status}"
    );
}

#[test]
fn no_color_drops_the_miss_and_keeps_the_bad_colour() {
    let state = arrived(
        &format!(
            "[theme.dark]\nselection = \"{}\"\ntext = \"red\"\n",
            dark_bg_hex()
        ),
        true,
    );
    assert_eq!(
        state.config_problems,
        ["theme.dark.text: \"red\" is not a #RRGGBB colour"]
    );
}

/// `docs/shots/m8-16-theme-problems.txt`. What right looks like: two
/// frames. First the graph at startup, its bottom row
/// `config: 2 problems · ? shows them`. Second, `F2`'s status screen whose
/// notes list `theme.dark.lane-3: lane colours are fixed, not themeable`
/// then `[theme.dark] selection on bg 1.0:1 — under 3.0` (file warnings
/// first, contrast misses last). Colour of an applied override is not in a
/// character grid: unverified (rule 5).
#[test]
fn capture_theme_problems() {
    let state = arrived(
        &format!(
            "[theme.dark]\nlane-3 = \"#FF0000\"\nselection = \"{}\"\n",
            dark_bg_hex()
        ),
        false,
    );
    assert_eq!(
        state.config_problems,
        [
            "theme.dark.lane-3: lane colours are fixed, not themeable",
            "[theme.dark] selection on bg 1.0:1 — under 3.0",
        ]
    );
    let graph = draw(&state);
    assert!(
        graph.contains("config: 2 problems · ? shows them"),
        "{graph}"
    );
    let status = draw(&f2(&state));
    for problem in &state.config_problems {
        assert!(status.contains(problem.as_str()), "{problem}:\n{status}");
    }
    graft_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-16-theme-problems.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!("{graph}\n{status}"),
    );
}
