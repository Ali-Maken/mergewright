//! The corpus's own guarantees: deterministic, cached, and honest about what
//! this machine cannot hold.
//!
//! Determinism is the one that everything else rests on. Snapshot tests in
//! later tickets compare object ids, so two builds of the same generator have
//! to produce the same ids — on this machine, at least, which is what
//! m1-the-graph/01 asks for.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use graft_fixtures::Shape;

/// Every object id a build produced, as `<shape> <refname> <id>` lines plus
/// the whole commit list per shape.
///
/// Refs alone would miss a commit that only `HEAD` reaches, and the commit
/// list alone would miss a ref moving, so this takes both.
fn identity(root: &Path) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();
    for shape in Shape::ALL {
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
        let head = git(&dir, &["symbolic-ref", "-q", "HEAD"]);
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

/// A read-only `git`, isolated the same way the generator's is.
///
/// `symbolic-ref -q` exits 1 on a detached `HEAD`, so a non-zero status is an
/// answer here, not a failure; the caller compares the output either way.
fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .args(args)
        .current_dir(dir)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("LC_ALL", "C")
        .output()
        .unwrap_or_else(|why| {
            panic!(
                "running `git {}` in {}: {why}",
                args.join(" "),
                dir.display()
            )
        });
    String::from_utf8_lossy(&out.stdout).into_owned()
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
    for shape in Shape::ALL {
        match graft_fixtures::try_borrow(*shape) {
            Ok(dir) => assert!(dir.is_dir(), "{shape} borrowed a path that is not there"),
            Err(why) => {
                let reason = why.to_string();
                assert!(
                    report.unavailable.iter().any(|skip| skip.shape == *shape),
                    "{shape} failed without being reported unavailable: {reason}"
                );
            }
        }
    }
}

#[test]
fn the_non_utf8_shape_exists_or_names_the_filesystem_it_cannot_live_on() {
    match graft_fixtures::try_borrow(Shape::NonUtf8Names) {
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
