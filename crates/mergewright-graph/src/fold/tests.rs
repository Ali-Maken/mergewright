use std::borrow::Cow;
use std::collections::BTreeSet;

use super::*;
use crate::layout::tests::{commit, oid};
use crate::layout::{layout, tests::identity_at};
use crate::{BranchIdentity, BranchSeed, Seeds};
use mergewright_core::commit::Commit;

mod in_view;
mod properties;

const TOPICS: u8 = 12;

/// `main` with twelve topic branches merged one after another: twelve merge
/// rows at the top, each opening a lane, then the twelve topic commits, then
/// the root. Ids: root 1, topic `k` is `1 + k`, its merge is `13 + k`.
fn wide_history() -> Vec<Row> {
    let mut commits = Vec::new();
    for k in (1..=TOPICS).rev() {
        let below = if k == 1 { 1 } else { 13 + k - 1 };
        let subject = format!("Merge branch 'topic-{k}'");
        commits.push(commit(
            13 + k,
            &[below, 1 + k],
            i64::from(100 - k),
            subject.as_bytes(),
        ));
    }
    for k in (1..=TOPICS).rev() {
        let subject = format!("topic-{k}: work");
        commits.push(commit(1 + k, &[1], i64::from(50 - k), subject.as_bytes()));
    }
    commits.push(commit(1, &[], 0, b"root"));
    layout(
        &commits,
        Seeds {
            default_branch: Some(BranchSeed::new(
                oid(13 + TOPICS),
                BranchIdentity::of_name(b"main"),
            )),
            ..Seeds::default()
        },
    )
}

/// The column the merge on `row` opened.
fn opened_on(rows: &[Row], row: usize) -> usize {
    rows[row].outgoing[1].0
}

fn none() -> Keep {
    Keep::default()
}

/// The window of the twelve topic commits: each a node in its own column,
/// lane 0 passing through with no node (its next commit is the root, below).
/// Window row `i` is the topic commit whose column every row above it
/// touched less recently.
fn topic_commits(rows: &[Row]) -> &[Row] {
    &rows[12..24]
}

/// The column the topic commit on window row `i` sits in.
fn topic_column(rows: &[Row], i: usize) -> usize {
    topic_commits(rows)[i].lane
}

#[test]
fn a_window_whose_every_open_column_holds_a_node_draws_as_laid_out() {
    let rows = wide_history();
    // The topic commits and the root: thirteen columns, a node in each.
    let window = &rows[12..];
    let fold = Fold::plan(16, &none(), window);

    assert!(fold.is_empty());
    assert_eq!(fold.marker(), None);
    assert_eq!(fold.drawn_columns(), 13, "lane 0 and twelve topic lanes");
    assert_eq!(fold.open_columns(), 13);
    for row in window {
        assert_eq!(&*fold.view(row).row, row);
    }
}

#[test]
fn the_least_recently_touched_node_columns_fold_first_and_the_marker_fits_the_limit() {
    let rows = wide_history();
    let fold = Fold::plan(8, &none(), topic_commits(&rows));

    // Thirteen columns may draw; seven drawn plus the marker is eight.
    assert_eq!(fold.drawn_columns(), 8);
    assert_eq!(fold.marker(), Some(7));
    let mut expected: Vec<usize> = (0..6).map(|i| topic_column(&rows, i)).collect();
    expected.sort_unstable();
    assert_eq!(
        fold.folded(),
        expected,
        "the columns of the top six topic commits were touched longest ago"
    );
    assert_eq!(
        fold.kept()[0],
        0,
        "lane 0 passes through with no node: kept"
    );
}

#[test]
fn a_pass_through_is_dropped_never_folded_and_never_counted() {
    let rows = wide_history();
    // Rows 3..12: merges in lane 0; the lanes the top three merges opened
    // only pass through, the rest open here — none holds a node here.
    let fold = Fold::plan(4, &none(), &rows[3..12]);

    assert_eq!(fold.kept(), [0]);
    assert_eq!(fold.folded_count(), 0, "a dropped lane is not folded");
    assert_eq!(fold.marker(), None);
    assert_eq!(
        fold.open_columns(),
        13,
        "lane 0 and twelve topic lanes open"
    );
    for row in 0..3 {
        let pass_through = opened_on(&rows, row);
        assert!(!fold.kept().contains(&pass_through));
        assert!(!fold.folded().contains(&pass_through));
    }
}

#[test]
fn edges_into_folded_lanes_leave_the_row_as_stubs_in_their_colour() {
    let rows = wide_history();
    // The topic commits and the root: every topic lane closes into the root.
    let window = &rows[12..];
    let fold = Fold::plan(8, &none(), window);
    let root = window.last().expect("the root");
    assert_eq!(fold.folded_count(), 6);

    let drawn = fold.view(root);
    assert_eq!(drawn.row.lane, 0);
    let kept_in: Vec<usize> = root
        .incoming
        .iter()
        .filter_map(|&c| fold.drawn(c))
        .collect();
    assert_eq!(drawn.row.incoming, kept_in);
    let expected: Vec<FoldedEdge> = root
        .incoming
        .iter()
        .filter(|c| fold.folded().contains(c))
        .map(|&c| FoldedEdge {
            identity: root.lanes[c].expect("a lane closes in"),
            toward: Toward::Above,
        })
        .collect();
    assert_eq!(drawn.folded, expected);
    let rail = fold.marker().expect("something folds");
    assert_eq!(drawn.row.lanes.get(rail).copied().flatten(), None);
}

#[test]
fn a_fold_that_changes_nothing_borrows_the_row_and_folds_no_edge() {
    let rows = wide_history();
    let fold = Fold::plan(64, &none(), &rows);
    assert!(fold.is_empty());
    assert_eq!(fold.folded_count(), 0);
    for row in &rows {
        let drawn = fold.view(row);
        assert!(
            matches!(drawn.row, Cow::Borrowed(_)),
            "{:?} was copied",
            row.commit.id
        );
        assert!(drawn.folded.is_empty());
    }
}

/// ADR-0025: the budget admits node columns in strict priority order —
/// selected over reserved, reserved over preferred, preferred over an
/// unclassed column, whatever their recency. The top three topic commits'
/// columns are the earliest touched — the first a pure recency rule would
/// fold — while the bottom one's is the latest-touched of the rest.
#[test]
fn the_budget_admits_selected_then_reserved_then_preferred_over_a_more_recently_touched_column() {
    let rows = wide_history();
    let window = topic_commits(&rows);
    let selected = topic_column(&rows, 0);
    let reserved = topic_column(&rows, 1);
    let preferred = topic_column(&rows, 2);
    let recent_unclassed = topic_column(&rows, 11);
    let keep = Keep {
        selected: Some(selected),
        reserved: BTreeSet::from([reserved]),
        preferred: BTreeSet::from([preferred]),
    };

    // Room for lane 0 plus all three classified columns: the nine unclassed
    // columns fold, `recent_unclassed` included, though it was touched more
    // recently than every classified column.
    let fold = Fold::plan(5, &keep, window);
    assert_eq!(fold.drawn_columns(), 5);
    for column in [selected, reserved, preferred] {
        assert!(fold.kept().contains(&column), "{column} should survive");
    }
    assert!(fold.folded().contains(&recent_unclassed));

    // One slot less: `preferred` folds too, `reserved` still survives.
    let fold = Fold::plan(4, &keep, window);
    assert_eq!(fold.drawn_columns(), 4);
    assert!(fold.folded().contains(&preferred), "preferred should fold");
    assert!(fold.kept().contains(&reserved), "reserved should survive");
    assert!(fold.kept().contains(&selected), "selected should survive");

    // One slot less again: `reserved` folds too, only `selected` survives.
    let fold = Fold::plan(3, &keep, window);
    assert_eq!(fold.drawn_columns(), 3);
    assert!(fold.folded().contains(&reserved), "reserved should fold");
    assert!(fold.kept().contains(&selected), "selected should survive");
}

#[test]
fn lane_zero_draws_alone_when_the_budget_allows_only_the_rail() {
    let rows = wide_history();
    // Eleven topic commits: lane 0 plus eleven node columns.
    let fold = Fold::plan(2, &none(), &rows[12..23]);

    assert_eq!(fold.kept(), [0]);
    assert_eq!(fold.folded_count(), 11);
    assert_eq!(fold.drawn_columns(), 2);
}

#[test]
fn a_thread_of_twelve_columns_still_folds_down_to_the_budget() {
    let rows = wide_history();
    let selected = topic_column(&rows, 0);
    let thread: BTreeSet<usize> = (0..12).map(|i| topic_column(&rows, i)).collect();
    let keep = Keep {
        selected: Some(selected),
        reserved: BTreeSet::new(),
        preferred: thread,
    };

    let fold = Fold::plan(8, &keep, topic_commits(&rows));
    assert_eq!(fold.drawn_columns(), 8);
    assert!(fold.folded_count() >= 5, "{}", fold.folded_count());
    assert!(
        fold.kept().contains(&selected),
        "the selected column is always drawn"
    );
}

#[test]
fn a_node_column_over_the_cap_folds_and_its_node_draws_on_the_rail() {
    let rows = wide_history();
    // Every topic commit is a node in this window; the cap still holds.
    let window = &rows[12..];
    let fold = Fold::plan(8, &none(), window);

    assert_eq!(fold.drawn_columns(), 8);
    assert_eq!(fold.folded_count(), 6);
    let rail = fold.marker().expect("node columns folded");
    let behind = window
        .iter()
        .find(|row| fold.folded().contains(&row.lane))
        .expect("a topic commit behind the rail");
    assert_eq!(fold.view(behind).row.lane, rail);
}

#[test]
fn a_column_open_in_no_row_or_with_no_node_is_dropped_without_counting() {
    let who = identity_at(1);
    let commit = Commit {
        id: oid(1),
        parents: Vec::new(),
        author: who.clone(),
        committer: who,
        message: b"c".to_vec(),
    };
    let some = |name: &str| Some(BranchIdentity::of_name(name.as_bytes()));
    let row = Row {
        commit,
        drawn_parents: Vec::new(),
        lane: 0,
        incoming: Vec::new(),
        outgoing: vec![(0, oid(2))],
        lanes: vec![
            some("main"),
            None,
            some("b"),
            some("c"),
            some("d"),
            some("e"),
            some("f"),
            some("g"),
            some("h"),
        ],
        marker: None,
    };
    let fold = Fold::plan(8, &none(), std::slice::from_ref(&row));

    assert_eq!(fold.marker(), None, "nothing folds");
    assert_eq!(fold.kept(), [0], "b to h hold no node: dropped");
    assert_eq!(fold.folded_count(), 0);
    assert_eq!(fold.open_columns(), 8, "column 1 is open in no row");
    assert!(!fold.is_empty(), "the row is not drawn as laid out");
    assert_eq!(fold.view(&row).row.lanes, [some("main")]);
}
