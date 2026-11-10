//! `conflict::operation_conflicts`: the files an operation conflicted on,
//! read from git and nothing else (m6-conflicts/09, Amendment 2 hard
//! choice 2). Truth is git's own state in a throwaway repository.
//!
//! **What git records, measured on git 2.54.0 (Apple Git-157):** a stopped
//! merge, pull (merge), rebase (merge backend), cherry-pick and revert each
//! write a `# Conflicts:` block into `$GIT_DIR/MERGE_MSG`, one `#\t<path>`
//! line per conflicted file, the path unquoted even with a space in it. A
//! conflicted `stash pop` writes no `MERGE_MSG` at all.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_ops::apply::{ApplyError, Resume};
use mergewright_ops::operation::{Operation, Refusal, RestartKind, RestartParams};
use mergewright_ops::snapshot::{LinkKind, backup_chain};

use crate::conflict_fixtures::{
    conflict_set, downstream_of, mergewright_merge_stops, mergewright_rebase_stops,
    mergewright_stage, paths, run_resume, two_file_conflict, two_stop_history,
};
use crate::support::{NOW, Never, TempRepo, try_run};
use mergewright_core::on_disk::DIR;

#[test]
fn a_merge_lists_resolved_files_too() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    mergewright_stage(&repo, "a.txt", NOW + 1);

    let set = conflict_set(&repo);
    // (a) the one still unmerged, then (b) git's list adds the resolved one.
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded, "git's list was found");
}

#[test]
fn a_relaunch_lists_the_same_files() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    // Resolved in the shell: no step of Mergewright's saw it.
    repo.write("a.txt", b"1\nboth\n3\n");
    repo.git(&["add", "a.txt"]);
    // `conflict_set` opens a new handle and reads fresh facts: nothing of
    // this run is carried over, as after a relaunch.
    let first = conflict_set(&repo);
    let again = conflict_set(&repo);
    assert_eq!(first.paths, paths(&["b.txt", "a.txt"]));
    assert_eq!(again.paths, first.paths);
    assert!(again.recorded);
}

/// Resolve `a.txt` in the shell so only git's list can name it, then read.
fn after_resolving_a(repo: &TempRepo) -> mergewright_ops::conflict::ConflictSet {
    repo.write("a.txt", b"1\nboth\n3\n");
    repo.git(&["add", "a.txt"]);
    conflict_set(repo)
}

/// A kind whose stop writes git's list: `a.txt` resolved in the shell, so
/// only the list can name it.
fn assert_listed(repo: &TempRepo, kind: &str) {
    let set = after_resolving_a(repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]), "{kind}");
    assert!(set.recorded, "{kind} writes Conflicts:");
}

fn stopped_shell_merge() -> TempRepo {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo
}

/// `git pull --no-rebase` of another repository's `feature`; the upstream
/// is returned so it outlives the read.
fn stopped_pull() -> (TempRepo, TempRepo) {
    let upstream = two_file_conflict();
    let repo = downstream_of(&upstream, true);
    let from = upstream.path().to_str().expect("utf-8 temp path");
    assert!(
        repo.try_git(&["pull", "--no-rebase", "--no-edit", from, "feature"])
            .is_err()
    );
    assert!(repo.exists(".git/MERGE_HEAD"), "the pull stopped mid-merge");
    (repo, upstream)
}

fn stopped_rebase() -> TempRepo {
    let repo = two_file_conflict();
    repo.git(&["switch", "-q", "feature"]);
    assert!(repo.try_git(&["rebase", "main"]).is_err());
    repo
}

fn stopped_pick() -> TempRepo {
    let repo = two_file_conflict();
    assert!(repo.try_git(&["cherry-pick", "feature"]).is_err());
    repo
}

/// `feature`'s commit reverted under a later edit of both files.
fn stopped_revert() -> TempRepo {
    let repo = two_file_conflict();
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"1\nG\n3\n");
    repo.write("b.txt", b"G\n");
    repo.commit_all("later on feature");
    assert!(repo.try_git(&["revert", "--no-edit", "HEAD~1"]).is_err());
    repo
}

fn stopped_stash_pop() -> TempRepo {
    let repo = two_file_conflict();
    repo.write("a.txt", b"1\nS\n3\n");
    repo.git(&["stash", "-q"]);
    repo.write("a.txt", b"1\nT\n3\n");
    repo.commit_all("moved on");
    assert!(repo.try_git(&["stash", "pop"]).is_err());
    repo
}

#[test]
fn each_kind_says_what_git_records() {
    assert_listed(&stopped_shell_merge(), "merge");
    assert_listed(&stopped_pull().0, "pull");
    assert_listed(&stopped_rebase(), "rebase");
    assert_listed(&stopped_pick(), "cherry-pick");
    assert_listed(&stopped_revert(), "revert");

    // stash pop: git writes no MERGE_MSG, so only the unmerged path is known
    let repo = stopped_stash_pop();
    assert!(
        !repo.exists(".git/MERGE_MSG"),
        "stash pop writes no MERGE_MSG"
    );
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["a.txt"]), "stash pop");
    assert!(!set.recorded, "stash pop records nothing");
}

#[test]
fn a_path_with_a_space_is_read_whole() {
    let repo = TempRepo::init();
    repo.write("b c.txt", b"x\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("b c.txt", b"M\n");
    repo.commit_all("main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("b c.txt", b"F\n");
    repo.commit_all("feature");
    repo.git(&["switch", "-q", "main"]);
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    repo.write("b c.txt", b"both\n");
    repo.git(&["add", "b c.txt"]);
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b c.txt"]));
    assert!(set.recorded);
}

#[test]
fn the_comment_character_is_git_s() {
    let repo = two_file_conflict();
    repo.git(&["config", "core.commentChar", ";"]);
    assert!(repo.try_git(&["merge", "feature"]).is_err());
    assert!(
        String::from_utf8_lossy(&repo.read(".git/MERGE_MSG")).contains("; Conflicts:"),
        "git wrote its list under ';'"
    );
    let set = after_resolving_a(&repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded);
}

#[test]
fn a_mergewright_step_is_remembered_without_git_s_list() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    mergewright_stage(&repo, "a.txt", NOW + 1);
    // Take git's list away: the backup chain alone still names `a.txt`.
    std::fs::remove_file(repo.path().join(".git/MERGE_MSG")).expect("MERGE_MSG goes");
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded, "the chain is a record too");
}

#[test]
fn nothing_in_progress_is_an_empty_unrecorded_set() {
    let repo = two_file_conflict();
    let set = conflict_set(&repo);
    assert!(set.paths.is_empty());
    assert!(!set.recorded);
}

/// Mergewright's rebase of `TwoStopRebase`'s history, stopped at step 1 on
/// `a.txt`, resolved and marked done through Mergewright, then continued: it
/// stops again at step 2 on `b.txt`.
fn at_step_two() -> TempRepo {
    let repo = two_stop_history();
    mergewright_rebase_stops(&repo);
    assert_eq!(conflict_set(&repo).paths, paths(&["a.txt"]), "step 1");
    repo.write("a.txt", b"1\nboth\n3\n");
    mergewright_stage(&repo, "a.txt", NOW + 1);
    let stopped = run_resume(&repo, Resume::Continue { solved: Vec::new() }, NOW + 2);
    assert!(
        matches!(stopped, Err(ApplyError::Conflicted { .. })),
        "step 2 stops: {stopped:?}"
    );
    repo
}

#[test]
fn a_rebase_lists_only_the_current_steps_files() {
    let repo = at_step_two();
    let set = conflict_set(&repo);
    // `a.txt` was step 1's; git's list and the index say only `b.txt` now.
    assert_eq!(set.paths, paths(&["b.txt"]));
    assert!(set.recorded);
}

#[test]
fn a_rebase_step_keeps_its_resolved_files() {
    let repo = at_step_two();
    repo.write("b.txt", b"both\n");
    mergewright_stage(&repo, "b.txt", NOW + 3);
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b.txt"]));
    assert!(set.recorded);
    // The chain alone, without git's list: step 2's link names `b.txt`, and
    // the walk stops before step 1's.
    std::fs::remove_file(repo.path().join(".git/MERGE_MSG")).expect("MERGE_MSG goes");
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b.txt"]));
    assert!(set.recorded, "the chain is a record too");
}

/// `d` at step 2 on the file step 1 conflicted on: step 1 is over, so its
/// stages are not this step's to restore (review m6-amend-10-22 M1). Before
/// the bound, step 1's link on the chain restored `M` against `F` into step
/// 2's index, and taking a side silently undid step 1's resolution.
#[test]
fn a_restart_never_reaches_a_finished_step() {
    let repo = at_step_two();
    let before = repo.read("a.txt");
    let path = std::path::PathBuf::from("a.txt");
    match try_run(
        &repo,
        &Operation::RestartConflict(RestartParams {
            path: path.clone(),
            kind: RestartKind::Markers,
        }),
        NOW + 3,
    ) {
        Err(ApplyError::Refused(Refusal::NoStagesKept { path: refused })) => {
            assert_eq!(refused, path);
        }
        other => panic!("expected NoStagesKept, got {other:?}"),
    }
    assert_eq!(repo.read("a.txt"), before, "step 1's resolution stays");
    assert!(
        !repo.git(&["status", "--porcelain"]).contains("UU a.txt"),
        "a.txt is not unmerged again"
    );
}

/// The bound does not cost the current step its own restart: `b.txt`,
/// resolved and staged at step 2, comes back from step 2's link.
#[test]
fn a_restart_reaches_the_current_steps_file() {
    let repo = at_step_two();
    repo.write("b.txt", b"both\n");
    mergewright_stage(&repo, "b.txt", NOW + 3);
    try_run(
        &repo,
        &Operation::RestartConflict(RestartParams {
            path: std::path::PathBuf::from("b.txt"),
            kind: RestartKind::Markers,
        }),
        NOW + 4,
    )
    .expect("step 2's own file restarts");
    let back = String::from_utf8_lossy(&repo.read("b.txt")).into_owned();
    assert!(back.contains("<<<<<<<"), "the markers are back: {back}");
}

/// A merge has one step and no `Resume` link before its finish: every link
/// counts, as in 09 (`a_merge_lists_resolved_files_too`,
/// `a_mergewright_step_is_remembered_without_git_s_list`,
/// `a_relaunch_lists_the_same_files`).
#[test]
fn a_merge_set_is_unchanged() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    mergewright_stage(&repo, "a.txt", NOW + 1);
    repo.write("b.txt", b"both\n");
    mergewright_stage(&repo, "b.txt", NOW + 2);
    std::fs::remove_file(repo.path().join(".git/MERGE_MSG")).expect("MERGE_MSG goes");
    let set = conflict_set(&repo);
    // Newest link first: before `b.txt` was staged (only `b.txt` open), then
    // before `a.txt` was (both open).
    assert_eq!(set.paths, paths(&["b.txt", "a.txt"]));
    assert!(set.recorded);
}

/// The one backup ref Mergewright's merge left.
fn backup_ref(repo: &TempRepo) -> String {
    let name = repo.git(&[
        "for-each-ref",
        "--format=%(refname)",
        &format!("refs/{DIR}/backup/"),
    ]);
    assert_eq!(name.lines().count(), 1, "one backup ref: {name}");
    name
}

/// Rewrite the ref's newest link with `message` in place of its own, same
/// tree and parent.
fn relabel_newest(repo: &TempRepo, name: &str, message: &str) {
    let tree = repo.git(&["rev-parse", &format!("{name}^{{tree}}")]);
    let parent = repo.git(&["rev-parse", &format!("{name}^")]);
    let commit = repo.git(&["commit-tree", &tree, "-p", &parent, "-m", message]);
    repo.git(&["update-ref", name, &commit]);
}

#[test]
fn a_link_without_a_kind_reads_as_step() {
    let repo = two_file_conflict();
    mergewright_merge_stops(&repo);
    repo.write("a.txt", b"1\nboth\n3\n");
    mergewright_stage(&repo, "a.txt", NOW + 1);
    let name = backup_ref(&repo);
    let written = repo.git(&["log", "-1", "--format=%B", &name]);
    assert_eq!(
        written.lines().last(),
        Some(format!("{} step", mergewright_core::on_disk::LINK_TRAILER).as_str()),
        "{written}"
    );

    let kinds = |repo: &TempRepo| -> Vec<LinkKind> {
        backup_chain(&repo.open(), &name, &Never)
            .expect("the chain reads")
            .into_iter()
            .map(|(_, link)| link)
            .collect()
    };
    // The link as m6-conflicts/09 wrote it: no `Mergewright-Link:` line.
    relabel_newest(&repo, &name, "graft x step within: merge feature");
    assert_eq!(kinds(&repo), [LinkKind::Step, LinkKind::Step]);
    // ...and so it bounds nothing: the set is 09's.
    std::fs::remove_file(repo.path().join(".git/MERGE_MSG")).expect("MERGE_MSG goes");
    assert_eq!(conflict_set(&repo).paths, paths(&["b.txt", "a.txt"]));

    // The same link marked `resume` does bound the walk.
    relabel_newest(&repo, &name, "graft x\n\nMergewright-Link: resume");
    assert_eq!(kinds(&repo), [LinkKind::Resume, LinkKind::Step]);
    let set = conflict_set(&repo);
    assert_eq!(set.paths, paths(&["b.txt"]));
    assert!(!set.recorded, "only the index names b.txt now");
}
