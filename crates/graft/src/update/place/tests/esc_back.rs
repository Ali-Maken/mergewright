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

/// The walk's answer to the `⏎` landing's find: rows 0..3, the merge
/// screen's two commits, then `third` on row 2.
fn find_answered(state: &AppState, commands: &[Command], third: Oid) -> AppState {
    use crate::history::HistoryUpdate;
    use crate::update::events::Progress;
    let (walk, request) = commands
        .iter()
        .find_map(|command| match command {
            Command::ShowRows(walk, request) => Some((*walk, request.id)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("`⏎` asks the walk to find the commit: {commands:?}"));
    let AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(mut window))) =
        crate::update::tests::rows_for(walk, request, 0..3, 3)
    else {
        unreachable!("`rows_for` answers with a window");
    };
    let mut rows = window.rows.to_vec();
    for (row, byte) in rows.iter_mut().zip([1u8, 2]) {
        row.commit.id = Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("an oid");
    }
    rows[2].commit.id = third;
    window.rows = rows.into();
    update(
        state,
        AppEvent::Progress(id, Progress::History(HistoryUpdate::Window(window))),
    )
    .0
}

fn selected(state: &AppState) -> Option<Oid> {
    state
        .history
        .as_ref()
        .and_then(crate::history::History::selected)
        .map(|row| row.commit.id)
}

/// Q1 (gap M7/32i-q1): `B ⏎ Esc` onto a commit the walk has not reached,
/// then the walk answers. Going back to blame abandons that `⏎`: the landing
/// neither selects the commit, nor opens the pane on it, nor says it missed.
#[test]
fn esc_back_drops_a_landing_still_seeking() {
    let before = pane_at(22);
    let state = blamed_at(22);
    let (landed, commands) = press(&state, KeyCode::Enter);
    assert!(
        landed.line_landing.is_some(),
        "Lina's commit is off the rows on hand: the landing seeks it"
    );
    let (back, _) = press(&landed, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "blame is shown again");

    for (third, case) in [(lina(), "reached"), (mo(), "missed")] {
        let after = find_answered(&back, &commands, third);
        assert_eq!(
            selected(&after),
            selected(&before),
            "{case}: graph selection"
        );
        assert_eq!(
            after.detail.shown, before.detail.shown,
            "{case}: pane shown"
        );
        assert_eq!(after.detail.spec, before.detail.spec, "{case}: pane spec");
        assert!(
            !format!("{:?}", after.status).contains("not in this view"),
            "{case}: no miss said under blame: {:?}",
            after.status
        );
        assert!(
            after.trail.on_screen().is_some(),
            "{case}: blame still shown"
        );
        assert_eq!(after.line_landing, None, "{case}: the landing is gone");

        // Leaving blame now: the origin is where `B` was pressed.
        let (left, _) = press(&after, KeyCode::Escape);
        assert!(!left.trail.is_live(), "{case}: left");
        assert_eq!(
            selected(&left),
            selected(&before),
            "{case}: origin selection"
        );
        assert_eq!(left.detail.spec, before.detail.spec, "{case}: origin spec");
        assert_eq!(
            left.detail.focus, before.detail.focus,
            "{case}: origin focus"
        );
    }
}

/// Review Minor 1 (20n C): leaving the trail where `B` was pressed — so
/// `leave` restores nothing (its `Spot::of(state) == origin.spot` return) —
/// with a `⏎` landing still seeking. `leave`'s `abandon` withdraws the seek
/// too, so the walk's answer does not move the graph cursor.
///
/// No key path reaches it: `⏎` hides the trail, and every way back to it
/// (`show_again`) abandons the landing first. Built instead: the landing
/// `⏎` started, the place shown again by hand, the origin set to where the
/// screen now stands.
#[test]
fn leaving_where_blame_started_drops_a_landing_still_seeking() {
    let state = blamed_at(22);
    let (mut landed, commands) = press(&state, KeyCode::Enter);
    assert!(
        landed.line_landing.is_some(),
        "Lina's commit is off the rows on hand: the landing seeks it"
    );
    landed.trail.shown = true;
    let here = Spot::of(&landed);
    landed.trail.origin.as_mut().expect("an origin").spot = here;
    let cursor = |state: &AppState| state.history.as_ref().map(|history| history.cursor);
    let before = cursor(&landed);

    let (left, _) = press(&landed, KeyCode::Escape);
    assert!(!left.trail.is_live(), "the last place popped: left");
    assert_eq!(left.line_landing, None, "the landing is gone");

    let after = find_answered(&left, &commands, lina());
    assert_eq!(cursor(&after), before, "the graph cursor does not move");
    assert_eq!(after.line_landing, None, "no landing came back");
}
