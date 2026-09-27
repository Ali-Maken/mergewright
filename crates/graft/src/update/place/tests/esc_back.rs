//! `Esc` back to a hidden trail from the graph screen (20i, Major 1): the
//! pane `⏎` opened is not closed first, whichever of graph and pane has
//! focus, and the bar leads `esc back to blame` exactly where that `Esc`
//! goes back (`place::esc_goes_back`, read by the key and the bar).

use super::hints::bar;
use super::*;

/// `B` at line 22, `⏎` onto its commit, focus back on the graph: the trail
/// waits under the graph and the pane `⏎` opened is still shown.
fn hidden_with_the_pane_open() -> AppState {
    let state = blamed_at(22);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.trail.hidden(), "the trail waits under the graph");
    assert!(landed.detail.shown, "the pane `⏎` opened is shown");
    landed
}

fn leads_back(state: &AppState) -> bool {
    bar(state).first() == Some(&("esc", "back to blame"))
}

/// One `Esc`, and the place is on screen again.
fn one_esc_goes_back(state: &AppState) -> bool {
    press(state, KeyCode::Escape).0.trail.on_screen().is_some()
}

fn a_finished_search() -> crate::search::Search {
    let query = crate::search::Query::parse("retry").expect("a non-empty query");
    let mut search = crate::search::Search::new(query, 0, (0, 0));
    search.state = crate::search::SearchState::Done { hits: 0 };
    search
}

#[test]
fn esc_on_the_graph_with_the_pane_open_returns_to_blame() {
    let landed = hidden_with_the_pane_open();
    assert_eq!(
        bar(&landed).first(),
        Some(&("esc", "back to blame")),
        "{:?}",
        bar(&landed)
    );
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "blame is back");
    assert_eq!(cursor_line(&back), 22, "on the line `⏎` left from");
}

#[test]
fn tab_then_esc_after_enter_is_one_esc_back_to_blame() {
    let state = blamed_at(22);
    let line = cursor_line(&state);
    let landed = press_all(&state, &[KeyCode::Enter, KeyCode::Tab]);
    assert!(landed.trail.hidden());
    // `commit-back-to-blame` is one key (note `:1250`), wherever `Tab` left
    // the focus.
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "one `Esc` is back");
    assert_eq!(cursor_line(&back), line);
}

#[test]
fn esc_on_the_graph_with_the_pane_open_closes_search_first() {
    // Landed and settled: `blamed_at`'s commit is not in `commit_pane`'s
    // graph, so after a real `⏎` the walk is still seeking it, and a seek's
    // `G` rung comes before the search's in `escape_key`. The trail waits
    // under the graph with the pane open, as after `⏎` once the seek found
    // the commit.
    let mut landed = pane_at(22);
    hide_a_trail(&mut landed);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.detail.shown && landed.trail.hidden());
    landed.search = Some(a_finished_search());

    let (closed, _) = press(&landed, KeyCode::Escape);
    assert!(closed.search.is_none(), "the search went first");
    assert!(closed.trail.hidden(), "blame still waits");
    assert!(closed.detail.shown, "the pane stays");

    let (back, _) = press(&closed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "then back to blame");
    assert_eq!(cursor_line(&back), 22);
}

#[test]
fn esc_on_the_graph_with_the_pane_open_and_no_trail_closes_the_pane() {
    let mut state = pane_at(22);
    state.detail.focus = DetailFocus::Graph;
    assert!(state.detail.shown && !state.trail.is_live());
    let (closed, _) = press(&state, KeyCode::Escape);
    assert!(!closed.detail.shown, "no trail: `Esc` closes the pane");
}

/// A table row: its name, the change that makes its state, and whether one
/// `Esc` there goes back.
type Row = (&'static str, fn(&mut AppState), bool);

/// The table's states: each a change to [`hidden_with_the_pane_open`], and
/// whether one `Esc` there goes back. Menu, box and prompt states are not
/// here: each draws its own bar (the menu's is
/// [`the_menu_over_a_hidden_trail_closes_first_and_the_bar_does_not_lead_back`]).
const TABLE: [Row; 8] = [
    (
        "graph focused, pane closed",
        |state| state.detail.shown = false,
        true,
    ),
    ("graph focused, pane open", |_| {}, true),
    (
        "pane focused on the diff",
        |state| state.detail.focus = DetailFocus::Diff,
        true,
    ),
    (
        "pane focused on the files",
        |state| state.detail.focus = DetailFocus::Files,
        true,
    ),
    (
        "graph search active",
        |state| state.search = Some(a_finished_search()),
        false,
    ),
    ("Focus mode", |state| state.focus = true, false),
    ("Explain open", explain_open, false),
    (
        "held failure",
        |state| state.status = Status::sticky("git said no"),
        false,
    ),
];

fn explain_open(state: &mut AppState) {
    state.detail.explain = Some(graft_ui::detail::ExplainView {
        title: graft_term::sanitize::sanitize_line("FOXTROT MERGE"),
        severity: graft_core::mistake_radar::Severity::Info,
        paragraphs: Vec::new(),
        sketch: Vec::new(),
    });
}

#[test]
fn the_bar_leads_esc_back_to_blame_exactly_where_esc_goes_back() {
    for (name, change, back) in TABLE {
        let mut state = hidden_with_the_pane_open();
        change(&mut state);
        assert_eq!(
            leads_back(&state),
            one_esc_goes_back(&state),
            "{name}: the bar {:?} and the key disagree",
            bar(&state)
        );
        assert_eq!(one_esc_goes_back(&state), back, "{name}");
        assert_eq!(esc_goes_back(&state), back, "{name}: the predicate");
    }
}

#[test]
fn the_menu_over_a_hidden_trail_closes_first_and_the_bar_does_not_lead_back() {
    let landed = hidden_with_the_pane_open();
    let (menu, _) = press(&landed, KeyCode::Char('.'));
    assert!(menu.context_menu.is_some(), "`.` opened the menu");
    assert!(!leads_back(&menu), "{:?}", bar(&menu));
    let (closed, _) = press(&menu, KeyCode::Escape);
    assert!(closed.context_menu.is_none(), "the menu went first");
    assert!(closed.trail.hidden(), "blame still waits");
    assert!(leads_back(&closed), "{:?}", bar(&closed));
}
