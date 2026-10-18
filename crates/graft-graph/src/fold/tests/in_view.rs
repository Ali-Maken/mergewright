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
