//! The compare read against real git (m7-14): listed and diffed from where
//! the two sides split, not two-dot.

use std::path::Path;

use graft_fixtures::git::{FIXED_DATE, run_at};

use super::*;
use crate::update::detail::{DetailSpec, DiffKey};

fn git(repo: &Path, args: &[&str]) -> String {
    let out = run_at(repo, args, FIXED_DATE).unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    String::from_utf8(out).expect("git speaks utf-8 here")
}

fn oid(repo: &Path, rev: &str) -> Oid {
    Oid::from_hex(git(repo, &["rev-parse", rev]).trim()).expect("an oid")
}

fn commit(repo: &Path, file: &str, body: &str, message: &str) {
    std::fs::write(repo.join(file), body).expect("write");
    git(repo, &["add", file]);
    git(repo, &["commit", "-q", "-m", message]);
}

/// `shared.txt` on the root; feature-a 3 commits off it (`a.txt`, `b.txt`,
/// a line added to `shared.txt`); main moved on by 2 (`m.txt`, and a
/// `main-only` line in `shared.txt`).
fn split_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary directory");
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"]);
    commit(repo, "shared.txt", "one\ntwo\nthree\n", "root");
    git(repo, &["checkout", "-q", "-b", "feature-a"]);
    commit(repo, "a.txt", "a\n", "a");
    commit(repo, "b.txt", "b\n", "b");
    commit(
        repo,
        "shared.txt",
        "one\ntwo\nthree\nfeature\n",
        "feature line",
    );
    git(repo, &["checkout", "-q", "main"]);
    commit(repo, "m.txt", "m\n", "m");
    commit(
        repo,
        "shared.txt",
        "main-only\none\ntwo\nthree\n",
        "main line",
    );
    dir
}

fn paths(list: &FileList) -> Vec<String> {
    list.files
        .iter()
        .map(|file| String::from_utf8_lossy(&file.path).into_owned())
        .collect()
}

#[test]
fn the_compare_diff_is_since_the_split() {
    let dir = split_repo();
    let repo = dir.path();
    let store = RepoStore::new(repo.to_path_buf(), None);
    let spec = DiffSpec::Range {
        base: oid(repo, "main"),
        head: oid(repo, "feature-a"),
    };
    let (view, list) =
        read_files(&store, spec, DiffOptions::default(), &Cancel::new()).expect("lists");
    assert_eq!(paths(&list), ["a.txt", "b.txt", "shared.txt"]);
    assert_eq!(
        view.compare,
        Some(CompareSummary::Since {
            split: oid(repo, "main~2"),
            commits: 3
        })
    );
}

#[test]
fn a_file_diff_in_a_compare_uses_the_split() {
    let dir = split_repo();
    let repo = dir.path();
    let store = RepoStore::new(repo.to_path_buf(), None);
    let (base, head) = (oid(repo, "main"), oid(repo, "feature-a"));
    let (_, list) = read_files(
        &store,
        DiffSpec::Range { base, head },
        DiffOptions::default(),
        &Cancel::new(),
    )
    .expect("lists");
    let entry = list
        .files
        .iter()
        .position(|file| file.path == b"shared.txt")
        .expect("shared.txt is listed");
    let key = DiffKey {
        spec: DetailSpec::Compare { base, head },
        entry: u32::try_from(entry).expect("small"),
        section: None,
        options: DiffOptions::default(),
        max_lines: 5_000,
    };
    let (_, diff) =
        read_file_diff(&store, &key, &list.files[entry], &Cancel::new()).expect("diffs");
    let text: String = diff
        .hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| String::from_utf8_lossy(&line.text).into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("feature"), "head's own line: {text}");
    assert!(
        !text.contains("main-only"),
        "main's later line is not in split..head: {text}"
    );
}

#[test]
fn unrelated_histories_compare_two_dot() {
    let dir = split_repo();
    let repo = dir.path();
    git(repo, &["checkout", "-q", "--orphan", "orphan"]);
    commit(repo, "o.txt", "o\n", "orphan root");
    let store = RepoStore::new(repo.to_path_buf(), None);
    let spec = DiffSpec::Range {
        base: oid(repo, "main"),
        head: oid(repo, "orphan"),
    };
    let (view, _) =
        read_files(&store, spec, DiffOptions::default(), &Cancel::new()).expect("lists");
    assert_eq!(view.compare, Some(CompareSummary::NoSharedHistory));
}
