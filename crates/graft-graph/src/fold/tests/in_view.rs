//! Lanes only for branches with a commit in view (ease-pass 06b): a column
//! open in the window with no node in it is dropped, at, under and over the
//! cap; column 0 stays while open; node columns fold behind the rail only
//! when they overflow the cap.

use super::*;
use crate::layout::tests::commit;

/// The eight columns' branches, every one open on every row.
const COLUMNS: usize = 8;
/// The columns a node sits in, column 0 among them.
const NODED: [usize; 5] = [0, 1, 3, 5, 6];
/// The columns open in the window with no node in it.
const NODE_LESS: [usize; 3] = [2, 4, 7];

fn every_lane() -> Vec<Option<BranchIdentity>> {
    (0..COLUMNS)
        .map(|c| Some(BranchIdentity::of_name(format!("lane-{c}").as_bytes())))
        .collect()
}

fn row(id: u8, lane: usize, incoming: Vec<usize>, outgoing: Vec<usize>) -> Row {
    let commit = commit(id, &[], i64::from(id), b"c");
    Row {
        outgoing: outgoing
            .into_iter()
            .enumerate()
            .map(|(k, c)| (c, oid(id + 100 + u8::try_from(k).expect("few"))))
            .collect(),
        drawn_parents: Vec::new(),
        commit,
        lane,
        incoming,
        lanes: every_lane(),
        marker: None,
    }
}

/// Five rows, one node in each of [`NODED`], top to bottom; every column
/// open on every row. The top row takes a lane closing in from column 2
/// (node-less); the bottom row is a merge whose second parent leaves into
/// column 7 (node-less).
fn window() -> Vec<Row> {
    vec![
        row(1, 0, vec![2], vec![0]),
        row(2, 1, vec![], vec![1]),
        row(3, 3, vec![], vec![3]),
        row(4, 5, vec![], vec![5]),
        row(5, 6, vec![], vec![6, 7]),
    ]
}

/// A node-less column the cursor's keep names: the highest priority there
/// is, and still not a reason to draw a column with nothing in it.
fn keep_node_less() -> Keep {
    Keep {
        selected: Some(4),
        ..Keep::default()
    }
}

#[test]
fn under_the_cap_only_columns_holding_a_node_are_drawn() {
    let fold = Fold::plan(10, &keep_node_less(), &window());

    assert_eq!(fold.kept(), NODED);
    assert_eq!(fold.folded_count(), 0);
    assert_eq!(fold.marker(), None, "nothing folds, so no rail");
    assert_eq!(fold.open_columns(), COLUMNS);
    assert!(
        !fold.kept().contains(&4),
        "the selected node-less column is dropped"
    );
}

#[test]
fn over_the_cap_node_columns_fold_and_node_less_ones_stay_dropped() {
    let rows = window();
    let fold = Fold::plan(3, &keep_node_less(), &rows);

    assert!(fold.kept().iter().all(|c| NODED.contains(c)), "{fold:?}");
    assert!(fold.kept().len() <= 3, "{fold:?}");
    assert_eq!(fold.drawn_columns(), 3, "two lanes and the rail");
    let mut both: Vec<usize> = fold.kept().iter().chain(fold.folded()).copied().collect();
    both.sort_unstable();
    assert_eq!(both, NODED, "folded = the node columns not kept");
    for dropped in NODE_LESS {
        assert!(!fold.kept().contains(&dropped) && !fold.folded().contains(&dropped));
    }
    assert_eq!(fold.kept(), [0, 6], "lane 0, then the last touched");
    assert_eq!(fold.open_columns(), COLUMNS);

    // Edges into dropped columns leave their rows as stubs: the merge's
    // second parent below (`▾`), the lane that closed into the top row
    // from above (`▴`).
    let merge = fold.view(&rows[4]);
    assert_eq!(merge.row.outgoing, [(1, rows[4].outgoing[0].1)]);
    assert_eq!(
        merge.folded,
        [FoldedEdge {
            identity: rows[4].lanes[7].expect("open"),
            toward: Toward::Below,
        }]
    );
    let top = fold.view(&rows[0]);
    assert!(top.row.incoming.is_empty());
    assert_eq!(
        top.folded,
        [FoldedEdge {
            identity: rows[0].lanes[2].expect("open"),
            toward: Toward::Above,
        }]
    );
}

#[test]
fn column_zero_stays_while_open_though_no_node_sits_in_it() {
    // Trunk scrolled out of view: its lane still passes through.
    let rows = window();
    for cap in [3, 10] {
        let fold = Fold::plan(cap, &Keep::default(), &rows[1..]);
        assert_eq!(fold.kept().first(), Some(&0), "cap {cap}: {fold:?}");
        assert!(!fold.folded().contains(&0));
    }
}

// ---------------------------------------------------------------------------
// Connectors (ease-pass 06u): a column with no node in the window is still
// drawn when it carries an edge whose two ends are both commits in the
// window.

/// A row with its edges spelled out: `outgoing` as `(column, parent id)`,
/// each parent also the commit's own parent, and `open` the columns a lane
/// occupies on this row, each its own branch.
fn edged(id: u8, lane: usize, incoming: &[usize], outgoing: &[(usize, u8)], open: &[usize]) -> Row {
    let parents: Vec<u8> = outgoing.iter().map(|&(_, parent)| parent).collect();
    let width = open.iter().max().map_or(0, |&c| c + 1);
    Row {
        commit: commit(id, &parents, i64::from(id), b"c"),
        drawn_parents: parents.iter().map(|&p| oid(p)).collect(),
        lane,
        incoming: incoming.to_vec(),
        outgoing: outgoing.iter().map(|&(c, p)| (c, oid(p))).collect(),
        lanes: (0..width)
            .map(|c| {
                open.contains(&c)
                    .then(|| BranchIdentity::of_name(format!("lane-{c}").as_bytes()))
            })
            .collect(),
        marker: None,
    }
}

/// The shape of merging main into a feature branch, as constructed rows:
/// the merge on row 0 sends its second parent down column 1, and that edge
/// joins the commit on row 3, which sits in column 0. Column 1 holds no
/// commit of its own; column 0 holds all five.
fn merged_main() -> Vec<Row> {
    vec![
        edged(1, 0, &[], &[(0, 2), (1, 4)], &[0, 1]),
        edged(2, 0, &[], &[(0, 3)], &[0, 1]),
        edged(3, 0, &[], &[(0, 4)], &[0, 1]),
        edged(4, 0, &[1], &[(0, 5)], &[0, 1]),
        edged(5, 0, &[], &[], &[0]),
    ]
}

#[test]
fn a_column_joining_two_commits_in_view_is_drawn() {
    let rows = merged_main();
    let fold = Fold::plan(10, &Keep::default(), &rows);

    assert_eq!(fold.kept(), [0, 1], "the connector is drawn");
    assert_eq!(fold.folded_count(), 0);
    assert_eq!(fold.marker(), None);
    assert_eq!(fold.open_columns(), 2);
    for row in &rows {
        assert_eq!(fold.view(row).folded, [], "{:?}: no stub", row.commit.id);
    }
}

#[test]
fn a_connector_with_an_end_out_of_view_is_a_pass_through() {
    let rows = merged_main();
    let one_stub = |toward| {
        [FoldedEdge {
            identity: BranchIdentity::of_name(b"lane-1"),
            toward,
        }]
    };

    // The merge scrolled off the top: the edge comes from out of view.
    let fold = Fold::plan(10, &Keep::default(), &rows[1..]);
    assert_eq!(fold.kept(), [0], "column 1 dropped");
    assert_eq!(fold.folded_count(), 0);
    assert_eq!(fold.open_columns(), 2, "the dropped column still counts");
    assert_eq!(fold.view(&rows[3]).folded, one_stub(Toward::Above));

    // The commit it joins scrolled off the bottom: it leads out of view.
    let fold = Fold::plan(10, &Keep::default(), &rows[..3]);
    assert_eq!(fold.kept(), [0], "column 1 dropped");
    assert_eq!(fold.folded_count(), 0);
    assert_eq!(fold.open_columns(), 2, "the dropped column still counts");
    assert_eq!(fold.view(&rows[0]).folded, one_stub(Toward::Below));
}

/// Node columns 0, 2 and 3, and a connector in column 1: the merge on row 0
/// sends its second parent down column 1 to the commit on row 3, in column
/// 0. Columns 2 and 3 carry two commits' lanes down to the root.
fn connector_beside_three_node_columns() -> Vec<Row> {
    vec![
        edged(1, 0, &[], &[(0, 4), (1, 4)], &[0, 1]),
        edged(2, 2, &[], &[(2, 5)], &[0, 1, 2]),
        edged(3, 3, &[], &[(3, 5)], &[0, 1, 2, 3]),
        edged(4, 0, &[1], &[(0, 5)], &[0, 1, 2, 3]),
        edged(5, 0, &[2, 3], &[], &[0, 2, 3]),
    ]
}

#[test]
fn a_connector_past_the_budget_drops_and_never_folds() {
    let rows = connector_beside_three_node_columns();
    let stubs = |fold: &Fold| (fold.view(&rows[0]).folded, fold.view(&rows[3]).folded);
    let lane_1 = BranchIdentity::of_name(b"lane-1");
    let both_ends = (
        vec![FoldedEdge {
            identity: lane_1,
            toward: Toward::Below,
        }],
        vec![FoldedEdge {
            identity: lane_1,
            toward: Toward::Above,
        }],
    );

    // The three node columns fill the budget exactly: nothing folds, no
    // rail, and the connector has no room.
    let fold = Fold::plan(3, &Keep::default(), &rows);
    assert_eq!(fold.kept(), [0, 2, 3]);
    assert!(fold.folded().is_empty());
    assert_eq!(fold.marker(), None, "no node column folded: no rail");
    assert_eq!(fold.open_columns(), 4);
    assert_eq!(stubs(&fold), both_ends);

    // One fewer: node columns 2 and 3 fold behind the rail; the connector,
    // whose ends both sit in lane 0, still only drops.
    let fold = Fold::plan(2, &Keep::default(), &rows);
    assert_eq!(fold.kept(), [0]);
    assert_eq!(fold.folded(), [2, 3], "node columns only");
    assert_eq!(fold.folded_count(), 2);
    assert_eq!(fold.marker(), Some(1), "a node column folded: the rail");
    assert_eq!(stubs(&fold), both_ends);

    // Room for all four: the connector is drawn, in its own place.
    let fold = Fold::plan(4, &Keep::default(), &rows);
    assert_eq!(fold.kept(), [0, 1, 2, 3]);
    assert_eq!(stubs(&fold), (vec![], vec![]));
}

#[test]
fn a_connector_to_a_commit_on_the_rail_drops() {
    // The merge on row 0 (column 0) sends its second parent down column 1
    // to the commit on row 1, which sits in column 2. Columns 2, 3 and 4
    // are node columns touched in that order, so column 2 folds first.
    let rows = vec![
        edged(1, 0, &[], &[(0, 6), (1, 2)], &[0, 1]),
        edged(2, 2, &[1], &[(2, 7)], &[0, 1, 2]),
        edged(3, 3, &[], &[(3, 8)], &[0, 2, 3]),
        edged(4, 4, &[], &[(4, 9)], &[0, 2, 3, 4]),
        edged(5, 0, &[], &[(0, 6)], &[0, 2, 3, 4]),
    ];
    let fold = Fold::plan(3, &Keep::default(), &rows);

    assert_eq!(fold.folded(), [2, 3], "the lower end's column folded");
    assert_eq!(fold.kept(), [0, 4]);
    assert!(!fold.kept().contains(&1) && !fold.folded().contains(&1));
    assert_eq!(fold.folded_count(), 2, "the connector is not counted");
    assert_eq!(
        fold.view(&rows[0]).folded,
        [FoldedEdge {
            identity: BranchIdentity::of_name(b"lane-1"),
            toward: Toward::Below,
        }],
        "the upper end draws a stub"
    );
    assert_eq!(
        Some(fold.view(&rows[1]).row.lane),
        fold.marker(),
        "the lower end's commit is on the rail"
    );
}

#[test]
fn a_drawn_connector_draws_all_it_carries() {
    // Column 1 joins the merge on row 0 to the commit on row 1, then carries
    // the second merge's edge from row 2 down out of view. Column 2 is a
    // pass-through on every row, so the fold is not empty.
    let rows = vec![
        edged(1, 0, &[], &[(0, 2), (1, 2)], &[0, 1, 2]),
        edged(2, 0, &[1], &[(0, 3)], &[0, 1, 2]),
        edged(3, 0, &[], &[(0, 4), (1, 9)], &[0, 1, 2]),
        edged(4, 0, &[], &[(0, 5)], &[0, 1, 2]),
    ];
    let fold = Fold::plan(10, &Keep::default(), &rows);

    assert_eq!(
        fold.kept(),
        [0, 1],
        "the connector drawn, the pass-through not"
    );
    let lane_1 = Some(BranchIdentity::of_name(b"lane-1"));
    let second_merge = fold.view(&rows[2]);
    assert_eq!(second_merge.folded, [], "no stub");
    assert_eq!(second_merge.row.outgoing, [(0, oid(4)), (1, oid(9))]);
    let below = fold.view(&rows[3]);
    assert_eq!(below.folded, []);
    assert_eq!(
        below.row.lanes.get(1).copied().flatten(),
        lane_1,
        "a lane line"
    );
}
