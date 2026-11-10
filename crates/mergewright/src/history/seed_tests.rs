//! The seeds the walk lays lanes out with.

use mergewright_fixtures::Shape;
use mergewright_git::branches::Branches;
use mergewright_git::repo::{RefTarget, RepoHandle};
use mergewright_graph::BranchIdentity;

use super::seeds::seeds_for;

/// The seed builder's choice, on the shape that needs it: a remote ahead of
/// its local branch is a known tip under its own name.
#[test]
fn seeds_anchor_the_default_branch_and_know_remote_tips() {
    let dir = mergewright_fixtures::borrow(Shape::RefStates);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let seeds = seeds_for(&refs, &branches);

    let default = branches.default_branch().expect("a default branch");
    let seed = seeds.default_branch.expect("lane 0 is seeded");
    assert_eq!(seed.identity, BranchIdentity::of_name(default.shorthand()));
    // One identity per tip: a remote level with a local branch adds no seed.
    let locals: Vec<_> = (refs.iter())
        .filter(|e| e.name.as_bytes().starts_with(b"refs/heads/"))
        .map(|e| &e.target)
        .collect();
    let remotes = (refs.iter())
        .filter(|e| e.name.as_bytes().starts_with(b"refs/remotes/"))
        .filter(|e| matches!(e.target, RefTarget::Object(_)) && !locals.contains(&&e.target))
        .count();
    assert!(remotes > 0, "the ref-states shape has remote-tracking refs");
    let remote_seeds = seeds
        .known_branches
        .iter()
        .filter(|known| {
            refs.iter().any(|entry| {
                entry.name.as_bytes().starts_with(b"refs/remotes/")
                    && BranchIdentity::of_name(entry.name.shorthand()) == known.identity
            })
        })
        .count();
    assert_eq!(remote_seeds, remotes);
}

/// ADR-0064: a remote-tracking seed names the local branch it is the copy of
/// (same short name), so a lane on that branch is not renamed `origin/x`.
#[test]
fn a_remote_seed_follows_the_local_branch_of_the_same_name() {
    let dir = mergewright_fixtures::borrow(Shape::RefStates);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let seeds = seeds_for(&refs, &branches);

    let local_names: Vec<&[u8]> = (refs.iter())
        .filter_map(|e| e.name.as_bytes().strip_prefix(b"refs/heads/"))
        .collect();
    let mut following = 0;
    for entry in &refs {
        let Some(rest) = entry.name.as_bytes().strip_prefix(b"refs/remotes/") else {
            continue;
        };
        let Some(known) = seeds
            .known_branches
            .iter()
            .find(|known| known.identity == BranchIdentity::of_name(entry.name.shorthand()))
        else {
            continue;
        };
        let branch = rest.splitn(2, |&b| b == b'/').nth(1).unwrap_or(rest);
        let expected = local_names
            .contains(&branch)
            .then(|| BranchIdentity::of_name(branch));
        assert_eq!(known.follows, expected);
        following += usize::from(expected.is_some());
    }
    assert!(
        following > 0,
        "the shape has a remote copy of a local branch"
    );
}

/// A configured upstream with a different name than its branch
/// (`branch.<n>.merge`): the remote seed follows that local branch, not a
/// same-named one.
#[test]
fn a_remote_seed_follows_the_branch_that_names_it_as_upstream() {
    use std::collections::HashMap;

    use mergewright_core::ref_name::RefName;

    use super::seeds::seeds_with_upstreams;

    let dir = mergewright_fixtures::borrow(Shape::RefStates);
    let handle = RepoHandle::open(&dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let plain = seeds_for(&refs, &branches);
    // A remote seed, and a local branch that does not share its short name
    // (so the same-name rule would pick another branch, or none).
    let remote = refs
        .iter()
        .filter(|e| e.name.as_bytes().starts_with(b"refs/remotes/"))
        .find(|e| {
            plain
                .known_branches
                .iter()
                .any(|known| known.identity == BranchIdentity::of_name(e.name.shorthand()))
        })
        .expect("a remote seed in the shape");
    let by_name = plain
        .known_branches
        .iter()
        .find(|known| known.identity == BranchIdentity::of_name(remote.name.shorthand()))
        .and_then(|known| known.follows);
    let local = refs
        .iter()
        .filter(|e| e.name.as_bytes().starts_with(b"refs/heads/"))
        .find(|e| Some(BranchIdentity::of_name(e.name.shorthand())) != by_name)
        .expect("a local branch of another name");
    let mut upstreams: HashMap<RefName, RefName> = HashMap::new();
    upstreams.insert(local.name.clone(), remote.name.clone());

    let seeds = seeds_with_upstreams(&refs, &branches, &upstreams);
    let known = seeds
        .known_branches
        .iter()
        .find(|known| known.identity == BranchIdentity::of_name(remote.name.shorthand()))
        .expect("the remote is still a seed");
    assert_eq!(
        known.follows,
        Some(BranchIdentity::of_name(local.name.shorthand()))
    );
}

/// pre-publish-cleanup/04, Part B: a Ghost Preview names a remote lane the
/// way the main graph behind it does, even when the branch's configured
/// upstream has another name (`feat` tracking `origin/feat-remote`). The
/// remote is ahead, so it opens its own lane in both.
#[test]
fn preview_seeds_follow_a_differently_named_upstream_like_the_graph() {
    use std::collections::HashMap;

    use mergewright_fixtures::git::run;

    use super::seeds::{preview_seeds, seeds_with_upstreams};
    use crate::refs::{AheadBehindCache, read};
    use crate::repos::RepoStore;

    let scratch = mergewright_fixtures::copy_into_temp(Shape::Linear);
    let dir = scratch.path();
    let git = |args: &[&str]| run(dir, args).expect("git runs on the scratch copy");
    git(&["branch", "feat"]);
    git(&["commit", "--allow-empty", "-m", "remote only"]);
    git(&["update-ref", "refs/remotes/origin/feat-remote", "HEAD"]);
    git(&["reset", "--hard", "HEAD~1"]);
    git(&[
        "remote",
        "add",
        "origin",
        "https://example.invalid/repo.git",
    ]);
    git(&["config", "branch.feat.remote", "origin"]);
    git(&["config", "branch.feat.merge", "refs/heads/feat-remote"]);

    let handle = RepoHandle::open(dir).expect("opens");
    let refs = handle.references().expect("refs read");
    let branches = Branches::resolve(&handle, None).expect("branches resolve");
    let mut upstreams = HashMap::new();
    for entry in refs
        .iter()
        .filter(|e| e.name.as_bytes().starts_with(b"refs/heads/"))
    {
        if let Some(up) = handle.upstream(&entry.name).expect("config reads") {
            upstreams.insert(entry.name.clone(), up);
        }
    }
    let graph = seeds_with_upstreams(&refs, &branches, &upstreams);
    let store = RepoStore::new(dir.to_path_buf(), None);
    let badges = read(&store, &AheadBehindCache::default()).expect("badges read");
    let preview = preview_seeds(None, &badges);

    let remote = BranchIdentity::of_name(b"origin/feat-remote");
    let follows = |seeds: &mergewright_graph::Seeds| {
        seeds
            .known_branches
            .iter()
            .find(|seed| seed.identity == remote)
            .map(|seed| seed.follows)
            .expect("the remote ahead is a seed")
    };
    assert_eq!(
        follows(&graph),
        Some(BranchIdentity::of_name(b"feat")),
        "the graph's resolver follows the configured upstream"
    );
    assert_eq!(
        follows(&preview),
        follows(&graph),
        "preview names the lane as the graph does"
    );
}
