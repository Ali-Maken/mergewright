use graft_core::commit::{Commit, Identity, Timestamp};
use graft_core::commit_handle::CommitHandle;
use graft_core::oid::Oid;

use super::*;

fn oid(byte: u8) -> Oid {
    Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
}

fn commit(id: u8, parents: &[u8], subject: &[u8]) -> Commit {
    let when = Identity {
        name: b"Test".to_vec(),
        email: b"test@example.invalid".to_vec(),
        when: Timestamp {
            seconds: 0,
            offset_seconds: 0,
        },
    };
    Commit {
        id: oid(id),
        parents: parents.iter().map(|&byte| oid(byte)).collect(),
        author: when.clone(),
        committer: when,
        message: subject.to_vec(),
    }
}

/// Handle `n` is the commit `oid(n)`.
fn id_of(handle: CommitHandle) -> Oid {
    oid(u8::try_from(handle.0).expect("a test handle fits in u8"))
}

fn merge_shape(row: u32, merge: u32) -> RowShape {
    RowShape {
        commit: CommitHandle(row),
        lane: 0,
        incoming: Vec::new(),
        outgoing: vec![(0, CommitHandle(9))],
        lanes: vec![
            Some(LaneIdentity::Known(BranchIdentity::of_name(b"main"))),
            Some(LaneIdentity::Merge {
                merge: CommitHandle(merge),
                parent: CommitHandle(8),
            }),
        ],
    }
}

#[test]
fn a_merge_lane_missing_from_the_names_is_an_error_not_a_panic() {
    let shape = merge_shape(1, 7);
    let names = MergeNames::default();

    let assembled = Row::assemble(shape, commit(1, &[9], b"tip"), &names, &id_of);

    assert_eq!(
        assembled,
        Err(AssembleError::MergeNotGathered {
            merge: CommitHandle(7)
        })
    );
}

#[test]
fn a_merge_on_hand_is_named_without_a_read_and_one_above_the_batch_with_one() {
    let on_hand = merge_shape(7, 7);
    let above = merge_shape(1, 5);
    let commits = [
        commit(7, &[9, 8], b"Merge branch 'topic'"),
        commit(1, &[9], b"tip"),
    ];

    let names = MergeNames::gather(
        std::slice::from_ref(&on_hand),
        &commits[..1],
        |_| -> Result<Vec<Commit>, ()> { panic!("the merge was on hand") },
        id_of,
    )
    .expect("nothing to read");
    assert_eq!(names.reads(), 0);
    let row = Row::assemble(on_hand, commits[0].clone(), &names, &id_of).expect("gathered");
    assert_eq!(row.lanes[1], Some(BranchIdentity::of_name(b"topic")));
    assert_eq!(
        row.drawn_parents,
        vec![oid(9)],
        "drawn parents are the shape's"
    );
    assert_eq!(
        row.commit.parents,
        vec![oid(9), oid(8)],
        "the commit keeps its own"
    );
    assert!(row.is_merge());

    let names = MergeNames::gather(
        std::slice::from_ref(&above),
        &commits[1..],
        |asked| {
            assert_eq!(asked, [CommitHandle(5)]);
            Ok::<_, ()>(vec![commit(5, &[9, 8], b"no pattern here")])
        },
        id_of,
    )
    .expect("the read succeeds");
    assert_eq!(names.reads(), 1);
    let row = Row::assemble(above, commits[1].clone(), &names, &id_of).expect("gathered");
    assert_eq!(row.lanes[1], Some(BranchIdentity::of_short_sha(oid(8))));
}
