//! The pushed stop binds every base (06e ruling, 2026-09-27): "your
//! commits" are `HEAD`'s first-parent commits down to the first one any
//! `refs/remotes/*` ref reaches, the base, a merge or the first commit — and,
//! only on the default or only branch with no base below `HEAD`, the last
//! 10. Absorb places only into them; `a` accepts only them, `HEAD` aside.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use mergewright_ops::absorb::StaysWhy;
use mergewright_ops::operation::Refusal;
use mergewright_ops::rewrite::{RewriteRequest, TodoAction};

use crate::absorb::oid;
use crate::absorb_unpushed::{
    absorb_map_of, both_reach_the_same, commit, on_main, one_line, planned, refused, stage_line_of,
};
use crate::support::TempRepo;

// ── Fixtures: one file per commit, so each line has one owner ───────────

/// `main`: `m1 m2`; `feature` off it: `f1` … `f<n>`, checked out.
fn feature(n: usize) -> TempRepo {
    let repo = on_main(0);
    commit(&repo, "m1");
    commit(&repo, "m2");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for k in 1..=n {
        commit(&repo, &format!("f{k}"));
    }
    repo
}

/// `feature` tracks `origin/feature`, which is at `f<at>`. No remote ref
/// for `main`.
fn tracking(repo: &TempRepo, at: &str) {
    for (key, value) in [
        ("remote.origin.url", "."),
        ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
        ("branch.feature.remote", "origin"),
        ("branch.feature.merge", "refs/heads/feature"),
    ] {
        repo.git(&["config", key, value]);
    }
    let tip = oid(repo, at);
    repo.git(&[
        "update-ref",
        "refs/remotes/origin/feature",
        &tip.to_string(),
    ]);
}

/// Test 1's and 2's: `feature` f1..f3 tracking `origin/feature` at f1.
fn upstream_at_f1() -> TempRepo {
    let repo = feature(3);
    tracking(&repo, "feature~2");
    repo
}

/// Test 3's (probe p7): `feature` f1..f3, `refs/remotes/origin/feature` at
/// f3, no `branch.feature.*`.
fn pushed_without_upstream() -> TempRepo {
    let repo = feature(3);
    let f3 = oid(&repo, "feature");
    repo.git(&["update-ref", "refs/remotes/origin/feature", &f3.to_string()]);
    repo
}

/// Test 4's: `feature` f1..f4 tracking `origin/feature` at f1;
/// `refs/remotes/origin/teammate` at `t1`, a child of f3 off `feature`.
fn teammate_on_f3() -> TempRepo {
    let repo = feature(3);
    repo.git(&["switch", "-q", "-c", "teammate"]);
    commit(&repo, "t1");
    let t1 = oid(&repo, "teammate");
    repo.git(&["switch", "-q", "feature"]);
    repo.git(&["branch", "-q", "-D", "teammate"]);
    repo.git(&[
        "update-ref",
        "refs/remotes/origin/teammate",
        &t1.to_string(),
    ]);
    commit(&repo, "f4");
    tracking(&repo, "feature~3");
    repo
}

/// Test 5's (probe p4): `main` c1..c4, no remote; `fix` made at its tip and
/// checked out.
fn new_branch_at_mains_tip() -> TempRepo {
    let repo = on_main(4);
    repo.git(&["switch", "-q", "-c", "fix"]);
    repo
}

fn short(commit: mergewright_core::oid::Oid) -> String {
    commit.to_string()[..7].to_owned()
}

// ── 1. `a` with an upstream ─────────────────────────────────────────────

#[test]
fn a_into_a_pushed_commit_with_an_upstream_is_already_pushed() {
    let repo = upstream_at_f1();
    stage_line_of(&repo, "f2");
    let f1 = oid(&repo, "feature~2");
    let f2 = oid(&repo, "feature~1");
    let m2 = oid(&repo, "main");

    for pushed in [f1, m2] {
        let refusal = refused(planned(
            &repo,
            &RewriteRequest::AmendInto { target: pushed },
        ));
        assert_eq!(refusal, Refusal::AlreadyPushed { commit: pushed });
    }
    planned(&repo, &RewriteRequest::AmendInto { target: f2 }).expect("f2 plans");
    assert_eq!(
        Refusal::AlreadyPushed { commit: f1 }.explain(&crate::support::Letters),
        format!(
            "{} is already pushed · a only changes older commits you haven't pushed",
            short(f1)
        )
    );
}

// ── 2. Absorb with an upstream ──────────────────────────────────────────

#[test]
fn absorb_with_an_upstream_leaves_a_pushed_hunk_staged() {
    let repo = upstream_at_f1();
    stage_line_of(&repo, "f1");
    stage_line_of(&repo, "f3");

    let map = absorb_map_of(&repo).expect("it maps");

    assert_eq!(
        map.placed,
        vec![(one_line("f3.txt"), oid(&repo, "feature"))]
    );
    assert_eq!(
        map.stays,
        vec![(one_line("f1.txt"), StaysWhy::AlreadyPushed)]
    );
}

// ── 3. Pushed without an upstream (probe p7) ────────────────────────────

#[test]
fn pushed_without_an_upstream_counts_as_pushed() {
    let repo = pushed_without_upstream();
    stage_line_of(&repo, "f2");
    let f2 = oid(&repo, "feature~1");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::Absorb)),
        Refusal::AllPushed {
            branch: "feature".to_owned()
        }
    );
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: f2 })),
        Refusal::AlreadyPushed { commit: f2 }
    );
}

// ── 4. A teammate's branch built on his (kills both M1 mutants) ─────────

#[test]
fn a_teammates_branch_on_mine_marks_my_commits_pushed() {
    let repo = teammate_on_f3();
    stage_line_of(&repo, "f3");
    stage_line_of(&repo, "f4");
    let f3 = oid(&repo, "feature~1");
    let f4 = oid(&repo, "feature");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: f3 })),
        Refusal::AlreadyPushed { commit: f3 }
    );
    let map = absorb_map_of(&repo).expect("it maps");
    assert_eq!(map.placed, vec![(one_line("f4.txt"), f4)]);
    assert_eq!(
        map.stays,
        vec![(one_line("f3.txt"), StaysWhy::AlreadyPushed)]
    );
    // f4 is HEAD: with nothing built on it, `a` there is the plain amend.
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: f4 })),
        Refusal::PlainAmend
    );
}

/// M1 on the fallback (review probe p3): `main` c1..c6, no upstream;
/// `refs/remotes/origin/teammate` at a child of c4 off `main`. c4 is below
/// a remote tip, and not on a `*/main` ref: both mutants take it.
#[test]
fn a_teammates_branch_on_main_marks_my_commits_pushed() {
    let repo = on_main(4);
    repo.git(&["switch", "-q", "-c", "teammate"]);
    commit(&repo, "t1");
    let t1 = oid(&repo, "teammate");
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["branch", "-q", "-D", "teammate"]);
    repo.git(&[
        "update-ref",
        "refs/remotes/origin/teammate",
        &t1.to_string(),
    ]);
    commit(&repo, "c5");
    commit(&repo, "c6");
    stage_line_of(&repo, "c4");
    stage_line_of(&repo, "c5");
    let c4 = oid(&repo, "main~2");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: c4 })),
        Refusal::AlreadyPushed { commit: c4 }
    );
    let map = absorb_map_of(&repo).expect("it maps");
    assert_eq!(map.placed, vec![(one_line("c5.txt"), oid(&repo, "main~1"))]);
    assert_eq!(
        map.stays,
        vec![(one_line("c4.txt"), StaysWhy::AlreadyPushed)]
    );
}

// ── 5. A new branch at main's tip (probe p4) ────────────────────────────

#[test]
fn a_new_branch_at_mains_tip_has_no_commits_of_its_own() {
    let repo = new_branch_at_mains_tip();
    stage_line_of(&repo, "c3");
    let c3 = oid(&repo, "main~1");
    let before = repo.git(&["for-each-ref", "refs/heads/"]);

    let refusal = refused(planned(&repo, &RewriteRequest::Absorb));
    assert_eq!(
        refusal,
        Refusal::NoCommitsOfItsOwn {
            branch: "fix".to_owned()
        }
    );
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        "fix has no commits of its own yet · c commits them"
    );
    assert!(matches!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: c3 })),
        Refusal::NotOnYourBranch { commit, .. } if commit == c3
    ));
    assert_eq!(repo.git(&["for-each-ref", "refs/heads/"]), before);
}

/// Q1's edge: a tracked branch pushed to its tip has no base below `HEAD`
/// either, but it has commits of its own — they are pushed. `Pushed` is
/// checked before `NotItsOwn`, so the words stay true.
#[test]
fn a_tracked_branch_pushed_to_its_tip_is_all_pushed() {
    let repo = feature(2);
    tracking(&repo, "feature");
    stage_line_of(&repo, "f1");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::Absorb)),
        Refusal::AllPushed {
            branch: "feature".to_owned()
        }
    );
    let f1 = oid(&repo, "feature~1");
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: f1 })),
        Refusal::AlreadyPushed { commit: f1 }
    );
}

/// Q1: a detached `HEAD` never reaches the fallback — both keys are
/// refused before any range is read.
#[test]
fn a_detached_head_never_reaches_the_fallback() {
    let repo = on_main(4);
    let c3 = oid(&repo, "main~1");
    repo.git(&["switch", "-q", "--detach", "main"]);
    stage_line_of(&repo, "c3");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::Absorb)),
        Refusal::NotOnABranch
    );
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: c3 })),
        Refusal::NotOnABranch
    );
}

// ── 7. A second-parent target on a feature branch ───────────────────────

#[test]
fn a_second_parent_target_is_merge_in_range_on_a_feature_branch() {
    let repo = feature(1);
    repo.git(&["switch", "-q", "-c", "side"]);
    commit(&repo, "s1");
    repo.git(&["switch", "-q", "feature"]);
    commit(&repo, "f2");
    repo.git(&["merge", "-q", "--no-ff", "-m", "M", "side"]);
    commit(&repo, "f3");
    stage_line_of(&repo, "s1");
    let merge = oid(&repo, "feature~1");
    let s1 = oid(&repo, "side");

    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: s1 })),
        Refusal::MergeInRange { merge }
    );
}

// ── 9. `a` on a pushed HEAD is amend ────────────────────────────────────

/// A dependent branch one commit above `HEAD`, then back on `home`.
fn dependent_on_head(repo: &TempRepo, home: &str) {
    repo.git(&["switch", "-q", "-c", "dependent"]);
    commit(repo, "d1");
    repo.git(&["switch", "-q", home]);
}

/// The line the rewrite moves holds `dependent` (review m7-06e-20h Minor 3).
fn carries_the_dependent(params: &mergewright_ops::rewrite::RewriteParams) -> bool {
    params
        .line
        .branches
        .iter()
        .any(|tip| tip.name == "refs/heads/dependent")
}

#[test]
fn a_into_a_pushed_head_plans_like_amend() {
    // A tracked feature branch, pushed to its tip.
    let repo = feature(2);
    tracking(&repo, "feature");
    dependent_on_head(&repo, "feature");
    stage_line_of(&repo, "f2");
    let f2 = oid(&repo, "feature");
    let params = planned(&repo, &RewriteRequest::AmendInto { target: f2 }).expect("f2 plans");
    assert_eq!(
        params.todo.last().map(|line| line.action),
        Some(TodoAction::Fixup)
    );
    assert!(carries_the_dependent(&params), "the dependent follows");

    // `main`, pushed to its tip, no upstream.
    let repo = on_main(3);
    let c3 = oid(&repo, "main");
    repo.git(&["update-ref", "refs/remotes/origin/main", &c3.to_string()]);
    dependent_on_head(&repo, "main");
    stage_line_of(&repo, "c3");
    let params = planned(&repo, &RewriteRequest::AmendInto { target: c3 }).expect("c3 plans");
    assert_eq!(
        params.todo.last().map(|line| line.action),
        Some(TodoAction::Fixup)
    );
    assert!(carries_the_dependent(&params), "the dependent follows");
}

// ── 10. One range for both (Minor 6: one test per fixture) ──────────────

#[test]
fn absorb_and_a_agree_with_an_upstream() {
    assert_eq!(
        both_reach_the_same(&upstream_at_f1()).len(),
        1,
        "f2 (f3 is HEAD)"
    );
}

#[test]
fn absorb_and_a_agree_when_pushed_without_an_upstream() {
    assert!(both_reach_the_same(&pushed_without_upstream()).is_empty());
}

#[test]
fn absorb_and_a_agree_under_a_teammates_branch() {
    // f4 is HEAD, set aside; f3 and below are the teammate's.
    assert!(both_reach_the_same(&teammate_on_f3()).is_empty());
}

#[test]
fn absorb_and_a_agree_on_a_new_branch_at_mains_tip() {
    assert!(both_reach_the_same(&new_branch_at_mains_tip()).is_empty());
}
