//! ease-pass 09p: measuring only what can win changes no answer. Every
//! topology the other test files build is checked too, through their
//! helpers ([`agrees`]): for the focused branch, the route and the whole
//! [`BranchStatus`] equal the oracle's, which measures every candidate as
//! before 09p.

use super::{Repo, find, stack, stack_candidates};
use crate::branch_status::measure::{Ask, How, measure};
use crate::branch_status::status::branch_status_by;
use crate::branch_status::{BranchStatus, Candidate, CandidateKind, route_by};

fn status_by(repo: &Repo, me: &Candidate, candidates: &[Candidate], how: How) -> BranchStatus {
    branch_status_by(&repo.handle(), me, candidates, None, how, &()).expect("the status")
}

/// For each of `foci`: [`route`](crate::branch_status::route) and
/// [`branch_status`](crate::branch_status::branch_status) (no upstream)
/// answer as the every-candidate oracle does.
pub(super) fn agrees(repo: &Repo, candidates: &[Candidate], foci: &[&Candidate]) {
    let handle = repo.handle();
    for &me in foci {
        let route = |how| route_by(&handle, me, candidates, how, &()).expect("the route");
        assert_eq!(
            route(How::Contenders),
            route(How::Every),
            "route of {}",
            me.name
        );
        assert_eq!(
            status_by(repo, me, candidates, How::Contenders),
            status_by(repo, me, candidates, How::Every),
            "status of {}",
            me.name
        );
    }
}

/// The 01r stack plus what 09p prunes, and what it must not:
///
/// ```text
/// m1 m2 m3 ─ main                       (m3 after the stack was cut)
///     └ w1 w2 ─ wallet-release
///           └ s1 s2 ─ sara/fix-toc
///     └ o1 ─ ops/ci
/// stale/m1, stale/m2, stale/w1, stale/s1     tips on those commits
/// sib/m1, sib/w2                             one commit off them
/// release/old at m1 (integration, under main)
/// release/next one commit off m2 (integration, not under main)
/// origin/sara at s1 (a remote ref)
/// moved: a candidate whose ref has moved on since it was listed
/// ```
fn pruned() -> (Repo, Vec<Candidate>) {
    let repo = stack();
    let mut candidates = stack_candidates(&repo);
    for (name, at) in [
        ("stale/m1", "main~1"),
        ("stale/m2", "main"),
        ("stale/w1", "wallet-release~1"),
        ("stale/s1", "sara/fix-toc~1"),
        ("release/old", "main~1"),
        ("moved", "main~1"),
    ] {
        repo.git(&["branch", name, at]);
    }
    for (name, off) in [
        ("sib/m1", "main~1"),
        ("sib/w2", "wallet-release"),
        ("release/next", "main"),
    ] {
        repo.git(&["switch", "-q", "-c", name, off]);
        repo.commit(&name.replace('/', "-"));
    }
    repo.git(&["update-ref", "refs/remotes/origin/sara", "sara/fix-toc~1"]);
    for name in [
        "stale/m1", "stale/m2", "stale/w1", "stale/s1", "sib/m1", "sib/w2", "moved",
    ] {
        candidates.push(repo.cand(name, CandidateKind::Other));
    }
    for name in ["release/old", "release/next"] {
        candidates.push(repo.cand(name, CandidateKind::Integration));
    }
    candidates.push(Candidate {
        name: "refs/remotes/origin/sara".to_owned(),
        tip: repo.oid("sara/fix-toc~1"),
        kind: CandidateKind::Other,
    });
    // The ref moves on after the caller listed it: the candidate keeps m1.
    repo.git(&["switch", "-q", "moved"]);
    repo.commit("moved-on");
    // main moves on past the stack.
    repo.git(&["switch", "-q", "main"]);
    repo.commit("m3");
    let candidates = candidates
        .into_iter()
        .map(|c| match c.name.as_str() {
            "refs/heads/main" => repo.cand("main", CandidateKind::Trunk),
            _ => c,
        })
        .collect();
    (repo, candidates)
}

#[test]
fn stale_and_sibling_others_answer_as_the_oracle() {
    let (repo, candidates) = pruned();
    // One of each sort: stacked, integration, a remote ref under a stacked
    // branch (with a twin at its tip), a sibling.
    let foci = [
        "refs/heads/sara/fix-toc",
        "refs/heads/wallet-release",
        "refs/remotes/origin/sara",
        "refs/heads/sib/w2",
    ]
    .map(|name| {
        candidates
            .iter()
            .find(|c| c.name == name)
            .expect("a candidate by that name")
    });
    agrees(&repo, &candidates, &foci);
}

/// Stale and sibling branches cost no count from `sara/fix-toc`: neither
/// can be its base. What can win or hold it is measured: the trunk, the
/// integration branches off the trunk, the top of what lies under her that
/// main lacks (`stale/s1` and `origin/sara`, one commit; `stale/w1` is under
/// it), and the candidate whose ref moved (the filters cannot speak for it).
#[test]
fn only_what_can_win_is_measured() {
    let (repo, candidates) = pruned();
    let sara = find(&candidates, "sara/fix-toc");
    let measured = measure(
        &repo.handle(),
        sara,
        &candidates,
        &[],
        Ask::Footing,
        How::Contenders,
        &(),
    )
    .expect("measured");
    let mut names: Vec<&str> = measured.iter().map(|(c, _)| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "refs/heads/main",
            "refs/heads/moved",
            "refs/heads/release/next",
            "refs/heads/stale/s1",
            "refs/heads/wallet-release",
            "refs/remotes/origin/sara",
        ]
    );
}

/// The team repository's shape (ease-pass 09p window): features merged
/// into main one after another, each cut after the last was merged, so
/// every older feature lies under the newer ones and main holds them all.
/// No trunk is able to win, so only `merge-base --independent` keeps the
/// count down; the answers must not move.
///
/// ```text
/// m1 ─ M(f1) ─ M(f2) ─ M(f3) ─ m2   main
///  └ f1 ┘  └ f2 ┘  └ f3 ┘           each feature merged --no-ff
///  stale/f1 at f1, side off m1
/// ```
#[test]
fn features_merged_one_after_another_answer_as_the_oracle() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["branch", "side"]);
    for name in ["f1", "f2", "f3"] {
        repo.git(&["switch", "-q", "-c", name, "main"]);
        repo.commit(name);
        repo.git(&["switch", "-q", "main"]);
        repo.git(&[
            "merge",
            "-q",
            "--no-ff",
            "-m",
            &format!("merge {name}"),
            name,
        ]);
    }
    repo.commit("m2");
    repo.git(&["branch", "stale/f1", "f1"]);
    repo.git(&["switch", "-q", "side"]);
    repo.commit("side");
    let mut candidates = vec![repo.cand("main", CandidateKind::Trunk)];
    for name in ["f1", "f2", "f3", "stale/f1", "side"] {
        candidates.push(repo.cand(name, CandidateKind::Other));
    }
    let foci: Vec<&Candidate> = ["f3", "f2", "side"]
        .iter()
        .map(|name| find(&candidates, name))
        .collect();
    agrees(&repo, &candidates, &foci);
    // From f3, f1, f2 and stale/f1 were all merged into main through second
    // parents: none is on f3's first-parent line, so only main is measured.
    let f3 = find(&candidates, "f3");
    let measured = measure(
        &repo.handle(),
        f3,
        &candidates,
        &[],
        Ask::Footing,
        How::Contenders,
        &(),
    )
    .expect("measured");
    let mut names: Vec<&str> = measured.iter().map(|(c, _)| c.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, ["refs/heads/main"]);
}
