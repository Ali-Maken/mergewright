use std::collections::BTreeSet;

use super::*;
use crate::fold::Keep;
use crate::layout::layout;
use crate::layout::tests::{commit, oid};
use crate::{BranchIdentity, BranchSeed, Fold, Seeds};

fn main_seed(tip: u8) -> Seeds {
    Seeds {
        default_branch: Some(BranchSeed {
            tip: oid(tip),
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    }
}

/// `main` 2 and `topic` 3 off root 1, merged by 4; `side` 5 off the root,
/// never merged. Rows, newest first: 4, 5, 3, 2, 1.
fn merged_topic() -> Vec<Row> {
    let commits = [
        commit(4, &[2, 3], 40, b"Merge branch 'topic'"),
        commit(5, &[1], 35, b"side: work"),
        commit(3, &[1], 30, b"topic: work"),
        commit(2, &[1], 20, b"main: work"),
        commit(1, &[], 0, b"root"),
    ];
    let rows = layout(&commits, main_seed(4));
    let ids: Vec<Oid> = rows.iter().map(|row| row.commit.id).collect();
    assert_eq!(
        ids,
        [4, 5, 3, 2, 1].map(oid),
        "the order these tests assume"
    );
    rows
}

#[test]
fn ancestors_below_and_descendants_above_are_kin_and_the_rest_is_not() {
    let rows = merged_topic();
    let ancestry = Ancestry::of(&rows, oid(3)).expect("the selection is in the window");
    let kin: Vec<Kin> = (0..rows.len()).map(|row| ancestry.kin(row)).collect();
    assert_eq!(
        kin,
        [
            Kin::Descendant,
            Kin::Unrelated,
            Kin::Selected,
            Kin::Unrelated,
            Kin::Ancestor,
        ]
    );
}

#[test]
fn the_thread_lights_the_edges_between_kin_and_nothing_else() {
    let rows = merged_topic();
    let ancestry = Ancestry::of(&rows, oid(3)).expect("selected");
    // The merge's second-parent edge carries the topic lane down to 3.
    let topic = rows[0].outgoing[1].0;
    assert_eq!(rows[2].lane, topic, "3 draws in the lane its merge opened");

    // The merge row: its own node and the edge branching off to the topic.
    assert!(ancestry.lit(0).contains(&rows[0].lane));
    assert!(ancestry.lit(0).contains(&topic));
    // The unrelated side commit: the topic lane passing it is lit, its own
    // node is not.
    assert!(ancestry.lit(1).contains(&topic));
    assert!(!ancestry.lit(1).contains(&rows[1].lane));
    // Main's own commit is not on the thread; the topic edge down to the root
    // still passes it.
    assert!(!ancestry.lit(3).contains(&rows[3].lane));
    assert!(ancestry.lit(3).contains(&topic));
    // The root: its node, and the topic edge closing into it.
    assert!(ancestry.lit(4).contains(&rows[4].lane));
    assert!(ancestry.lit(4).contains(&topic));
    // The main edge from the merge to 2 is not the thread: lane 0 on the
    // side commit's row carries it.
    assert!(!ancestry.lit(1).contains(&rows[0].lane) || rows[0].lane == topic);
}

#[test]
fn a_descendant_edge_entering_from_above_the_window_is_lit_from_the_top() {
    let rows = merged_topic();
    // The merge scrolled off the top: 3's lane still comes in from above.
    let window = &rows[1..];
    let ancestry = Ancestry::of(window, oid(3)).expect("selected");
    let topic = rows[2].lane;
    assert_eq!(ancestry.kin(0), Kin::Unrelated);
    assert!(ancestry.lit(0).contains(&topic), "{:?}", ancestry.lit(0));
    assert!(!ancestry.lit(0).contains(&rows[1].lane));
}

#[test]
fn a_selection_outside_the_window_has_no_ancestry() {
    let rows = merged_topic();
    assert!(Ancestry::of(&rows[..2], oid(3)).is_none());
}

#[test]
fn columns_are_every_lit_column_in_the_window() {
    let rows = merged_topic();
    let ancestry = Ancestry::of(&rows, oid(3)).expect("selected");
    let union: BTreeSet<usize> = (0..rows.len())
        .flat_map(|row| ancestry.lit(row).iter().copied())
        .collect();
    assert_eq!(ancestry.columns(), union);
}

/// ADR-0025: the selection's thread is a budget priority, not a veto. Twelve
/// topics merged into main; at a lane limit of three the thread (lane 0 and
/// the merged branch's own column) fits the budget and survives, but at a
/// tight enough limit the thread's own branch column folds too — only lane 0
/// (the one class that never folds) is guaranteed.
#[test]
fn a_lane_on_the_selected_ancestry_survives_while_the_budget_allows_it() {
    const TOPICS: u8 = 12;
    let mut commits = Vec::new();
    for k in (1..=TOPICS).rev() {
        let below = if k == 1 { 1 } else { 13 + k - 1 };
        commits.push(commit(
            13 + k,
            &[below, 1 + k],
            i64::from(100 - k),
            b"merge",
        ));
    }
    for k in (1..=TOPICS).rev() {
        commits.push(commit(1 + k, &[1], i64::from(50 - k), b"topic"));
    }
    commits.push(commit(1, &[], 0, b"root"));
    let rows = layout(&commits, main_seed(13 + TOPICS));

    // The window is the twelve merges and the twelve topic commits: every
    // topic lane holds a node here, so a fold may take it rather than drop
    // it (a lane with no node in view is never drawn, ease-pass 06b).
    let window = &rows[..2 * usize::from(TOPICS)];
    for k in 1..=4 {
        let selected = 13 + k;
        let Some(ancestry) = Ancestry::of(window, oid(selected)) else {
            panic!("merge {k} is in the window");
        };
        let keep = Keep {
            selected: None,
            reserved: BTreeSet::new(),
            preferred: ancestry.columns(),
        };

        // A budget with room for the thread keeps it whole.
        let fold = Fold::plan(6, &keep, window);
        assert!(
            !fold.folded().is_empty(),
            "merge {k}: the limit forces a fold"
        );
        for column in ancestry.columns() {
            assert!(
                !fold.folded().contains(&column),
                "merge {k}: thread column {column} folded even though the budget had room"
            );
        }
        assert!(
            fold.drawn(rows[usize::from(TOPICS - k)].outgoing[1].0)
                .is_some()
        );

        // A tight enough budget still folds part of the thread: it is a
        // priority, not a veto (ADR-0025). Lane 0 is the only column ever
        // guaranteed.
        let tight = Fold::plan(1, &keep, window);
        assert!(
            tight.kept().contains(&0),
            "merge {k}: lane 0 is never folded"
        );
        if ancestry.columns().len() > 1 {
            assert!(
                !ancestry
                    .columns()
                    .iter()
                    .all(|column| tight.kept().contains(column)),
                "merge {k}: the whole thread survived an impossibly tight budget"
            );
        }
    }
}
