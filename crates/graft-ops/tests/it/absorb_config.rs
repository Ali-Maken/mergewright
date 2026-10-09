//! Absorb against his config and odd entries (review m7-06 M1, M4, M5):
//! `blame.ignoreRevsFile`, `apply.whitespace` and a staged submodule bump
//! must not change where a hunk goes, or fail the map.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use graft_ops::absorb::StaysWhy;
use graft_ops::operation::Operation;

use crate::absorb::{branch, hunk, map, oid, planned, stage};
use crate::support::{NOW, try_run};

// ── M1: blame.ignoreRevsFile ────────────────────────────────────────────

#[test]
fn absorb_ignores_a_configured_ignore_revs_file_that_is_missing() {
    // The common case: the setting is global, and this repository has no
    // such file. git's blame fails outright on it.
    let repo = branch();
    repo.git(&["config", "blame.ignoreRevsFile", ".git-blame-ignore-revs"]);
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 3..4, 3..4), oid(&repo, "feature~1"))]
    );
}

#[test]
fn absorb_ignores_a_configured_ignore_revs_file_that_is_present() {
    // `reformat` last changed line 2; listed in the file, blame would pass
    // it by to f1, and the hunk would stay.
    let repo = branch();
    repo.write("r.txt", b"r1\nr2\nr3\n");
    repo.commit_all("r1");
    repo.write("r.txt", b"r1\nR2 formatted\nr3\n");
    repo.commit_all("reformat");
    let reformat = oid(&repo, "HEAD");
    repo.write(".git/ignore-revs", format!("{reformat}\n").as_bytes());
    repo.git(&["config", "blame.ignoreRevsFile", ".git/ignore-revs"]);
    stage(&repo, "r.txt", b"r1\nR2 formatted more\nr3\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![(hunk("r.txt", 2..3, 2..3), reformat)]);
    assert_eq!(map.stays, vec![]);
}

// ── M4: a staged submodule bump ─────────────────────────────────────────

#[test]
fn a_staged_submodule_bump_stays_and_the_rest_is_placed() {
    let repo = branch();
    let first = "1".repeat(40);
    let second = "2".repeat(40);
    repo.git(&[
        "update-index",
        "--add",
        "--cacheinfo",
        &format!("160000,{first},sub"),
    ]);
    repo.git(&["commit", "-q", "-m", "f4 adds sub"]);
    repo.git(&[
        "update-index",
        "--cacheinfo",
        &format!("160000,{second},sub"),
    ]);
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 3..4, 3..4), oid(&repo, "feature~2"))]
    );
    assert_eq!(
        map.stays,
        vec![(hunk("sub", 0..0, 0..0), StaysWhy::Submodule)]
    );
}

// ── M5: apply.whitespace ────────────────────────────────────────────────

/// `B3` with trailing spaces staged; after the absorb, f2 holds exactly
/// what was staged, spaces and all.
fn whitespace_is_absorbed_as_staged(setting: &str) {
    let repo = branch();
    repo.git(&["config", "apply.whitespace", setting]);
    stage(&repo, "b.txt", b"b1\nb2\nB3   \nb4\nb5\n");
    let staged = repo.git(&["write-tree"]);

    let params = planned(&repo).unwrap_or_else(|why| panic!("it plans: {why}"));
    try_run(&repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        repo.git(&["rev-parse", "HEAD^{tree}"]),
        staged,
        "HEAD holds what was staged"
    );
    assert_eq!(repo.read("b.txt"), b"b1\nb2\nB3   \nb4\nb5\n".to_vec());
    assert_eq!(repo.git(&["status", "--porcelain"]), "");
}

#[test]
fn apply_whitespace_fix_does_not_change_what_is_absorbed() {
    whitespace_is_absorbed_as_staged("fix");
}

#[test]
fn apply_whitespace_error_does_not_fail_the_map() {
    whitespace_is_absorbed_as_staged("error");
}

// ── 06e M3: apply.whitespace in the put-back ────────────────────────────

/// A b.txt hunk is placed; `old.txt` staged as `o2   ` (trailing spaces)
/// stays, older than the branch. After the absorb it is still staged,
/// spaces and all: the put-back's `stash apply --index` must not read his
/// `apply.whitespace` (review 06c M3, probe p5).
fn a_stay_with_trailing_spaces_stays_staged(setting: &str) {
    let repo = branch();
    repo.git(&["config", "apply.whitespace", setting]);
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "old.txt", b"o1\no2   \no3\no4\no5\n");
    let staged_old = repo.git(&["rev-parse", ":old.txt"]);

    let params = planned(&repo).unwrap_or_else(|why| panic!("it plans: {why}"));
    try_run(&repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        repo.git(&["rev-parse", ":old.txt"]),
        staged_old,
        "the stay is staged as it was"
    );
    assert_eq!(repo.read("old.txt"), b"o1\no2   \no3\no4\no5\n".to_vec());
}

#[test]
fn a_stay_with_trailing_spaces_stays_staged_under_apply_whitespace_fix() {
    a_stay_with_trailing_spaces_stays_staged("fix");
}

#[test]
fn a_stay_with_trailing_spaces_stays_staged_under_apply_whitespace_error() {
    a_stay_with_trailing_spaces_stays_staged("error");
}
