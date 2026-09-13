//! Every fixture in the corpus, opened with `graft-git`.
//!
//! Two jobs. The obvious one is proving each shape is what its name says:
//! the ref set is written out in full, so a change to the generator that
//! quietly drops a branch fails here rather than in whatever graph test
//! happens to notice first. The less obvious one is proving the corpus is
//! reachable — `graft_fixtures::borrow` is the only way any test in the
//! workspace is allowed to get a repository (m1-the-graph/01), and this file
//! is the proof that it works for all eighteen shapes.
//!
//! Tips are counted as *distinct* target ids, not as refs. A branch level
//! with its upstream is two refs and one tip, which is exactly the
//! distinction the graph draws.

use std::collections::BTreeSet;
use std::path::Path;

use graft_core::ref_name::RefName;
use graft_fixtures::Shape;
use graft_git::repo::{HeadState, RefTarget, RepoHandle};

/// Opens the fixture for `shape` and asserts its whole ref set and tip count.
fn assert_shape(shape: Shape, tips: usize, refs: &[&str]) -> RepoHandle {
    let dir = graft_fixtures::borrow(shape);
    let repo = open(&dir);
    let entries = repo
        .references()
        .unwrap_or_else(|why| panic!("reading the refs of the {shape} fixture: {why}"));

    let found: Vec<String> = entries
        .iter()
        .map(|entry| String::from_utf8_lossy(entry.name.as_bytes()).into_owned())
        .collect();
    assert_eq!(found, refs, "ref set of the {shape} fixture");

    let distinct: BTreeSet<_> = entries
        .iter()
        .filter_map(|entry| match &entry.target {
            RefTarget::Object(id) => Some(*id),
            RefTarget::Symbolic(_) => None,
        })
        .collect();
    assert_eq!(distinct.len(), tips, "tip count of the {shape} fixture");

    repo
}

fn open(dir: &Path) -> RepoHandle {
    RepoHandle::open(dir).unwrap_or_else(|why| panic!("opening {}: {why}", dir.display()))
}

fn head(repo: &RepoHandle) -> HeadState {
    repo.head()
        .unwrap_or_else(|why| panic!("reading HEAD: {why}"))
}

fn branch(name: &str) -> HeadState {
    HeadState::Branch(ref_name(name))
}

fn ref_name(name: &str) -> RefName {
    RefName::new(name).unwrap_or_else(|why| panic!("`{name}` is not a valid ref name: {why}"))
}

#[test]
fn trivial() {
    let repo = assert_shape(Shape::Trivial, 1, &["refs/heads/main"]);
    assert_eq!(head(&repo), branch("refs/heads/main"));
}

#[test]
fn linear() {
    assert_shape(Shape::Linear, 1, &["refs/heads/main"]);
}

#[test]
fn feature_branches() {
    assert_shape(
        Shape::FeatureBranches,
        4,
        &[
            "refs/heads/feature/one",
            "refs/heads/feature/three",
            "refs/heads/feature/two",
            "refs/heads/main",
        ],
    );
}

#[test]
fn octopus_merge() {
    assert_shape(
        Shape::OctopusMerge,
        4,
        &[
            "refs/heads/main",
            "refs/heads/topic-1",
            "refs/heads/topic-2",
            "refs/heads/topic-3",
        ],
    );
}

#[test]
fn criss_cross_merge() {
    assert_shape(
        Shape::CrissCrossMerge,
        3,
        &["refs/heads/alpha", "refs/heads/beta", "refs/heads/main"],
    );
}

#[test]
fn foxtrot_merge() {
    assert_shape(
        Shape::FoxtrotMerge,
        2,
        &["refs/heads/feature", "refs/heads/main"],
    );
}

#[test]
fn backward_merge() {
    assert_shape(
        Shape::BackwardMerge,
        2,
        &["refs/heads/feature", "refs/heads/main"],
    );
}

#[test]
fn evil_merge() {
    assert_shape(
        Shape::EvilMerge,
        2,
        &["refs/heads/alpha", "refs/heads/main"],
    );
}

#[test]
fn accidental_pull_merge() {
    assert_shape(
        Shape::AccidentalPullMerge,
        2,
        &["refs/heads/main", "refs/remotes/origin/main"],
    );
}

#[test]
fn orphan_branch() {
    assert_shape(
        Shape::OrphanBranch,
        2,
        &["refs/heads/docs", "refs/heads/main"],
    );
}

#[test]
fn cherry_pick_duplicates() {
    assert_shape(
        Shape::CherryPickDuplicates,
        2,
        &["refs/heads/main", "refs/heads/topic"],
    );
}

#[test]
fn tags() {
    assert_shape(
        Shape::Tags,
        5,
        &[
            "refs/heads/main",
            "refs/tags/v0.1",
            "refs/tags/v0.2",
            "refs/tags/v1.0",
            "refs/tags/v2.0",
        ],
    );
}

#[test]
fn unicode_names() {
    assert_shape(
        Shape::UnicodeNames,
        5,
        &[
            "refs/heads/fonctionnalité",
            "refs/heads/main",
            "refs/heads/ветка",
            "refs/heads/機能",
            "refs/tags/версия-1.0",
        ],
    );
}

#[test]
fn ref_states() {
    // Seven refs, six tips: `main` and `origin/main` are the branch that is
    // level with its upstream, so they share an id.
    assert_shape(
        Shape::RefStates,
        6,
        &[
            "refs/heads/ahead",
            "refs/heads/behind",
            "refs/heads/main",
            "refs/remotes/origin/ahead",
            "refs/remotes/origin/behind",
            "refs/remotes/origin/main",
            "refs/tags/archived",
        ],
    );
}

#[test]
fn detached_head() {
    let repo = assert_shape(Shape::DetachedHead, 1, &["refs/heads/main"]);
    assert!(
        matches!(head(&repo), HeadState::Detached(_)),
        "the detached-head fixture must have a detached HEAD"
    );
}

#[test]
fn unborn_branch() {
    let repo = assert_shape(
        Shape::UnbornBranch,
        2,
        &["refs/heads/main", "refs/heads/other"],
    );
    assert_eq!(head(&repo), HeadState::Unborn(ref_name("refs/heads/fresh")));
}

#[test]
fn stash() {
    let repo = assert_shape(Shape::Stash, 2, &["refs/heads/main", "refs/stash"]);
    assert_eq!(head(&repo), branch("refs/heads/main"));
}

#[test]
fn non_utf8_path() {
    // A path lives inside a tree object, so every filesystem can hold this
    // one — it is the half of the non-UTF-8 coverage that is not skippable.
    let dir = graft_fixtures::borrow(Shape::NonUtf8Path);
    assert_shape(
        Shape::NonUtf8Path,
        2,
        &["refs/heads/latin1", "refs/heads/main"],
    );
    let listed = graft_fixtures::git::run(&dir, &["ls-tree", "-r", "-z", "--name-only", "latin1"])
        .unwrap_or_else(|why| panic!("listing the non-utf8-path fixture's tree: {why}"));
    assert!(
        listed
            .split(|byte| *byte == 0)
            .any(|path| !path.is_empty() && std::str::from_utf8(path).is_err()),
        "the non-utf8-path fixture must hold a path that is not UTF-8"
    );
}

#[test]
fn non_utf8_ref_name() {
    // The one shape a machine may legitimately not be able to hold: a loose
    // ref is a file. On APFS it must report a skip that names the
    // filesystem, never pass quietly (m1-the-graph/01).
    match graft_fixtures::try_borrow(Shape::NonUtf8RefName) {
        Ok(dir) => {
            let repo = open(&dir);
            let entries = repo
                .references()
                .unwrap_or_else(|why| panic!("reading the refs of the non-utf8 fixture: {why}"));
            let names: Vec<_> = entries
                .iter()
                .map(|entry| entry.name.as_bytes().to_vec())
                .collect();
            assert!(
                names.iter().any(|name| std::str::from_utf8(name).is_err()),
                "the non-utf8-ref-name fixture must hold a ref name that is not UTF-8"
            );
        }
        Err(why) => {
            let reason = why.to_string();
            assert!(
                reason.contains("not valid UTF-8"),
                "a skip must state why: {reason}"
            );
            eprintln!("skipped: {reason}");
        }
    }
}

/// A test that writes gets its own copy; the shared corpus stays clean.
#[test]
fn a_scratch_copy_is_writable_and_separate() {
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let shared = graft_fixtures::borrow(Shape::Trivial);
    assert_ne!(scratch.path(), shared.as_path());

    let repo = open(scratch.path());
    assert_eq!(head(&repo), branch("refs/heads/main"));

    std::fs::write(scratch.path().join("scribble.txt"), b"written\n").expect("writing a scratch");
    assert!(!shared.join("scribble.txt").exists());
}

/// The one shape where commit-date order is not topological: the fast-clock
/// commit comes second by date and last-but-one topologically, so the two
/// walks the ADR-0008 promotion swaps between really do differ here.
#[test]
fn skewed_dates() {
    use graft_git::history::{CHUNK, Order, Walk};

    let repo = assert_shape(
        Shape::SkewedDates,
        2,
        &["refs/heads/feature", "refs/heads/main"],
    );
    let subjects = |mut walk: Walk<'_>| {
        let mut subjects = Vec::new();
        loop {
            let chunk = walk.next_chunk(CHUNK, &|| false).expect("walks");
            subjects.extend(
                chunk
                    .commits
                    .iter()
                    .map(|commit| String::from_utf8_lossy(commit.subject()).into_owned()),
            );
            if chunk.done {
                return subjects;
            }
        }
    };
    let by_date = subjects(Walk::all_refs(&repo, Order::CommitDate).expect("starts"));
    let tips = repo.commit_tips().expect("tips");
    let topological = subjects(Walk::full_topological(&repo, &tips).expect("starts"));
    let fast = "Commit from a fast clock";
    let row = |order: &[_]| order.iter().position(|subject| *subject == fast);
    assert_eq!(row(&by_date), Some(1), "{by_date:?}");
    assert_eq!(row(&topological), Some(13), "{topological:?}");
}
