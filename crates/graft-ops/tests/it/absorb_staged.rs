//! Absorb and what it leaves staged (06b, review m7-06 C1, M2, M3, M6): a
//! hunk that fits no commit **stays staged** (the note's promise, :39,
//! :560), including one only the index holds; a staged rename stays whole;
//! a fixup that empties its target drops it; and the working tree edited on
//! past what was staged comes back without conflict markers. Each run is
//! undone after, and the repository must be exactly as it was.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_core::journal::EntryStatus;
use graft_ops::absorb::{StaysWhy, absorb_map};
use graft_ops::apply::{AfterState, ApplyError, Resume};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FactsScope, Operation, Tip};
use graft_ops::rewrite::{RewriteParams, RewriteRequest, TodoAction, plan};

use crate::absorb::{branch, oid, stage};
use crate::set_aside::resume_with;
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
    try_run(repo, &Operation::Rewrite(params.clone()), NOW).unwrap_or_else(|why| {
        panic!(
            "absorb goes through: {}",
            why.explain(&crate::support::Letters)
        )
    });
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

/// The preview's set-aside line: absorb sets aside both staged files. It
/// once also checked `SetAside::staged` was 0; 30c removed that field
/// (every put-back keeps the index), so only the count is left to check.
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

    assert_eq!(aside.files, 2);
}

/// The preview's `your edits: N files set aside` count, for `request`.
fn previewed_aside(repo: &TempRepo, request: &RewriteRequest) -> Option<usize> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    let params = plan(&handle, &git_version(), request, &facts, None, &Never)
        .unwrap_or_else(|why| panic!("it plans: {why}"));
    let op = Operation::Rewrite(params);
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope())
        .expect("the preview's facts read");
    graft_ops::preview::plan(&handle, &git_version(), &op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .set_aside
        .map(|aside| aside.files)
}

/// Review Minor 6: a put-in's and an absorb's staged edits go into
/// commits, so the preview's set-aside row counts only what is set aside
/// **and put back**: the unstaged part, as the landing's "your unstaged
/// edits" says. Two staged files and one unstaged edit count 1.
#[test]
fn the_preview_counts_only_the_unstaged_edits_set_aside() {
    let repo = branch();
    stage_placed(&repo);
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");
    repo.write("old.txt", b"o1\no2\no3\no4\nO5\n");

    assert_eq!(previewed_aside(&repo, &RewriteRequest::Absorb), Some(1));
    let target = oid(&repo, "feature~1");
    assert_eq!(
        previewed_aside(&repo, &RewriteRequest::AmendInto { target }),
        Some(1)
    );
}

/// Nothing unstaged: no count at all, though the staged edits are still
/// set aside around the rewrite (`Operation::sets_aside`).
#[test]
fn the_preview_counts_nothing_set_aside_with_nothing_unstaged() {
    let repo = branch();
    stage_placed(&repo);
    assert_eq!(previewed_aside(&repo, &RewriteRequest::Absorb), Some(0));
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

/// Review 06b Minor 5 (probe p6): a rename with an edit staged, and the
/// working tree edited on past it. Both sides stay; the index comes back
/// byte for byte and the tree as he left it (`RM`).
#[test]
fn a_staged_rename_with_an_edit_and_the_tree_edited_on_stays_whole() {
    let repo = branch();
    repo.git(&["mv", "b.txt", "moved.txt"]);
    stage(&repo, "moved.txt", b"b1\nB2\nb3\nb4\nb5\n");
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");
    repo.write("moved.txt", b"b1\nB2\nb3\nb4\nB5\n");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1", "--", "b.txt", "moved.txt"]),
        "RM b.txt -> moved.txt",
        "the setup is a rename with an edit, edited on"
    );
    let index_before = repo.git(&["write-tree"]);

    absorb(&repo);

    assert_eq!(repo.git(&["write-tree"]), index_before, "still staged");
    assert_eq!(repo.read("moved.txt"), b"b1\nB2\nb3\nb4\nB5\n");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        "RM b.txt -> moved.txt"
    );
    assert_eq!(stash_list(&repo), "", "the set-aside copy was dropped");
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
    let f3 = oid(&repo, "feature~1");

    let params = absorb(&repo);

    // Review 06b Minor 4: the pick below the dropped commit is kept as it
    // is (`rewrite/plan.rs`'s claim), not rewritten to an equal copy.
    assert_eq!(oid(&repo, "feature"), f3, "f3 keeps its id");
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

// ── Review 06b C1 (carried by 30/30b): a staged whole-file deletion ─────

/// Review 06b probe q8: `f2` made all of `b.txt`; he stages `git rm b.txt`
/// and absorbs. Before 31's `deleted file mode` header, the piece's patch
/// wrote an empty `dev/null` into history and `f2` survived as its adder.
/// Now the deletion empties `f2`, so M3's drop fires: `f2` is gone, no
/// `dev/null` anywhere, `f3` untouched, and `u` puts it all back.
#[test]
fn a_staged_file_deletion_drops_the_commit_that_made_it() {
    let repo = branch();
    repo.git(&["rm", "-q", "--", "b.txt"]);
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "D  b.txt");
    let f2 = oid(&repo, "feature~1");
    let before = state_of(&repo);

    let params = planned(&repo);
    let applied = try_run(&repo, &Operation::Rewrite(params.clone()), NOW).unwrap_or_else(|why| {
        panic!(
            "absorb goes through: {}",
            why.explain(&crate::support::Letters)
        )
    });

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    assert!(
        params
            .todo
            .iter()
            .any(|line| line.commit == f2 && line.action == TodoAction::Drop),
        "f2 is planned as a drop: {:?}",
        params.todo
    );
    assert_eq!(subjects(&repo), vec!["f3", "f1"], "f2 dropped, f3 kept");
    let tree = repo.git(&["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(
        !tree
            .lines()
            .any(|name| name == "b.txt" || name == "dev/null"),
        "{tree}"
    );
    assert_eq!(
        repo.git(&["show", "HEAD:a.txt"]),
        "a1\na2\na3\na4\na5 changed",
        "f3's change is still there"
    );
    // The placed deletion was the only thing staged; nothing else moved.
    assert_eq!(repo.git(&["status", "--porcelain=v1"]), "");

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

// ── review m7-06b C2 and M1: an absorb that stops, then goes on ─────────

/// Absorb planned over what the caller staged (a placed hunk and an
/// index-only stay), with `f3` set to `Edit` so the rewrite stops there,
/// after `B3` went into `f2` (as `rewrite_run.rs`'s `edit_a1`). It must stop
/// with nothing unmerged and `S` waiting in the list.
fn absorb_stopped_at_f3(repo: &TempRepo) {
    let mut params = planned(repo);
    let at = params
        .todo
        .iter()
        .position(|line| line.subject == "f3")
        .expect("f3 is a row");
    params.todo[at].action = TodoAction::Edit;
    match try_run(repo, &Operation::Rewrite(params), NOW) {
        Err(ApplyError::Conflicted { paths, .. }) => assert!(paths.is_empty(), "{paths:?}"),
        other => panic!("absorb should stop at f3: {other:?}"),
    }
    assert_eq!(stash_list(repo).lines().count(), 1, "S waits in the list");
}

/// C2: aborted, `HEAD` is back on `S`'s base, so `--index` is exact: the
/// index-only `O3` is still staged, the placed `B3` staged again, and the
/// set-aside copy dropped.
#[test]
fn abort_after_absorb_stops_keeps_index_only_lines() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);
    let before = state_of(&repo);
    absorb_stopped_at_f3(&repo);

    let applied = resume_with(&repo, Resume::Abort, NOW + 1)
        .unwrap_or_else(|why| panic!("abort: {}", why.explain(&crate::support::Letters)));

    assert!(matches!(applied.entry.status, EntryStatus::Aborted { .. }));
    assert_eq!(repo.git(&["show", ":old.txt"]), "o1\no2\nO3\no4\no5");
    assert_eq!(repo.read("old.txt"), b"o1\no2\no3\no4\no5\n");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        "M  b.txt\nMM old.txt"
    );
    assert_eq!(stash_list(&repo), "", "S dropped");
    assert_eq!(state_of(&repo), before, "exactly as before the absorb");
}

/// The abort's undo row: an `Aborted` entry is not in force, so `u` has
/// nothing of it to undo, and the repository stays exactly as before the
/// absorb.
#[test]
fn undo_after_absorb_stops_then_aborts() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);
    let before = state_of(&repo);
    absorb_stopped_at_f3(&repo);
    resume_with(&repo, Resume::Abort, NOW + 1)
        .unwrap_or_else(|why| panic!("abort: {}", why.explain(&crate::support::Letters)));
    let undone = run_undo(&repo, NOW + 2, false);
    assert!(
        format!("{undone:?}").contains("NothingToUndo"),
        "the abort was the way back: {undone:?}"
    );
    assert_eq!(state_of(&repo), before);
}

/// M1: `c` at the stop with nothing changed lands on the plan's tree, so
/// the rebuilt stash goes back: `B3` placed, `O3` still staged, `S` dropped.
#[test]
fn absorb_stopped_then_continued_as_planned_keeps_the_stay() {
    let repo = branch();
    stage_placed(&repo);
    index_only_stay(&repo);
    let before = state_of(&repo);
    absorb_stopped_at_f3(&repo);

    let applied = resume_with(&repo, Resume::Continue { solved: Vec::new() }, NOW + 1)
        .unwrap_or_else(|why| panic!("continue: {}", why.explain(&crate::support::Letters)));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    f2_carries_b3(&repo);
    assert_eq!(repo.git(&["show", ":old.txt"]), "o1\no2\nO3\no4\no5");
    assert_eq!(
        repo.git(&["status", "--porcelain=v1"]),
        "MM old.txt",
        "B3 went into f2; the stay is still staged"
    );
    assert_eq!(stash_list(&repo), "");
    run_undo(&repo, NOW + 2, false).unwrap_or_else(|why| panic!("{why:?}"));
    assert_eq!(state_of(&repo), before, "undo exact");
}

mod continued_elsewhere;
