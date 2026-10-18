//! `[theme.*]` problems at startup, through `Arrival::first` and
//! `seed_keys` as `run_repo` does (m8-polish-and-ship/16).

use std::sync::Arc;

use mergewright_core::event::{Event, KeyCode, KeyPress};

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
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/m8-16-theme-problems.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!("{graph}\n{status}"),
    );
}

/// Ease-pass 06h: `graph.merged_branches = "open"` beside a bad `[keys]`
/// entry and a bad theme colour — three problems, the graph's last; the
/// app starts folded. `docs/shots/ease-pass/config-merged-branches-problem-120.txt`:
/// the status row, then `?` open with `YOUR CONFIG` first.
#[test]
fn a_graph_mode_it_does_not_take_is_the_last_problem_and_the_app_starts_folded() {
    let file = "[graph]\nmerged_branches = \"open\"\n\n[keys.app]\nundo = \"o\"\n\n\
                [theme.dark]\ntext = \"red\"\n";
    let arrival = Arrival::first(&loaded_from_a_file(file), false);
    assert_eq!(arrival.graph_mode, crate::config::MergedBranches::Folded);
    let mut state = AppState {
        history: Some(crate::view::tests::history_of(5, 0)),
        size: Size {
            columns: 120,
            rows: 36,
        },
        ..AppState::default()
    };
    super::super::startup::arrive(&mut state, arrival);
    assert_eq!(
        state.config_problems.len(),
        3,
        "{:?}",
        state.config_problems
    );
    assert_eq!(
        state.config_problems[2],
        "graph.merged_branches = \"open\" — takes \"folded\" or \"shown\"; using folded"
    );
    assert_eq!(state.graph_mode, crate::config::MergedBranches::Folded);
    let walk = crate::history::History::started(state.graph_mode);
    assert_eq!(walk.view, crate::history::View::folded(), "it folds");

    let graph = draw(&state);
    assert!(
        graph.contains("config: 3 problems · ? shows them"),
        "{graph}"
    );
    let press = KeyPress::plain(KeyCode::Char('?'));
    let help = draw(&update(&state, AppEvent::Core(Event::Key(press))).0);
    let config = help.find("YOUR CONFIG").expect("YOUR CONFIG listed");
    let positions: Vec<usize> = state
        .config_problems
        .iter()
        .map(|problem| {
            let cut: String = problem.chars().take(30).collect();
            help.find(&cut)
                .unwrap_or_else(|| panic!("{problem}:\n{help}"))
        })
        .collect();
    assert!(positions.iter().all(|&at| at > config), "{help}");
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]), "{help}");
    mergewright_fixtures::capture(
        format!(
            "{}/../../docs/shots/ease-pass/config-merged-branches-problem-120.txt",
            env!("CARGO_MANIFEST_DIR")
        ),
        format!(
            "ease-pass 06h, 120x36: `[graph] merged_branches = \"open\"` beside a bad `[keys]` \
             entry and a bad theme colour.\nWhat right looks like: first the graph, its bottom \
             row `config: 3 problems · ? shows them`; then `?` open, `YOUR CONFIG` first, the \
             graph problem last, reading `graph.merged_branches = \"open\" — takes \"folded\" \
             or \"shown\"; using folded`. Colour unverified (rule 5).\n\n{graph}\n{help}\n"
        ),
    );
}
