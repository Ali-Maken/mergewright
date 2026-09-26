//! Absorb and what it leaves staged (06b, review m7-06 C1, M2, M3, M6): a
//! hunk that fits no commit **stays staged** (the note's promise, :39,
//! :560), including one only the index holds; a staged rename stays whole;
//! a fixup that empties its target drops it; and the working tree edited on
//! past what was staged comes back without conflict markers. Each run is
//! undone after, and the repository must be exactly as it was.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_ops::absorb::{StaysWhy, absorb_map};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Operation, Tip};
use graft_ops::rewrite::{RewriteParams, RewriteRequest, TodoAction, plan};

use crate::absorb::{branch, oid, stage};
use crate::support::{
    NOW, Never, RepoState, TempRepo, git_version, run_undo, stash_list, state_of, try_run,
};

/// `b.txt`'s third line, staged: `f2` made it, so it is placed into `f2`.
fn stage_placed(repo: &TempRepo) {
    stage(repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
}

fn planned(repo: &TempRepo) -> RewriteParams {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(
        &handle,
        &git_version(),
        &RewriteRequest::Absorb,
        &facts,
        None,
        &Never,
    )
    .unwrap_or_else(|why| panic!("it plans: {why}"))
}

/// Plan and run absorb; it must go through (no clash, no stop).
fn absorb(repo: &TempRepo) -> RewriteParams {
    let params = planned(repo);
    try_run(repo, &Operation::Rewrite(params.clone()), NOW)
        .unwrap_or_else(|why| panic!("absorb goes through: {}", why.explain()));
    params
}

/// `u` puts every ref, the index and every file back.
fn undo_restores(repo: &TempRepo, before: &RepoState) {
    run_undo(repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(&state_of(repo), before, "undo put everything back");
}

fn subjects(repo: &TempRepo) -> Vec<String> {
    repo.git(&["log", "--format=%s", "main..feature"])
        .lines()
        .map(str::to_owned)
        .collect()
}

/// A placed hunk went into `f2`: its new commit adds `B3`.
fn f2_carries_b3(repo: &TempRepo) {
    let back = subjects(repo)
        .iter()
        .position(|subject| subject == "f2")
        .expect("f2 is still there");
    let shown = repo.git(&[
        "show",
        "--format=",
        &format!("feature~{back}"),
        "--",
        "b.txt",
    ]);
    assert!(shown.lines().any(|line| line == "+B3"), "{shown}");
}

// ── C1: a stay that only the index holds ────────────────────────────────

/// `old.txt`'s `O3` staged, then taken back out of the working tree — as
/// `add -p` and an editor's undo leave it. Only the index has it.
fn index_only_stay(repo: &TempRepo) {
    stage(repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    repo.write("old.txt", b"o1\no2\no3\no4\no5\n");
}

#[test]
fn an_index_only_stay_survives_absorb() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);

    absorb(&repo);

    f2_carries_b3(&repo);
    assert_eq!(
        repo.git(&["show", ":old.txt"]),
        "o1\no2\nO3\no4\no5",
        "the staged line is still staged"
    );
    assert_eq!(repo.read("old.txt"), b"o1\no2\no3\no4\no5\n");
    assert_eq!(stash_list(&repo), "", "the set-aside copy was dropped");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1", "--", "old.txt"]),
        "MM old.txt"
    );
}

#[test]
fn undo_after_absorb_with_an_index_only_stay() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);
    let before = state_of(&repo);

    absorb(&repo);

    undo_restores(&repo, &before);
}

/// `old.txt` made executable in the index alone (`update-index --chmod`):
/// the working tree's mode never changes.
fn index_only_mode(repo: &TempRepo) {
    repo.git(&["update-index", "--chmod=+x", "--", "old.txt"]);
}

#[test]
fn an_index_only_mode_change_survives_absorb() {
    let repo = branch();
    stage_placed(&repo);
    index_only_mode(&repo);
    assert!(
        map_stays(&repo).contains(&StaysWhy::NoCommitChangedIt),
        "the mode change stays"
    );

    absorb(&repo);

    f2_carries_b3(&repo);
    let listed = repo.git(&["ls-files", "-s", "--", "old.txt"]);
    assert!(listed.starts_with("100755 "), "still staged: {listed}");
    assert_eq!(stash_list(&repo), "");
}

#[test]
fn undo_after_absorb_with_an_index_only_mode_change() {
    let repo = branch();
    stage_placed(&repo);
    index_only_mode(&repo);
    let before = state_of(&repo);

    absorb(&repo);

    undo_restores(&repo, &before);
}

/// The ruling on review Question 1: a stay that the working tree holds too
/// is still staged afterwards, not merely back as an edit.
#[test]
fn a_stay_stays_staged() {
    let repo = branch();
    stage_placed(&repo);
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    let index_before = repo.git(&["write-tree"]);

    absorb(&repo);

    assert_eq!(
        repo.git(&["status", "--porcelain=v1", "--", "old.txt"]),
        "M  old.txt"
    );
    assert_eq!(
        repo.git(&["write-tree"]),
        index_before,
        "the index is what it was: the stays on the new HEAD"
    );
}

/// The preview's set-aside line: absorb loses no staging, so none is
/// counted as coming back unstaged.
#[test]
fn absorb_sets_aside_with_no_staging_lost() {
    let repo = branch();
    stage_placed(&repo);
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    let params = planned(&repo);
    let facts = read_facts(&repo.open(), git_version(), &Never, &FactsScope::default())
        .expect("the facts read");

    let aside = Operation::Rewrite(params)
        .sets_aside(&facts)
        .expect("two staged files are set aside");

    assert_eq!((aside.files, aside.staged), (2, 0));
}

fn map_stays(repo: &TempRepo) -> Vec<StaysWhy> {
    absorb_map(
        &repo.open(),
        &git_version(),
        &Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(repo, "feature"),
        },
        oid(repo, "main"),
        &Never,
    )
    .expect("the map reads")
    .stays
    .into_iter()
    .map(|(_, why)| why)
    .collect()
}

// ── M2: a staged rename ─────────────────────────────────────────────────

/// `b.txt` (all `f2`'s) renamed, and `a.txt`'s `A2` staged beside it for
/// something to place.
fn staged_rename(repo: &TempRepo) {
    repo.git(&["mv", "b.txt", "moved.txt"]);
    stage(repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");
}

#[test]
fn a_staged_rename_stays_whole() {
    let repo = branch();
    staged_rename(&repo);
    let map = absorb_map(
        &repo.open(),
        &git_version(),
        &Tip {
            name: "refs/heads/feature".to_owned(),
            oid: oid(&repo, "feature"),
        },
        oid(&repo, "main"),
        &Never,
    )
    .expect("the map reads");
    assert!(
        map.placed
            .iter()
            .all(|(hunk, _)| hunk.path.to_string_lossy() == "a.txt"),
        "no side of the rename is placed: {map:?}"
    );
    assert!(
        map.stays.iter().any(|(_, why)| *why == StaysWhy::Renamed),
        "{map:?}"
    );
    let index_before = repo.git(&["write-tree"]);

    absorb(&repo);

    assert_eq!(
        repo.git(&["show", "--format=", "--name-only", "feature~1"]),
        "b.txt",
        "f2 still makes b.txt"
    );
    assert_eq!(repo.git(&["write-tree"]), index_before, "still staged");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        "R  b.txt -> moved.txt"
    );
}

#[test]
fn undo_after_absorb_with_a_staged_rename() {
    let repo = branch();
    staged_rename(&repo);
    let before = state_of(&repo);

    absorb(&repo);

    undo_restores(&repo, &before);
}

// ── M3: a fixup that empties its target ─────────────────────────────────

/// `f4` only adds `DEBUG` inside `b.txt`; he stages taking it out again.
fn debug_line_taken_out(repo: &TempRepo) {
    repo.write("b.txt", b"b1\nb2\nDEBUG\nb3\nb4\nb5\n");
    repo.commit_all("f4");
    stage(repo, "b.txt", b"b1\nb2\nb3\nb4\nb5\n");
}

#[test]
fn a_fixup_that_empties_its_target_drops_it() {
    let repo = branch();
    debug_line_taken_out(&repo);
    let f4 = oid(&repo, "feature");

    let params = absorb(&repo);

    assert!(
        params
            .todo
            .iter()
            .any(|line| line.commit == f4 && line.action == TodoAction::Drop),
        "f4 is planned as a drop: {:?}",
        params.todo
    );
    assert!(
        params
            .todo
            .iter()
            .all(|line| line.action != TodoAction::Fixup),
        "nothing melds into a dropped commit"
    );
    assert_eq!(subjects(&repo), vec!["f3", "f2", "f1"]);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "");
    assert!(
        !repo.path().join(".git/rebase-merge").exists(),
        "no rebase left stopped"
    );
}

/// The review's own probe: the emptied commit has another on top of it.
#[test]
fn a_fixup_that_empties_a_commit_below_the_top_drops_it() {
    let repo = branch();
    repo.write("b.txt", b"b1\nb2\nDEBUG\nb3\nb4\nb5\n");
    repo.commit_all("f4");
    repo.write("c.txt", b"c\n");
    repo.commit_all("f5");
    stage(&repo, "b.txt", b"b1\nb2\nb3\nb4\nb5\n");

    absorb(&repo);

    assert_eq!(subjects(&repo), vec!["f5", "f3", "f2", "f1"]);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "");
}

#[test]
fn undo_after_absorb_that_drops_its_target() {
    let repo = branch();
    debug_line_taken_out(&repo);
    let before = state_of(&repo);

    absorb(&repo);

    undo_restores(&repo, &before);
}

// ── M6: `add -p`, keep editing, absorb ──────────────────────────────────

/// `B3` staged, and the working tree edited on past it.
fn kept_editing(repo: &TempRepo) {
    stage_placed(repo);
    repo.write("b.txt", b"b1\nb2\nB3 and more\nb4\nb5\n");
}

#[test]
fn absorb_after_editing_on_leaves_no_markers() {
    let repo = branch();
    kept_editing(&repo);

    absorb(&repo);

    f2_carries_b3(&repo);
    assert_eq!(repo.read("b.txt"), b"b1\nb2\nB3 and more\nb4\nb5\n");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        " M b.txt",
        "the rest of the edit is back, unstaged, nothing unmerged"
    );
    assert_eq!(stash_list(&repo), "");
}

#[test]
fn undo_after_absorb_after_editing_on() {
    let repo = branch();
    kept_editing(&repo);
    let before = state_of(&repo);

    absorb(&repo);

    undo_restores(&repo, &before);
}
