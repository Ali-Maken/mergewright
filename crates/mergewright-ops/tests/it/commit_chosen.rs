//! A commit of chosen files on a scratch index (ease-pass/04): what goes in,
//! what stays staged, `u`, amend, a failing hook, and where the copy lives.

#![allow(clippy::expect_used, reason = "a failing test should panic")]

use std::path::{Path, PathBuf};

use mergewright_core::journal::EntryStatus;
use mergewright_ops::apply::{ApplyError, apply};
use mergewright_ops::journal::journal_path;
use mergewright_ops::operation::{Chosen, CommitParams, Include, Operation, Part};

use crate::support::{
    NOW, Never, TempRepo, context, journal_for, run, run_undo, state_of, try_run,
};

pub(super) fn whole(path: &str) -> Chosen {
    Chosen {
        path: PathBuf::from(path),
        part: Part::Whole,
    }
}

fn as_staged(path: &str) -> Chosen {
    Chosen {
        path: PathBuf::from(path),
        part: Part::StagedPart,
    }
}

fn params(message: &str, include: Include) -> CommitParams {
    CommitParams {
        message: message.to_owned(),
        sign_off: false,
        no_verify: false,
        allow_empty: false,
        include,
    }
}

pub(super) fn commit_of(files: Vec<Chosen>) -> Operation {
    Operation::Commit(params("chosen\n", Include::Chosen(files)))
}

/// `a.rs`, `b.rs`, `gone.rs` committed twice over, so `HEAD` has a parent.
pub(super) fn repo() -> TempRepo {
    let repo = TempRepo::init();
    for name in ["a.rs", "b.rs", "gone.rs"] {
        repo.write(name, b"one\n");
    }
    repo.commit_all("first");
    repo.write("b.rs", b"two\n");
    repo.commit_all("second");
    repo
}

/// `a.rs` partly staged (`a2` staged, `a3` on disk), `b.rs` staged whole.
pub(super) fn partly_staged(repo: &TempRepo) {
    repo.write("a.rs", b"a2\n");
    repo.git(&["add", "a.rs"]);
    repo.write("a.rs", b"a3\n");
    repo.write("b.rs", b"b-staged\n");
    repo.git(&["add", "b.rs"]);
}

pub(super) fn at_head(repo: &TempRepo, path: &str) -> String {
    repo.git(&["show", &format!("HEAD:{path}")])
}

fn staged_line(repo: &TempRepo, path: &str) -> String {
    repo.git(&["ls-files", "-s", "--", path])
}

/// The names dir, where the scratch index is made.
pub(super) fn names_dir(repo: &TempRepo) -> PathBuf {
    let path = journal_path(&repo.open()).expect("the journal path resolves");
    let path = if path.is_absolute() {
        path
    } else {
        repo.path().join(path)
    };
    path.parent()
        .expect("the journal has a directory")
        .to_path_buf()
}

/// The names dir's scratch index copies, `commit-index` and
/// `commit-index-<pid>-<n>`.
pub(super) fn scratch_copies(repo: &TempRepo) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(names_dir(repo))
        .expect("the names dir lists")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.starts_with("commit-index"))
        .collect();
    names.sort();
    names
}

pub(super) fn no_scratch_left(repo: &TempRepo) {
    assert_eq!(scratch_copies(repo), Vec::<String>::new());
}

/// A hook in a directory under `.git`, so it never shows in the status.
pub(super) fn hook(repo: &TempRepo, name: &str, body: &str) {
    let hooks = repo.path().join(".git").join("test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks directory");
    let file = hooks.join(name);
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n")).expect("the hook writes");
    let mut permissions = std::fs::metadata(&file).expect("metadata").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    std::fs::set_permissions(&file, permissions).expect("executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);
}

#[test]
fn whole_on_a_partly_staged_file_commits_the_working_file_and_leaves_the_rest_staged() {
    let repo = repo();
    partly_staged(&repo);
    let b_before = staged_line(&repo, "b.rs");

    run(&repo, &commit_of(vec![whole("a.rs")]), NOW);

    assert_eq!(at_head(&repo, "a.rs"), "a3");
    assert_eq!(at_head(&repo, "b.rs"), "two", "b.rs stayed out");
    assert_eq!(
        staged_line(&repo, "b.rs"),
        b_before,
        "b.rs still staged as it was"
    );
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", "a.rs"]),
        "",
        "a.rs is clean: the real index has the new HEAD's a.rs"
    );
    no_scratch_left(&repo);
}

#[test]
fn staged_part_commits_the_staged_content_only() {
    let repo = repo();
    partly_staged(&repo);
    let b_before = staged_line(&repo, "b.rs");

    run(&repo, &commit_of(vec![as_staged("a.rs")]), NOW);

    assert_eq!(at_head(&repo, "a.rs"), "a2");
    assert_eq!(repo.read("a.rs"), b"a3\n", "the working file is untouched");
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", "a.rs"]),
        " M a.rs",
        "a3 is still an unstaged change"
    );
    assert_eq!(staged_line(&repo, "b.rs"), b_before);
    assert_eq!(at_head(&repo, "b.rs"), "two");
}

#[test]
fn deleted_and_untracked_files_chosen_commit_as_removal_and_addition() {
    let repo = repo();
    std::fs::remove_file(repo.path().join("gone.rs")).expect("removed");
    repo.write("new.rs", b"new\n");
    repo.write("left.rs", b"left out\n");

    run(
        &repo,
        &commit_of(vec![whole("gone.rs"), whole("new.rs")]),
        NOW,
    );

    assert_eq!(
        repo.git(&["ls-tree", "--name-only", "HEAD"]),
        "a.rs\nb.rs\nnew.rs"
    );
    assert_eq!(
        repo.git(&["status", "--porcelain", "--", "gone.rs", "new.rs"]),
        ""
    );
    assert_eq!(repo.git(&["status", "--porcelain"]), "?? left.rs");
}

#[test]
fn undo_after_a_chosen_commit_puts_head_index_and_files_back() {
    let repo = repo();
    partly_staged(&repo);
    repo.write("new.rs", b"new\n");
    let before = state_of(&repo);

    run(&repo, &commit_of(vec![whole("a.rs"), whole("new.rs")]), NOW);
    assert_ne!(state_of(&repo), before);

    run_undo(&repo, NOW + 1, false).expect("undo runs");
    assert_eq!(
        state_of(&repo),
        before,
        "HEAD, ls-files -s and files as before"
    );
}

#[test]
fn amend_of_a_chosen_file_keeps_the_parents_and_adds_only_that_file() {
    let repo = repo();
    let old = repo.git(&["rev-parse", "HEAD"]);
    let old_parents = repo.git(&["rev-parse", "HEAD^@"]);
    repo.write("a.rs", b"amended\n");
    repo.write("b.rs", b"b-staged\n");
    repo.git(&["add", "b.rs"]);
    let b_before = staged_line(&repo, "b.rs");
    let before = state_of(&repo);

    run(
        &repo,
        &Operation::Amend(params(
            "second, amended\n",
            Include::Chosen(vec![whole("a.rs")]),
        )),
        NOW,
    );

    assert_eq!(repo.git(&["rev-parse", "HEAD^@"]), old_parents);
    assert_eq!(
        repo.git(&["diff", "--name-only", &old, "HEAD"]),
        "a.rs",
        "the old tree plus a.rs, nothing else"
    );
    assert_eq!(at_head(&repo, "a.rs"), "amended");
    assert_eq!(staged_line(&repo, "b.rs"), b_before);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "second, amended");
    no_scratch_left(&repo);

    run_undo(&repo, NOW + 1, false).expect("undo runs");
    assert_eq!(state_of(&repo), before);
}

#[test]
fn amend_of_what_is_staged_with_nothing_staged_rewords_only() {
    let repo = repo();
    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    repo.write("a.rs", b"not staged\n");

    run(
        &repo,
        &Operation::Amend(params("reworded\n", Include::Staged)),
        NOW,
    );

    assert_eq!(repo.git(&["rev-parse", "HEAD^{tree}"]), tree);
    assert_eq!(repo.git(&["log", "-1", "--format=%s"]), "reworded");
}

#[test]
fn a_failing_pre_commit_hook_leaves_head_and_the_index_and_no_scratch() {
    let repo = repo();
    partly_staged(&repo);
    hook(&repo, "pre-commit", "echo refused >&2\nexit 1");
    let head = repo.git(&["rev-parse", "HEAD"]);
    let index = repo.git(&["ls-files", "-s"]);

    let failed = try_run(&repo, &commit_of(vec![whole("a.rs")]), NOW);

    assert!(matches!(failed, Err(ApplyError::Git { .. })), "{failed:?}");
    assert_eq!(repo.git(&["rev-parse", "HEAD"]), head);
    assert_eq!(repo.git(&["ls-files", "-s"]), index);
    no_scratch_left(&repo);
}

#[test]
fn hooks_see_the_scratch_index() {
    let repo = repo();
    partly_staged(&repo);
    // What the hook reads as staged is what goes in: a.rs as on disk, no b.rs.
    let seen = repo.path().join(".git").join("seen");
    hook(
        &repo,
        "pre-commit",
        &format!("git diff --cached --name-only > '{}'", seen.display()),
    );

    run(&repo, &commit_of(vec![whole("a.rs")]), NOW);

    assert_eq!(
        std::fs::read_to_string(&seen).expect("the hook ran"),
        "a.rs\n"
    );
}

#[test]
fn the_command_log_names_the_scratch_index_it_ran_on() {
    let repo = repo();
    partly_staged(&repo);

    let entry = run(&repo, &commit_of(vec![whole("a.rs")]), NOW);

    // This run's own name: `commit-index-<pid>-<n>` (review ease-pass-04, M1).
    let scratch = names_dir(&repo).join(format!("commit-index-{}-", std::process::id()));
    let commit_line = entry
        .commands
        .iter()
        .find(|line| line.contains("commit -F -"))
        .expect("the commit line is logged");
    let named = commit_line
        .strip_prefix(&format!("GIT_INDEX_FILE={}", scratch.display()))
        .and_then(|rest| rest.split_once(' '))
        .expect("the line names this run's scratch index");
    assert!(named.0.parse::<u64>().is_ok(), "{commit_line}");
    assert_eq!(named.1, "git commit -F -");
    assert_eq!(entry.status, EntryStatus::Committed);
}

#[test]
fn a_linked_worktree_commits_from_its_own_index() {
    let repo = repo();
    repo.git(&["worktree", "add", "-q", "-b", "side", ".git/wt"]);
    let wt = Path::new(".git/wt");
    repo.harden(wt);
    std::fs::write(repo.path().join(".git/wt/a.rs"), b"in wt\n").expect("writes");
    std::fs::write(repo.path().join(".git/wt/b.rs"), b"wt staged\n").expect("writes");
    repo.git_in(wt, &["add", "b.rs"]);
    let main_index = repo.git(&["ls-files", "-s"]);

    let handle = repo.open_at(".git/wt");
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    apply(&mut ctx, &commit_of(vec![whole("a.rs")])).expect("the commit runs");

    assert_eq!(repo.git_in(wt, &["show", "HEAD:a.rs"]), "in wt");
    assert_eq!(repo.git_in(wt, &["show", "HEAD:b.rs"]), "two");
    assert_eq!(
        repo.git_in(wt, &["diff", "--cached", "--name-only"]),
        "b.rs",
        "b.rs still staged in the worktree"
    );
    assert_eq!(repo.git(&["ls-files", "-s"]), main_index);
}

/// Step 4 (the real index reset) failing after the commit landed: the entry
/// closes `Aborted` with git's words and `u` puts everything back. A held
/// `index.lock` is the one way a fixture can fail that step alone.
#[test]
fn a_failed_reset_after_the_commit_is_undone_by_u() {
    let repo = repo();
    partly_staged(&repo);
    let lock = repo.path().join(".git").join("index.lock");
    hook(&repo, "post-commit", &format!("touch '{}'", lock.display()));
    let before = state_of(&repo);

    let failed = try_run(&repo, &commit_of(vec![whole("a.rs")]), NOW);

    assert!(matches!(failed, Err(ApplyError::Git { .. })), "{failed:?}");
    let journal = journal_for(&repo.open());
    let entry = journal.entries().last().expect("the entry is journaled");
    assert!(
        matches!(&entry.status, EntryStatus::Aborted { reason } if reason.contains("index.lock")),
        "{:?}",
        entry.status
    );
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "chosen",
        "HEAD moved"
    );
    no_scratch_left(&repo);
    std::fs::remove_file(&lock).expect("the lock goes");

    run_undo(&repo, NOW + 1, false).expect("undo runs");
    assert_eq!(state_of(&repo), before);
}
