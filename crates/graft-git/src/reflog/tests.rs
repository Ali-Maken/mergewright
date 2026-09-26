#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::fixture::Fixture;

fn oid(hex: &str) -> Oid {
    Oid::from_hex(hex).expect("a hex id")
}

/// One reflog line as git writes it: `<old> <new> <who> <when> <tz>\t<msg>`.
fn line(old: &str, new: &str, at: i64, message: &str) -> String {
    format!("{old} {new} Graft Fixture <fixture@example.invalid> {at} +0000\t{message}\n")
}

/// A repository whose `HEAD`, `main` and `feature` logs are written by hand
/// with distinct times, so the merge order is observable: the fixture's
/// fixed clock would give every record git writes the same second.
fn hand_written_logs() -> (Fixture, String, String) {
    let fixture = Fixture::with_two_commits();
    fixture.git(&["branch", "feature"]);
    let first = fixture.rev_parse("HEAD~1");
    let second = fixture.rev_parse("HEAD");
    let zero = "0".repeat(40);
    let logs = fixture.root().join(".git/logs");
    std::fs::write(
        logs.join("HEAD"),
        [
            line(&zero, &first, 100, "commit (initial): first"),
            line(&first, &second, 300, "commit: second"),
            line(
                &second,
                &second,
                500,
                "checkout: moving from main to feature",
            ),
        ]
        .concat(),
    )
    .expect("HEAD's log is written");
    std::fs::write(
        logs.join("refs/heads/main"),
        [
            line(&zero, &first, 100, "commit (initial): first"),
            line(&first, &second, 300, "commit: second"),
        ]
        .concat(),
    )
    .expect("main's log is written");
    std::fs::write(
        logs.join("refs/heads/feature"),
        line(&zero, &second, 400, "branch: Created from HEAD"),
    )
    .expect("feature's log is written");
    (fixture, first, second)
}

#[test]
fn records_reads_head_and_branches_newest_first() {
    let (fixture, first, second) = hand_written_logs();
    let repo = RepoHandle::open(fixture.root()).expect("it opens");

    let read = records(&repo, 100).expect("the logs read");

    let seen: Vec<(&str, i64, &str)> = read
        .iter()
        .map(|record| (record.refname.as_str(), record.at, record.subject.as_str()))
        .collect();
    assert_eq!(
        seen,
        vec![
            ("HEAD", 500, "checkout: moving from main to feature"),
            ("refs/heads/feature", 400, "branch: Created from HEAD"),
            // A tie in time: HEAD's record first, then the branch's.
            ("HEAD", 300, "commit: second"),
            ("refs/heads/main", 300, "commit: second"),
            ("HEAD", 100, "commit (initial): first"),
            ("refs/heads/main", 100, "commit (initial): first"),
        ]
    );
    // git's zero id is `None`; every other value is read exactly.
    assert_eq!(read[1].old, None);
    assert_eq!(read[1].new, Some(oid(&second)));
    assert_eq!(read[2].old, Some(oid(&first)));
    assert_eq!(read[2].new, Some(oid(&second)));
}

/// The same read against logs real git wrote: every record of the fixture's
/// two commits, `HEAD`'s and `main`'s, with git's own subjects.
#[test]
fn records_reads_what_git_wrote() {
    let fixture = Fixture::with_two_commits();
    let repo = RepoHandle::open(fixture.root()).expect("it opens");

    let read = records(&repo, 100).expect("the logs read");

    let mut seen: Vec<(String, String)> = read
        .iter()
        .map(|record| (record.refname.clone(), record.subject.clone()))
        .collect();
    seen.sort();
    assert_eq!(
        seen,
        vec![
            ("HEAD".to_owned(), "commit (initial): first".to_owned()),
            ("HEAD".to_owned(), "commit: second".to_owned()),
            (
                "refs/heads/main".to_owned(),
                "commit (initial): first".to_owned()
            ),
            ("refs/heads/main".to_owned(), "commit: second".to_owned()),
        ]
    );
}

#[test]
fn a_missing_log_is_empty() {
    // No commit, so no log at all.
    let unborn = Fixture::empty_repo();
    let repo = RepoHandle::open(unborn.root()).expect("it opens");
    assert_eq!(records(&repo, 100).expect("nothing to read"), vec![]);

    // A branch whose log file is gone reads as no records, not an error.
    let fixture = Fixture::with_two_commits();
    fixture.git(&["branch", "feature"]);
    std::fs::remove_file(fixture.root().join(".git/logs/refs/heads/feature"))
        .expect("the log is removed");
    let repo = RepoHandle::open(fixture.root()).expect("it opens");
    let read = records(&repo, 100).expect("the rest reads");
    assert!(
        read.iter()
            .all(|record| record.refname != "refs/heads/feature"),
        "{read:?}"
    );
    assert_eq!(read.len(), 4, "HEAD's two and main's two: {read:?}");
}

#[test]
fn records_caps() {
    let (fixture, _, _) = hand_written_logs();
    let repo = RepoHandle::open(fixture.root()).expect("it opens");

    let read = records(&repo, 3).expect("the logs read");

    let times: Vec<i64> = read.iter().map(|record| record.at).collect();
    assert_eq!(times, vec![500, 400, 300], "the newest three overall");
    assert_eq!(records(&repo, 0).expect("reads"), vec![]);
}

/// Remote-tracking and tag logs are not read: a fetch loses nothing.
#[test]
fn a_remote_tracking_log_is_not_read() {
    let fixture = Fixture::with_two_commits();
    fixture.git(&[
        "update-ref",
        "-m",
        "fetch",
        "refs/remotes/origin/main",
        "HEAD",
    ]);
    let repo = RepoHandle::open(fixture.root()).expect("it opens");

    let read = records(&repo, 100).expect("the logs read");

    assert!(
        read.iter()
            .all(|record| record.refname == "HEAD" || record.refname.starts_with("refs/heads/")),
        "{read:?}"
    );
}

#[test]
fn first_parent_distance_counts_back_along_the_first_parent() {
    let fixture = Fixture::with_two_commits();
    fixture.commit("third");
    let repo = RepoHandle::open(fixture.root()).expect("it opens");
    let tip = oid(&fixture.rev_parse("HEAD"));
    let root = oid(&fixture.rev_parse("HEAD~2"));

    assert_eq!(
        first_parent_distance(&repo, tip, root, 100).expect("reads"),
        Some(2)
    );
    assert_eq!(
        first_parent_distance(&repo, tip, tip, 100).expect("reads"),
        Some(0)
    );
    // Not an ancestor: the root's chain never reaches the tip.
    assert_eq!(
        first_parent_distance(&repo, root, tip, 100).expect("reads"),
        None
    );
    // Past the limit.
    assert_eq!(
        first_parent_distance(&repo, tip, root, 1).expect("reads"),
        None
    );
}
