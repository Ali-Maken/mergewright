//! Lanes only for branches with a commit in view, and the top line's account
//! of them (ease-pass 06b): a column with no node in view is dropped,
//! expanded or not; node columns over the cap fold behind the rail; the top
//! line ends `lanes drawn: N of M · K merged branches folded`, each phrase
//! only while it has something to say, dropped whole and first when short.

use std::collections::BTreeSet;

use graft_graph::fold::Keep;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use graft_term::sanitize_line;
use ratatui::layout::Rect;
use ratatui::style::Style;

use super::super::header_tail::{Drop, TailItem, fitted_tail, lane_phrases};
use super::super::layout::body_layout;
use super::super::{Compaction, FoldCount, FoldMark, LaneTally, WalkView, max_lanes_for};
use super::{cells, draw, linear, text, view};
use crate::graph_column::{GraphColumn, RAIL};
use crate::test_support::wide_history;

/// Five commits over eight columns, every column open on every row: a node
/// in columns 0, 1, 3, 5 and 6, none in 2, 4 and 7 (as graft-graph's
/// `fold::tests::in_view`).
fn eight_columns() -> Vec<Row> {
    let identity = |c: usize| BranchIdentity::of_name(format!("lane-{c}").as_bytes());
    let mut rows = linear(5);
    for (row, lane) in rows.iter_mut().zip([0, 1, 3, 5, 6]) {
        row.lane = lane;
        row.incoming = Vec::new();
        row.outgoing = row
            .commit
            .parents
            .first()
            .map(|&p| (lane, p))
            .into_iter()
            .collect();
        row.lanes = (0..8).map(|c| Some(identity(c))).collect();
    }
    rows
}

#[test]
fn expanded_lanes_still_draw_only_the_columns_holding_a_node() {
    let rows = eight_columns();
    let mut expanded = view(&rows, WalkView::Complete { rows: 5 }, 0);
    expanded.compaction.expanded = true;

    let (_, _, fold) = body_layout(&expanded, Rect::new(0, 0, 120, 12));
    assert_eq!(fold.kept(), [0, 1, 3, 5, 6]);
    assert_eq!(fold.kept().len(), 5);
    assert_eq!(fold.folded_count(), 0);
    let screen = text(&draw(&expanded, 120, 12));
    assert!(
        screen[0].trim_end().ends_with("lanes drawn: 5 of 8"),
        "{screen:#?}"
    );
    assert!(
        !screen.iter().any(|line| line.contains(RAIL)),
        "{screen:#?}"
    );
}

#[test]
fn the_lane_words_say_only_what_there_is_to_say() {
    let tally = |drawn, open, folded_in_view| LaneTally {
        drawn,
        open,
        folded_in_view,
    };
    assert_eq!(
        lane_phrases(tally(5, 8, 3)),
        ["lanes drawn: 5 of 8", "3 merged branches folded"]
    );
    assert!(lane_phrases(tally(8, 8, 0)).is_empty());
    assert_eq!(lane_phrases(tally(8, 8, 1)), ["1 merged branch folded"]);
    assert_eq!(lane_phrases(tally(5, 8, 0)), ["lanes drawn: 5 of 8"]);
}

/// The tail as one string, separators included.
fn fitted(room: usize) -> String {
    let item = |text: &str, drop| TailItem {
        text: text.to_owned(),
        style: Style::new(),
        drop,
    };
    let items = vec![
        item("first parent", Drop::Filters),
        item("gfx ok", Drop::Gfx),
        item("lanes drawn: 5 of 8", Drop::Lanes),
        item("3 merged branches folded", Drop::Lanes),
    ];
    fitted_tail(items, room, room)
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn the_lane_phrases_drop_first_whole_and_right_to_left() {
    // 12 + 2 + 6 + 2 + 19 + 3 + 24: both lane phrases joined ` · `.
    let all = "first parent  gfx ok  lanes drawn: 5 of 8 · 3 merged branches folded";
    assert_eq!(all.chars().count(), 68);
    assert_eq!(fitted(68), all);
    assert_eq!(fitted(67), "first parent  gfx ok  lanes drawn: 5 of 8");
    // 12 + 2 + 6 + 2 + 19 = 41: one short of the lanes phrase alone.
    assert_eq!(fitted(40), "first parent  gfx ok");
    for room in 20..=68 {
        let tail = fitted(room);
        assert!(!tail.contains('…'), "{room}: {tail:?}");
        assert!(tail.starts_with("first parent  gfx ok"), "{room}: {tail:?}");
    }
}

/// The lane phrases are the first thing on the top line to give way: before
/// the branch `HEAD` is on, which the path and every other tail item yield
/// to (ease-pass 06b; at 50 columns `main-folded-50` lost its branch).
#[test]
fn the_lane_phrases_give_way_before_the_branch_name() {
    let rows = linear(4);
    let mut screen_view = view(&rows, WalkView::Complete { rows: 4 }, 0);
    if let Some(location) = screen_view.location.as_mut() {
        location.branch = sanitize_line("wallet-release");
    }
    let closed = FoldMark {
        open: false,
        count: FoldCount::Known(2),
        from: None,
    };
    screen_view.rows[1].fold = Some(closed.clone());
    screen_view.rows[2].fold = Some(closed);

    // 80: 71 cells of tail room; `4 commits  2 merged branches folded` (35)
    // leaves 36 for `  /code/api · wallet-release` (28).
    let wide = text(&draw(&screen_view, 80, 12));
    assert!(
        wide[0].contains("/code/api \u{b7} wallet-release")
            && wide[0]
                .trim_end()
                .ends_with("4 commits  2 merged branches folded"),
        "{wide:#?}"
    );
    // 50: 41 cells; with the phrase, 6 are left, short of ` · wallet-release`
    // (17). The phrase drops; the branch stays.
    let narrow = text(&draw(&screen_view, 50, 12));
    assert!(narrow[0].contains("wallet-release"), "{narrow:#?}");
    assert!(!narrow[0].contains("merged branch"), "{narrow:#?}");
    assert!(narrow[0].trim_end().ends_with("4 commits"), "{narrow:#?}");
}

#[test]
fn the_tally_counts_the_closed_folds_in_view() {
    let rows = linear(4);
    let mut screen_view = view(&rows, WalkView::Complete { rows: 4 }, 0);
    let mark = |open| FoldMark {
        open,
        count: FoldCount::Known(2),
        from: Some(sanitize_line("sara/fix-toc")),
    };
    for (entry, fold) in screen_view.rows.iter_mut().zip([
        Some(mark(false)),
        Some(mark(false)),
        Some(mark(true)),
        None,
    ]) {
        entry.fold = fold;
    }
    let (_, window, fold) = body_layout(&screen_view, Rect::new(0, 0, 80, 12));
    let tally = LaneTally::of(&fold, window);
    assert_eq!(tally.folded_in_view, 2);
    assert_eq!((tally.drawn, tally.open), (1, 1));
}

/// The twelve merge rows of a twelve-topic history: every topic lane opens
/// here and holds no node here.
fn wide_merges() -> Vec<Row> {
    let mut rows = wide_history(12);
    rows.truncate(12);
    rows
}

/// The topic commits of a `topics`-wide history: topic commit `i` (top
/// first) sits in column `i + 1`; lane 0 passes through with no node.
fn topic_commits(topics: u8) -> Vec<Row> {
    let rows = wide_history(topics);
    let topics = usize::from(topics);
    rows[topics..2 * topics].to_vec()
}

#[test]
fn a_lane_with_no_commit_in_view_is_dropped_and_its_merge_draws_a_stub() {
    let rows = wide_merges();
    for expanded in [false, true] {
        let mut screen_view = view(&rows, WalkView::Complete { rows: 12 }, 0);
        screen_view.compaction.expanded = expanded;
        let screen = text(&draw(&screen_view, 80, 24));

        // Lane 0 alone, the merge's second parent a stub; no rail.
        assert_eq!(cells(&screen[1], 3), "◉▾ ", "{screen:#?}");
        assert!(
            screen[0].trim_end().ends_with("lanes drawn: 1 of 13"),
            "{screen:#?}"
        );
        assert!(
            !screen.iter().any(|line| line.contains(RAIL)),
            "{screen:#?}"
        );
        assert!(screen[1].contains("Merge branch 'topic-12'"), "{screen:#?}");
    }
}

#[test]
fn more_node_columns_than_the_limit_fold_behind_a_rail_and_leave_the_subject_readable() {
    let rows = topic_commits(12);
    let screen = text(&draw(
        &view(&rows, WalkView::Complete { rows: 12 }, 0),
        80,
        24,
    ));

    // Seven lanes plus the rail: 16 cells; the rail is the last of them on
    // every row, but a row whose node's column folded shows its node there
    // instead — six rows, one per folded column (thirteen open, seven kept).
    // The count is the header's, never a row's.
    let on_rail: Vec<char> = screen[1..13]
        .iter()
        .map(|line| cells(line, 16).chars().nth(14).unwrap_or(' '))
        .collect();
    assert!(
        on_rail.iter().all(|&c| c == RAIL || c == '●'),
        "{screen:#?}"
    );
    assert_eq!(
        on_rail.iter().filter(|&&c| c == '●').count(),
        6,
        "{screen:#?}"
    );
    assert!(
        screen[0].trim_end().ends_with("lanes drawn: 7 of 13"),
        "{screen:#?}"
    );
    assert!(screen[1].contains("topic-12: work"), "{screen:#?}");
    assert!(screen[12].contains("topic-1: work"), "{screen:#?}");
    assert!(screen[23].contains("⏎ open"), "{screen:#?}");
}

#[test]
fn expanded_every_node_column_draws_in_its_real_column_from_the_same_rows() {
    let rows = topic_commits(12);
    let mut expanded = view(&rows, WalkView::Complete { rows: 12 }, 0);
    expanded.compaction.expanded = true;
    let (_, _, fold) = body_layout(&expanded, Rect::new(0, 0, 80, 24));
    assert_eq!(fold.kept(), (0..13).collect::<Vec<_>>());
    assert_eq!(GraphColumn::width_through(&fold), 26);

    let screen = text(&draw(&expanded, 80, 24));
    assert!(
        !screen.iter().any(|line| line.contains(RAIL)),
        "{screen:#?}"
    );
    assert!(
        !screen[0].contains("lanes drawn"),
        "every open column drawn: {screen:#?}"
    );
}

/// ADR-0025: a thread twenty columns wide is a priority, not a veto — the
/// configured cap still holds. Twenty topic commits, each a node in its own
/// column, the cursor on the topmost, its whole thread kept: the column
/// still stops at `max_lanes = 8`, the rail is the last column drawn, and
/// the header's count is exact.
#[test]
fn a_thread_of_twenty_columns_still_fits_the_configured_cap() {
    let rows = topic_commits(20);
    let selected = rows[0].lane;
    let thread: BTreeSet<usize> = rows.iter().map(|row| row.lane).collect();

    let mut screen_view = view(&rows, WalkView::Complete { rows: 20 }, 0);
    screen_view.compaction = Compaction {
        max_lanes: 8,
        expanded: false,
        keep: Keep {
            selected: Some(selected),
            reserved: BTreeSet::new(),
            preferred: thread,
        },
    };
    let (_, _, fold) = body_layout(&screen_view, Rect::new(0, 0, 80, 24));
    let width = usize::from(GraphColumn::width_through(&fold));
    assert!(width <= 40, "graph column {width} cells wide");
    assert!(fold.kept().contains(&selected));

    let screen = text(&draw(&screen_view, 80, 24));
    assert!(
        screen[0].trim_end().ends_with("lanes drawn: 7 of 21"),
        "{screen:#?}"
    );
    let rail = fold.marker().expect("the thread does not fit unfolded");
    assert_eq!(rail, fold.kept().len());
    for line in &screen[1..21] {
        assert_eq!(
            cells(line, width).chars().count(),
            width,
            "row shorter than the column it was given: {line:?}"
        );
    }
}

// m1-the-graph/44: the lane cap follows the terminal's width (ADR-0022
// decision 3, ADR-0025 decision 3).

#[test]
fn a_resize_from_80_to_160_columns_changes_the_folds_lane_count() {
    let rows = topic_commits(30);
    let mut view = view(&rows, WalkView::Complete { rows: rows.len() }, 0);
    view.compaction = Compaction::at(max_lanes_for(80, None));
    let narrow = text(&draw(&view, 80, 24));

    view.compaction = Compaction::at(max_lanes_for(160, None));
    let wide = text(&draw(&view, 160, 40));

    // 22 rows in view at 80×24 hold 22 topic commits and lane 0; the other
    // eight topic lanes pass through: 31 open. 8 lanes at 80 columns draw
    // 7 and the rail; at 160 columns all 30 commits are in view and 12
    // lanes draw 11 and the rail.
    assert!(
        narrow[0].trim_end().ends_with("lanes drawn: 7 of 31"),
        "{narrow:#?}"
    );
    assert!(
        wide[0].trim_end().ends_with("lanes drawn: 11 of 31"),
        "{wide:#?}"
    );
}

#[test]
fn a_configured_cap_larger_than_the_screen_never_cuts_the_graph_column() {
    let rows = topic_commits(30);
    let mut view = view(&rows, WalkView::Complete { rows: rows.len() }, 0);
    let cap = max_lanes_for(80, Some(30));
    assert_eq!(cap, 19, "clamped to what 80 columns can hold");
    view.compaction = Compaction::at(cap);
    let screen = text(&draw(&view, 80, 24));

    // 19 lanes (rail included) at 2 cells each is 38 cells — exactly the
    // half-of-the-body bound `graph_width` enforces, so the column is not
    // cut. The body starts after the two-cell gutter.
    // A row whose node folded shows the node on the rail instead, so look
    // down the body for the first row drawing the rail itself.
    let rail_at = screen[1..]
        .iter()
        .find_map(|line| line.chars().position(|c| c == RAIL))
        .expect("rail is drawn");
    // The rail is the last drawn column: column (cap - 1), 2 cells wide.
    assert_eq!(rail_at, (cap - 1) * 2 + 2, "{screen:#?}");
    assert!(rail_at <= 40, "cut past the half-screen bound: {screen:#?}");
}
