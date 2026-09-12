//! Tests for ADR-0001's precedence.
//!
//! Each rule is asserted in a repository where every rule *below* it would
//! give a different answer, which is the only way to show the order is real
//! rather than accidental. Some of them need a repository state no corpus
//! shape carries — `origin/HEAD`, a local `init.defaultBranch`, a project
//! that calls its default branch neither `main` nor `master` — so those run
//! against a writable copy of a corpus fixture rather than an invented
//! repository.

use std::path::Path;

use graft_core::ref_name::RefName;
use graft_fixtures::{Scratch, Shape};

use super::{Branches, Chosen};
use crate::fixture::Fixture;
use crate::repo::{RepoHandle, SharedRepo};

fn git(repo: &Path, args: &[&str]) {
    graft_fixtures::git::run(repo, args).expect("git runs on a fixture copy");
}

fn name(full: &str) -> RefName {
    RefName::new(full).expect("a valid ref name")
}

fn resolve(handle: &RepoHandle, over: Option<&RefName>) -> Branches {
    Branches::resolve(handle, over).expect("the fixture's branches resolve")
}

/// A name no fixture uses, written into a copy's local `init.defaultBranch`.
///
/// `SharedRepo` opens a repository the way git does, global config included
/// — which is correct, and which means rule 3 answers differently depending
/// on whether the developer running the tests has `init.defaultBranch` set.
/// A local value overrides the global one, and a local value naming a
/// branch that does not exist takes rule 3 out of the picture entirely. Any
/// test asserting a rule *below* rule 3 has to do this to be deterministic.
const NO_SUCH_BRANCH: &str = "no-such-default-branch";

/// A writable copy of a fixture with rule 3 neutralised.
fn copy_without_init_default_branch(shape: Shape) -> Scratch {
    let scratch = graft_fixtures::copy_into_temp(shape);
    git(
        scratch.path(),
        &["config", "init.defaultBranch", NO_SUCH_BRANCH],
    );
    scratch
}

/// A writable copy whose default branch is called `trunk` and whose rule 3
/// is neutralised, so neither `main`/`master` nor anything above it answers.
fn renamed_to_trunk(shape: Shape) -> Scratch {
    let scratch = copy_without_init_default_branch(shape);
    git(scratch.path(), &["branch", "-m", "main", "trunk"]);
    scratch
}

#[test]
fn the_conventional_name_wins_when_nothing_else_says_otherwise() {
    let scratch = copy_without_init_default_branch(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/main")));
    assert_eq!(branches.chosen_by(), Some(Chosen::ConventionalName));
}

#[test]
fn master_is_the_default_when_there_is_no_main() {
    let scratch = copy_without_init_default_branch(Shape::FeatureBranches);
    git(scratch.path(), &["branch", "-m", "main", "master"]);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/master")));
    assert_eq!(branches.chosen_by(), Some(Chosen::ConventionalName));
}

#[test]
fn origin_head_beats_the_conventional_name() {
    // `main` exists and would win on name alone; the remote says otherwise.
    let scratch = graft_fixtures::copy_into_temp(Shape::RefStates);
    git(scratch.path(), &["branch", "trunk", "main"]);
    git(
        scratch.path(),
        &["update-ref", "refs/remotes/origin/trunk", "main"],
    );
    git(
        scratch.path(),
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ],
    );
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/trunk")));
    assert_eq!(branches.chosen_by(), Some(Chosen::OriginHead));
}

#[test]
fn an_origin_head_with_no_local_branch_falls_through() {
    // A mirror knows what the remote's default is but has no branch of that
    // name to put in lane 0, so the next rule answers instead.
    let scratch = copy_without_init_default_branch(Shape::RefStates);
    git(
        scratch.path(),
        &["update-ref", "refs/remotes/origin/trunk", "main"],
    );
    git(
        scratch.path(),
        &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ],
    );
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/main")));
    assert_eq!(branches.chosen_by(), Some(Chosen::ConventionalName));
}

#[test]
fn init_default_branch_is_used_when_there_is_no_origin_head() {
    let scratch = renamed_to_trunk(Shape::FeatureBranches);
    git(scratch.path(), &["config", "init.defaultBranch", "trunk"]);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/trunk")));
    assert_eq!(branches.chosen_by(), Some(Chosen::InitDefaultBranch));
}

#[test]
fn an_init_default_branch_that_does_not_exist_is_skipped() {
    let scratch = copy_without_init_default_branch(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/main")));
    assert_eq!(branches.chosen_by(), Some(Chosen::ConventionalName));
}

#[test]
fn the_most_merged_into_branch_is_the_last_resort() {
    // No `origin/HEAD`, no `init.defaultBranch`, no `main` and no `master`:
    // only history is left to go on, and the feature branches were merged
    // into `trunk`, not the other way round.
    let scratch = renamed_to_trunk(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/trunk")));
    assert_eq!(branches.chosen_by(), Some(Chosen::MostMergedInto));
}

#[test]
fn a_configured_override_beats_every_other_rule() {
    let path = graft_fixtures::borrow(Shape::FeatureBranches);
    let shared = SharedRepo::open(&path).expect("the fixture opens");
    let chosen = name("refs/heads/feature/one");
    let branches = resolve(&shared.handle(), Some(&chosen));

    assert_eq!(branches.default_branch(), Some(&chosen));
    assert_eq!(branches.chosen_by(), Some(Chosen::Configured));
}

#[test]
fn an_override_naming_a_branch_that_is_gone_is_ignored() {
    // Deleting a branch must not leave the graph with no spine.
    let scratch = copy_without_init_default_branch(Shape::FeatureBranches);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let stale = name("refs/heads/deleted-last-week");
    let branches = resolve(&shared.handle(), Some(&stale));

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/main")));
    assert_eq!(branches.chosen_by(), Some(Chosen::ConventionalName));
}

#[test]
fn integration_branches_are_named_and_never_take_lane_zero() {
    let scratch = copy_without_init_default_branch(Shape::FeatureBranches);
    git(scratch.path(), &["branch", "develop", "main"]);
    git(scratch.path(), &["branch", "release/1.0", "main"]);
    git(scratch.path(), &["branch", "release/2.0", "main"]);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/main")));
    assert_eq!(
        branches.integration(),
        [
            name("refs/heads/develop"),
            name("refs/heads/release/1.0"),
            name("refs/heads/release/2.0"),
        ],
    );
}

#[test]
fn the_default_branch_is_not_also_listed_as_an_integration_branch() {
    // A project whose only branch is `develop` puts it in lane 0, and lane 0
    // is not adjacent to itself.
    let scratch = copy_without_init_default_branch(Shape::Linear);
    git(scratch.path(), &["branch", "-m", "main", "develop"]);
    let shared = SharedRepo::open(scratch.path()).expect("the fixture opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), Some(&name("refs/heads/develop")));
    assert!(branches.integration().is_empty());
}

#[test]
fn a_feature_branch_is_not_an_integration_branch() {
    let path = graft_fixtures::borrow(Shape::FeatureBranches);
    let shared = SharedRepo::open(&path).expect("the fixture opens");

    assert!(resolve(&shared.handle(), None).integration().is_empty());
}

#[test]
fn a_repository_with_no_branch_at_all_has_no_default() {
    let fixture = Fixture::empty_repo();
    let shared = SharedRepo::open(fixture.root()).expect("an empty repository opens");
    let branches = resolve(&shared.handle(), None);

    assert_eq!(branches.default_branch(), None);
    assert_eq!(branches.chosen_by(), None);
    assert!(branches.integration().is_empty());
}
