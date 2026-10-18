//! What counts as pushed for a fork (ADR-0059 §6.2 as amended by 14e),
//! against real repositories with a bare `origin`. Truth after `y` is read
//! with git (`rev-parse`, `merge-base --is-ancestor`), never Mergewright's reads.
//!
//! - (b) a remote-tracking ref of the branch's own name counts, `-u` or not;
//! - a local upstream (`--track feature-a`) is not a push.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_ops::preview::{ForkFate, PreviewPlan, WhyStays};

use crate::fork_follows::{
    as_run, bare_origin, carries, carrying, fate_of, fates, forked, id, own_diff, push, reword_c1,
    stays,
};
use crate::support::{NOW, TempRepo, run};

/// The ref a `Pushed` names, in full; panics on any other reason.
fn pushed(why: &WhyStays) -> String {
    match why {
        WhyStays::Pushed { upstream } => String::from_utf8_lossy(upstream.as_bytes()).into_owned(),
        other => panic!("expected Pushed, got {other:?}"),
    }
}

/// `git merge-base --is-ancestor <a> <b>`, read with git.
fn is_ancestor(repo: &TempRepo, a: &str, b: &str) -> bool {
    repo.try_git(&["merge-base", "--is-ancestor", a, b]).is_ok()
}

fn follows(drawn: &PreviewPlan, branch: &str) {
    assert!(
        matches!(fate_of(drawn, branch), ForkFate::Follows { .. }),
        "{branch}: {:?}",
        fates(drawn)
    );
}

/// No fork's `Pushed`, bare or carried, names a local branch.
fn no_local_pushed(drawn: &PreviewPlan) {
    for (branch, fate) in fates(drawn) {
        let ForkFate::Stays(why) = fate else { continue };
        let why = match &why {
            WhyStays::Carries { why, .. } => why.as_ref().clone(),
            other => other.clone(),
        };
        if let WhyStays::Pushed { upstream } = why {
            assert!(
                !upstream.as_bytes().starts_with(b"refs/heads/"),
                "{branch} reads pushed to a local branch: {upstream:?}"
            );
        }
    }
}

// ── M1: a same-name remote is a push, `-u` or not ──────────────────────────

#[test]
fn a_fork_with_no_commits_pushed_without_u_stays() {
    let repo = forked();
    repo.git(&["branch", "-q", "park", "feature-a~1"]);
    bare_origin(&repo);
    push(&repo, "park", "park", false);
    let was = id(&repo, "park");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    let why = stays(&fate_of(&drawn, "park")).clone();
    assert_eq!(pushed(&why), "refs/remotes/origin/park");
    assert_eq!(why.words(), "pushed to origin/park");
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "park"), was, "park stayed");
    assert!(is_ancestor(&repo, "origin/park", "park"));
}

#[test]
fn a_fork_pushed_without_u_at_its_fork_point_then_built_on_stays() {
    let repo = forked();
    bare_origin(&repo);
    // feature-x is c2 + x1: pushed at c2, before x1, with no upstream set.
    push(&repo, "feature-a~1", "feature-x", false);
    let was = id(&repo, "feature-x");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    let why = stays(&fate_of(&drawn, "feature-x")).clone();
    assert_eq!(pushed(&why), "refs/remotes/origin/feature-x");
    assert_eq!(why.words(), "pushed to origin/feature-x");
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-x"), was, "feature-x stayed");
    assert!(is_ancestor(&repo, "origin/feature-x", "feature-x"));
}

/// `carrying()` with `feature-w` pushed by name (no `-u`) at `rev`, then put
/// back at x1: `feature-x` must stay, naming `feature-w`.
fn carried_pushed_at(rev: &str) {
    let repo = carrying();
    bare_origin(&repo);
    push(&repo, rev, "feature-w", false);
    let refs = repo.git(&["rev-parse", "feature-x", "feature-w"]);

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    let why = stays(&fate_of(&drawn, "feature-x")).clone();
    let (branch, inner) = carries(&why);
    assert_eq!(branch, "refs/heads/feature-w");
    assert_eq!(pushed(inner), "refs/remotes/origin/feature-w");
    assert_eq!(why.words(), "feature-w is pushed to origin/feature-w");
    run(&repo, &op, NOW);

    assert_eq!(repo.git(&["rev-parse", "feature-x", "feature-w"]), refs);
}

#[test]
fn a_fork_carrying_a_branch_pushed_without_u_below_it_stays() {
    // Pushed at c2, then moved to x1 (`branch -f`): its remote holds only
    // the fork point.
    carried_pushed_at("feature-a~1");
}

#[test]
fn a_fork_carrying_a_branch_pushed_without_u_at_its_tip_names_it() {
    // Review m3(a): was F's own `Pushed { origin/feature-w }`.
    carried_pushed_at("feature-w");
}

#[test]
fn a_fork_follows_when_only_his_line_is_pushed() {
    let repo = forked();
    bare_origin(&repo);
    push(&repo, "feature-a~1", "feature-a", true);
    let x1_diff = own_diff(&repo, "feature-x");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    follows(&drawn, "feature-x");
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-x~1"), id(&repo, "feature-a~1"));
    assert_eq!(own_diff(&repo, "feature-x"), x1_diff);
}

#[test]
fn a_fork_whose_own_name_remote_holds_only_onto_follows() {
    for track in [false, true] {
        let repo = forked();
        bare_origin(&repo);
        push(&repo, "main", "feature-x", track);

        let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
        follows(&drawn, "feature-x");
        run(&repo, &op, NOW);

        assert_eq!(
            id(&repo, "feature-x~1"),
            id(&repo, "feature-a~1"),
            "track: {track}"
        );
    }
}

// ── m1: a local upstream is not a push ─────────────────────────────────────

#[test]
fn a_fork_tracking_a_local_branch_follows() {
    let repo = forked();
    repo.git(&[
        "branch",
        "-q",
        "--set-upstream-to",
        "feature-a",
        "feature-x",
    ]);
    let x1_diff = own_diff(&repo, "feature-x");

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    no_local_pushed(&drawn);
    follows(&drawn, "feature-x");
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-x~1"), id(&repo, "feature-a~1"));
    assert_eq!(own_diff(&repo, "feature-x"), x1_diff);
}

#[test]
fn a_fork_carrying_a_branch_tracking_a_local_branch_follows() {
    let repo = carrying();
    repo.git(&[
        "branch",
        "-q",
        "--set-upstream-to",
        "feature-a",
        "feature-w",
    ]);

    let (op, drawn) = as_run(&repo, reword_c1(&repo, &[]));
    no_local_pushed(&drawn);
    follows(&drawn, "feature-x");
    run(&repo, &op, NOW);

    assert_eq!(id(&repo, "feature-w"), id(&repo, "feature-x~1"));
    assert_eq!(id(&repo, "feature-x~2"), id(&repo, "feature-a~1"));
}
