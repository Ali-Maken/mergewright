//! Absorb (M7 absorb-map): each staged hunk goes to the commit on his branch
//! that last changed its lines, and one that fits none stays staged, with
//! the reason. The oracle, which plans, applies and judges the result by the
//! trees git holds afterwards, is `absorb_oracle.rs`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_ops::absorb::{AbsorbMap, StagedHunk, StaysWhy, absorb_map};
use graft_ops::facts::{FactsError, read_facts};
use graft_ops::operation::{FactsScope, Operation, Refusal, Tip};
use graft_ops::rewrite::{RewriteError, RewriteParams, RewriteRequest, plan};

use crate::support::{NOW, Never, TempRepo, git_version, run_undo, state_of, try_run};

// ── The branch every test stands on ─────────────────────────────────────

/// `main`: `m1` (`old.txt`). `feature`, `HEAD`: `f1` makes `a.txt`, `f2`
/// makes `b.txt`, `f3` changes `a.txt`'s last line.
pub(crate) fn branch() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("old.txt", b"o1\no2\no3\no4\no5\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("a.txt", b"a1\na2\na3\na4\na5\n");
    repo.commit_all("f1");
    repo.write("b.txt", b"b1\nb2\nb3\nb4\nb5\n");
    repo.commit_all("f2");
    repo.write("a.txt", b"a1\na2\na3\na4\na5 changed\n");
    repo.commit_all("f3");
    repo
}

pub(crate) fn stage(repo: &TempRepo, path: &str, bytes: &[u8]) {
    repo.write(path, bytes);
    repo.git(&["add", "--", path]);
}

pub(crate) fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a commit id")
}

fn tip(repo: &TempRepo) -> Tip {
    Tip {
        name: "refs/heads/feature".to_owned(),
        oid: oid(repo, "feature"),
    }
}

fn map_with(repo: &TempRepo, stop: &dyn Stop) -> Result<AbsorbMap, FactsError> {
    absorb_map(
        &repo.open(),
        &git_version(),
        &tip(repo),
        oid(repo, "main"),
        stop,
    )
}

pub(crate) fn map(repo: &TempRepo) -> AbsorbMap {
    map_with(repo, &Never).unwrap_or_else(|why| panic!("the map reads: {why}"))
}

pub(crate) fn hunk(path: &str, old: std::ops::Range<u32>, new: std::ops::Range<u32>) -> StagedHunk {
    StagedHunk {
        path: PathBuf::from(path),
        old,
        new,
    }
}

pub(crate) fn planned(repo: &TempRepo) -> Result<RewriteParams, RewriteError> {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    plan(
        &handle,
        &git_version(),
        &RewriteRequest::Absorb,
        &facts,
        None,
        &Never,
    )
}

struct Always;

impl Stop for Always {
    fn should_stop(&self) -> bool {
        true
    }
}

// ── Placing each hunk ───────────────────────────────────────────────────

#[test]
fn a_hunk_goes_to_the_commit_that_last_changed_its_lines() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 3..4, 3..4), oid(&repo, "feature~1"))]
    );
    assert_eq!(map.stays, vec![]);
}

#[test]
fn an_insertion_between_one_commits_lines_goes_there() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nnew\nb3\nb4\nb5\n");

    let map = map(&repo);

    // git's header for it: `@@ -2,0 +3 @@`, after line 2.
    assert_eq!(
        map.placed,
        vec![(hunk("b.txt", 2..2, 3..4), oid(&repo, "feature~1"))]
    );
    assert_eq!(map.stays, vec![]);
}

#[test]
fn an_insertion_at_the_top_of_a_file_stays() {
    // No line above it: the rule needs both neighbours from one commit.
    let repo = branch();
    stage(&repo, "b.txt", b"new\nb1\nb2\nb3\nb4\nb5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("b.txt", 0..0, 1..2), StaysWhy::NoCommitChangedIt)]
    );
}

#[test]
fn a_hunk_over_two_commits_stays() {
    let repo = branch();
    // a4 is f1's, a5 is f3's.
    stage(&repo, "a.txt", b"a1\na2\na3\nA4\nA5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("a.txt", 4..6, 4..6), StaysWhy::TwoCommitsChangedIt)]
    );
}

#[test]
fn a_hunk_on_lines_older_than_the_branch_stays() {
    let repo = branch();
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("old.txt", 3..4, 3..4), StaysWhy::BeforeYourBranch)]
    );
}

#[test]
fn a_new_file_stays() {
    let repo = branch();
    stage(&repo, "new.txt", b"n1\nn2\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("new.txt", 0..0, 1..3), StaysWhy::NewFile)]
    );
}

#[test]
fn a_hunk_that_does_not_commute_stays() {
    // `d1` is f1's, but f2 changed the line right below it: put into f1,
    // the change would meet f2's on replay.
    let repo = TempRepo::init();
    repo.write("old.txt", b"o\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("d.txt", b"d1\nd2\nd3\n");
    repo.commit_all("f1");
    repo.write("d.txt", b"d1\nd2 later\nd3\n");
    repo.commit_all("f2");
    stage(&repo, "d.txt", b"D1\nd2 later\nd3\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("d.txt", 1..2, 1..2), StaysWhy::TwoCommitsChangedIt)]
    );
}

/// `main`: `m1` makes `c.txt`; `feature`: `f1` changes its second line.
fn one_line_on_the_branch() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("c.txt", b"c1\nc2\nc3\nc4\n");
    repo.commit_all("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("c.txt", b"c1\nc2 f1\nc3\nc4\n");
    repo.commit_all("f1");
    repo
}

#[test]
fn a_hunk_over_the_branch_and_older_lines_stays() {
    // Line 1 is older than the branch, line 2 is f1's: not "all older", so
    // not `BeforeYourBranch`; two commits, so not placed.
    let repo = one_line_on_the_branch();
    stage(&repo, "c.txt", b"C1\nC2\nc3\nc4\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("c.txt", 1..3, 1..3), StaysWhy::TwoCommitsChangedIt)]
    );
}

#[test]
fn an_insertion_below_a_branch_line_and_above_an_older_one_stays() {
    // Above: f1's line 2. Below: m1's line 3. Both neighbours must be one
    // commit's; judged by the line above alone it would go into f1.
    let repo = one_line_on_the_branch();
    stage(&repo, "c.txt", b"c1\nc2 f1\nnew\nc3\nc4\n");

    let map = map(&repo);

    assert_eq!(map.placed, vec![]);
    assert_eq!(
        map.stays,
        vec![(hunk("c.txt", 2..2, 3..4), StaysWhy::TwoCommitsChangedIt)]
    );
}

#[test]
fn absorb_with_nothing_placed_is_refused() {
    let repo = branch();
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");

    let refused = planned(&repo);

    let Err(RewriteError::Refused(refusal)) = refused else {
        panic!("refused, got {refused:?}");
    };
    assert_eq!(refusal, Refusal::NothingToAbsorb);
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "none of your staged lines belong to a commit on your branch · c commits them"
    );
}

#[test]
fn absorb_map_writes_no_ref_index_or_file() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "a.txt", b"a1\nA2\na3\na4\na5 changed\n");
    repo.write("old.txt", b"unsaved\n");
    let before = state_of(&repo);
    let git_files_before = git_dir_files(&repo);

    map(&repo);
    planned(&repo).expect("it plans");

    assert_eq!(state_of(&repo), before);
    assert_eq!(
        git_dir_files(&repo),
        git_files_before,
        "no file left in .git"
    );
}

#[test]
fn absorb_map_honours_stop() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");

    let stopped = map_with(&repo, &Always);

    assert!(matches!(stopped, Err(FactsError::Stopped)), "{stopped:?}");
}

#[test]
fn undo_after_absorb_restores_index_and_commits() {
    let repo = branch();
    stage(&repo, "b.txt", b"b1\nb2\nB3\nb4\nb5\n");
    stage(&repo, "old.txt", b"o1\no2\nO3\no4\no5\n");
    let before = state_of(&repo);

    let params = planned(&repo).expect("it plans");
    try_run(&repo, &Operation::Rewrite(params), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_ne!(state_of(&repo), before, "the absorb changed something");
    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("{why:?}"));

    assert_eq!(state_of(&repo), before);
}

/// Every file under `.git` except the ones any git command rewrites
/// (`logs/`, the journal's directory is Graft's and not touched by a read).
fn git_dir_files(repo: &TempRepo) -> Vec<String> {
    fn walk(root: &Path, dir: &Path, found: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("the directory reads") {
            let path = entry.expect("the entry reads").path();
            let relative = path
                .strip_prefix(root)
                .expect("under the root")
                .to_string_lossy()
                .into_owned();
            // New objects are allowed: the plan writes the commits it will
            // put in (as `a` into an older commit does).
            if relative.starts_with("objects") {
                continue;
            }
            if path.is_dir() {
                walk(root, &path, found);
            } else {
                found.push(relative);
            }
        }
    }
    let root = repo.path().join(".git");
    let mut found = Vec::new();
    walk(&root, &root, &mut found);
    found.sort();
    found
}
