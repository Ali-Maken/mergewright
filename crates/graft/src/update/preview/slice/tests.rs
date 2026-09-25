//! The slice's rules, checked without a repository: `build_slice` is pure
//! over the rows it is handed and the plan it is drawing.

use std::collections::HashMap;
use std::time::Instant;

use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::HeadState;
use graft_ops::preview::{
    ConflictForecast, GhostCommit, HeadMove, PreviewPlan, RefMove, Target, UndoNote, WorktreeEffect,
};

use super::{BadgeOverlay, SliceError, badges_for_preview, build_slice, short_of};
use crate::refs::{RefBadges, RefLabel, Upstream};

/// A distinct id per small number, the way `view::strip`'s fixtures do it.
fn oid(n: u8) -> Oid {
    let mut bytes = [0u8; 20];
    bytes[19] = n;
    Oid::from_sha1(bytes)
}

fn who() -> Identity {
    Identity {
        name: b"Ada".to_vec(),
        email: b"ada@example.com".to_vec(),
        when: Timestamp {
            seconds: 1_700_000_000,
            offset_seconds: 0,
        },
    }
}

fn commit(id: u8, parents: &[u8]) -> Commit {
    Commit {
        id: oid(id),
        parents: parents.iter().copied().map(oid).collect(),
        author: who(),
        committer: who(),
        message: format!("commit {id}").into_bytes(),
    }
}

fn plan() -> PreviewPlan {
    PreviewPlan {
        description: "rebase feature onto main".to_owned(),
        commands: vec!["git rebase refs/heads/main".to_owned()],
        ghosts: Vec::new(),
        ref_moves: Vec::new(),
        head: None,
        worktree: WorktreeEffect::Untouched,
        conflicts: ConflictForecast::Clean,
        undo: UndoNote::Undoable,
        against: Vec::new(),
        set_aside: None,
        put_back: None,
    }
}

fn ghost(id: u8, parents: Vec<Target>, subject: &str) -> GhostCommit {
    GhostCommit {
        oid: oid(id),
        parents,
        subject: subject.to_owned(),
        author: who(),
        replays: None,
    }
}

/// The rule the whole picture hangs on: a ghost goes immediately above the
/// topmost slice row among its transitive real parents, and a chain of them
/// comes out newest-first — the order history itself is drawn in.
///
/// Rows 10 (main) and 20, 21 (feature) are resident; the two ghosts replay
/// feature's commits onto main, so both belong directly above main's row
/// and the newer replay belongs above the older.
#[test]
fn slice_places_ghosts_above_their_topmost_real_parent() {
    let resident = vec![commit(10, &[9]), commit(21, &[20]), commit(20, &[9])];
    let mut drawing = plan();
    drawing.ghosts = vec![
        ghost(100, vec![Target::Real(oid(10))], "replay of 20"),
        ghost(101, vec![Target::Ghost(0)], "replay of 21"),
    ];

    let slice = build_slice(&resident, &[], &drawing).expect("every parent is resident");

    let order: Vec<Oid> = slice.commits.iter().map(|commit| commit.id).collect();
    assert_eq!(
        order,
        vec![oid(101), oid(100), oid(10), oid(21), oid(20)],
        "the chain must sit above `onto`, newest replay first"
    );
    assert_eq!(
        slice.ghost_at,
        vec![1, 0],
        "ghost_at follows the plan's order"
    );
    // The chain hangs together: ghost 101's parent is ghost 100, whose
    // parent is main.
    assert_eq!(slice.commits[0].parents, vec![oid(100)]);
    assert_eq!(slice.commits[1].parents, vec![oid(10)]);
}

/// A ghost merge sits above the higher of its two parents, not above
/// whichever one the plan happens to list first.
#[test]
fn a_ghost_merge_sits_above_the_higher_of_its_parents() {
    let resident = vec![commit(10, &[9]), commit(20, &[9]), commit(9, &[])];
    let mut drawing = plan();
    // Second parent listed first on purpose: the rule is about position in
    // the slice, not about parent order.
    drawing.ghosts = vec![ghost(
        100,
        vec![Target::Real(oid(20)), Target::Real(oid(10))],
        "Merge branch 'topic'",
    )];

    let slice = build_slice(&resident, &[], &drawing).expect("both parents are resident");

    let order: Vec<Oid> = slice.commits.iter().map(|commit| commit.id).collect();
    assert_eq!(order, vec![oid(100), oid(10), oid(20), oid(9)]);
}

/// `layout` panics on a merge whose parent is not in the batch, so a
/// bounded slice drops the links that leave it — and says how many, because
/// a lane that stops at the slice edge is a cost, not a free lunch.
#[test]
fn slice_clips_parents_outside_and_counts_them() {
    // Row 9 is off the window; both resident rows point at it, and row 5 is
    // a merge with a second parent that is off it too.
    let resident = vec![commit(10, &[9]), commit(5, &[9, 4])];

    let slice = build_slice(&resident, &[], &plan()).expect("no ghosts to place");

    assert_eq!(slice.clipped, 3, "9, 9 and 4 all leave the slice");
    assert!(slice.commits.iter().all(|commit| commit.parents.is_empty()));
    assert_eq!(slice.commits.len(), 2, "clipping never drops a row");
}

/// An anchor the job had to read goes after the last resident row, with its
/// own parents cleared: whatever it points at is certainly not in the slice
/// either, and inventing a link would draw a lane that is not there.
#[test]
fn slice_appends_anchors_last() {
    let resident = vec![commit(21, &[20]), commit(20, &[9])];
    let anchors = vec![commit(9, &[8, 7])];
    let mut drawing = plan();
    drawing.ghosts = vec![ghost(100, vec![Target::Real(oid(9))], "replay")];

    let slice = build_slice(&resident, &anchors, &drawing).expect("9 is an anchor");

    let order: Vec<Oid> = slice.commits.iter().map(|commit| commit.id).collect();
    assert_eq!(
        order,
        vec![oid(21), oid(20), oid(100), oid(9)],
        "the anchor is last and the ghost sits directly above it"
    );
    let anchor = slice.commits.last().expect("the anchor row");
    assert!(
        anchor.parents.is_empty(),
        "an anchor's own parents are cleared, not clipped"
    );
    assert_eq!(
        slice.clipped, 2,
        "the anchor's two parents are the lanes that leave the view, and the box says so"
    );
}

/// Two builds of the same inputs give the same picture. Ghost ids are
/// deterministic in `graft-ops` for the same reason: the eye must not see
/// the rows churn between frames.
#[test]
fn slice_is_deterministic() {
    let resident = vec![commit(10, &[9]), commit(21, &[20]), commit(20, &[9])];
    let mut drawing = plan();
    drawing.ghosts = vec![
        ghost(100, vec![Target::Real(oid(10))], "one"),
        ghost(101, vec![Target::Ghost(0)], "two"),
    ];

    let first = build_slice(&resident, &[], &drawing).expect("built");
    let again = build_slice(&resident, &[], &drawing).expect("built");

    assert_eq!(first, again);
}

/// The conductor's binding condition (ADR-0038 §5): when the part of the
/// preview that matters falls outside the slice, the preview says so rather
/// than drawing a shape the user could misread. This is where it is
/// decided; `preview::preview_box` turns it into the line, and
/// `plan_whose_ghost_parent_is_missing_says_so` checks those words.
#[test]
fn slice_refuses_a_missing_ghost_parent() {
    let resident = vec![commit(21, &[20]), commit(20, &[9])];
    let mut drawing = plan();
    drawing.ghosts = vec![ghost(100, vec![Target::Real(oid(9))], "replay onto 9")];

    let refused =
        build_slice(&resident, &[], &drawing).expect_err("9 is neither resident nor an anchor");

    assert_eq!(refused, SliceError::MissingParent(oid(9)));
}

/// Hard choice 1's budget: the whole point of laying out a bounded slice
/// instead of re-walking is that it costs nothing on the frame. Three
/// screens of rows is what `open_preview` asks for; the real bound is 16 ms,
/// and this asserts a generous 50 ms so a loaded debug CI box does not
/// produce a flake that means nothing.
#[test]
fn slice_of_three_screens_is_built_well_inside_the_frame_budget() {
    let resident: Vec<Commit> = (0..150u8)
        .map(|n| {
            let mut commit = commit(n, &[]);
            commit.id = oid(n);
            if n > 0 {
                commit.parents = vec![oid(n - 1)];
            }
            commit
        })
        .collect();
    let mut drawing = plan();
    drawing.ghosts = (0..20)
        .map(|n| {
            let parent = if n == 0 {
                Target::Real(oid(149))
            } else {
                Target::Ghost(n - 1)
            };
            let mut made = ghost(0, vec![parent], &format!("replay {n}"));
            made.oid = Oid::from_sha1([u8::try_from(n).unwrap_or(0); 20]);
            made
        })
        .collect();

    let started = Instant::now();
    let slice = build_slice(&resident, &[], &drawing).expect("built");
    let took = started.elapsed();

    assert_eq!(slice.commits.len(), 170);
    assert!(
        took.as_millis() < 50,
        "a three-screen slice took {took:?}, which is not a frame's worth of work"
    );
}

// --- the badge overlay -------------------------------------------------

/// Badges built by the real builder, so the overlay is tested against the
/// shape the app actually gets rather than a hand-made one.
fn badges_with(refs: &[(&str, Oid)], head: &HeadState, upstreams: &[(&str, &str)]) -> RefBadges {
    let named: Vec<(RefName, Oid)> = refs
        .iter()
        .map(|(name, id)| (ref_name(name), *id))
        .collect();
    let tracking: HashMap<RefName, RefName> = upstreams
        .iter()
        .map(|(local, remote)| (ref_name(local), ref_name(remote)))
        .collect();
    crate::refs::badges(&named, head, &tracking, &[], |_, _| {
        Ok(graft_git::history::AheadBehind {
            ahead: 0,
            behind: 0,
        })
    })
    .expect("counting never fails here")
}

fn ref_name(name: &str) -> RefName {
    RefName::new(name.as_bytes().to_vec()).expect("a valid ref name")
}

fn branch(name: &str, current: bool) -> RefLabel {
    RefLabel::Branch {
        name: name.as_bytes().to_vec(),
        current,
        upstream: Upstream::None,
    }
}

/// A ref that moves leaves its old row and arrives on the new one — and
/// keeps the chip it had, rather than being redrawn as a plain branch it is
/// not.
#[test]
fn a_moved_branch_leaves_its_row_and_arrives_on_the_target() {
    let badges = badges_with(
        &[
            ("refs/heads/main", oid(10)),
            ("refs/heads/feature", oid(21)),
            ("refs/remotes/origin/feature", oid(21)),
        ],
        &HeadState::Branch(ref_name("refs/heads/feature")),
        &[("refs/heads/feature", "refs/remotes/origin/feature")],
    );
    let mut drawing = plan();
    drawing.ghosts = vec![ghost(100, vec![Target::Real(oid(10))], "replay")];
    drawing.ref_moves = vec![RefMove {
        name: "refs/heads/feature".to_owned(),
        from: Some(oid(21)),
        to: Some(Target::Ghost(0)),
    }];

    let overlay = badges_for_preview(&badges, &drawing);

    assert!(
        overlay.on(oid(21)).is_empty(),
        "the badge must leave the row it is moving off"
    );
    let arrived = overlay.on(oid(100));
    assert_eq!(arrived.len(), 1);
    assert_eq!(
        arrived[0],
        RefLabel::Branch {
            name: b"feature".to_vec(),
            current: true,
            upstream: Upstream::InSync {
                remote: b"origin".to_vec(),
            },
        },
        "the chip travels with the branch"
    );
    assert_eq!(
        overlay.head(),
        Some(oid(100)),
        "HEAD is on feature, so it follows it onto the ghost"
    );
    assert_eq!(overlay.on(oid(10)), vec![branch("main", false)]);
}

/// A delete is a `RefMove` with nowhere to go, and needs no special case:
/// the badge leaves its row and arrives nowhere.
#[test]
fn a_deleted_branch_simply_leaves() {
    let badges = badges_with(
        &[("refs/heads/main", oid(10)), ("refs/heads/old", oid(10))],
        &HeadState::Branch(ref_name("refs/heads/main")),
        &[],
    );
    let mut drawing = plan();
    drawing.ref_moves = vec![RefMove {
        name: "refs/heads/old".to_owned(),
        from: Some(oid(10)),
        to: None,
    }];

    let overlay = badges_for_preview(&badges, &drawing);

    assert_eq!(overlay.on(oid(10)), vec![branch("main", true)]);
    assert_eq!(
        overlay.head(),
        Some(oid(10)),
        "HEAD is on main, which stays"
    );
}

/// A checkout moves `HEAD` without moving any ref.
#[test]
fn a_detaching_head_move_puts_the_mark_on_the_target() {
    let badges = badges_with(
        &[("refs/heads/main", oid(10))],
        &HeadState::Branch(ref_name("refs/heads/main")),
        &[],
    );
    let mut drawing = plan();
    drawing.head = Some(HeadMove::Detached(Target::Real(oid(20))));

    let overlay: BadgeOverlay<'_> = badges_for_preview(&badges, &drawing);

    assert_eq!(overlay.head(), Some(oid(20)));
    assert_eq!(
        overlay.on(oid(10)),
        vec![branch("main", true)],
        "the branch does not move because HEAD left it"
    );
}

/// `HEAD → <branch>` lands on wherever that branch is when the plan does
/// not move it.
#[test]
fn head_to_an_unmoved_branch_finds_where_it_already_is() {
    let badges = badges_with(
        &[("refs/heads/main", oid(10)), ("refs/heads/topic", oid(20))],
        &HeadState::Branch(ref_name("refs/heads/main")),
        &[],
    );
    let mut drawing = plan();
    drawing.head = Some(HeadMove::ToBranch("refs/heads/topic".to_owned()));

    let overlay = badges_for_preview(&badges, &drawing);

    assert_eq!(overlay.head(), Some(oid(20)));
}

#[test]
fn short_of_strips_the_ref_prefix_and_leaves_a_remote_shorthand_alone() {
    assert_eq!(short_of("refs/heads/feature/x"), b"feature/x".to_vec());
    assert_eq!(short_of("refs/tags/v1.0"), b"v1.0".to_vec());
    assert_eq!(
        short_of("refs/remotes/origin/main"),
        b"origin/main".to_vec()
    );
    assert_eq!(short_of("origin/main"), b"origin/main".to_vec());
}

/// Review 08 Minor 11. A tag and a branch can share a short name on one
/// commit; the overlay removed by short name alone, so moving the branch
/// took the tag's badge off the picture too — a ref the operation does not
/// touch, missing from the row it is standing on.
#[test]
fn moving_a_branch_leaves_a_tag_of_the_same_name_where_it_is() {
    let badges = badges_with(
        &[("refs/heads/v1", oid(21)), ("refs/tags/v1", oid(21))],
        &HeadState::Branch(ref_name("refs/heads/v1")),
        &[],
    );
    let mut drawing = plan();
    drawing.ghosts = vec![ghost(100, vec![Target::Real(oid(10))], "replay")];
    drawing.ref_moves = vec![RefMove {
        name: "refs/heads/v1".to_owned(),
        from: Some(oid(21)),
        to: Some(Target::Ghost(0)),
    }];

    let overlay = badges_for_preview(&badges, &drawing);

    assert_eq!(
        overlay.on(oid(21)),
        vec![RefLabel::Tag(b"v1".to_vec())],
        "the tag stays: the plan moves the branch, not it"
    );
    assert!(
        overlay
            .on(oid(100))
            .iter()
            .any(|label| matches!(label, RefLabel::Branch { name, .. } if name == b"v1")),
        "and the branch arrives on the ghost"
    );
}
