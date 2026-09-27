//! 20's Hints box: what the bars draw around a place.

use super::*;

/// The keys the footer draws for `state`, at 200 columns.
pub(super) fn bar(state: &AppState) -> Vec<(&'static str, &'static str)> {
    use graft_ui::graph_screen::{Divider, hints_for};
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    hints_for(&context, Divider::Hidden, 200)
        .into_iter()
        .map(|hint| (hint.key, hint.verb))
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

#[test]
fn pane_bar_hides_b_while_the_offer_is_up() {
    let mut state = pane_at(22);
    state.commit_graph = crate::update::CommitGraphOffer::Offered;
    state.system_git = Some(crate::update::GitCheck::Ran(
        graft_git::version::SystemGit::Supported(graft_git::version::GitVersion::MINIMUM),
    ));
    assert!(state.commit_graph_actionable(), "the offer is up");
    assert!(!bar(&state).contains(&("B", "blame")), "{:?}", bar(&state));
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
