//! `u` then `U` over a stash pop that stopped on a conflict: the stash list
//! is read from `git stash list` after each step, never from a returned value
//! (m6-conflicts/07, F2).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_core::oid::Oid;
use graft_ops::apply::ApplyError;
use graft_ops::operation::Operation;

use crate::support::{NOW, TempRepo, run, run_redo, run_undo, try_run};
use crate::unmerged_index::about_to_pop_a_conflict;

/// The stash list, newest first, as commit ids.
fn stash_commits(repo: &TempRepo) -> Vec<String> {
    repo.git(&["stash", "list", "--format=%H"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The stash list after each step of the walk.
struct Lists {
    popped: Vec<String>,
    undone: Vec<String>,
    redone: Vec<String>,
}

/// Pop `stash@{index}` into a conflict, then `u`, then `U`.
fn pop_conflict_undo_redo(repo: &TempRepo, index: usize, commit: Oid) -> Lists {
    let failed = try_run(repo, &Operation::StashPop { index, commit }, NOW);
    assert!(
        matches!(failed, Err(ApplyError::Clashed { .. })),
        "the pop should have conflicted: {failed:?}"
    );
    let popped = stash_commits(repo);
    run_undo(repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    let undone = stash_commits(repo);
    run_redo(repo, NOW + 2, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        repo.git(&["ls-files", "-u"]).lines().count(),
        3,
        "the redo put the conflict back"
    );
    Lists {
        popped,
        undone,
        redone: stash_commits(repo),
    }
}

/// A pop that stopped on a conflict never dropped its stash — git drops only
/// after a clean apply — so neither `u` nor `U` may take it off the list.
#[test]
fn redo_of_a_conflicted_pop_keeps_the_stash_git_kept() {
    let (repo, stashed) = about_to_pop_a_conflict();
    let kept = vec![stashed.to_string()];

    let lists = pop_conflict_undo_redo(&repo, 0, stashed);

    assert_eq!(
        lists.popped, kept,
        "git kept the stash when the pop conflicted"
    );
    assert_eq!(
        lists.undone, kept,
        "the undo stored nothing: it was never gone"
    );
    assert_eq!(lists.redone, kept, "so the redo drops nothing");
}

/// The same for a stash below the top of the list, where `refs/stash` does
/// not move whether a pop drops it or not: the redo cannot read the answer
/// off the popped ref.
#[test]
fn redo_of_a_conflicted_pop_below_the_top_keeps_both_stashes() {
    let (repo, stashed) = about_to_pop_a_conflict();
    repo.write("other.txt", b"stashed on top\n");
    repo.git(&["stash", "push", "-q", "-u", "-m", "on top"]);
    let on_top = repo.git(&["rev-parse", "stash@{0}"]);
    let both = vec![on_top, stashed.to_string()];

    let lists = pop_conflict_undo_redo(&repo, 1, stashed);

    assert_eq!(
        lists.popped, both,
        "git kept stash@{{1}} when the pop conflicted"
    );
    assert_eq!(lists.undone, both, "the undo stored nothing");
    assert_eq!(lists.redone, both, "and the redo drops nothing");
}

/// The other side: a clean pop of `stash@{1}` did drop it, so `u` stores it
/// back and `U` drops it again — even though the pop never moved
/// `refs/stash`.
#[test]
fn redo_of_a_clean_pop_below_the_top_drops_it_again() {
    let repo = TempRepo::init();
    repo.write("p.txt", b"base\n");
    repo.commit_all("base");
    repo.write("p.txt", b"stashed\n");
    repo.git(&["stash", "push", "-q", "-m", "below"]);
    let below = repo.git(&["rev-parse", "stash@{0}"]);
    repo.write("other.txt", b"stashed on top\n");
    repo.git(&["stash", "push", "-q", "-u", "-m", "on top"]);
    let on_top = repo.git(&["rev-parse", "stash@{0}"]);
    let commit = Oid::from_hex(&below).expect("the stash commit reads");

    run(&repo, &Operation::StashPop { index: 1, commit }, NOW);
    assert_eq!(
        stash_commits(&repo),
        std::slice::from_ref(&on_top),
        "the pop dropped it"
    );
    assert_eq!(repo.read("p.txt"), b"stashed\n");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(
        stash_commits(&repo),
        [below, on_top.clone()],
        "the undo stored it back, on top"
    );

    run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{}", why.explain()));
    assert_eq!(stash_commits(&repo), [on_top], "the redo dropped it again");
    assert_eq!(repo.read("p.txt"), b"stashed\n", "and applied it again");
}
