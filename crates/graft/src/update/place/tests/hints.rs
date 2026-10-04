//! 20's Hints box: what the bars draw around a place.

use super::*;
use crate::spelled::Spelled as _;

/// The keys the footer draws for `state`, at 200 columns.
pub(super) fn bar(state: &AppState) -> Vec<(&'static str, &'static str)> {
    use graft_ui::graph_screen::{Divider, hints_for};
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(
        &context,
        Divider::Hidden,
        200,
        graft_ui::keymap::Keymap::shipped(),
    )
    .into_iter()
    .map(|hint| (hint.key(), hint.verb))
    .collect()
}

#[test]
fn pane_bar_offers_b_blame() {
    assert!(
        bar(&pane_at(22)).contains(&("B", "blame")),
        "{:?}",
        bar(&pane_at(22))
    );
}

/// m7-power-tools/36 (M3): in the pane `B` is blame (keymap §2.5), offer or
/// not — the offer's keys live only on the graph.
#[test]
fn the_pane_bar_keeps_b_blame_while_the_offer_is_up() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    assert!(state.commit_graph_actionable(), "the offer is up");
    assert!(blamable(&state));
    assert!(bar(&state).contains(&("B", "blame")), "{:?}", bar(&state));
}

#[test]
fn graph_bar_leads_esc_back_to_blame_while_hidden() {
    let state = blamed_at(22);
    let (mut landed, _) = press(&state, KeyCode::Enter);
    landed.detail.focus = DetailFocus::Graph;
    assert!(landed.trail.hidden());
    assert_eq!(
        bar(&landed).first(),
        Some(&("esc", "back to blame")),
        "{:?}",
        bar(&landed)
    );
}

#[test]
fn the_place_bar_offers_n_only_while_finding() {
    let finding = place_on_screen(pane_at(22), true, false);
    let not = place_on_screen(pane_at(22), false, false);
    assert!(bar(&finding).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("n N", "match")));
    assert!(!bar(&not).contains(&("i", "ignore list")));
}

#[test]
fn a_live_global_passes_through_the_place() {
    assert!(live_global(KeyPress::plain(KeyCode::Char('q'))));
    assert!(!live_global(KeyPress::plain(KeyCode::Char('m'))));
}

/// m7-power-tools/21b: the history place's bar, one row per state it can
/// be in — rows with each focus, then tracing, failed and zero rows, which
/// offer only the way back.
#[test]
fn history_rows_in_the_place_bar_table() {
    use super::history::{
        blame_on_22, history, lines_history, refresh_token, three_rows, trace_job, traced,
    };
    use graft_git::trace::{SHALLOW_END, TraceEnd};

    let list = lines_history(three_rows(), refresh_token());
    let (file, _) = press(&list, KeyCode::Tab);
    assert_eq!(history(&file).focus, graft_ui::history::HistoryFocus::File);
    let (tracing, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (failed, _) = traced(
        &tracing,
        job,
        Err(TraceFailure::Failed("fatal: bad revision".to_owned())),
    );
    let (none, _) = traced(
        &tracing,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: Vec::new(),
            end: TraceEnd::GitStops(SHALLOW_END.to_owned()),
        }),
    );
    let rows = [
        ("j/k", "older·newer"),
        ("⇥", "file"),
        ("n/N", "next change"),
        ("B", "blame here"),
        ("⏎", "that commit"),
        ("esc", "back"),
    ];
    let on_file = [
        ("j/k", "scroll"),
        ("⇥", "list"),
        ("n/N", "next change"),
        ("B", "blame here"),
        ("⏎", "that commit"),
        ("w", "whitespace"),
        ("esc", "back"),
    ];
    let back = [("esc", "back")];
    let table: [BarRow; 5] = [
        ("rows, list focus", &list, &rows),
        ("rows, file focus", &file, &on_file),
        ("tracing", &tracing, &back),
        ("failed", &failed, &back),
        ("zero rows", &none, &back),
    ];
    for (name, state, want) in table {
        assert_eq!(bar(state), want, "{name}");
    }
}

/// A row of a bar table: its name, the screen, the (key, verb)s it draws.
type BarRow<'a> = (&'a str, &'a AppState, &'a [(&'a str, &'a str)]);

/// m7-power-tools/21b: on a file the pane's `F` is history, so the bar
/// says `F history` right after `B blame`, and no `F focus`.
#[test]
fn pane_bar_offers_f_history_on_a_file() {
    let drawn = bar(&pane_at(22));
    let b = drawn.iter().position(|hint| *hint == ("B", "blame"));
    let f = drawn.iter().position(|hint| *hint == ("F", "history"));
    assert!(b.is_some(), "{drawn:?}");
    assert_eq!(f, b.map(|b| b + 1), "{drawn:?}");
    assert!(!drawn.contains(&("F", "focus")), "{drawn:?}");
}

/// Off a file (`commit_pane`'s `src/` row) `F` falls through to Focus
/// (`place::f_key`'s guard), and the bar says so.
#[test]
fn pane_bar_keeps_f_focus_off_a_file() {
    let mut state = commit_pane(JobId::new(1), true);
    state.size = crate::update::Size {
        columns: 120,
        rows: 40,
    };
    let drawn = bar(&state);
    assert!(!drawn.contains(&("F", "history")), "{drawn:?}");
    assert!(drawn.contains(&("F", "focus")), "{drawn:?}");
}
