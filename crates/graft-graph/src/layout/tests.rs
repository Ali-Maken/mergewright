use graft_core::commit::{Identity, Timestamp};

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

fn identity_at(seconds: i64) -> Identity {
    Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds,
            offset_seconds: 0,
        },
    }
}

/// A commit with `id` and `parents` given as raw bytes (see [`oid`]), authored
/// and committed at `seconds`, with `subject` as its whole message.
fn commit(id: u8, parents: &[u8], seconds: i64, subject: &[u8]) -> Commit {
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: identity_at(seconds),
        committer: identity_at(seconds),
        message: subject.to_vec(),
    }
}

fn seed(tip: u8, identity: BranchIdentity) -> BranchSeed {
    BranchSeed {
        tip: oid(tip),
        identity,
    }
}

#[test]
fn linear_chain_stays_in_lane_zero_for_its_whole_life() {
    let commits = [
        commit(3, &[2], 300, b"third"),
        commit(2, &[1], 200, b"second"),
        commit(1, &[], 100, b"first"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(3, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert!(rows.iter().all(|row| row.lane == 0));
    assert!(rows.iter().all(|row| row.incoming.is_empty()));
    assert_eq!(
        rows[2].outgoing,
        Vec::new(),
        "a root commit has no parents to draw"
    );
}

#[test]
fn root_commit_reached_by_two_branches_lane_zero_wins() {
    // Both the default branch and an ordinary branch trace back to the same
    // root. Lane 0 must win the convergence even though nothing about the
    // numeric tie-break is special-cased for it here — it wins precisely
    // because it is always the lowest-indexed lane there is.
    let commits = [
        commit(3, &[1], 300, b"feature tip"),
        commit(2, &[1], 200, b"main tip"),
        commit(1, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(2, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    let root = &rows[2];
    assert_eq!(root.lane, 0);
    assert_eq!(root.incoming, vec![1]);
    assert_eq!(root.outgoing, Vec::new());
}

#[test]
fn freed_column_is_not_reused_on_the_freeing_row_only_the_next_one() {
    // T1's lane converges into F alongside F's own merge parent opening a
    // brand-new lane in the very same row: the merge lane must NOT grab the
    // column T1 just vacated. Only on the following row, when G needs a
    // fresh lane, does the recycled column come back.
    let t1 = commit(30, &[20], 400, b"topic tip");
    let f = commit(20, &[21, 22], 300, b"Merge branch 'x'");
    let g = commit(99, &[], 100, b"unrelated tip");
    let seeds = Seeds {
        default_branch: Some(seed(20, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let rows = LayoutCursor::new(seeds).extend(&[t1, f, g]);

    assert_eq!(
        rows[0].lane, 1,
        "T1 opens the first ordinary lane, column 1"
    );
    assert_eq!(
        rows[1].incoming,
        vec![1],
        "T1's lane converges into F and closes"
    );
    assert_eq!(
        rows[1].outgoing,
        vec![(0, oid(21)), (2, oid(22))],
        "F's merge parent opens column 2, not the column F just freed"
    );
    assert_eq!(
        rows[2].lane, 1,
        "G reuses column 1 on the row after it was freed"
    );
}

#[test]
fn reserved_lane_root_never_reused() {
    let commits = [
        commit(2, &[1], 200, b"main tip"),
        commit(1, &[], 100, b"root"),
        commit(50, &[], 50, b"unrelated tip"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(2, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[1].lane, 0, "lane 0 closes at the root");
    assert_eq!(
        rows[2].lane, 1,
        "the closed reserved lane is never handed back; the next lane opens fresh"
    );
}

#[test]
fn reserved_integration_lane_never_reused_via_convergence() {
    // Both main and develop trace back to the same root. Develop's reserved
    // lane closes there by convergence, not by reaching a root of its own —
    // the guard against reuse must cover that path too.
    let commits = [
        commit(1, &[2], 200, b"main tip"),
        commit(2, &[], 100, b"shared root, also develop's tip"),
        commit(50, &[], 50, b"unrelated tip"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(1, BranchIdentity::of_name(b"main"))),
        integration_branches: vec![seed(2, BranchIdentity::of_name(b"develop"))],
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[1].lane, 0);
    assert_eq!(
        rows[1].incoming,
        vec![1],
        "develop's reserved lane converges here"
    );
    assert_eq!(
        rows[2].lane, 2,
        "neither reserved lane (0 or 1) is ever handed back for reuse"
    );
}

#[test]
fn octopus_merge_opens_a_distinct_lane_per_extra_parent() {
    let m = commit(10, &[11, 12, 13, 14], 100, b"Octopus merge of three topics");
    let seeds = Seeds {
        default_branch: Some(seed(10, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let mut cursor = LayoutCursor::new(seeds);
    let rows = cursor.extend(&[m]);

    assert_eq!(rows[0].lane, 0);
    let lanes: Vec<usize> = rows[0].outgoing.iter().map(|&(lane, _)| lane).collect();
    assert_eq!(
        lanes,
        vec![0, 1, 2, 3],
        "each extra parent opens its own new lane"
    );

    let active = cursor.active_lanes();
    let identities: std::collections::HashSet<_> = [1, 2, 3]
        .iter()
        .map(|&lane| active[lane].as_ref().expect("lane still open").identity)
        .collect();
    assert_eq!(
        identities.len(),
        3,
        "an unrecognised octopus message still tells its parents apart"
    );
}

#[test]
fn merge_lane_falls_back_to_short_sha_identity_when_no_pattern_matches() {
    let parent = oid(11);
    let m = commit(
        10,
        &[12, 11],
        100,
        b"a merge message matching no known pattern",
    );
    let seeds = Seeds {
        default_branch: Some(seed(10, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let mut cursor = LayoutCursor::new(seeds);
    let _rows = cursor.extend(&[m]);

    let lane = cursor.active_lanes()[1]
        .as_ref()
        .expect("merge lane still open");
    assert_eq!(lane.identity, BranchIdentity::of_short_sha(parent));
}

#[test]
fn two_layout_runs_over_the_same_commits_are_byte_identical() {
    let commits = [
        commit(30, &[20], 400, b"topic tip"),
        commit(20, &[21, 22], 300, b"Merge branch 'x'"),
        commit(99, &[], 100, b"unrelated tip"),
    ];
    let seeds = || Seeds {
        default_branch: Some(seed(20, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    assert_eq!(layout(&commits, seeds()), layout(&commits, seeds()));
}

#[test]
fn chunk_boundaries_do_not_change_the_rows_produced() {
    let commits = [
        commit(30, &[20], 400, b"topic tip"),
        commit(20, &[21, 22], 300, b"Merge branch 'x'"),
        commit(99, &[], 100, b"unrelated tip"),
    ];
    let seeds = || Seeds {
        default_branch: Some(seed(20, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let one_call = layout(&commits, seeds());

    let mut cursor = LayoutCursor::new(seeds());
    let mut split_calls = cursor.extend(&commits[0..1]);
    split_calls.extend(cursor.extend(&commits[1..2]));
    split_calls.extend(cursor.extend(&commits[2..3]));

    assert_eq!(
        one_call, split_calls,
        "the free pool must not depend on how the caller chunks extend() calls"
    );
}

#[test]
fn equal_dates_change_nothing_and_two_runs_still_agree() {
    // The walk breaks date ties — date descending, then id ascending
    // (DESIGN.md §3.2; tested where the walk lives, in `graft-git`). The
    // layout takes the sequence it is handed and never reads a date, so a
    // tie-broken case is a non-event here: equal dates must give the same
    // rows as distinct ones, and two runs must agree byte for byte.
    let tied = [
        commit(30, &[20], 100, b"topic tip"),
        commit(20, &[21, 22], 100, b"Merge branch 'x'"),
        commit(21, &[], 100, b"main root"),
        commit(22, &[], 100, b"x root"),
    ];
    let dated = [
        commit(30, &[20], 400, b"topic tip"),
        commit(20, &[21, 22], 300, b"Merge branch 'x'"),
        commit(21, &[], 200, b"main root"),
        commit(22, &[], 100, b"x root"),
    ];
    let seeds = || Seeds {
        default_branch: Some(seed(20, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    assert_eq!(layout(&tied, seeds()), layout(&tied, seeds()));

    let shape = |rows: Vec<Row>| -> Vec<(usize, Vec<usize>, Vec<(usize, Oid)>)> {
        rows.into_iter()
            .map(|row| (row.lane, row.incoming, row.outgoing))
            .collect()
    };
    assert_eq!(
        shape(layout(&tied, seeds())),
        shape(layout(&dated, seeds())),
        "dates play no part in lane assignment"
    );
}

#[test]
fn an_edge_to_a_commit_already_drawn_is_recorded_and_its_lane_closes_at_once() {
    // Commit-date order can put a parent above its child — ADR-0008's
    // provisional first paint, or plain clock skew. The edge is still real
    // and still recorded; the lane has nothing left to wait for, so it
    // closes on the spot instead of waiting for a commit that will never
    // come. A reserved lane closing this way is still not handed back.
    let commits = [
        commit(1, &[], 300, b"parent, dated later than its child"),
        commit(2, &[1], 200, b"child, main's tip"),
        commit(3, &[], 100, b"unrelated tip"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(2, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let mut cursor = LayoutCursor::new(seeds);
    let rows = cursor.extend(&commits);

    assert_eq!(rows[0].lane, 1, "the early parent opens an ordinary lane");
    assert_eq!(rows[1].lane, 0);
    assert_eq!(
        rows[1].outgoing,
        vec![(0, oid(1))],
        "the edge to the already-drawn parent is recorded"
    );
    assert!(
        cursor.active_lanes()[0].is_none(),
        "lane 0 closes at once: its parent is already above it"
    );
    assert_eq!(
        rows[2].lane, 1,
        "the ordinary column freed by the root is reused; lane 0 is not"
    );
}

mod properties;
