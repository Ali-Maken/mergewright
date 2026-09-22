//! The corpus's own guarantees: deterministic, cached, and honest about what
//! this machine cannot hold.
//!
//! Determinism is the one that everything else rests on. Snapshot tests in
//! later tickets compare object ids, so two builds of the same generator have
//! to produce the same ids — on this machine, at least, which is what
//! m1-the-graph/01 asks for.

use std::collections::BTreeMap;
use std::path::Path;

use graft_fixtures::Shape;
use graft_fixtures::git::GitError;

/// Every object id a build produced, as `<shape> <refname> <id>` lines plus
/// the whole commit list per shape.
///
/// Refs alone would miss a commit that only `HEAD` reaches, and the commit
/// list alone would miss a ref moving, so this takes both.
fn identity(root: &Path) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for shape in Shape::every() {
        let dir = root.join(shape.name());
        if !dir.join(".git").is_dir() {
            continue;
        }
        let refs = git(&dir, &["for-each-ref", "--format=%(refname) %(objectname)"]);
        // Guards the comparison below: two empty strings also match.
        assert!(
            refs.contains("refs/"),
            "no refs read from the {} fixture — the comparison would be vacuous",
            shape.name()
        );
        let commits = git(&dir, &["rev-list", "--all", "--reflog"]);
        let head = head(&dir);
        found.insert(
            shape.name().to_owned(),
            format!("{refs}\n{commits}\n{head}"),
        );
    }
    assert!(
        !found.is_empty(),
        "no fixture repositories in {}",
        root.display()
    );
    found
}

/// A read-only `git`, isolated and deadlined by the generator's own runner —
/// the same rules, not a second copy of them.
///
/// Any failure is fatal. A test that compared the output of two failed
/// commands would compare two empty strings and pass.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = graft_fixtures::git::run(dir, args).unwrap_or_else(|why| panic!("{why}"));
    String::from_utf8_lossy(&out).into_owned()
}

/// The branch `HEAD` points at, or `detached`.
///
/// `symbolic-ref -q` exits 1 when `HEAD` is detached: that is its answer, not
/// a failure. Every other way of failing still is one.
fn head(dir: &Path) -> String {
    match graft_fixtures::git::run(dir, &["symbolic-ref", "-q", "HEAD"]) {
        Ok(out) => String::from_utf8_lossy(&out).into_owned(),
        Err(GitError::Failed { status, .. }) if status.code() == Some(1) => "detached".to_owned(),
        Err(why) => panic!("{why}"),
    }
}

fn build(base: &Path) -> graft_fixtures::Report {
    graft_fixtures::build_into(base)
        .unwrap_or_else(|why| panic!("building the corpus in {}: {why}", base.display()))
}

#[test]
fn two_builds_produce_the_same_object_ids() {
    let first_base = tempfile::tempdir().expect("a temporary directory");
    let second_base = tempfile::tempdir().expect("a temporary directory");

    let first = build(first_base.path());
    let second = build(second_base.path());
    assert!(
        first.built && second.built,
        "both builds must be real builds"
    );
    assert_ne!(
        first.root, second.root,
        "the two builds must be independent"
    );

    let left = identity(&first.root);
    let right = identity(&second.root);
    for (shape, ids) in &left {
        assert_eq!(
            Some(ids),
            right.get(shape),
            "the {shape} fixture differs between two builds"
        );
    }
    assert_eq!(
        left.len(),
        right.len(),
        "the two builds hold different shapes"
    );

    eprintln!(
        "corpus: {} shapes, {} KiB, built in {:?}",
        left.len(),
        first.bytes / 1024,
        first.elapsed
    );
}

#[test]
fn a_second_build_in_the_same_place_is_a_no_op() {
    let base = tempfile::tempdir().expect("a temporary directory");
    let first = build(base.path());
    let second = build(base.path());

    assert!(first.built, "the first build must do the work");
    assert!(
        !second.built,
        "the second build must find the corpus already there"
    );
    assert_eq!(first.root, second.root, "the cache key must not change");
    assert_eq!(
        first.bytes, second.bytes,
        "a no-op must not change the corpus"
    );
}

#[test]
fn the_shared_corpus_is_reachable_and_every_shape_accounted_for() {
    let report = graft_fixtures::ensure().expect("the shared corpus");
    for shape in Shape::every() {
        match graft_fixtures::try_borrow(shape) {
            Ok(dir) => assert!(dir.is_dir(), "{shape} borrowed a path that is not there"),
            Err(why) => {
                let reason = why.to_string();
                assert!(
                    report.unavailable.iter().any(|skip| skip.shape == shape),
                    "{shape} failed without being reported unavailable: {reason}"
                );
            }
        }
    }
}

#[test]
fn the_non_utf8_ref_name_shape_exists_or_names_the_filesystem_it_cannot_live_on() {
    match graft_fixtures::try_borrow(Shape::NonUtf8RefName) {
        Ok(dir) => assert!(
            dir.join(".git").is_dir(),
            "a built fixture must be a repository"
        ),
        Err(why) => {
            let reason = why.to_string();
            // APFS rejects a file name that is not valid UTF-8, so on this
            // machine the skip is the expected outcome — but it has to say
            // which filesystem refused, never pass quietly.
            assert!(
                reason.contains(" at /"),
                "a skip must name the filesystem and its mount point: {reason}"
            );
            eprintln!("skipped: {reason}");
        }
    }
}

#[test]
fn the_shared_corpus_is_read_only() {
    let dir = graft_fixtures::borrow(Shape::Trivial);
    let head_file = dir.join(".git").join("HEAD");
    let refused = std::fs::OpenOptions::new().write(true).open(&head_file);
    let why = refused.expect_err("the shared corpus must refuse a write");
    assert_eq!(
        why.kind(),
        std::io::ErrorKind::PermissionDenied,
        "writing {} was refused for the wrong reason: {why}",
        head_file.display()
    );
}
