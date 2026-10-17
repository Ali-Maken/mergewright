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

fn none() -> Keep {
    Keep::default()
}

#[test]
fn a_window_within_the_limit_draws_as_laid_out() {
    let rows = wide_history();
    let fold = Fold::plan(8, &none(), &rows[..3]);

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
    let fold = Fold::plan(8, &none(), merges);

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
    let fold = Fold::plan(4, &none(), &rows[3..12]);

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
    let fold = Fold::plan(8, &none(), &rows[..12]);
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

/// ADR-0025: the budget admits columns in strict priority order — selected
/// over reserved, reserved over preferred, preferred over an unclassed
/// column, whatever their recency. Twelve merges open thirteen columns
/// (lane 0 plus one per merge); `opened_on(&rows, 0..3)` are the three
/// earliest-touched — the first candidates a pure recency rule would fold —
/// while `opened_on(&rows, 11)` is the latest-touched of the unclassed rest.
#[test]
fn the_budget_admits_selected_then_reserved_then_preferred_over_a_more_recently_touched_column() {
    let rows = wide_history();
    let merges = &rows[..12];
    let selected = opened_on(&rows, 0);
    let reserved = opened_on(&rows, 1);
    let preferred = opened_on(&rows, 2);
    let recent_unclassed = opened_on(&rows, 11);
    let keep = Keep {
        selected: Some(selected),
        reserved: BTreeSet::from([reserved]),
        preferred: BTreeSet::from([preferred]),
    };

    // Room for lane 0 plus all three classified columns: the nine unclassed
    // columns fold, `recent_unclassed` included, though it was touched more
    // recently than every classified column.
    let fold = Fold::plan(5, &keep, merges);
    assert_eq!(fold.drawn_columns(), 5);
    for column in [selected, reserved, preferred] {
        assert!(fold.kept().contains(&column), "{column} should survive");
    }
    assert!(fold.folded().contains(&recent_unclassed));

    // One slot less: `preferred` folds too, `reserved` still survives.
    let fold = Fold::plan(4, &keep, merges);
    assert_eq!(fold.drawn_columns(), 4);
    assert!(fold.folded().contains(&preferred), "preferred should fold");
    assert!(fold.kept().contains(&reserved), "reserved should survive");
    assert!(fold.kept().contains(&selected), "selected should survive");

    // One slot less again: `reserved` folds too, only `selected` survives.
    let fold = Fold::plan(3, &keep, merges);
    assert_eq!(fold.drawn_columns(), 3);
    assert!(fold.folded().contains(&reserved), "reserved should fold");
    assert!(fold.kept().contains(&selected), "selected should survive");
}

#[test]
fn lane_zero_draws_alone_when_the_budget_allows_only_the_rail() {
    let rows = wide_history();
    // Eleven merges: lane 0 plus eleven opened lanes, twelve open columns.
    let fold = Fold::plan(2, &none(), &rows[..11]);

    assert_eq!(fold.kept(), [0]);
    assert_eq!(fold.folded_count(), 11);
    assert_eq!(fold.drawn_columns(), 2);
}

#[test]
fn a_thread_of_twelve_columns_still_folds_down_to_the_budget() {
    let rows = wide_history();
    let merges = &rows[..12];
    let selected = opened_on(&rows, 0);
    let thread: BTreeSet<usize> = (0..12).map(|row| opened_on(&rows, row)).collect();
    let keep = Keep {
        selected: Some(selected),
        reserved: BTreeSet::new(),
        preferred: thread,
    };

    let fold = Fold::plan(8, &keep, merges);
    assert_eq!(fold.drawn_columns(), 8);
    assert!(fold.folded_count() >= 5, "{}", fold.folded_count());
    assert!(
        fold.kept().contains(&selected),
        "the selected column is always drawn"
    );
}

#[test]
fn when_only_nodes_remain_the_window_draws_wider_than_the_limit() {
    let rows = wide_history();
    // Every topic commit is a node in this window: nothing may fold.
    let fold = Fold::plan(8, &none(), &rows[12..]);

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

/// A laid-out history, a window into it, a lane limit and a keep.
#[derive(Debug)]
struct Case {
    history: History,
    rows: Vec<Row>,
    window: std::ops::Range<usize>,
    max_lanes: usize,
    selected: Option<usize>,
    preferred: BTreeSet<usize>,
}

fn case() -> impl Strategy<Value = Case> {
    history().prop_flat_map(|history| {
        let rows = layout(&history.commits, history.seeds());
        let len = rows.len();
        (
            Just(history),
            Just(rows),
            (0..len).prop_flat_map(move |start| (Just(start), start + 1..=len)),
            2..=6usize,
            proptest::option::of(0..8usize),
            proptest::collection::btree_set(0..8usize, 0..=3),
        )
            .prop_map(
                |(history, rows, (start, end), max_lanes, selected, preferred)| Case {
                    history,
                    rows,
                    window: start..end,
                    max_lanes,
                    selected,
                    preferred,
                },
            )
    })
}

impl Case {
    fn window(&self) -> &[Row] {
        &self.rows[self.window.clone()]
    }

    fn keep(&self) -> Keep {
        Keep {
            selected: self.selected,
            reserved: self.history.reserved_columns(),
            preferred: self.preferred.clone(),
        }
    }

    fn plan(&self) -> Fold {
        Fold::plan(self.max_lanes, &self.keep(), self.window())
    }
}

proptest! {
    #![proptest_config(tier())]

    #[test]
    fn kept_and_folded_partition_every_open_column_and_the_plan_is_deterministic(case in case()) {
        let fold = case.plan();
        let kept: BTreeSet<usize> = fold.kept().iter().copied().collect();
        let folded: BTreeSet<usize> = fold.folded().iter().copied().collect();
        prop_assert!(kept.is_disjoint(&folded), "a column is both kept and folded");
        prop_assert_eq!(kept.len() + folded.len(), fold.kept().len() + fold.folded().len());
        prop_assert_eq!(&fold, &case.plan(), "not deterministic");
    }

    #[test]
    fn only_required_columns_survive_when_the_budget_cannot_fit_more(case in case()) {
        let fold = case.plan();
        let open: BTreeSet<usize> = fold.kept().iter().chain(fold.folded()).copied().collect();
        let noded: BTreeSet<usize> = case.window().iter().map(|row| row.lane).collect();
        let required: BTreeSet<usize> = open
            .iter()
            .copied()
            .filter(|&c| c == 0 || noded.contains(&c))
            .collect();
        for &column in fold.folded() {
            prop_assert!(!required.contains(&column), "required column {column} folded");
        }
        if required.len() < case.max_lanes {
            prop_assert!(fold.drawn_columns() <= case.max_lanes);
        } else {
            let kept: BTreeSet<usize> = fold.kept().iter().copied().collect();
            prop_assert_eq!(
                &kept,
                &required,
                "no selected, reserved, preferred or other column should survive"
            );
        }
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
            // Every kept incoming edge comes down a lane the drawn row draws
            // (m1-the-graph/68): no `─` into an empty column.
            for &c in &drawn.row.incoming {
                prop_assert!(
                    drawn.row.lanes.get(c).copied().flatten().is_some(),
                    "drawn column {} bends in with no lane above it", c
                );
            }
            if let Some(rail) = rail {
                prop_assert_eq!(drawn.row.lanes.get(rail).copied().flatten(), None);
            }
        }
    }
}
