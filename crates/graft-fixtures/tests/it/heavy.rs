//! The heavy tier's own tests: the synthetic repository, at the size
//! PROMPT.md §7.1 asks for and at a size a person can wait for.
//!
//! Every test here is gated by `#[cfg(feature = "heavy-tests")]` alone
//! (ADR-0018). The feature is off by default and nothing turns it on, so a
//! fast-tier build does not even compile these tests.
//!
//! See `TESTING.md` for the commands and what they cost.

use std::path::Path;

use graft_fixtures::git;
use graft_fixtures::synthetic::{self, Scale};

/// Runs a read-only `git` in the fixture and returns its trimmed stdout.
fn read(repo: &Path, args: &[&str]) -> String {
    let out = git::run(repo, args).unwrap_or_else(|why| panic!("{why}"));
    String::from_utf8_lossy(&out).trim().to_owned()
}

/// Mebibytes, to one decimal place, without a float: a size is for a human
/// to read, and `u64 as f64` is a precision-loss warning for no gain.
fn mib(bytes: u64) -> String {
    const MIB: u64 = 1024 * 1024;
    format!("{}.{} MiB", bytes / MIB, (bytes % MIB) * 10 / MIB)
}

fn count(repo: &Path, args: &[&str]) -> usize {
    let out = read(repo, args);
    if out.is_empty() {
        0
    } else {
        out.lines().count()
    }
}

/// The size a person can run while working. Everything the generator does
/// at 100k it already does here — the trunk, the merges, the open lanes,
/// the tags — so this is the test that says the generator is *correct*, and
/// the one below only says it scales.
const SMALL: Scale = Scale::of(5_000);

#[test]
fn the_generator_builds_exactly_the_history_it_promises() {
    let report = synthetic::ensure(SMALL).unwrap_or_else(|why| panic!("{why}"));
    let repo = report.root.as_path();
    println!(
        "{}: {} commits, {}, {:?}{}",
        report.scale.name(),
        report.scale.commits(),
        mib(report.bytes),
        report.elapsed,
        if report.built { "" } else { " (cached)" },
    );

    // The count is the whole point: a budget measured against the wrong
    // number of commits measures nothing. `ensure` checks this too and
    // refuses to publish a fixture that fails it; this is the check saying
    // so from the outside.
    assert_eq!(
        read(repo, &["rev-list", "--count", "--all"]),
        SMALL.commits().to_string(),
    );

    // A graph, not a line. Merges, and branches with nowhere to land.
    let merges = count(repo, &["rev-list", "--all", "--merges"]);
    assert!(merges > 300, "only {merges} merges in 5,000 commits");
    let branches = count(
        repo,
        &["for-each-ref", "--format=%(refname)", "refs/heads/"],
    );
    assert!(branches > 10, "only {branches} branches");
    let tags = count(repo, &["for-each-ref", "--format=%(refname)", "refs/tags/"]);
    assert!(tags > 10, "only {tags} tags");

    // The branch the throwaway topics were built on is gone, and `main` is
    // checked out with a real working tree.
    assert_eq!(
        read(
            repo,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/heads/topic-scratch"
            ]
        ),
        "",
    );
    assert_eq!(read(repo, &["rev-parse", "--abbrev-ref", "HEAD"]), "main");
    assert!(repo.join("trunk.txt").is_file(), "no working tree");

    // Nothing of the build is left behind inside the published fixture.
    assert!(
        !repo.join("history.fi").exists(),
        "the stream was published"
    );
}

#[test]
fn the_same_scale_twice_is_the_same_repository() {
    let first = synthetic::ensure(SMALL).unwrap_or_else(|why| panic!("{why}"));
    let second = synthetic::ensure(SMALL).unwrap_or_else(|why| panic!("{why}"));
    assert_eq!(first.root, second.root);
    assert!(!second.built, "the second call rebuilt an existing fixture");
    assert_eq!(
        read(&first.root, &["rev-parse", "refs/heads/main"]),
        read(&second.root, &["rev-parse", "refs/heads/main"]),
    );
}

#[test]
fn the_realistic_variant_has_git_git_sized_commits_on_the_same_shape() {
    let plain = synthetic::borrow(SMALL).unwrap_or_else(|why| panic!("{why}"));
    let scale = SMALL.with_realistic_commits();
    let real = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));
    assert_ne!(plain, real, "the realistic variant shares the plain cache");

    // The same graph: the budget tests compare the two on history shape.
    for args in [
        &["rev-list", "--count", "--all"][..],
        &["rev-list", "--count", "--all", "--merges"][..],
    ] {
        assert_eq!(read(&plain, args), read(&real, args), "{args:?}");
    }

    // git/git (m1-the-graph/27d): mean commit object 726 bytes, median 577.
    let sizes = read(
        &real,
        &[
            "cat-file",
            "--batch-all-objects",
            "--batch-check=%(objecttype) %(objectsize)",
        ],
    );
    let commits: Vec<u64> = sizes
        .lines()
        .filter_map(|line| line.strip_prefix("commit "))
        .map(|size| size.parse().unwrap_or_else(|why| panic!("{size}: {why}")))
        .collect();
    let mean = commits.iter().sum::<u64>() / commits.len() as u64;
    assert!((500..=1000).contains(&mean), "mean commit is {mean} bytes");

    let authors = read(&real, &["log", "--all", "--format=%ae"]);
    let distinct: std::collections::HashSet<&str> = authors.lines().collect();
    assert!(distinct.len() > 500, "only {} authors", distinct.len());
}

#[test]
fn the_commit_graph_variant_is_a_separate_fixture_that_has_one() {
    let plain = synthetic::borrow(SMALL).unwrap_or_else(|why| panic!("{why}"));
    let scale = SMALL.with_commit_graph();
    let graphed = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));

    // Two directories, so ADR-0008's two first-paint paths can be measured
    // against each other rather than one clobbering the other.
    assert_ne!(plain, graphed);
    assert!(
        !plain.join(".git/objects/info/commit-graph").exists(),
        "the default fixture has a commit-graph; the budgets assume it does not",
    );
    assert!(
        graphed.join(".git/objects/info/commit-graph").exists(),
        "no commit-graph was written",
    );
    assert_eq!(
        read(&plain, &["rev-parse", "refs/heads/main"]),
        read(&graphed, &["rev-parse", "refs/heads/main"]),
        "the two variants are meant to be the same history",
    );
}

#[test]
fn the_heavy_fixture_is_the_size_prompt_md_asks_for() {
    let report = synthetic::ensure(Scale::HEAVY).unwrap_or_else(|why| panic!("{why}"));
    println!(
        "{}: {} commits, {}, {:?}{}",
        report.scale.name(),
        report.scale.commits(),
        mib(report.bytes),
        report.elapsed,
        if report.built { "" } else { " (cached)" },
    );
    assert_eq!(
        read(&report.root, &["rev-list", "--count", "--all"]),
        "100000",
    );
}

/// m1-the-graph/30d: the many-refs variant has exactly the refs it promises,
/// packed, each branch tracking its upstream through a `[branch]` section.
#[test]
fn the_many_refs_variant_packs_branches_that_each_track_an_upstream() {
    let scale = SMALL.with_refs(1_000);
    let repo = synthetic::borrow(scale).unwrap_or_else(|why| panic!("{why}"));

    assert_eq!(
        count(
            &repo,
            &["for-each-ref", "--format=%(refname)", "refs/heads/many/"]
        ),
        500
    );
    assert_eq!(
        count(
            &repo,
            &[
                "for-each-ref",
                "--format=%(refname)",
                "refs/remotes/origin/many/"
            ]
        ),
        500
    );
    assert!(
        !repo.join(".git/refs/heads/many").exists(),
        "the extra refs are packed, not loose"
    );
    assert_eq!(
        read(&repo, &["config", "--get", "branch.many/000123.merge"]),
        "refs/heads/many/000123"
    );
    assert_eq!(
        read(&repo, &["rev-parse", "refs/heads/many/000001"]),
        read(&repo, &["rev-parse", "refs/remotes/origin/many/000001"]),
        "an ordinary branch is in sync with its upstream"
    );
    assert_eq!(
        count(
            &repo,
            &[
                "rev-list",
                "refs/remotes/origin/many/000000..refs/heads/many/000000"
            ]
        ),
        1,
        "every hundredth branch is one commit ahead"
    );
    assert_eq!(
        count(&repo, &["rev-list", "--all"]),
        SMALL.commits() as usize,
        "the extra refs add no commits"
    );
    assert_ne!(scale.name(), SMALL.name(), "a separate fixture");
}
