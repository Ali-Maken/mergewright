//! ADR-0064: a lane that reaches another branch's tip carries that branch's
//! name from that row down.

use super::*;
use crate::layout::assign_row::renamed_at_tip;
use crate::role::made_on;

fn name(text: &str) -> BranchIdentity {
    BranchIdentity::of_name(text.as_bytes())
}

fn follower(tip: u8, text: &str, of: &str) -> BranchSeed {
    BranchSeed {
        tip: oid(tip),
        identity: name(text),
        follows: Some(name(of)),
    }
}

/// `main(1) <- L1(2) <- T(3) <- U1(4) <- U2(5)`, newest first. The upper
/// branch is at U2, the lower at T.
fn stack() -> [Commit; 5] {
    [
        commit(5, &[4], 500, b"U2"),
        commit(4, &[3], 400, b"U1"),
        commit(3, &[2], 300, b"T"),
        commit(2, &[1], 200, b"L1"),
        commit(1, &[], 100, b"main"),
    ]
}

fn stack_seeds(known: Vec<BranchSeed>) -> Seeds {
    Seeds {
        default_branch: Some(seed(1, name("main"))),
        known_branches: known,
        ..Seeds::default()
    }
}

fn made_on_each(rows: &[Row]) -> Vec<Option<BranchIdentity>> {
    rows.iter().map(made_on).collect()
}

pub(crate) fn upper_lower_seeds() -> Seeds {
    stack_seeds(vec![seed(5, name("upper")), seed(3, name("lower"))])
}

pub(crate) fn stack_commits() -> [Commit; 5] {
    stack()
}

#[test]
fn a_stack_names_each_branchs_own_commits_in_one_column() {
    let rows = layout(&stack(), upper_lower_seeds());
    let (upper, lower) = (Some(name("upper")), Some(name("lower")));
    assert_eq!(made_on_each(&rows[..4]), [upper, upper, lower, lower]);
    assert!(rows[..4].iter().all(|row| row.lane == 1));
}

#[test]
fn a_tip_on_the_trunk_line_leaves_trunk_alone() {
    // main is at U2; `release` points at T, on main's own first-parent line.
    let seeds = Seeds {
        default_branch: Some(seed(5, name("main"))),
        known_branches: vec![seed(3, name("release"))],
        ..Seeds::default()
    };
    let rows = layout(&stack(), seeds);
    let main = Some(name("main"));
    assert!(rows.iter().all(|row| row.lane == 0));
    assert_eq!(made_on_each(&rows), [main; 5]);
}

#[test]
fn the_remote_copy_of_the_lanes_own_branch_does_not_rename_it() {
    let rows = layout(
        &stack(),
        stack_seeds(vec![
            seed(5, name("upper")),
            follower(4, "origin/upper", "upper"),
            seed(3, name("lower")),
        ]),
    );
    let upper = Some(name("upper"));
    assert_eq!(
        made_on_each(&rows[..3]),
        [upper, upper, Some(name("lower"))]
    );
}

#[test]
fn a_merge_parent_that_is_a_branch_tip_names_its_lane_from_the_merge_row() {
    // M(6) merges side(4) into main; side is a live branch.
    let commits = [
        commit(6, &[2, 4], 600, b"Merge branch 'old-name'"),
        commit(4, &[2], 500, b"side work"),
        commit(2, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(6, name("main"))),
        known_branches: vec![seed(4, name("side"))],
        ..Seeds::default()
    };
    let rows = layout(&commits, seeds);
    assert_eq!(rows[0].lanes[rows[0].outgoing[1].0], Some(name("side")));
    assert_eq!(made_on(&rows[1]), Some(name("side")));
}

#[test]
fn a_merge_lane_reaching_another_tip_lower_down_is_renamed_there() {
    // M(6) merges 'topic' (5); topic's parent 4 is the tip of `base`.
    let commits = [
        commit(6, &[2, 5], 600, b"Merge branch 'topic'"),
        commit(5, &[4], 550, b"topic work"),
        commit(4, &[2], 500, b"base work"),
        commit(2, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(6, name("main"))),
        known_branches: vec![seed(4, name("base"))],
        ..Seeds::default()
    };
    let rows = layout(&commits, seeds);
    assert_eq!(made_on(&rows[1]), Some(name("topic")));
    assert_eq!(made_on(&rows[2]), Some(name("base")));
}

#[test]
fn renamed_at_tip_keeps_a_reserved_lane_and_the_lanes_own_name() {
    let lane = |identity, reserved| Lane {
        index: 1,
        identity,
        waiting_for: CommitHandle(0),
        reserved,
    };
    let own = LaneIdentity::Known(name("a"));
    let tip = seed(9, name("b"));
    assert_eq!(renamed_at_tip(&lane(own, true), Some(&tip)), None);
    assert_eq!(renamed_at_tip(&lane(own, false), None), None);
    assert_eq!(
        renamed_at_tip(&lane(own, false), Some(&seed(9, name("a")))),
        None
    );
    assert_eq!(
        renamed_at_tip(&lane(own, false), Some(&tip)),
        Some(LaneIdentity::Known(name("b")))
    );
}
