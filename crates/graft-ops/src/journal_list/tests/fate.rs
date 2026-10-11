//! m8/46: `fate_of`, the rule that tells a commit `u` brings back from one
//! that is really lost. Entries written by hand in the shapes Graft writes;
//! `tests/it/journal_fate.rs` proves the rule on real repositories.

use graft_core::journal::OperationKind;

use super::{entry, list, lost, oid, record, run};
use crate::journal_list::{Fate, ListRow, fate_of};

#[test]
fn a_moved_branch_replaces_its_old_tip() {
    let rebase = entry(
        10,
        OperationKind::Rebase,
        "rebase",
        &[("refs/heads/feature", 4, 5)],
    );
    assert_eq!(fate_of(Some((&rebase, true)), oid(4)), Fate::Replaced);
}

#[test]
fn a_moved_head_replaces_its_old_tip() {
    let reset = entry(10, OperationKind::Reset, "reset", &[("HEAD", 4, 3)]);
    assert_eq!(fate_of(Some((&reset, true)), oid(4)), Fate::Replaced);
}

#[test]
fn a_deleted_ref_reads_deleted() {
    let delete = entry(
        10,
        OperationKind::BranchDelete,
        "delete",
        &[("refs/heads/spike", 4, 0)],
    );
    assert_eq!(fate_of(Some((&delete, true)), oid(4)), Fate::Deleted);
}

#[test]
fn a_ref_not_in_the_after_state_is_unknown_and_stays_lost() {
    let mut rebase = entry(
        10,
        OperationKind::Rebase,
        "rebase",
        &[("refs/heads/feature", 4, 5)],
    );
    rebase.refs_after.clear();
    assert_eq!(fate_of(Some((&rebase, true)), oid(4)), Fate::Lost);
}

#[test]
fn an_entry_not_in_force_stays_lost() {
    let rebase = entry(
        10,
        OperationKind::Rebase,
        "rebase",
        &[("refs/heads/feature", 4, 5)],
    );
    assert_eq!(fate_of(Some((&rebase, false)), oid(4)), Fate::Lost);
}

#[test]
fn a_shell_event_stays_lost() {
    assert_eq!(fate_of(None, oid(4)), Fate::Lost);
}

#[test]
fn a_tip_no_before_ref_held_stays_lost() {
    let rebase = entry(
        10,
        OperationKind::Rebase,
        "rebase",
        &[("refs/heads/feature", 4, 5)],
    );
    assert_eq!(fate_of(Some((&rebase, true)), oid(9)), Fate::Lost);
}

#[test]
fn a_tag_is_not_a_ref_a_run_is_placed_by() {
    let delete = entry(
        10,
        OperationKind::TagDelete,
        "delete",
        &[("refs/tags/v1", 4, 0)],
    );
    assert_eq!(fate_of(Some((&delete, true)), oid(4)), Fate::Lost);
}

/// Fetch, push and migrate are transparent to `u`; undo and redo are moves
/// along the stack. `u` cannot take any of them back, so nothing they left
/// behind is "replaced", even where a branch ref moved off the tip.
#[test]
fn fetch_and_push_never_replace() {
    for kind in [
        OperationKind::Fetch,
        OperationKind::Push,
        OperationKind::Migrate,
        OperationKind::Undo,
        OperationKind::Redo,
    ] {
        let moved = entry(10, kind, "x", &[("refs/heads/main", 4, 5)]);
        assert_eq!(
            fate_of(Some((&moved, true)), oid(4)),
            Fate::Lost,
            "{kind:?}"
        );
        let deleted = entry(10, kind, "x", &[("refs/heads/main", 4, 0)]);
        assert_eq!(
            fate_of(Some((&deleted, true)), oid(4)),
            Fate::Lost,
            "{kind:?}"
        );
    }
}

/// A pull that moved the branch is undoable, so it replaces.
#[test]
fn a_pull_that_moved_the_branch_replaces() {
    let moved = entry(
        10,
        OperationKind::Pull,
        "pull",
        &[("refs/heads/main", 4, 5)],
    );
    assert_eq!(fate_of(Some((&moved, true)), oid(4)), Fate::Replaced);
}

/// Two of Graft's rebases in one second (a chain of dependent branches
/// rebased one after the other): each `HEAD` run pairs with the finish of
/// the branch it returns to, whatever order the branch records come in, so
/// `one`'s old tip sits under `one`'s rebase and `u` reaches it.
#[test]
fn two_rebases_in_one_second_keep_their_own_finish() {
    let rebase_one = entry(
        1010,
        OperationKind::Rebase,
        "rebase one",
        &[("refs/heads/one", 4, 5)],
    );
    let rebase_two = entry(
        1011,
        OperationKind::Rebase,
        "rebase two",
        &[("refs/heads/two", 6, 7)],
    );
    let outside = two_rebases_records();
    let found = lost(&[run(6, 0), run(4, 0)]);
    let built = list(
        &[rebase_one.clone(), rebase_two],
        &outside,
        Some(&found),
        &[],
    );

    let under = |tip: u8| {
        built.rows.iter().find_map(|row| match row {
            ListRow::Lost { event, commit } if commit.run.tip == oid(tip) => {
                Some((built.rows[*event].clone(), commit.fate))
            }
            _ => None,
        })
    };
    assert_eq!(
        under(4),
        Some((ListRow::Graft(rebase_one.id.clone()), Fate::Replaced)),
        "{built:#?}"
    );
    assert_eq!(
        under(6).map(|(_, fate)| fate),
        Some(Fate::Replaced),
        "{built:#?}"
    );
}

/// `HEAD`'s log and the two branches' finishes, newest first, all in one
/// second.
fn two_rebases_records() -> [super::OutsideRecord; 9] {
    [
        record(
            "HEAD",
            7,
            7,
            1010,
            "rebase (finish): returning to refs/heads/two",
        ),
        record("HEAD", 1, 7, 1010, "rebase (pick): work on two"),
        record("HEAD", 6, 1, 1010, "rebase (start): checkout main"),
        record("HEAD", 5, 6, 1010, "checkout: moving from one to two"),
        record(
            "HEAD",
            5,
            5,
            1010,
            "rebase (finish): returning to refs/heads/one",
        ),
        record("HEAD", 1, 5, 1010, "rebase (pick): work on one"),
        record("HEAD", 4, 1, 1010, "rebase (start): checkout main"),
        // The other branch's finish nearer the newer run than its own.
        record(
            "refs/heads/one",
            4,
            5,
            1010,
            "rebase (finish): refs/heads/one onto 0101",
        ),
        record(
            "refs/heads/two",
            6,
            7,
            1010,
            "rebase (finish): refs/heads/two onto 0101",
        ),
    ]
}
