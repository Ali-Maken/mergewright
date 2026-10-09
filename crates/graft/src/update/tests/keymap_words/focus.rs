//! `⇧⇥` with a banner drawn (m8/20d): keymap §2.2's previous pane, the
//! forward cycle run backwards — driven through `update`.

use graft_core::event::KeyCode as CoreKey;
use graft_ui::detail::DetailFocus;

use super::resolve::{Spot, key, stopped_at};

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
