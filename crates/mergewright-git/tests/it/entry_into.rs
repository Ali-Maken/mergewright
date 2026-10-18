//! `entry_into`: where a commit entered a line's first-parent chain
//! (ease-pass 01r: "merged into" and "on its line" for the right pane).

use std::path::Path;
use std::sync::atomic::AtomicBool;

use mergewright_git::history_reads::{LineEntry, entry_into};
use mergewright_git::run::RunError;

use crate::history_reads::{git_ok, init, open, rev_parse, scratch, write_and_commit};

fn commit(dir: &Path, name: &str) {
    write_and_commit(dir, &format!("{name}.txt"), name, name);
}

fn merge(dir: &Path, from: &str, message: &str) {
    git_ok(dir, &["merge", "-q", "--no-ff", "-m", message, from]);
}

/// `main m1`, `sara s1 s2` off m1, merged `--no-ff` into main, then m2 on
/// main: the entry is the merge, whatever came after it.
#[test]
fn a_direct_merge_is_the_entry() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    commit(dir, "m1");
    let m1 = rev_parse(dir, "HEAD");
    git_ok(dir, &["switch", "-q", "-c", "sara"]);
    commit(dir, "s1");
    commit(dir, "s2");
    let sara = rev_parse(dir, "sara");
    git_ok(dir, &["switch", "-q", "main"]);
    merge(dir, "sara", "merge sara");
    let merged = rev_parse(dir, "HEAD");
    commit(dir, "m2");
    let main = rev_parse(dir, "main");

    let entry = entry_into(&open(dir), sara, main, &()).expect("rev-list runs");
    assert_eq!(
        entry,
        Some(LineEntry {
            commit: merged,
            parents: vec![m1, sara],
        })
    );
}

/// `foo` left at m1 while main gets m2 m3: the entry is m2, whose first
/// parent is `foo` itself.
#[test]
fn a_fast_forward_enters_at_the_commit_on_top() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    commit(dir, "m1");
    let m1 = rev_parse(dir, "HEAD");
    commit(dir, "m2");
    let m2 = rev_parse(dir, "HEAD");
    commit(dir, "m3");
    let main = rev_parse(dir, "HEAD");

    let entry = entry_into(&open(dir), m1, main, &()).expect("rev-list runs");
    assert_eq!(
        entry,
        Some(LineEntry {
            commit: m2,
            parents: vec![m1],
        })
    );
}

/// sara → wallet → main, both `--no-ff`: main's line took sara in by its
/// merge of wallet, not by wallet's merge of sara.
#[test]
fn a_transitive_merge_enters_at_the_lines_own_merge() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    commit(dir, "m1");
    git_ok(dir, &["switch", "-q", "-c", "wallet"]);
    commit(dir, "w1");
    git_ok(dir, &["switch", "-q", "-c", "sara"]);
    commit(dir, "s1");
    let sara = rev_parse(dir, "sara");
    git_ok(dir, &["switch", "-q", "wallet"]);
    commit(dir, "w2");
    merge(dir, "sara", "merge sara");
    let wallet = rev_parse(dir, "wallet");
    git_ok(dir, &["switch", "-q", "main"]);
    commit(dir, "m2");
    let m2 = rev_parse(dir, "HEAD");
    merge(dir, "wallet", "merge wallet");
    let into_main = rev_parse(dir, "HEAD");
    commit(dir, "m3");
    let main = rev_parse(dir, "main");

    let entry = entry_into(&open(dir), sara, main, &()).expect("rev-list runs");
    assert_eq!(
        entry,
        Some(LineEntry {
            commit: into_main,
            parents: vec![m2, wallet],
        })
    );
}

#[test]
fn none_when_the_line_is_the_commit_or_does_not_hold_it() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    commit(dir, "m1");
    git_ok(dir, &["switch", "-q", "-c", "side"]);
    commit(dir, "x1");
    let side = rev_parse(dir, "side");
    git_ok(dir, &["switch", "-q", "main"]);
    commit(dir, "m2");
    let main = rev_parse(dir, "main");
    let repo = open(dir);

    assert_eq!(entry_into(&repo, main, main, &()).expect("runs"), None);
    // Diverged: main does not hold side, nor side main.
    assert_eq!(entry_into(&repo, side, main, &()).expect("runs"), None);
    // main's tip is a descendant of m1, but m1 does not hold main.
    let m1 = rev_parse(dir, "main~1");
    assert_eq!(entry_into(&repo, main, m1, &()).expect("runs"), None);
}

#[test]
fn entry_into_honours_stop() {
    let scratch = scratch();
    let dir = scratch.path();
    init(dir);
    commit(dir, "m1");
    let m1 = rev_parse(dir, "HEAD");
    commit(dir, "m2");
    let main = rev_parse(dir, "HEAD");
    let stopped = AtomicBool::new(true);
    assert!(matches!(
        entry_into(&open(dir), m1, main, &stopped),
        Err(RunError::Stopped)
    ));
}
