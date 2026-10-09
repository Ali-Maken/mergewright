//! `⇧⇥` with a banner drawn (m8/20d): keymap §2.2's previous pane, the
//! forward cycle run backwards — driven through `update`.

use graft_core::event::KeyCode as CoreKey;
use graft_ui::detail::DetailFocus;

use std::sync::Arc;

use super::resolve::{Spot, key, stopped_at, with};

/// Pane open, graph focused: one `⇧⇥` focuses the banner; from the banner
/// `⇧⇥` goes to the diff; from the file list `⇧⇥` goes to the graph.
#[test]
fn shift_tab_from_the_graph_reaches_the_banner() {
    let mut graph = stopped_at(Spot::ConflictRow);
    graph.detail.focus = DetailFocus::Graph;
    assert!(graph.detail.shown && !graph.banner_focus);

    let (banner, _) = key(&graph, CoreKey::BackTab);
    assert!(banner.banner_focus, "one ⇧⇥ from the graph");

    let (diff, _) = key(&banner, CoreKey::BackTab);
    assert!(!diff.banner_focus);
    assert_eq!(diff.detail.focus, DetailFocus::Diff, "banner → diff");

    let files = stopped_at(Spot::OtherRow);
    assert_eq!(files.detail.focus, DetailFocus::Files);
    let (back, _) = key(&files, CoreKey::BackTab);
    assert!(!back.banner_focus);
    assert_eq!(back.detail.focus, DetailFocus::Graph, "files → graph");
}

/// m8/32 V0: `app.prev_pane = "f6"` — `F6` from the graph (pane open)
/// focuses the banner, `F6` again the diff; the moved `⇧⇥` is unbound.
#[test]
fn the_remapped_way_to_the_banner_is_the_key_that_takes_it() {
    let mut graph = stopped_at(Spot::ConflictRow);
    graph.keymap = Arc::new(with("app.prev_pane", "f6"));
    graph.detail.focus = DetailFocus::Graph;
    assert!(graph.detail.shown && !graph.banner_focus);

    let (banner, _) = key(&graph, CoreKey::Function(6));
    assert!(banner.banner_focus, "one F6 from the graph");
    let (diff, _) = key(&banner, CoreKey::Function(6));
    assert!(!diff.banner_focus);
    assert_eq!(diff.detail.focus, DetailFocus::Diff, "banner → diff");

    let (still, _) = key(&graph, CoreKey::BackTab);
    assert!(!still.banner_focus, "the moved ⇧⇥ is unbound");
    assert_eq!(still.detail.focus, DetailFocus::Graph);
}

/// m8/32 V0: `app.next_pane = "f5"` — `F5` lands where `⇥` lands on the
/// shipped keymap, from the graph (pane open) and from the banner.
#[test]
fn the_remapped_next_pane_runs_the_tab_cycle() {
    let mut graph = stopped_at(Spot::ConflictRow);
    graph.detail.focus = DetailFocus::Graph;
    let mut banner = stopped_at(Spot::Banner);
    assert!(banner.banner_focus);
    for (from, start) in [(&mut graph, "graph"), (&mut banner, "banner")] {
        let (want, _) = key(from, CoreKey::Tab);
        from.keymap = Arc::new(with("app.next_pane", "f5"));
        let (got, _) = key(from, CoreKey::Function(5));
        let place = |s: &crate::update::AppState| (s.banner_focus, s.detail.focus);
        assert_ne!(place(&got), place(from), "F5 moves focus from the {start}");
        assert_eq!(place(&got), place(&want), "F5 is ⇥ from the {start}");
    }
}
