//! The undo gate (m6-conflicts/57, decision 3): while git has an operation
//! in progress, `u`, `U` and each step of a revert refuse
//! `NotInvertible::OperationLive` — nothing written, no drift box — unless
//! the entry being reversed was recorded **inside that same merge**. That
//! one exception is keymap §2.6a's: in a merge begun in the shell, the
//! steps Graft journaled inside it (a whole-file choice, the abort put back)
//! are `u`'s to walk. Ops owns the rule; the app only words it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_git::state::InProgress;
use graft_ops::operation::{BranchCreateParams, CommitParams, Operation};
use graft_ops::undo::{self, NotInvertible, UndoError};

use crate::abort_merge::{entries, merge_files, merging, oid, two_files};
use crate::conflict_ops::journal_ids;
use crate::support::{NOW, Never, TempRepo, context, journal_for, run, run_redo, run_undo};

/// A commit made through Graft on `main`, so the journal has an entry from
/// before the merge.
fn a_graft_commit(repo: &TempRepo, at: i64) {
    repo.write("extra.txt", b"an unrelated file\n");
    repo.git(&["add", "--", "extra.txt"]);
    run(
        repo,
        &Operation::Commit(CommitParams {
            message: "add extra".to_owned(),
            sign_off: false,
            no_verify: false,
            allow_empty: false,
        }),
        at,
    );
}

/// Everything a refused undo must leave exactly as it was.
#[derive(Debug, PartialEq, Eq)]
struct Untouched {
    journal: usize,
    head: String,
    stages: String,
    merge_files: Vec<(&'static str, Option<Vec<u8>>)>,
    status: String,
}

fn untouched(repo: &TempRepo) -> Untouched {
    Untouched {
        journal: entries(repo).len(),
        head: repo.git(&["rev-parse", "HEAD"]),
        stages: repo.git(&["ls-files", "--stage"]),
        merge_files: merge_files(repo),
        status: repo.git(&["status", "--porcelain=v2"]),
    }
}

/// A commit, then a merge begun in the shell stops: `u` would take back
/// the commit from under the merge. Refused by name, before the drift rail
/// — no `a merge is in progress now` box to force past.
#[test]
fn undo_refuses_an_entry_from_before_a_live_merge() {
    let repo = two_files();
    a_graft_commit(&repo, NOW);
    assert_eq!(journal_ids(&repo).len(), 1, "the commit is journaled");
    assert!(
        repo.try_git(&["merge", "--no-edit", "feature"]).is_err(),
        "the merge is meant to stop"
    );
    assert!(merging(&repo));
    let before = untouched(&repo);

    for force in [false, true] {
        let refused = run_undo(&repo, NOW + 1, force).expect_err("refused");
        assert!(
            matches!(
                refused,
                UndoError::Nothing(NotInvertible::OperationLive {
                    kind: InProgress::Merge
                })
            ),
            "force {force}: {}",
            refused.explain(&crate::support::Letters)
        );
        assert_eq!(
            refused.explain(&crate::support::Letters),
            "finish or abort the merge first",
            "ops' own words"
        );
        assert_eq!(untouched(&repo), before, "force {force}: nothing written");
    }
    assert_eq!(oid(&repo, "MERGE_HEAD"), oid(&repo, "feature"));
}

/// A branch created through Graft: an entry that touches one ref and no
/// file, so its undo and redo need nothing clean.
fn a_graft_branch(repo: &TempRepo, at: i64) {
    let at_commit = oid(repo, "HEAD");
    run(
        repo,
        &Operation::BranchCreate(BranchCreateParams {
            name: "topic".to_owned(),
            at: at_commit,
            checkout: false,
            on_edits: graft_ops::operation::OnEdits::Refuse,
        }),
        at,
    );
}

fn has_topic(repo: &TempRepo) -> bool {
    repo.try_git(&["rev-parse", "-q", "--verify", "refs/heads/topic"])
        .is_ok()
}

/// `U` reaches the gate too: a branch created and undone, then a rebase
/// begun in the shell stops — the redo would put `topic` back under the
/// rebase. Refused by name, nothing written.
#[test]
fn redo_refuses_while_a_rebase_is_live() {
    let repo = two_files();
    a_graft_branch(&repo, NOW);
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert!(!has_topic(&repo), "the undo took topic back");
    assert!(
        repo.try_git(&["rebase", "feature"]).is_err(),
        "the rebase is meant to stop"
    );
    assert_eq!(
        graft_git::state::in_progress(&repo.open()).expect("the state reads"),
        Some(InProgress::Rebase)
    );
    let before = untouched(&repo);

    for force in [false, true] {
        let refused = run_redo(&repo, NOW + 2, force).expect_err("refused");
        assert!(
            matches!(
                refused,
                UndoError::Nothing(NotInvertible::OperationLive {
                    kind: InProgress::Rebase
                })
            ),
            "force {force}: {}",
            refused.explain(&crate::support::Letters)
        );
        assert_eq!(
            refused.explain(&crate::support::Letters),
            "finish or abort the rebase first"
        );
        assert_eq!(untouched(&repo), before, "force {force}: nothing written");
    }
    assert!(!has_topic(&repo), "topic stays gone");
}

/// Each step of a revert reaches the gate: two entries from before a merge
/// begun in the shell, and a revert to the first stops at its first step
/// with nothing done and nothing written.
#[test]
fn revert_to_refuses_past_a_live_merge() {
    let repo = two_files();
    a_graft_branch(&repo, NOW);
    a_graft_commit(&repo, NOW + 1);
    let first = entries(&repo)[0].id.clone();
    assert!(
        repo.try_git(&["merge", "--no-edit", "feature"]).is_err(),
        "the merge is meant to stop"
    );
    let before = untouched(&repo);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 2, &never);
    let refused = undo::revert_to(&mut ctx, &first, true).expect_err("refused");
    match refused {
        UndoError::PartWay { done, error } => {
            assert!(done.is_empty(), "no step ran: {} did", done.len());
            assert!(
                matches!(
                    *error,
                    UndoError::Nothing(NotInvertible::OperationLive {
                        kind: InProgress::Merge
                    })
                ),
                "{}",
                error.explain(&crate::support::Letters)
            );
        }
        other => panic!(
            "expected the gate at the first step, got {}",
            other.explain(&crate::support::Letters)
        ),
    }
    assert_eq!(untouched(&repo), before, "nothing written");
    assert!(has_topic(&repo), "topic is still there");
}
