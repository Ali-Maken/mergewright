//! Stop words: the graph bar's way to the banner. Moved out of `stops.rs` (m8/21b-5, move-only).

use super::*;

/// The graph bar's hints at 120 columns, spelled from the state's own
/// keymap: the footer context `view` draws, through mergewright-ui's fitting.
fn graph_bar(state: &AppState) -> String {
    use mergewright_ui::graph_screen::{Divider, hints_for, hints_text};
    let banner = crate::update::banner::banner_view(state);
    let context = crate::view::rows::footer_context(state, &[], banner.as_ref());
    let divider = if state.detail.shown {
        Divider::Shown
    } else {
        Divider::Hidden
    };
    hints_text(
        &hints_for(&context, divider, 120, &state.keymap),
        &state.keymap,
    )
}

/// Graph `OperationLive` bar, graph focused, pane open (m8/20d-b B+1): one
/// `⇧⇥` reaches the banner (`shift_tab_from_the_graph_reaches_the_banner`),
/// so the bar says `⇧⇥ banner` — spelled from his keymap. From the file
/// list neither way is one press: no banner hint. `prev_pane` unbound → no
/// one-press way from the graph, no hint (never a false promise).
#[test]
fn the_graph_bar_names_the_way_to_the_banner_with_the_pane_open() {
    use mergewright_ui::keymap::KeySwitches;
    let graph = super::super::resolve::stopped_at(super::super::resolve::Spot::ConflictRow);
    let mut graph = graph;
    graph.detail.focus = mergewright_ui::detail::DetailFocus::Graph;
    assert!(graph.detail.shown && !graph.banner_focus);
    let bar = graph_bar(&graph);
    assert!(bar.contains("⇧⇥ banner"), "{bar}");

    let with = |id: &str, key: &str| {
        let (map, problems) = Keymap::with_overrides(
            &[(id.to_owned(), vec![key.to_owned()])],
            KeySwitches::default(),
        );
        assert_eq!(problems, [], "{id}={key} reads cleanly");
        let mut state = graph.clone();
        state.keymap = Arc::new(map);
        state
    };
    let moved = graph_bar(&with("app.prev_pane", "f6"));
    assert!(moved.contains("F6 banner"), "{moved}");
    let unbound = graph_bar(&with("app.prev_pane", "none"));
    assert!(!unbound.contains("banner"), "{unbound}");

    let files = super::super::resolve::stopped_at(super::super::resolve::Spot::OtherRow);
    let bar = graph_bar(&files);
    assert!(!bar.contains("banner"), "{bar}");
}
