//! `jobs::rewrite_open` against real git: R1's mock, his branch four
//! commits since `main` (one a `fixup!`), a teammate's built on it.

use std::path::Path;
use std::process::Command as Shell;

use mergewright_core::oid::Oid;
use mergewright_ops::operation::Refusal;
use mergewright_ops::rewrite::{RewriteRequest, TodoAction};

use super::{RewriteOpened, open};
use crate::repos::RepoStore;

fn git(repo: &Path, args: &[&str]) -> String {
    let out = Shell::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn commit(repo: &Path, file: &str, message: &str) -> Oid {
    std::fs::write(repo.join(file), message).expect("writable");
    git(repo, &["add", file]);
    git(repo, &["commit", "-q", "-m", message]);
    Oid::from_hex(git(repo, &["rev-parse", "HEAD"]).trim()).expect("an id")
}

/// `main` (2 commits) → `feature-a` (4: `Add OAuth callback`, its
/// `fixup!`, `wip: tests` with a body, `wip`) → `feature-b` (1), and
/// `feature-a` checked out.
fn mock() -> (tempfile::TempDir, Vec<Oid>) {
    let dir = tempfile::tempdir().expect("a temp dir");
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"]);
    commit(repo, "base.txt", "Initial");
    let base = commit(repo, "base.txt", "Bump version to 2.4");
    git(repo, &["checkout", "-q", "-b", "feature-a"]);
    let callback = commit(repo, "oauth.txt", "Add OAuth callback");
    let fixup = commit(repo, "oauth.txt", "fixup! Add OAuth callback");
    std::fs::write(repo.join("tests.txt"), "t").expect("writable");
    git(repo, &["add", "tests.txt"]);
    git(
        repo,
        &[
            "commit",
            "-q",
            "-m",
            "wip: tests",
            "-m",
            "Covers the expired path.",
        ],
    );
    let tests = Oid::from_hex(git(repo, &["rev-parse", "HEAD"]).trim()).expect("an id");
    let wip = commit(repo, "wip.txt", "wip");
    git(repo, &["checkout", "-q", "-b", "feature-b"]);
    commit(repo, "settings.txt", "Settings page skeleton");
    git(repo, &["checkout", "-q", "feature-a"]);
    (dir, vec![base, callback, fixup, tests, wip])
}

#[test]
fn open_reads_every_rows_whole_message() {
    let (dir, oids) = mock();
    let [base, callback, fixup, tests, wip] = oids[..] else {
        panic!("five commits");
    };
    let store = RepoStore::new(dir.path().to_path_buf(), None);
    let opened = open(&store, &RewriteRequest::Open { from: wip }, &()).expect("the read works");
    let RewriteOpened::Editor(opened) = opened else {
        panic!("not refused: {opened:?}");
    };
    // The rows `plan` gave, the fixup already moved above its target.
    let rows: Vec<(TodoAction, Oid)> = opened
        .params
        .todo
        .iter()
        .map(|row| (row.action, row.commit))
        .collect();
    assert_eq!(
        rows,
        vec![
            (TodoAction::Pick, callback),
            (TodoAction::Fixup, fixup),
            (TodoAction::Pick, tests),
            (TodoAction::Pick, wip),
        ]
    );
    // Every row's whole message, the body included.
    assert_eq!(opened.whole.len(), 4);
    assert_eq!(
        opened.whole.get(&tests).map(String::as_str),
        Some("wip: tests\n\nCovers the expired path.\n")
    );
    assert_eq!(opened.whole.get(&wip).map(String::as_str), Some("wip\n"));
    // The base row: `onto`, its subject, named by the default branch.
    assert_eq!(opened.base.oid, base);
    assert_eq!(opened.base.subject, "Bump version to 2.4");
    assert_eq!(opened.base.label, "main");
    // The dependent follows.
    let names: Vec<&str> = opened
        .params
        .line
        .branches
        .iter()
        .map(|tip| tip.name.as_str())
        .collect();
    assert_eq!(names, vec!["refs/heads/feature-a", "refs/heads/feature-b"]);
}

/// Two branches both built on his: `plan` refuses, and the job hands the
/// refusal back as a refusal — words, no editor.
#[test]
fn open_refuses_forked_dependents_in_words() {
    let (dir, oids) = mock();
    let repo = dir.path();
    git(repo, &["checkout", "-q", "-b", "feature-c"]);
    commit(repo, "other.txt", "Another teammate's work");
    git(repo, &["checkout", "-q", "feature-a"]);
    let store = RepoStore::new(repo.to_path_buf(), None);
    let opened =
        open(&store, &RewriteRequest::Open { from: oids[4] }, &()).expect("the read works");
    let RewriteOpened::Refused(refusal) = opened else {
        panic!("refused: {opened:?}");
    };
    assert!(
        matches!(refusal, Refusal::ForkedDependents { .. }),
        "{refusal:?}"
    );
    assert_eq!(
        refusal.explain(&crate::update::op_keys::OpKeys(
            mergewright_ui::keymap::Keymap::shipped()
        )),
        "feature-b and feature-c are both built on yours · Mergewright moves one line"
    );
}
