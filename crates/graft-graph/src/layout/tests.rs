use graft_core::commit::{Identity, Timestamp};
use graft_core::commit_handle::{CommitHandle, Link};

use super::*;

pub(crate) fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

pub(crate) fn identity_at(seconds: i64) -> Identity {
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
pub(crate) fn commit(id: u8, parents: &[u8], seconds: i64, subject: &[u8]) -> Commit {
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: identity_at(seconds),
        committer: identity_at(seconds),
        message: subject.to_vec(),
    }
}

/// Where a row sits and which edges touch it — everything about a row except
/// the commit itself.
type Placement = (usize, Vec<usize>, Vec<(usize, Oid)>);

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

    let commits = [t1, f, g];
    let handles = Handles::of(&commits);
    let rows = extend_rows(
        &mut LayoutCursor::new(seeds, &handles),
        &commits,
        &commits,
        &handles,
        &mut EmittedOrder::new(),
    );

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
        Vec::<usize>::new(),
        "develop's reserved lane converges here, but was never drawn above \
         its tip, so it bends no edge in (m1-the-graph/68)"
    );
    assert_eq!(
        rows[2].lane, 2,
        "neither reserved lane (0 or 1) is ever handed back for reuse"
    );
}

/// m1-the-graph/68, the minimal case: an integration branch whose tip is the
/// default branch's tip. Its reserved lane was never drawn — nothing sits
/// above its tip — so it bends nothing into the node: no `─` into a column
/// the row draws no lane in.
#[test]
fn a_reserved_lane_closing_at_its_own_tip_bends_no_edge_in() {
    let commits = [commit(1, &[], 100, b"main and develop tip")];
    let seeds = Seeds {
        default_branch: Some(seed(1, BranchIdentity::of_name(b"main"))),
        integration_branches: vec![seed(1, BranchIdentity::of_name(b"develop"))],
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[0].lane, 0);
    assert_eq!(rows[0].lanes.len(), 1, "only main's column is drawn");
    assert_eq!(
        rows[0].incoming,
        Vec::<usize>::new(),
        "develop's lane was never drawn, so nothing bends in from it"
    );
}

#[test]
fn octopus_merge_opens_a_distinct_lane_per_extra_parent() {
    let m = commit(10, &[11, 12, 13, 14], 100, b"Octopus merge of three topics");
    let seeds = Seeds {
        default_branch: Some(seed(10, BranchIdentity::of_name(b"main"))),
        ..Seeds::default()
    };

    let commits = [m];
    let handles = Handles::of(&commits);
    let mut cursor = LayoutCursor::new(seeds, &handles);
    let rows = extend_rows(
        &mut cursor,
        &commits,
        &commits,
        &handles,
        &mut EmittedOrder::new(),
    );

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

    let commits = [m];
    let handles = Handles::of(&commits);
    let mut cursor = LayoutCursor::new(seeds, &handles);
    let rows = extend_rows(
        &mut cursor,
        &commits,
        &commits,
        &handles,
        &mut EmittedOrder::new(),
    );

    let lane = cursor.active_lanes()[1]
        .as_ref()
        .expect("merge lane still open");
    assert_eq!(
        lane.identity,
        LaneIdentity::Merge {
            merge: handles.handle_of(&oid(10)).expect("interned"),
            parent: handles.handle_of(&parent).expect("interned"),
        },
        "the layout defers the name to assembly"
    );
    assert_eq!(rows[0].lanes[1], Some(BranchIdentity::of_short_sha(parent)));
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

    let handles = Handles::of(&commits);
    let mut cursor = LayoutCursor::new(seeds(), &handles);
    let mut order = EmittedOrder::new();
    let mut split_calls = extend_rows(&mut cursor, &commits[0..1], &commits, &handles, &mut order);
    split_calls.extend(extend_rows(
        &mut cursor,
        &commits[1..2],
        &commits,
        &handles,
        &mut order,
    ));
    split_calls.extend(extend_rows(
        &mut cursor,
        &commits[2..3],
        &commits,
        &handles,
        &mut order,
    ));

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

    let shape = |rows: Vec<Row>| -> Vec<Placement> {
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

    let handles = Handles::of(&commits);
    let mut cursor = LayoutCursor::new(seeds, &handles);
    let rows = extend_rows(
        &mut cursor,
        &commits,
        &commits,
        &handles,
        &mut EmittedOrder::new(),
    );

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

#[test]
fn a_checkpoint_costs_the_same_after_a_long_history_as_after_a_short_one() {
    // The cursor is what a checkpoint clones (DESIGN.md §3.6), so it must
    // not remember history: KNOWN_GAPS R2-1 was exactly a cursor that did.
    // A linear chain keeps one lane open however long it runs, so the two
    // clones below must weigh the same.
    let chain = |length: usize| -> Vec<Commit> {
        (0..length)
            .map(|i| {
                let id = u16::try_from(length - i).expect("short chain");
                let mut commit = commit(0, &[], 1_000_000 - i64::from(id), b"step");
                commit.id = wide_oid(id);
                commit.parents = if i + 1 < length {
                    vec![wide_oid(id - 1)]
                } else {
                    Vec::new()
                };
                commit
            })
            .collect()
    };
    let seeds = |tip: Oid| Seeds {
        default_branch: Some(BranchSeed {
            tip,
            identity: BranchIdentity::of_name(b"main"),
        }),
        ..Seeds::default()
    };

    let short = chain(20);
    let short_handles = Handles::of(&short);
    let mut short_cursor = LayoutCursor::new(seeds(short[0].id), &short_handles);
    let short_rows = extend_rows(
        &mut short_cursor,
        &short,
        &short,
        &short_handles,
        &mut EmittedOrder::new(),
    );

    let long = chain(5_000);
    let long_handles = Handles::of(&long);
    let mut long_cursor = LayoutCursor::new(seeds(long[0].id), &long_handles);
    let long_rows = extend_rows(
        &mut long_cursor,
        &long,
        &long,
        &long_handles,
        &mut EmittedOrder::new(),
    );

    assert!(short_rows.iter().chain(&long_rows).all(|row| row.lane == 0));
    assert_eq!(long_cursor.rows_emitted(), 5_000);
    assert_eq!(
        long_cursor.approximate_bytes(),
        short_cursor.approximate_bytes(),
        "a checkpoint's size depends on open lanes, not on rows emitted"
    );
    assert!(
        long_cursor.approximate_bytes() < 1024,
        "one open lane is a few hundred bytes, got {}",
        long_cursor.approximate_bytes()
    );
}

/// An oid distinct for every `u16`, for histories longer than a byte counts.
fn wide_oid(n: u16) -> Oid {
    let mut bytes = [0u8; Oid::SHA1_BYTES];
    bytes[..2].copy_from_slice(&n.to_be_bytes());
    Oid::from_bytes(&bytes).expect("20 bytes is a valid oid length")
}

/// A test source's id table: dense handles over every id its commits name,
/// parents included, in first-seen order.
#[derive(Debug, Default)]
pub(crate) struct Handles {
    ids: Vec<Oid>,
    by_id: std::collections::HashMap<Oid, u32>,
}

impl Handles {
    pub(crate) fn of(commits: &[Commit]) -> Self {
        let mut handles = Handles::default();
        for commit in commits {
            for id in std::iter::once(&commit.id).chain(&commit.parents) {
                if !handles.by_id.contains_key(id) {
                    let next =
                        u32::try_from(handles.ids.len()).expect("a test history fits in u32");
                    handles.by_id.insert(*id, next);
                    handles.ids.push(*id);
                }
            }
        }
        handles
    }

    pub(crate) fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.by_id.get(id).copied().map(CommitHandle)
    }

    pub(crate) fn id_of(&self, handle: CommitHandle) -> Oid {
        self.ids[handle.index()]
    }

    /// The links of `commits`, parents untrimmed, looked up in `by_id`.
    pub(crate) fn links(
        &self,
        by_id: &std::collections::HashMap<Oid, Commit>,
        commits: &[CommitHandle],
    ) -> Vec<Link> {
        commits
            .iter()
            .map(|&handle| {
                let commit = &by_id[&self.id_of(handle)];
                Link {
                    commit: handle,
                    parents: commit
                        .parents
                        .iter()
                        .map(|parent| self.handle_of(parent).expect("interned with the commit"))
                        .collect(),
                }
            })
            .collect()
    }
}

impl crate::layout::HandleLookup for Handles {
    fn handle_of(&self, id: Oid) -> CommitHandle {
        Handles::handle_of(self, &id).expect("id was interned by Handles::of")
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        Handles::id_of(self, handle)
    }
}

/// Lays out `batch` with `cursor` and assembles its rows, reading any merge
/// named above the batch from `all`: `extend` plus the assembly a streaming
/// layout does, for tests that drive the cursor directly.
pub(crate) fn extend_rows(
    cursor: &mut LayoutCursor,
    batch: &[Commit],
    all: &[Commit],
    handles: &Handles,
    order: &mut EmittedOrder,
) -> Vec<Row> {
    let shapes = cursor.extend(&links_of(batch, handles), handles, order);
    let by_id: std::collections::HashMap<Oid, &Commit> =
        all.iter().map(|commit| (commit.id, commit)).collect();
    let names = MergeNames::gather(
        &shapes,
        batch,
        |merges| {
            Ok::<_, std::convert::Infallible>(
                merges
                    .iter()
                    .map(|&merge| by_id[&handles.id_of(merge)].clone())
                    .collect(),
            )
        },
        |handle| handles.id_of(handle),
    )
    .unwrap_or_else(|never| match never {});
    shapes
        .into_iter()
        .zip(batch)
        .map(|(shape, commit)| {
            Row::assemble(shape, commit.clone(), &names, &|handle| {
                handles.id_of(handle)
            })
            .expect("names gathered over these shapes")
        })
        .collect()
}

pub(crate) mod properties;

#[test]
fn a_row_records_the_identity_of_every_column_it_draws() {
    let main = BranchIdentity::of_name(b"main");
    let commits = [
        commit(6, &[5, 4], 600, b"Merge branch 'b'"),
        commit(4, &[5], 500, b"b: work"),
        commit(5, &[1], 400, b"main: work"),
        commit(1, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(6, main)),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    let b = BranchIdentity::of_name(b"b");
    assert_eq!(
        rows[0].lanes,
        vec![Some(main), Some(b)],
        "the merge opens b"
    );
    assert_eq!(
        rows[1].lanes,
        vec![Some(main), Some(b)],
        "main passes b's node"
    );
    assert_eq!(
        rows[2].lanes,
        vec![Some(main), Some(b)],
        "b closes into main"
    );
    assert_eq!(
        rows[3].lanes,
        vec![Some(main)],
        "trailing columns are trimmed"
    );
}

#[test]
fn a_reserved_lane_is_not_drawn_above_its_tip() {
    let main = BranchIdentity::of_name(b"main");
    let commits = [
        commit(3, &[1], 300, b"feature tip"),
        commit(2, &[1], 200, b"main tip"),
        commit(1, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(2, main)),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[0].lane, 1);
    assert_eq!(rows[0].lanes[0], None, "main's tip is not reached yet");
    assert_eq!(rows[1].lanes[0], Some(main));
}

#[test]
fn a_root_that_is_also_a_tip_still_names_its_lane() {
    let lone = BranchIdentity::of_name(b"lone");
    let commits = [commit(1, &[], 100, b"root and tip")];
    let seeds = Seeds {
        known_branches: vec![seed(1, lone)],
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[0].lanes, vec![Some(lone)]);
}

#[test]
fn a_reused_column_carries_the_new_branch_not_the_old_one() {
    let main = BranchIdentity::of_name(b"main");
    let commits = [
        commit(6, &[5, 4], 600, b"Merge branch 'b'"),
        commit(4, &[5], 550, b"b: work"),
        commit(5, &[3], 500, b"main: work"),
        commit(3, &[1, 2], 300, b"Merge branch 'c'"),
        commit(2, &[1], 200, b"c: work"),
        commit(1, &[], 100, b"root"),
    ];
    let seeds = Seeds {
        default_branch: Some(seed(6, main)),
        ..Seeds::default()
    };

    let rows = layout(&commits, seeds);

    assert_eq!(rows[1].lane, 1);
    assert_eq!(rows[4].lane, 1, "c reuses the column b freed");
    assert_eq!(rows[1].lanes[1], Some(BranchIdentity::of_name(b"b")));
    assert_eq!(rows[4].lanes[1], Some(BranchIdentity::of_name(b"c")));
}
