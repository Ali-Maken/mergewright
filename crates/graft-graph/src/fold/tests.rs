use std::borrow::Cow;
use std::collections::BTreeSet;

use proptest::prelude::*;

use super::*;
use crate::layout::tests::properties::{History, history};
use crate::layout::tests::{commit, oid};
use crate::layout::{layout, tests::identity_at};
use crate::{BranchIdentity, BranchSeed, Seeds};
use graft_core::commit::Commit;

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
            default_branch: Some(BranchSeed {
                tip: oid(13 + TOPICS),
                identity: BranchIdentity::of_name(b"main"),
            }),
            ..Seeds::default()
        },
    )
}

/// The column the merge on `row` opened.
fn opened_on(rows: &[Row], row: usize) -> usize {
    rows[row].outgoing[1].0
}

fn none() -> BTreeSet<usize> {
    BTreeSet::new()
}

#[test]
fn a_window_within_the_limit_draws_as_laid_out() {
    let rows = wide_history();
    let fold = Fold::plan(&rows[..3], 8, &none());

    assert!(fold.is_empty());
    assert_eq!(fold.marker(), None);
    assert_eq!(fold.drawn_columns(), 4, "lane 0 and three topic lanes");
    for row in &rows[..3] {
        assert_eq!(&*fold.view(row).row, row);
    }
}

#[test]
fn the_least_recently_touched_lanes_fold_first_and_the_marker_fits_the_limit() {
    let rows = wide_history();
    let merges = &rows[..12];
    let fold = Fold::plan(merges, 8, &none());

    // Thirteen columns open; seven drawn plus the marker is eight.
    assert_eq!(fold.drawn_columns(), 8);
    assert_eq!(fold.marker(), Some(7));
    let expected: Vec<usize> = (0..6).map(|row| opened_on(&rows, row)).collect();
    let mut expected_sorted = expected.clone();
    expected_sorted.sort_unstable();
    assert_eq!(
        fold.folded(),
        expected_sorted,
        "the lanes opened by the top six merges were touched longest ago"
    );
    assert_eq!(fold.kept()[0], 0);
}

#[test]
fn a_lane_no_row_touches_folds_before_one_a_row_touched() {
    let rows = wide_history();
    // Rows 3..12: the lanes the top three merges opened only pass through.
    let fold = Fold::plan(&rows[3..12], 4, &none());

    let pass_throughs: BTreeSet<usize> = (0..3).map(|row| opened_on(&rows, row)).collect();
    let folded: BTreeSet<usize> = fold.folded().iter().copied().collect();
    assert!(
        pass_throughs.is_subset(&folded),
        "pass-throughs {pass_throughs:?} should be in {folded:?}"
    );
}

#[test]
fn edges_into_folded_lanes_leave_the_row_as_stubs_in_their_colour() {
    let rows = wide_history();
    let fold = Fold::plan(&rows[..12], 8, &none());
    let top = &rows[0];
    let folded_column = opened_on(&rows, 0);
    assert!(fold.folded().contains(&folded_column));

    let drawn = fold.view(top);
    assert_eq!(drawn.row.lane, 0);
    assert_eq!(
        drawn.row.outgoing,
        [top.outgoing[0]],
        "only the first parent's edge is drawn; the second went to the fold"
    );
    assert_eq!(
        drawn.folded,
        [FoldedEdge {
            identity: top.lanes[folded_column].expect("the merge opened a lane"),
            toward: Toward::Below,
        }]
    );
    let rail = fold.marker().expect("something folds");
    assert_eq!(drawn.row.lanes.get(rail).copied().flatten(), None);
    assert_eq!(fold.folded_count(), fold.folded().len());
}

#[test]
fn a_fold_that_changes_nothing_borrows_the_row_and_folds_no_edge() {
    let rows = wide_history();
    let fold = Fold::plan(&rows, 64, &none());
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

#[test]
fn a_pinned_column_stays_and_the_next_candidate_folds_instead() {
    let rows = wide_history();
    let pinned: BTreeSet<usize> = [opened_on(&rows, 0)].into_iter().collect();
    let fold = Fold::plan(&rows[..12], 8, &pinned);

    assert!(!fold.folded().contains(&opened_on(&rows, 0)));
    assert!(fold.kept().contains(&opened_on(&rows, 0)));
    let mut expected: Vec<usize> = (1..7).map(|row| opened_on(&rows, row)).collect();
    expected.sort_unstable();
    assert_eq!(fold.folded(), expected);
    assert_eq!(fold.drawn_columns(), 8);
}

#[test]
fn when_only_nodes_and_pins_remain_the_window_draws_wider_than_the_limit() {
    let rows = wide_history();
    // Every topic commit is a node in this window: nothing may fold.
    let fold = Fold::plan(&rows[12..], 8, &none());

    assert!(fold.folded().is_empty());
    assert_eq!(fold.drawn_columns(), 13);
    assert!(fold.is_empty());
}

#[test]
fn a_column_open_in_no_row_is_dropped_without_counting() {
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
    let fold = Fold::plan([&row], 8, &none());

    assert_eq!(
        fold.marker(),
        None,
        "eight open columns fit without a marker"
    );
    assert_eq!(fold.kept(), [0, 2, 3, 4, 5, 6, 7, 8]);
    assert!(
        !fold.is_empty(),
        "column 1 is dropped, so the row is not drawn as laid out"
    );
    let view = fold.view(&row).row;
    assert_eq!(view.lanes.len(), 8);
    assert_eq!(view.lanes[1], some("b"));
}

fn tier() -> ProptestConfig {
    ProptestConfig::with_cases(graft_fixtures::proptest_cases())
}

/// A laid-out history, a window into it, a lane limit and a pinned set.
#[derive(Debug)]
struct Case {
    history: History,
    rows: Vec<Row>,
    window: std::ops::Range<usize>,
    max_lanes: usize,
    pinned: BTreeSet<usize>,
}

fn case() -> impl Strategy<Value = Case> {
    history().prop_flat_map(|history| {
        let rows = layout(&history.commits, history.seeds());
        let len = rows.len();
        (
            Just(history),
            Just(rows),
            (0..len).prop_flat_map(move |start| (Just(start), start + 1..=len)),
            1..=6usize,
            proptest::collection::btree_set(0..8usize, 0..=3),
        )
            .prop_map(|(history, rows, (start, end), max_lanes, pinned)| Case {
                history,
                rows,
                window: start..end,
                max_lanes,
                pinned,
            })
    })
}

impl Case {
    fn window(&self) -> &[Row] {
        &self.rows[self.window.clone()]
    }

    fn plan(&self) -> Fold {
        let mut pinned = self.history.reserved_columns();
        pinned.extend(&self.pinned);
        Fold::plan(self.window(), self.max_lanes, &pinned)
    }
}

proptest! {
    #![proptest_config(tier())]

    #[test]
    fn never_folds_lane_zero_a_reserved_lane_a_pinned_lane_or_a_node(case in case()) {
        let fold = case.plan();
        let noded: BTreeSet<usize> = case.window().iter().map(|row| row.lane).collect();
        for &column in fold.folded() {
            prop_assert_ne!(column, 0, "lane 0 folded");
            prop_assert!(!case.history.reserved_columns().contains(&column), "reserved {column} folded");
            prop_assert!(!case.pinned.contains(&column), "pinned {column} folded");
            prop_assert!(!noded.contains(&column), "a node's column {column} folded");
        }
        prop_assert_eq!(&fold, &case.plan(), "not deterministic");
    }

    #[test]
    fn expanding_restores_the_pre_fold_columns_and_changes_no_other_row(case in case()) {
        let fold = case.plan();
        let folded: BTreeSet<usize> = fold.folded().iter().copied().collect();
        for row in case.window() {
            let view = fold.view(row).row;
            // The node and every kept lane are where the fold says they are.
            prop_assert_eq!(fold.real(view.lane), Some(row.lane));
            for (drawn, &real) in fold.kept().iter().enumerate() {
                let shown = view.lanes.get(drawn).copied().flatten();
                let laid_out = row.lanes.get(real).copied().flatten();
                prop_assert_eq!(shown, laid_out, "column {} at {}", real, drawn);
            }
            // A row touching no folded lane maps back to itself exactly.
            let touches_folded = touched(row).any(|c| folded.contains(&c));
            if !touches_folded {
                let incoming: Option<Vec<usize>> = view.incoming.iter().map(|&c| fold.real(c)).collect();
                prop_assert_eq!(incoming.as_deref(), Some(row.incoming.as_slice()));
                let outgoing: Option<Vec<(usize, _)>> = view
                    .outgoing
                    .iter()
                    .map(|&(c, parent)| fold.real(c).map(|real| (real, parent)))
                    .collect();
                prop_assert_eq!(outgoing.as_deref(), Some(row.outgoing.as_slice()));
            }
        }
    }

    #[test]
    fn kept_and_folded_edges_are_exactly_the_laid_out_edges(case in case()) {
        let fold = case.plan();
        let rail = fold.marker();
        for row in case.window() {
            let drawn = fold.view(row);
            // Kept edges map back to real columns that are not folded.
            let kept_out: Vec<(usize, _)> = drawn.row.outgoing.iter()
                .map(|&(c, parent)| (fold.real(c).expect("an edge drawn on the rail"), parent))
                .collect();
            let kept_in: Vec<usize> = drawn.row.incoming.iter()
                .map(|&c| fold.real(c).expect("an edge drawn on the rail"))
                .collect();
            let below = drawn.folded.iter().filter(|e| e.toward == Toward::Below).count();
            let above = drawn.folded.iter().filter(|e| e.toward == Toward::Above).count();
            // None lost, none doubled: the kept edges are the laid-out edges
            // minus exactly those into folded columns, in order.
            let expected_out: Vec<(usize, _)> = row.outgoing.iter().copied()
                .filter(|&(c, _)| fold.drawn(c).is_some()).collect();
            let expected_in: Vec<usize> = row.incoming.iter().copied()
                .filter(|&c| fold.drawn(c).is_some()).collect();
            prop_assert_eq!(&kept_out, &expected_out);
            prop_assert_eq!(&kept_in, &expected_in);
            prop_assert_eq!(kept_out.len() + below, row.outgoing.len());
            prop_assert_eq!(kept_in.len() + above, row.incoming.len());
            // No folded lane's column appears in the drawn row.
            for &(c, _) in &drawn.row.outgoing { prop_assert!(Some(c) != rail); }
            for &c in &drawn.row.incoming { prop_assert!(Some(c) != rail); }
            prop_assert!(drawn.row.lanes.len() <= fold.kept().len());
            if let Some(rail) = rail {
                prop_assert_eq!(drawn.row.lanes.get(rail).copied().flatten(), None);
            }
        }
    }

    #[test]
    fn the_window_draws_within_the_limit_unless_only_unfoldables_remain(case in case()) {
        let fold = case.plan();
        if fold.drawn_columns() > case.max_lanes {
            let noded: BTreeSet<usize> = case.window().iter().map(|row| row.lane).collect();
            let mut pinned = case.history.reserved_columns();
            pinned.extend(&case.pinned);
            for &column in fold.kept() {
                prop_assert!(
                    column == 0 || noded.contains(&column) || pinned.contains(&column),
                    "column {} could still fold", column
                );
            }
        }
    }
}
