//! A fold's laws over generated histories, windows, caps and keeps.

use std::collections::{BTreeMap, BTreeSet};

use proptest::prelude::*;

use super::super::*;
use crate::fold::scan::touched;
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

    /// Connectors, found by walking each column down the window — not by
    /// parent id, as the fold does: an edge enters a column from a node in
    /// another column (`outgoing`), the lane runs unbroken, and it closes
    /// into a node in another column (`incoming`). Kept: columns with no
    /// node in the window, lane 0 aside, each with its edges' end columns
    /// `(upper, lower)`.
    fn connectors(&self) -> BTreeMap<usize, Vec<(usize, usize)>> {
        let has_node: BTreeSet<usize> = self.window().iter().map(|row| row.lane).collect();
        let mut entered: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        let mut found: BTreeMap<usize, Vec<(usize, usize)>> = BTreeMap::new();
        for row in self.window() {
            for &column in &row.incoming {
                if let Some(uppers) = entered.remove(&column) {
                    let ends = found.entry(column).or_default();
                    ends.extend(uppers.into_iter().map(|upper| (upper, row.lane)));
                }
            }
            // A lane that stops without closing into a node ends its edges.
            entered.retain(|&column, _| {
                touched(row).any(|c| c == column)
                    || row.lanes.get(column).copied().flatten().is_some()
            });
            for &(column, _) in &row.outgoing {
                if column != row.lane {
                    entered.entry(column).or_default().push(row.lane);
                }
            }
        }
        found.retain(|column, _| *column != 0 && !has_node.contains(column));
        found
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

    /// Ease-pass 06b, restated by 06u (renamed from
    /// `only_columns_holding_a_node_and_lane_zero_are_drawn_or_folded`):
    /// every node column and lane 0 while open is drawn or folded; only node
    /// columns fold; a connector is drawn only while an edge of it joins two
    /// drawn nodes, and otherwise dropped; pass-throughs never draw. With no
    /// budget every connector draws and nothing folds. The budget holds; M
    /// counts every open column, dropped ones included.
    #[test]
    fn node_columns_draw_or_fold_connectors_draw_or_drop_and_pass_throughs_never_draw(case in case()) {
        let fold = case.plan();
        let open = case.open();
        let has_node: BTreeSet<usize> = case.window().iter().map(|row| row.lane).collect();
        let node_columns: BTreeSet<usize> = open
            .iter()
            .copied()
            .filter(|&c| c == 0 || has_node.contains(&c))
            .collect();
        let connectors = case.connectors();
        let connector_columns: BTreeSet<usize> = connectors.keys().copied().collect();
        let drawable: BTreeSet<usize> = node_columns.union(&connector_columns).copied().collect();
        let required: BTreeSet<usize> = open.iter().copied().filter(|&c| c == 0).collect();
        let kept: BTreeSet<usize> = fold.kept().iter().copied().collect();
        let folded: BTreeSet<usize> = fold.folded().iter().copied().collect();

        prop_assert!(kept.is_disjoint(&folded));
        let both: BTreeSet<usize> = kept.union(&folded).copied().collect();
        prop_assert!(both.is_subset(&drawable), "a pass-through drawn: {:?}", fold);
        prop_assert!(node_columns.is_subset(&both), "a node column dropped: {:?}", fold);
        prop_assert!(folded.is_subset(&node_columns), "a connector folded: {:?}", fold);
        prop_assert!(required.is_subset(&kept), "lane 0 folded or dropped while open");
        for column in kept.intersection(&connector_columns) {
            let joins_drawn = connectors[column]
                .iter()
                .any(|(upper, lower)| kept.contains(upper) && kept.contains(lower));
            prop_assert!(joins_drawn, "connector {} drawn with an end not drawn", column);
        }
        prop_assert!(fold.kept().len() <= case.max_lanes.max(required.len()));
        prop_assert!(fold.drawn_columns() <= case.max_lanes, "the budget holds, rail included");
        prop_assert_eq!(fold.open_columns(), open.len());
        prop_assert!(fold.open_columns() >= fold.kept().len() + fold.folded_count());

        let unbounded = Fold::plan(usize::MAX, &case.keep(), case.window());
        let unbounded_kept: BTreeSet<usize> = unbounded.kept().iter().copied().collect();
        prop_assert_eq!(&unbounded_kept, &drawable, "with no budget: every node column and connector");
        prop_assert!(unbounded.folded().is_empty());
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
