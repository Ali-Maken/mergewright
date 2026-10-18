//! A fold's laws over generated histories, windows, caps and keeps.

use std::collections::BTreeSet;

use proptest::prelude::*;

use super::super::*;
use crate::layout::layout;
use crate::layout::tests::properties::{History, history};

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

    /// Columns open in the window, counted from the rows themselves: a
    /// lane present, or a node or edge in the column.
    fn open(&self) -> BTreeSet<usize> {
        let mut open = BTreeSet::new();
        for row in self.window() {
            open.extend(
                row.lanes
                    .iter()
                    .enumerate()
                    .filter_map(|(c, l)| l.map(|_| c)),
            );
            open.extend(touched(row));
        }
        open
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

    /// Ease-pass 06b: kept ∪ folded is exactly the open columns holding a
    /// node in the window, and lane 0 while open; the budget holds; M counts
    /// every open column, dropped ones included.
    #[test]
    fn only_columns_holding_a_node_and_lane_zero_are_drawn_or_folded(case in case()) {
        let fold = case.plan();
        let open = case.open();
        let noded: BTreeSet<usize> = case.window().iter().map(|row| row.lane).collect();
        let drawable: BTreeSet<usize> = open
            .iter()
            .copied()
            .filter(|&c| c == 0 || noded.contains(&c))
            .collect();
        let required: BTreeSet<usize> = open.iter().copied().filter(|&c| c == 0).collect();
        let kept: BTreeSet<usize> = fold.kept().iter().copied().collect();
        let folded: BTreeSet<usize> = fold.folded().iter().copied().collect();

        prop_assert!(kept.is_disjoint(&folded));
        let both: BTreeSet<usize> = kept.union(&folded).copied().collect();
        prop_assert_eq!(&both, &drawable);
        prop_assert!(required.is_subset(&kept), "lane 0 folded or dropped while open");
        prop_assert!(fold.kept().len() <= case.max_lanes.max(required.len()));
        prop_assert!(fold.drawn_columns() <= case.max_lanes, "the budget holds, rail included");
        prop_assert_eq!(fold.open_columns(), open.len());
        prop_assert!(fold.open_columns() >= fold.kept().len() + fold.folded_count());
    }

    #[test]
    fn expanding_restores_the_pre_fold_columns_and_changes_no_other_row(case in case()) {
        let fold = case.plan();
        for row in case.window() {
            let view = fold.view(row).row;
            // The node is where the fold says it is: its kept column, or the
            // rail when its column folded.
            if fold.drawn(row.lane).is_some() {
                prop_assert_eq!(fold.real(view.lane), Some(row.lane));
            } else {
                prop_assert_eq!(Some(view.lane), fold.marker());
            }
            // Every kept lane is where the fold says it is.
            for (drawn, &real) in fold.kept().iter().enumerate() {
                let shown = view.lanes.get(drawn).copied().flatten();
                let laid_out = row.lanes.get(real).copied().flatten();
                prop_assert_eq!(shown, laid_out, "column {} at {}", real, drawn);
            }
            // A row touching no column the fold leaves undrawn maps back to
            // itself exactly.
            let touches_undrawn = touched(row).any(|c| fold.drawn(c).is_none());
            if !touches_undrawn {
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
            // minus exactly those into undrawn columns, in order.
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
