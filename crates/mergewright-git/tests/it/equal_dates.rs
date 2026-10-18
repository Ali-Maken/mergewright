//! Ahead/behind on commits that share one committer date, against
//! `git rev-list --left-right --count` (m8-polish-and-ship/14: `main ↑6 ↓1`
//! where git said `1 1`). A script, a rebase or a quick series of commits
//! makes many commits inside one second; only the id orders them then.

use std::path::Path;
use std::process::Command;

use mergewright_git::history::ahead_behind;

use crate::session::{git_ok, open, rev};

/// `git` with every author and committer date pinned to one second.
fn git_at_one_date(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("LC_ALL", "C")
        .env("GIT_AUTHOR_NAME", "Graft Test")
        .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Test")
        .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
        .env("GIT_AUTHOR_DATE", "1700000000 +0000")
        .env("GIT_COMMITTER_DATE", "1700000000 +0000")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap_or_else(|why| panic!("running git: {why}"));
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn commit(dir: &Path, message: &str) {
    git_at_one_date(dir, &["commit", "-q", "--allow-empty", "-m", message]);
}

/// Every pair of the named branches, ours against git's.
fn assert_counts_match_git(dir: &Path, branches: &[&str]) -> usize {
    let repo = open(dir);
    let mut checked = 0;
    for a in branches {
        for b in branches {
            let counted = ahead_behind(&repo, rev(dir, a), rev(dir, b))
                .unwrap_or_else(|why| panic!("{a}...{b}: {why}"));
            let range = format!("{a}...{b}");
            let git = String::from_utf8(git_ok(
                dir,
                &["rev-list", "--left-right", "--count", &range],
            ))
            .unwrap_or_else(|why| panic!("rev-list printed non-UTF-8: {why}"));
            assert_eq!(
                format!("{}\t{}", counted.ahead, counted.behind),
                git.trim(),
                "{range}"
            );
            checked += 1;
        }
    }
    checked
}

/// Several shapes, each built with a few seeds so the ids — the only
/// order — fall differently: a fork, a merge of the upstream, a criss-cross.
#[test]
fn ahead_behind_on_one_date_matches_rev_list() {
    let mut checked = 0;
    for seed in 0..6 {
        let temp = tempfile::tempdir().unwrap_or_else(|why| panic!("tempdir: {why}"));
        let dir = temp.path();
        git_at_one_date(dir, &["init", "-q", "-b", "main"]);
        for n in 0..4 {
            commit(dir, &format!("base {seed}.{n}"));
        }
        git_at_one_date(dir, &["branch", "up"]);
        for n in 0..3 {
            commit(dir, &format!("local {seed}.{n}"));
        }
        git_at_one_date(dir, &["switch", "-q", "up"]);
        for n in 0..2 {
            commit(dir, &format!("up {seed}.{n}"));
        }
        // `merged` takes `main` in, then goes on; `cross` is a criss-cross
        // partner of it.
        git_at_one_date(dir, &["switch", "-q", "-c", "merged"]);
        git_at_one_date(dir, &["merge", "-q", "--no-edit", "main"]);
        commit(dir, &format!("after merge {seed}"));
        git_at_one_date(dir, &["switch", "-q", "-c", "cross", "main"]);
        git_at_one_date(dir, &["merge", "-q", "--no-edit", "up"]);
        commit(dir, &format!("cross {seed}"));
        checked += assert_counts_match_git(dir, &["main", "up", "merged", "cross"]);
    }
    assert_eq!(checked, 6 * 16);
}
