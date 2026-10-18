//! The look's reads against real git (ease-pass/12b).

use std::path::{Path, PathBuf};

use mergewright_git::diff::Origin;

use super::super::tests::{git, solved_merge};
use super::{combined_look, look_result};

/// A merge stopped by `--no-commit` with `both.txt` changed on both sides
/// (different lines), `mine.txt` on yours only and `theirs.txt` on theirs.
fn combined_merge() -> mergewright_fixtures::Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let write = |name: &str, text: &str| std::fs::write(repo.join(name), text).expect("writable");
    let ten = "line 1\nline 2\nline 3\nline 4\nline 5\nline 6\nline 7\nline 8\nline 9\nline 10\n";
    write("both.txt", ten);
    write("mine.txt", "a\nb\n");
    write("theirs.txt", "x\ny\n");
    git(repo, &["add", "--all"]);
    git(repo, &["commit", "-q", "-m", "base"]);
    git(repo, &["checkout", "-q", "-b", "feature"]);
    write("both.txt", &ten.replace("line 9\n", "line 9\nTHEIRS\n"));
    write("theirs.txt", "x\nY\nz\n");
    git(repo, &["commit", "-q", "-am", "theirs"]);
    git(repo, &["checkout", "-q", "-"]);
    write("both.txt", &ten.replace("line 2\n", "line 2\nYOURS\n"));
    write("mine.txt", "a\n");
    git(repo, &["commit", "-q", "-am", "yours"]);
    git(repo, &["merge", "-q", "--no-commit", "--no-ff", "feature"]);
    scratch
}

/// `(added, deleted)` from `git diff --numstat <base> <side> -- <path>`.
fn numstat(repo: &Path, side: &str, path: &str) -> (u32, u32) {
    let base = git(repo, &["merge-base", "HEAD", "MERGE_HEAD"]);
    let out = git(repo, &["diff", "--numstat", base.trim(), side, "--", path]);
    let mut parts = out.split_whitespace();
    let mut next = || parts.next().map_or(0, |n| n.parse().expect("a count"));
    (next(), next())
}

#[test]
fn the_counts_equal_git_numstat_per_side() {
    let scratch = combined_merge();
    let repo = scratch.path();
    let handle = mergewright_git::repo::RepoHandle::open(repo).expect("a repo");
    let paths: Vec<PathBuf> = ["both.txt", "mine.txt", "theirs.txt"]
        .iter()
        .map(PathBuf::from)
        .collect();
    let (counts, sample) = combined_look(&handle, &paths, &()).expect("the look reads");
    for count in &counts {
        let path = count.path.to_str().expect("utf-8");
        assert_eq!(count.yours, numstat(repo, "HEAD", path), "{path} yours");
        assert_eq!(
            count.theirs,
            numstat(repo, "MERGE_HEAD", path),
            "{path} theirs"
        );
    }
    assert_eq!(counts[1].theirs, (0, 0), "mine.txt: theirs unchanged");
    let sample = sample.expect("a sample");
    assert_eq!(
        sample.path,
        PathBuf::from("both.txt"),
        "changed on both sides"
    );
    let marked: Vec<(&str, Origin)> = sample
        .lines
        .iter()
        .filter_map(|(_, text, origin)| origin.map(|o| (text.as_str(), o)))
        .collect();
    assert!(marked.contains(&("THEIRS", Origin::Theirs)), "{sample:?}");
}

#[test]
fn a_solved_file_s_result_is_its_worktree_text() {
    let scratch = solved_merge();
    let handle = mergewright_git::repo::RepoHandle::open(scratch.path()).expect("a repo");
    assert_eq!(
        look_result(&handle, Path::new("notes.txt")).expect("reads"),
        "a\nTHEIRS\nc\n"
    );
}
