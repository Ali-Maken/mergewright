//! `GRAPH MODE` in the `?` list (ease-pass 06i): listed after `HERE` on
//! the graph, found by its words, `⏎` (or a double-click) switches and asks
//! the loop to keep it; the bar says whether it was kept.

use super::graph_mode::{WALK, oid, one_request};
use super::*;
use crate::config::MergedBranches;
use crate::update::palette::{AppAction, PaletteTarget, rows};
use crate::update::tests::reveal::{folded_hits, on};
use mergewright_ui::palette::RowKind;

/// Every shown row: header, label, kind, target.
fn shown(state: &AppState) -> Vec<(String, String, RowKind, PaletteTarget)> {
    let palette = state.palette.as_ref().expect("the palette is up");
    rows(state, palette)
        .into_iter()
        .flat_map(|(header, rows)| {
            let header = header.as_str().to_owned();
            rows.into_iter().map(move |(row, target)| {
                (
                    header.clone(),
                    row.label.as_str().to_owned(),
                    row.kind,
                    target,
                )
            })
        })
        .collect()
}

fn headers(state: &AppState) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (header, ..) in shown(state) {
        if out.last() != Some(&header) {
            out.push(header);
        }
    }
    out
}

fn typed(state: &AppState, word: &str) -> AppState {
    word.chars().fold(state.clone(), |state, c| {
        update(&state, key(KeyCode::Char(c))).0
    })
}

/// `?` on the graph, the merge fixture (Folded merges in force).
fn help() -> AppState {
    let (mut state, _) = merge_on_screen(WALK, 1);
    state.size = Size {
        columns: 120,
        rows: 36,
    };
    update(&state, key(KeyCode::Char('?'))).0
}

fn cursor_on(mut state: AppState, label: &str) -> AppState {
    let at = shown(&state)
        .iter()
        .position(|(h, l, ..)| h == "GRAPH MODE" && l == label)
        .unwrap_or_else(|| panic!("{label}: {:?}", shown(&state)));
    state.palette.as_mut().expect("up").cursor = at;
    state
}

fn enter(state: &AppState) -> (AppState, Vec<Command>) {
    update(state, key(KeyCode::Enter))
}

fn kept(commands: &[Command]) -> Vec<MergedBranches> {
    commands
        .iter()
        .filter_map(|command| match command {
            Command::KeepGraphMode(mode) => Some(*mode),
            _ => None,
        })
        .collect()
}

#[test]
fn help_on_the_graph_lists_graph_mode_after_here_the_on_row_marked() {
    let state = help();
    let order = headers(&state);
    let at = |name: &str| order.iter().position(|h| h == name).expect(name);
    assert_eq!(at("GRAPH MODE"), at("HERE") + 1, "{order:?}");
    assert_eq!(at("EVERYWHERE"), at("GRAPH MODE") + 1, "{order:?}");
    let modes: Vec<_> = shown(&state)
        .into_iter()
        .filter(|(h, ..)| h == "GRAPH MODE")
        .map(|(_, label, kind, target)| (label, kind, target))
        .collect();
    assert_eq!(
        modes,
        vec![
            (
                "Folded merges".to_owned(),
                RowKind::Setting { on: true },
                PaletteTarget::App(AppAction::GraphMode(MergedBranches::Folded)),
            ),
            (
                "Every branch".to_owned(),
                RowKind::Setting { on: false },
                PaletteTarget::App(AppAction::GraphMode(MergedBranches::Shown)),
            ),
        ]
    );
}

#[test]
fn typed_graph_fold_and_branch_each_keep_both_rows_and_ctrl_p_finds_them() {
    let both = |state: &AppState| {
        let labels: Vec<String> = shown(state).into_iter().map(|(_, l, ..)| l).collect();
        labels.contains(&"Folded merges".to_owned()) && labels.contains(&"Every branch".to_owned())
    };
    for word in ["graph", "fold", "branch"] {
        assert!(both(&typed(&help(), word)), "{word}");
    }
    let (state, _) = merge_on_screen(WALK, 1);
    let (open, _) = update(
        &state,
        AppEvent::Core(Event::Key(KeyPress::control(KeyCode::Char('p')))),
    );
    assert!(both(&typed(&open, "graph")), "⌃P graph");
}

#[test]
fn enter_on_every_branch_switches_keeps_and_records_nothing() {
    let state = cursor_on(help(), "Every branch");
    let before_cursor = on(&state);
    let before_recent = state.palette_recent.clone();
    let (after, commands) = enter(&state);
    assert!(after.palette.is_none(), "closed");
    let history = after.history.as_ref().expect("a walk");
    assert_eq!(history.merged_branches(), MergedBranches::Shown);
    assert_eq!(kept(&commands), vec![MergedBranches::Shown]);
    // The switch's own row request and the keep: nothing journaled.
    assert!(
        commands
            .iter()
            .all(|c| matches!(c, Command::ShowRows(..) | Command::KeepGraphMode(_))),
        "{commands:?}"
    );
    let request = one_request(&commands);
    assert_eq!(request.keep, before_cursor, "lands on the same commit");
    assert_eq!(after.palette_recent, before_recent, "not a recent run");
}

#[test]
fn enter_on_the_on_row_closes_and_does_nothing() {
    let state = cursor_on(help(), "Folded merges");
    let (after, commands) = enter(&state);
    assert!(after.palette.is_none(), "closed");
    assert!(commands.is_empty(), "{commands:?}");
    assert_eq!(after.palette_recent, state.palette_recent);
}

/// A click moves the cursor; a second click runs the row (§2.12 decision 6)
/// — as `⏎` does.
#[test]
fn a_double_click_on_a_mode_row_is_enter() {
    // Typed, as the journey does: the group on screen.
    let state = typed(&help(), "graph");
    let frame = crate::view::tests::draw(&state, 120, 36);
    let row = frame
        .lines()
        .position(|line| line.contains("Every branch"))
        .unwrap_or_else(|| panic!("drawn\n{frame}"));
    let click = |state: &AppState| {
        update(
            state,
            AppEvent::Core(Event::Mouse(MouseInput {
                kind: MouseKind::Down(MouseButton::Left),
                column: 60,
                row: u16::try_from(row).expect("a row"),
                modifiers: Modifiers::NONE,
            })),
        )
    };
    let (once, _) = click(&state);
    let (twice, commands) = click(&once);
    let (by_key, by_key_commands) = enter(&cursor_on(state, "Every branch"));
    assert!(twice.palette.is_none());
    assert_eq!(commands, by_key_commands);
    assert_eq!(
        twice
            .history
            .as_ref()
            .map(crate::history::History::merged_branches),
        by_key
            .history
            .as_ref()
            .map(crate::history::History::merged_branches),
    );
}

#[test]
fn a_live_search_is_recounted_by_the_switch_and_stays_live() {
    let mut state = help();
    let query = crate::search::Query::parse("login").expect("a query");
    let mut search = Search::new(query, 0, (0, 0));
    search.batches = vec![Arc::from([1, 4, 5, 6].map(oid).to_vec())];
    search.state = SearchState::Done { hits: 4 };
    search.current = Some(0);
    state.search = Some(search);
    let (state, _) = update(&state, folded_hits(WALK, 0, 3, 0));
    let (after, commands) = enter(&cursor_on(state, "Every branch"));
    let ask = one_request(&commands)
        .folded
        .expect("the switch asks the count");
    let (after, _) = update(&after, folded_hits(WALK, ask.search, 0, 0));
    let search = after.search.as_ref().expect("still live");
    let line = search.line(&after.keymap);
    assert!(!line.contains("in folded merges"), "{line}");
}

fn bottom_row(state: &AppState) -> String {
    let frame = crate::view::tests::draw(state, 120, 36);
    frame
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .expect("a row")
        .trim_matches('"')
        .trim()
        .to_owned()
}

#[test]
fn the_bar_says_kept_or_why_not_with_its_mark_drawn() {
    let (state, _) = enter(&cursor_on(help(), "Every branch"));
    let (ok, _) = update(
        &state,
        AppEvent::GraphModeKept {
            mode: MergedBranches::Shown,
            result: Ok(()),
        },
    );
    assert!(
        bottom_row(&ok).starts_with("✓ Graph: every branch · kept for next launch"),
        "{}",
        bottom_row(&ok)
    );
    let (failed, _) = update(
        &state,
        AppEvent::GraphModeKept {
            mode: MergedBranches::Shown,
            result: Err("can't write /x/config.toml: denied".to_owned()),
        },
    );
    let drawn = bottom_row(&failed);
    assert!(
        drawn.starts_with("⊗ Graph: every branch for this run · not kept: can't write"),
        "{drawn}"
    );
    let history = failed.history.as_ref().expect("a walk");
    assert_eq!(
        history.merged_branches(),
        MergedBranches::Shown,
        "the switch stands"
    );
}

/// Typing the setting's name lists it before the HERE rows (ease-pass 25).
#[test]
fn a_setting_named_by_the_query_lists_before_here() {
    for word in ["graph", "every", "fold"] {
        let order = headers(&typed(&help(), word));
        assert_eq!(
            order.first().map(String::as_str),
            Some("GRAPH MODE"),
            "{word}: {order:?}"
        );
    }
}

/// `? graph ↓ ⏎` switches the mode: four steps, typed word one (ease-pass 25).
#[test]
fn the_graph_switch_journey_is_four_steps() {
    let steps = [
        key(KeyCode::Char('?')),
        key(KeyCode::Down),
        key(KeyCode::Enter),
    ];
    let (mut state, _) = merge_on_screen(WALK, 1);
    state.size = Size {
        columns: 120,
        rows: 36,
    };
    let (state, _) = update(&state, steps[0].clone());
    let state = typed(&state, "graph"); // one step: the word
    let (state, _) = update(&state, steps[1].clone());
    let (after, commands) = update(&state, steps[2].clone());
    assert_eq!(kept(&commands), vec![MergedBranches::Shown]);
    assert_eq!(
        after
            .history
            .as_ref()
            .map(crate::history::History::merged_branches),
        Some(MergedBranches::Shown)
    );
    assert_eq!(steps.len() + 1, 4);
}
