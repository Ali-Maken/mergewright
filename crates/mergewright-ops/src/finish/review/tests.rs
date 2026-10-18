//! `finish_preview` with a solved list the app could not build: since
//! ease-pass/05s it is crate-only, so the case lives beside it (moved from
//! `tests/it/merge_review.rs`, same name).

use std::path::{Path, PathBuf};
use std::process::Command;

use mergewright_git::repo::RepoHandle;

use super::finish_preview;
use crate::finish::SummaryError;

/// `git` in `dir`, its success checked unless `may_fail`.
fn git(dir: &Path, args: &[&str], may_fail: bool) {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_AUTHOR_NAME", "Alice Ames")
        .env("GIT_AUTHOR_EMAIL", "alice@example.com")
        .env("GIT_COMMITTER_NAME", "Alice Ames")
        .env("GIT_COMMITTER_EMAIL", "alice@example.com")
        .output()
        .expect("git runs");
    assert!(may_fail || out.status.success(), "git {args:?}: {out:?}");
}

fn write(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).expect("the scratch repository is writable");
}

/// `main` merging `feature`, stopped on `a.txt` and `b.txt`, one clash each.
fn two_file_conflict() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a scratch directory");
    let path = dir.path();
    git(path, &["init", "-q", "-b", "main"], false);
    git(path, &["config", "commit.gpgsign", "false"], false);
    write(path, "a.txt", "1\n2\n3\n");
    write(path, "b.txt", "x\n");
    git(path, &["add", "-A"], false);
    git(path, &["commit", "-qm", "base"], false);
    git(path, &["branch", "feature"], false);
    write(path, "a.txt", "1\nM\n3\n");
    write(path, "b.txt", "M\n");
    git(path, &["commit", "-qam", "main's side"], false);
    git(path, &["switch", "-q", "feature"], false);
    write(path, "a.txt", "1\nF\n3\n");
    write(path, "b.txt", "F\n");
    git(path, &["commit", "-qam", "feature's side"], false);
    git(path, &["switch", "-q", "main"], false);
    git(path, &["merge", "-q", "feature"], true);
    dir
}

#[test]
fn the_review_counts_only_the_unmerged_files_not_named_solved() {
    let dir = two_file_conflict();
    write(dir.path(), "a.txt", "1\nF\n3\n");
    let repo = RepoHandle::open(dir.path()).expect("it opens");
    let why = finish_preview(&repo, &[PathBuf::from("a.txt")], &()).expect_err("b.txt is open");
    assert!(
        matches!(why, SummaryError::StillUnmerged { count: 1 }),
        "{why:?}"
    );
    let journal = crate::journal::journal_path(&repo).expect("the journal path");
    let copies = match std::fs::read_dir(journal.parent().expect("a directory")) {
        Ok(dir) => dir
            .map(|entry| entry.expect("an entry"))
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("tmp-review-index")
            })
            .count(),
        // No journal directory yet: nothing was copied into it.
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => 0,
        Err(why) => panic!("the journal's directory: {why}"),
    };
    assert_eq!(copies, 0, "no review copy left behind");
}
