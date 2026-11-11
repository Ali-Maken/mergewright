//! J1's table, one test per row.

use super::{record, with_back};
use crate::journal_list::words;

fn said(refname: &str, old: u8, new: u8, subject: &str, back: Option<usize>) -> String {
    let outside = record(refname, old, new, 1000, subject);
    words(&outside.record, back)
}

#[test]
fn a_commit_is_committed_on_its_branch() {
    assert_eq!(
        said("refs/heads/main", 1, 2, "commit: Add rate limiter", None),
        r#"committed "Add rate limiter" on main"#
    );
}

#[test]
fn an_amend_is_amended_on_its_branch() {
    assert_eq!(
        said("refs/heads/main", 1, 2, "commit (amend): Fix typo", None),
        r#"amended "Fix typo" on main"#
    );
}

#[test]
fn a_reset_back_along_the_branch_counts_commits() {
    let outside = with_back(
        record("refs/heads/main", 4, 3, 1000, "reset: moving to HEAD~1"),
        1,
    );
    assert_eq!(
        words(&outside.record, outside.back),
        "reset main 1 commit back"
    );
    assert_eq!(
        said("refs/heads/main", 4, 2, "reset: moving to HEAD~2", Some(2)),
        "reset main 2 commits back"
    );
}

#[test]
fn a_reset_elsewhere_names_the_target() {
    assert_eq!(
        said("refs/heads/main", 4, 9, "reset: moving to 0909090", None),
        "moved main to 0909090"
    );
}

#[test]
fn a_checkout_is_a_switch_between_names() {
    assert_eq!(
        said("HEAD", 4, 5, "checkout: moving from spike to main", None),
        "switch from spike to main"
    );
    // A detached HEAD's full id is cut to seven.
    let from = "ab".repeat(20);
    assert_eq!(
        said(
            "HEAD",
            4,
            5,
            &format!("checkout: moving from {from} to main"),
            None
        ),
        "switch from abababa to main"
    );
}

#[test]
fn a_fast_forward_counts_commits() {
    assert_eq!(
        said("refs/heads/main", 2, 4, "merge feat: Fast-forward", Some(2)),
        "fast-forwarded main 2 commits"
    );
    assert_eq!(
        said("refs/heads/main", 2, 4, "pull: Fast-forward", Some(1)),
        "fast-forwarded main 1 commit"
    );
    assert_eq!(
        said("refs/heads/main", 2, 4, "pull: Fast-forward", None),
        "fast-forwarded main to 0404040"
    );
}

#[test]
fn a_new_branch_is_made_at_its_commit() {
    assert_eq!(
        said("refs/heads/spike", 0, 4, "branch: Created from HEAD", None),
        "made spike at 0404040"
    );
}

#[test]
fn a_rebase_finish_names_its_upstream() {
    let onto = "06".repeat(20);
    // The branch's record carries only the id; with no name, the id's seven.
    assert_eq!(
        said(
            "refs/heads/feat",
            3,
            4,
            &format!("rebase (finish): refs/heads/feat onto {onto}"),
            None
        ),
        "rebased feat onto 0606060"
    );
}

#[test]
fn a_cherry_pick_is_taken_onto_the_branch() {
    assert_eq!(
        said(
            "refs/heads/main",
            3,
            4,
            "cherry-pick: Try redis cache",
            None
        ),
        r#"took "Try redis cache" onto main"#
    );
}

/// 22b's question: the first commit's record reads as any commit does
/// (m7/24 "Decided", rows).
#[test]
fn the_first_commit_reads_committed() {
    assert_eq!(
        said(
            "refs/heads/main",
            0,
            4,
            "commit (initial): Start the api",
            None
        ),
        r#"committed "Start the api" on main"#
    );
}

#[test]
fn an_unknown_subject_is_gits_own() {
    assert_eq!(
        said("refs/heads/main", 3, 4, "commit (merge): Merge spike", None),
        "commit (merge): Merge spike"
    );
    assert_eq!(
        said("refs/heads/main", 3, 4, "other undo", None),
        "other undo"
    );
}
