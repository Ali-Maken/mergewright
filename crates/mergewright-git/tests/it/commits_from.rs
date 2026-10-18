//! `commits_from`: the newest commits reachable from a tip, with no lower
//! bound (ease-pass 01: the trunk's last commit, and `Who` for a branch
//! with no base).

use std::sync::atomic::AtomicBool;

use mergewright_git::history_reads::commits_from;
use mergewright_git::run::RunError;

use crate::history_reads::{init, open, rev_parse, scratch, write_and_commit};

#[test]
fn commits_from_reaches_the_root_and_caps_to_the_commits_nearest_the_tip() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "f.txt", "0\n", "c0");
    let c0 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "1\n", "c1");
    let c1 = rev_parse(dir, "HEAD");
    write_and_commit(dir, "f.txt", "2\n", "c2");
    let c2 = rev_parse(dir, "HEAD");
    let repo = open(dir);

    // Uncapped: every commit down to the root (the scratch copy's own
    // history lies under c0), oldest first.
    let all = commits_from(&repo, c2, 1_000, &()).expect("rev-list runs");
    assert!(!all.capped);
    let ids: Vec<_> = all.commits.iter().map(|c| c.id).collect();
    assert!(ids.ends_with(&[c0, c1, c2]), "{ids:?}");
    assert_eq!(all.commits[0].parents, [], "the first one is the root");

    // Capped: the commits nearest the tip, still oldest first.
    let near = commits_from(&repo, c2, 2, &()).expect("rev-list runs");
    assert!(near.capped);
    let ids: Vec<_> = near.commits.iter().map(|c| c.id).collect();
    assert_eq!(ids, [c1, c2]);
    assert_eq!(near.commits[1].subject, "c2");

    // A root tip: that one commit, nothing capped.
    let root_id = all.commits[0].id;
    let root = commits_from(&repo, root_id, 1, &()).expect("rev-list runs");
    assert!(!root.capped);
    let ids: Vec<_> = root.commits.iter().map(|c| c.id).collect();
    assert_eq!(ids, [root_id]);
}

#[test]
fn commits_from_honours_stop() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    write_and_commit(dir, "f.txt", "0\n", "c0");
    let c0 = rev_parse(dir, "HEAD");
    let stopped = AtomicBool::new(true);
    assert!(matches!(
        commits_from(&open(dir), c0, 1, &stopped),
        Err(RunError::Stopped)
    ));
}
