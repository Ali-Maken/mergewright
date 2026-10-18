//! The lane cap comes from the graph's room, not the screen's width
//! (m5/21 item 4), counted in the header's `lanes drawn: N of M`
//! (ease-pass 06b).

use super::*;

/// A window of `lanes` occupied columns: Commit 3, then one commit in each
/// of columns 1.., then Commits 2 and 1, every column open on every row.
/// Ease-pass 06b: a column draws only while a node sits in it in view, so
/// each column takes a commit of its own for the lane cap to decide how
/// many draw.
pub(crate) fn history_with_lanes(lanes: usize) -> History {
    let mut history = history_of(3, 0);
    let window = history.window.as_mut().expect("history_of built a window");
    let laid: Vec<mergewright_graph::row::Row> = window.rows.to_vec();
    let mut rows = vec![laid[0].clone()];
    for lane in 1..lanes {
        let byte = 100 + u8::try_from(lane).expect("a few lanes");
        let mut row = laid[1].clone();
        row.commit.id = Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes");
        row.commit.parents.clear();
        row.commit.message = format!("Lane {lane} work").into_bytes();
        row.drawn_parents.clear();
        row.lane = lane;
        row.incoming.clear();
        row.outgoing.clear();
        rows.push(row);
    }
    rows.extend(laid[1..].iter().cloned());
    for row in &mut rows {
        row.lanes = (0..lanes)
            .map(|lane| {
                Some(mergewright_graph::identity::BranchIdentity::of_name(
                    format!("lane-{lane}").as_bytes(),
                ))
            })
            .collect();
    }
    window.laid_out = rows.len();
    let count = rows.len();
    window.rows = rows.into();
    history.rows = count;
    history
}

/// ADR-0039's 2026-09-21 amendment, third instance (m5/21 item 4): **ask
/// how much room this thing has, not how big the screen is.** The lane cap
/// came off the screen's width, so a graph with the detail pane beside it on
/// a 160-column terminal folded to the twelve lanes of a wide screen while
/// standing in 92 columns — a narrow view by DESIGN.md's own breakpoint.
/// The header's `lanes drawn: N of M` (ease-pass 06b) is the count made
/// visible: eight drawn columns (seven lanes and the rail) draw seven of
/// fourteen, twelve draw eleven. Sixteen rows, so all fourteen columns hold
/// a commit in view and the cap, not the window, decides.
#[test]
fn the_lane_cap_comes_from_the_graphs_room_not_the_screens_width() {
    use mergewright_ui::detail::DetailFocus;

    let mut state = AppState {
        history: Some(history_with_lanes(14)),
        size: Size {
            columns: 160,
            rows: 16,
        },
        ..AppState::default()
    };
    state.detail.shown = true;
    state.detail.focus = DetailFocus::Files;

    assert_eq!(
        super::frame::graph_columns(&state, ratatui::layout::Rect::new(0, 0, 160, 16)),
        92,
        "the pane leaves the graph 92 of the 160 columns"
    );
    let with_pane = draw(&state, 160, 16);
    assert!(
        with_pane.contains("lanes drawn: 7 of 14"),
        "92 columns is a narrow view: eight drawn columns, seven of the \
         fourteen lanes folded away:\n{with_pane}"
    );

    // The same screen with no pane really is wide, and keeps its twelve.
    state.detail.shown = false;
    let full_width = draw(&state, 160, 16);
    assert!(
        full_width.contains("lanes drawn: 11 of 14"),
        "160 columns to itself folds at twelve lanes:\n{full_width}"
    );
}
