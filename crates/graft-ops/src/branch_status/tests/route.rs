//! The route against real repositories: a stacked branch, the walk's
//! guards, and 01r's R1 (siblings, a base that moved on, and a branch cut
//! from an older commit are never "built on"); stops, at entry and mid-walk.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use graft_git::run::{RunError, Stop};

use super::{Repo, find, names, stack, stack_candidates};
use crate::branch_status::{Candidate, CandidateKind, branch_status, route};

/// ```text
/// m1 m2 ─ main
///     └ w1 w2 [w3] ─ wallet-release
///           ├ s1 s2 ─ sara/fix-toc
///           └ t1 ─ ali/x            (only when `sibling`)
/// ```
fn siblings(wallet: CandidateKind, moved: bool, sibling: bool) -> (Repo, Vec<Candidate>) {
    let repo = Repo::new();
    repo.commit("m1");
    repo.commit("m2");
    repo.git(&["switch", "-q", "-c", "wallet-release"]);
    repo.commit("w1");
    repo.commit("w2");
    repo.git(&["switch", "-q", "-c", "sara/fix-toc"]);
    repo.commit("s1");
    repo.commit("s2");
    let mut candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("sara/fix-toc", CandidateKind::Other),
    ];
    if sibling {
        repo.git(&["switch", "-q", "-c", "ali/x", "wallet-release"]);
        repo.commit("t1");
        candidates.push(repo.cand("ali/x", CandidateKind::Other));
    }
    if moved {
        repo.git(&["switch", "-q", "wallet-release"]);
        repo.commit("w3");
    }
    candidates.push(repo.cand("wallet-release", wallet));
    (repo, candidates)
}

fn route_of(repo: &Repo, candidates: &[Candidate], branch: &str) -> Vec<String> {
    let me = find(candidates, branch).clone();
    let route = route(&repo.handle(), &me, candidates, &()).expect("the route");
    names(&route).into_iter().map(str::to_owned).collect()
}

/// Major 1 (a): `ali/x` is as near to `sara/fix-toc` as `wallet-release`
/// is, and sorts first by name; it has a commit sara lacks, so it is a
/// sibling, never a base.
#[test]
fn sibling_is_not_built_on() {
    let (repo, candidates) = siblings(CandidateKind::Other, true, true);
    assert_eq!(
        route_of(&repo, &candidates, "sara/fix-toc"),
        ["refs/heads/main"]
    );
    let (repo, candidates) = siblings(CandidateKind::Integration, true, true);
    assert_eq!(
        route_of(&repo, &candidates, "sara/fix-toc"),
        ["refs/heads/wallet-release", "refs/heads/main"]
    );
}

/// Major 1 (b): from `wallet-release` on, `ali/x` is nearer than `main`;
/// an integration branch is never built on an ordinary one.
#[test]
fn a_bases_base_is_never_its_sibling() {
    let (repo, candidates) = siblings(CandidateKind::Integration, true, true);
    let route = route_of(&repo, &candidates, "sara/fix-toc");
    assert_eq!(route.get(1).map(String::as_str), Some("refs/heads/main"));
}

/// Major 1, reversed: `wallet-release` moved on (w3) after `sara/fix-toc`
/// and `ali/x` left it; neither is what it was built on.
#[test]
fn a_moved_base_is_not_built_on_its_dependent() {
    let (repo, candidates) = siblings(CandidateKind::Other, true, true);
    assert_eq!(
        route_of(&repo, &candidates, "wallet-release"),
        ["refs/heads/main"]
    );
}

/// Major 1, a child: `bob/x` was cut from `sara`@s1 and has x1 of its own;
/// it is one commit from sara, nearer than main, and still not her base.
#[test]
fn a_branch_cut_from_an_older_commit_is_not_the_base() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "sara"]);
    repo.commit("s1");
    repo.git(&["switch", "-q", "-c", "bob/x"]);
    repo.commit("x1");
    repo.git(&["switch", "-q", "sara"]);
    repo.commit("s2");
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("sara", CandidateKind::Other),
        repo.cand("bob/x", CandidateKind::Other),
    ];
    assert_eq!(route_of(&repo, &candidates, "sara"), ["refs/heads/main"]);
}

/// Regression: an ordinary base that has not moved on still counts.
#[test]
fn a_dependent_on_an_unmoved_branch_is_built_on_it() {
    let (repo, candidates) = siblings(CandidateKind::Other, false, false);
    assert_eq!(
        route_of(&repo, &candidates, "sara/fix-toc"),
        ["refs/heads/wallet-release", "refs/heads/main"]
    );
}

/// A [`Stop`] that answers "go on" `n` times, then "stop" from then on.
struct StopAfter(AtomicUsize);

impl StopAfter {
    fn new(n: usize) -> Self {
        StopAfter(AtomicUsize::new(n))
    }
}

impl Stop for StopAfter {
    fn should_stop(&self) -> bool {
        self.0
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_err()
    }
}

/// Minor 1: a stop that arrives mid-walk, after the entry guard, still
/// ends `route` and `branch_status` with `Stopped`; a far-off one does not.
#[test]
fn stop_mid_walk() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let sara = find(&candidates, "sara/fix-toc");
    let handle = repo.handle();
    for n in [1, 3, 10] {
        assert!(
            matches!(
                route(&handle, sara, &candidates, &StopAfter::new(n)),
                Err(RunError::Stopped)
            ),
            "route, stopped after {n} checks"
        );
        assert!(
            matches!(
                branch_status(&handle, sara, &candidates, None, &StopAfter::new(n)),
                Err(RunError::Stopped)
            ),
            "branch_status, stopped after {n} checks"
        );
    }
    let far = 1_000_000;
    assert!(route(&handle, sara, &candidates, &StopAfter::new(far)).is_ok());
    assert!(branch_status(&handle, sara, &candidates, None, &StopAfter::new(far)).is_ok());
}
#[test]
fn route_of_a_stacked_branch_ends_at_the_trunk() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let sara = find(&candidates, "sara/fix-toc");
    let route = route(&repo.handle(), sara, &candidates, &()).expect("the route");
    assert_eq!(
        names(&route),
        ["refs/heads/wallet-release", "refs/heads/main"]
    );
}

#[test]
fn route_of_the_trunk_is_empty() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let main = find(&candidates, "main");
    let route = route(&repo.handle(), main, &candidates, &()).expect("the route");
    assert!(route.is_empty(), "{route:?}");
}

/// A candidate at the branch's own tip holds every commit of it: skipped,
/// so the base is the next one down.
#[test]
fn route_skips_a_candidate_at_the_same_tip() {
    let repo = stack();
    repo.git(&["branch", "sara/twin", "sara/fix-toc"]);
    let mut candidates = stack_candidates(&repo);
    candidates.push(repo.cand("sara/twin", CandidateKind::Other));
    let sara = find(&candidates, "sara/fix-toc").clone();
    let route = route(&repo.handle(), &sara, &candidates, &()).expect("the route");
    assert_eq!(
        names(&route),
        ["refs/heads/wallet-release", "refs/heads/main"]
    );
}

/// Two integration branches at one tip, nothing else but a third diverged
/// from them: each sits on the other's line, so the walk must not loop.
/// (01 had these `Other`, expecting `[b]`; under 01r's R1 an ordinary
/// branch with commits of its own is never a base, so they are long-lived
/// here to keep the cycle possible.)
#[test]
fn route_does_not_cycle() {
    let repo = Repo::new();
    repo.commit("r1");
    repo.git(&["switch", "-q", "-c", "a"]);
    repo.commit("a1");
    repo.git(&["branch", "a-twin"]);
    repo.git(&["switch", "-q", "-c", "b", "main"]);
    repo.commit("b1");
    // No trunk among the candidates: the route can only end by the guard.
    let candidates = vec![
        repo.cand("a", CandidateKind::Integration),
        repo.cand("a-twin", CandidateKind::Integration),
        repo.cand("b", CandidateKind::Integration),
    ];
    let a = candidates[0].clone();
    let route = route(&repo.handle(), &a, &candidates, &()).expect("the route");
    assert_eq!(names(&route), ["refs/heads/a-twin", "refs/heads/b"]);
}

/// A base that has moved on past the branch is not walked back through:
/// `wallet-release` gains a commit after `sara/fix-toc` left it, so
/// `sara/fix-toc` now has the fewest commits `wallet-release` lacks; the
/// walk from `wallet-release` must still go to `main`, not back.
#[test]
fn route_does_not_walk_back_through_the_branch() {
    let repo = stack();
    repo.git(&["switch", "-q", "wallet-release"]);
    repo.commit("w3");
    let candidates = stack_candidates(&repo);
    let sara = find(&candidates, "sara/fix-toc");
    let route = route(&repo.handle(), sara, &candidates, &()).expect("the route");
    assert_eq!(
        names(&route),
        ["refs/heads/wallet-release", "refs/heads/main"]
    );
}

#[test]
fn route_stops_at_depth_eight() {
    let repo = Repo::new();
    repo.commit("root");
    let mut candidates = vec![repo.cand("main", CandidateKind::Trunk)];
    let mut on = "main".to_owned();
    for n in 1..=10 {
        let branch = format!("b{n}");
        repo.git(&["switch", "-q", "-c", &branch, &on]);
        repo.commit(&branch);
        candidates.push(repo.cand(&branch, CandidateKind::Other));
        on = branch;
    }
    let tip = find(&candidates, "b10").clone();
    let route = route(&repo.handle(), &tip, &candidates, &()).expect("the route");
    assert_eq!(
        names(&route),
        [
            "refs/heads/b9",
            "refs/heads/b8",
            "refs/heads/b7",
            "refs/heads/b6",
            "refs/heads/b5",
            "refs/heads/b4",
            "refs/heads/b3",
            "refs/heads/b2",
        ]
    );
}

#[test]
fn route_and_status_honour_stop() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let sara = find(&candidates, "sara/fix-toc");
    let stopped = AtomicBool::new(true);
    let handle = repo.handle();
    assert!(matches!(
        route(&handle, sara, &candidates, &stopped),
        Err(RunError::Stopped)
    ));
    assert!(matches!(
        branch_status(&handle, sara, &candidates, None, &stopped),
        Err(RunError::Stopped)
    ));
}
