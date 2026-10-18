//! m7/38: a finished rebase's in-between commits are not lost work, and a
//! lost commit names a branch only when that branch's own log held it.
//! Record shapes copied from the oracle (`journal_list_oracle.rs`,
//! `a_finished_squash_leaves_no_lost_row`, git 2.54.0).

use std::collections::{BTreeMap, HashSet};

use mergewright_core::journal::HeadRef;
use mergewright_git::status::WorkingTreeStatus;
use mergewright_git::version::GitVersion;

use super::{list, lost, oid, record, run};
use crate::journal_list::scratch::rebase_scratch;
use crate::journal_list::{HeldBy, JournalList, ListRow, OutsideRecord, Recover, recover_for};
use crate::operation::RepoFacts;

fn scratch(outside: &[OutsideRecord]) -> HashSet<u8> {
    rebase_scratch(outside.iter().map(|outside| &outside.record))
        .into_iter()
        .map(|oid| oid.as_bytes()[0])
        .collect()
}

fn set(bytes: &[u8]) -> HashSet<u8> {
    bytes.iter().copied().collect()
}

/// `rebase -i` squashing 3 → 1, finished: the pick's copy (5) and the first
/// combination (6) are scratch; the result (7) and the branch's old tip (4)
/// are named by the finish and start records.
fn finished_squash() -> [OutsideRecord; 6] {
    [
        record(
            "HEAD",
            7,
            7,
            1010,
            "rebase -i (finish): returning to refs/heads/feat",
        ),
        record("HEAD", 6, 7, 1010, "rebase -i (squash): D"),
        record(
            "HEAD",
            5,
            6,
            1010,
            "rebase -i (squash): # This is a combination of 2 commits.",
        ),
        record("HEAD", 1, 5, 1010, "rebase -i (pick): B"),
        record("HEAD", 4, 1, 1010, "rebase -i (start): checkout HEAD~3"),
        record(
            "refs/heads/feat",
            4,
            7,
            1010,
            "rebase -i (finish): refs/heads/feat onto 0101",
        ),
    ]
}

/// An `edit` stop with a commit (8) and a `reset` back to 5 inside a
/// finished span.
fn edit_stop_with_a_reset() -> [OutsideRecord; 6] {
    [
        record(
            "HEAD",
            6,
            6,
            1010,
            "rebase (finish): returning to refs/heads/feat",
        ),
        record("HEAD", 5, 6, 1010, "rebase (pick): D"),
        record("HEAD", 8, 5, 1010, "reset: moving to HEAD~1"),
        record("HEAD", 5, 8, 1010, "commit: E"),
        record("HEAD", 1, 5, 1010, "rebase (edit): C"),
        record("HEAD", 4, 1, 1010, "rebase (start): checkout HEAD~2"),
    ]
}

/// Each case: `HEAD`'s log newest first, plus any branch record, and the
/// commits that are scratch.
#[test]
fn rebase_scratch_table() {
    let squash = finished_squash();
    assert_eq!(scratch(&squash), set(&[5, 6]), "the -i form, finished");

    // The same in today's form (`rebase (…)`).
    let plain: Vec<_> = squash
        .iter()
        .map(|outside| {
            let mut outside = outside.clone();
            outside.record.subject = outside.record.subject.replace("rebase -i (", "rebase (");
            outside
        })
        .collect();
    assert_eq!(scratch(&plain), set(&[5, 6]), "the plain form, finished");

    // Aborted: nothing is scratch, his work may be in it.
    let aborted = [
        record(
            "HEAD",
            6,
            4,
            1010,
            "rebase (abort): returning to refs/heads/feat",
        ),
        record("HEAD", 5, 6, 1010, "rebase (squash): C"),
        record("HEAD", 1, 5, 1010, "rebase (pick): B"),
        record("HEAD", 4, 1, 1010, "rebase (start): checkout side"),
    ];
    assert_eq!(scratch(&aborted), set(&[]), "aborted");

    // Unfinished (the log ends mid-span): nothing.
    let unfinished = &squash[1..5];
    assert_eq!(scratch(unfinished), set(&[]), "unfinished");

    // A second `(start)` closes the first span unfinished.
    let restarted = [
        record(
            "HEAD",
            9,
            9,
            1020,
            "rebase (finish): returning to refs/heads/feat",
        ),
        record("HEAD", 8, 9, 1020, "rebase (pick): E"),
        record("HEAD", 6, 8, 1020, "rebase (start): checkout main"),
        record("HEAD", 5, 6, 1010, "rebase (squash): C"),
        record("HEAD", 1, 5, 1010, "rebase (pick): B"),
        record("HEAD", 4, 1, 1010, "rebase (start): checkout side"),
    ];
    assert_eq!(
        scratch(&restarted),
        set(&[]),
        "the first span never finished"
    );

    // The span stays open over an `edit` stop's commit and reset; the
    // reset-away commit (8) and the one reset to (5) are named by non-step
    // records, so neither is scratch.
    assert_eq!(
        scratch(&edit_stop_with_a_reset()),
        set(&[]),
        "edit stop with a reset"
    );

    // Step records with no span open (its start beyond the read cap) are
    // not inside a span.
    let no_start = &squash[..4];
    assert_eq!(scratch(no_start), set(&[]), "no start in the log");
}

/// A branch record naming a scratch commit keeps it.
#[test]
fn rebase_scratch_table_another_log_names_it() {
    let mut held = finished_squash().to_vec();
    held.push(record(
        "refs/heads/spike",
        0,
        6,
        1011,
        "branch: Created from HEAD",
    ));
    assert_eq!(scratch(&held), set(&[5]), "a branch record names 6");
}

/// The usability walk's case (m7 usability M5): Mergewright's `R` on `feature-b`
/// leaves a step's commit (6) nothing holds; `HEAD`'s records infer
/// `feature-b`, which exists, but `feature-b`'s own log never had 6.
fn squash_on_feature_b(older: &[OutsideRecord]) -> JournalList {
    let mut outside = vec![
        record(
            "HEAD",
            7,
            7,
            1010,
            "rebase (finish): returning to refs/heads/feature-b",
        ),
        record("HEAD", 6, 7, 1010, "rebase (squash): D"),
        record("HEAD", 1, 6, 1010, "rebase (pick): B"),
        record("HEAD", 4, 1, 1010, "rebase (start): checkout HEAD~3"),
        record(
            "refs/heads/feature-b",
            4,
            7,
            1010,
            "rebase (finish): refs/heads/feature-b onto 0101",
        ),
        record(
            "HEAD",
            2,
            4,
            1005,
            "checkout: moving from main to feature-b",
        ),
    ];
    outside.extend_from_slice(older);
    let found = lost(&[run(6, 0)]);
    list(&[], &outside, Some(&found), &[])
}

fn held_by(list: &JournalList) -> Vec<&HeldBy> {
    list.rows
        .iter()
        .filter_map(|row| match row {
            ListRow::Lost { commit, .. } => Some(&commit.held_by),
            _ => None,
        })
        .collect()
}

#[test]
fn a_lost_commit_names_no_branch_whose_reflog_lacks_it() {
    let built = squash_on_feature_b(&[]);

    assert_eq!(held_by(&built), [&HeldBy::Head], "{built:?}");
}

/// An inferred branch that is gone: its log went with it, so today's rule
/// stands (as oracle `a_deleted_branch_once_checked_out_is_found_from_head`).
/// Here `HEAD`'s reset on `spike` (inferred, `Hint::On`) lost 6.
#[test]
fn a_gone_inferred_branch_still_names_itself() {
    let outside = [
        record("HEAD", 2, 1, 1013, "checkout: moving from spike to main"),
        record("HEAD", 6, 2, 1012, "reset: moving to HEAD~1"),
        record("HEAD", 2, 6, 1011, "commit: Try"),
        record("HEAD", 1, 2, 1010, "checkout: moving from main to spike"),
    ];
    let found = lost(&[run(6, 0)]);

    let built = list(&[], &outside, Some(&found), &["spike"]);

    assert_eq!(
        held_by(&built),
        [&HeldBy::Branch {
            name: "spike".to_owned(),
            gone: true
        }],
        "{built:?}"
    );
}

/// The same, but `feature-b`'s own log once had 6: it still names it.
#[test]
fn an_inferred_branch_whose_log_has_it_names_it() {
    let built = squash_on_feature_b(&[record(
        "refs/heads/feature-b",
        4,
        6,
        1003,
        "reset: moving to 0606",
    )]);

    assert_eq!(
        held_by(&built),
        [&HeldBy::Branch {
            name: "feature-b".to_owned(),
            gone: false
        }],
        "{built:?}"
    );
}

/// `⏎` on a row this change turned `Head` gets what a `Head` row gets
/// today (`recover.rs` `lost_commit`): a new branch at it, offered by name.
/// Before m7/38 the same row put `feature-b` back onto a commit it never
/// had.
#[test]
fn a_head_row_still_recovers() {
    let built = squash_on_feature_b(&[]);
    let row = built
        .rows
        .iter()
        .position(|row| matches!(row, ListRow::Lost { .. }))
        .expect("a lost row");
    let facts = RepoFacts {
        bare: false,
        head: HeadRef::Branch("refs/heads/feature-b".to_owned()),
        in_progress: None,
        status: WorkingTreeStatus { entries: vec![] },
        stashes: vec![],
        git: GitVersion {
            major: 2,
            minor: 54,
            patch: 0,
        },
        remotes: vec![],
        upstream: None,
        refs: BTreeMap::from([("refs/heads/feature-b".to_owned(), Some(oid(7)))]),
        markers: vec![],
        incoming: None,
        edit_stop: None,
    };

    assert_eq!(
        recover_for(&built, row, &[], &facts),
        Ok(Recover::OfferBranch {
            at: oid(6),
            name: "restored".to_owned(),
            why: "no branch held it, only HEAD".to_owned(),
        })
    );
}

/// Today's `reset --keep` case: the branch's own record names its branch.
#[test]
fn a_branch_reflog_record_still_names_its_branch() {
    let outside = [
        record("HEAD", 4, 3, 1005, "reset: moving to HEAD~1"),
        record("refs/heads/main", 4, 3, 1005, "reset: moving to HEAD~1"),
    ];
    let found = lost(&[run(4, 0)]);

    let built = list(&[], &outside, Some(&found), &[]);

    assert_eq!(
        held_by(&built),
        [&HeldBy::Branch {
            name: "main".to_owned(),
            gone: false
        }]
    );
    assert_eq!(oid(4), found.lost[0].tip);
}
