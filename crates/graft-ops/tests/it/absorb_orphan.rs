//! A branch that shares no history with the default one (06f ruling,
//! 2026-09-27): gh-pages, docs, a vendored import. Its commits are its own,
//! so the walk falls back exactly as on the default branch; "no commits of
//! its own" is kept for a base that *is* `HEAD`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use graft_ops::absorb::StaysWhy;
use graft_ops::operation::Refusal;
use graft_ops::rewrite::RewriteRequest;

use crate::absorb::oid;
use crate::absorb_unpushed::{
    absorb_map_of, both_reach_the_same, commit, on_main, one_line, planned, refused, stage_line_of,
};
use crate::support::TempRepo;

/// `main` c1 c2; `docs`, an orphan, d1 … d<n>, checked out.
fn orphan(n: usize) -> TempRepo {
    let repo = on_main(2);
    repo.git(&["switch", "-q", "--orphan", "docs"]);
    for k in 1..=n {
        commit(&repo, &format!("d{k}"));
    }
    repo
}

/// Test 3's (gh-pages): `docs` d1..d3, `refs/remotes/origin/docs` at d2.
fn pushed_orphan() -> TempRepo {
    let repo = orphan(3);
    let d2 = oid(&repo, "docs~1");
    repo.git(&["update-ref", "refs/remotes/origin/docs", &d2.to_string()]);
    repo
}

fn short(commit: graft_core::oid::Oid) -> String {
    commit.to_string()[..7].to_owned()
}

#[test]
fn an_orphan_branch_falls_back() {
    let repo = orphan(3);
    let main_before = oid(&repo, "main");
    stage_line_of(&repo, "d2");
    let d1 = oid(&repo, "docs~2");
    let d2 = oid(&repo, "docs~1");
    let c2 = oid(&repo, "main");

    let map = absorb_map_of(&repo).expect("it maps");
    assert_eq!(map.placed, vec![(one_line("d2.txt"), d2)]);
    planned(&repo, &RewriteRequest::Absorb).expect("absorb plans");

    let params = planned(&repo, &RewriteRequest::AmendInto { target: d2 }).expect("d2 plans");
    let names: Vec<_> = params
        .line
        .branches
        .iter()
        .map(|tip| tip.name.clone())
        .collect();
    assert_eq!(
        names,
        vec!["refs/heads/docs".to_owned()],
        "the line: docs alone"
    );
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: d1 })),
        Refusal::RootCommit { commit: d1 }
    );
    assert!(matches!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: c2 })),
        Refusal::NotOnYourBranch { commit, .. } if commit == c2
    ));
    assert_eq!(oid(&repo, "main"), main_before);
}

#[test]
fn an_orphan_branch_past_ten_is_beyond_reach() {
    let repo = orphan(12);
    stage_line_of(&repo, "d2");
    let d2 = oid(&repo, "docs~10");

    let refusal = refused(planned(&repo, &RewriteRequest::AmendInto { target: d2 }));
    assert_eq!(
        refusal.explain(&crate::support::Letters),
        format!(
            "{} is more than 10 commits back · on docs a reaches the last 10 unpushed commits",
            short(d2)
        )
    );
}

#[test]
fn a_pushed_orphan_branch_stops_at_the_remote() {
    let repo = pushed_orphan();
    let d2 = oid(&repo, "docs~1");
    let d3 = oid(&repo, "docs");
    stage_line_of(&repo, "d2");
    assert_eq!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: d2 })),
        Refusal::AlreadyPushed { commit: d2 }
    );

    stage_line_of(&repo, "d3");
    let map = absorb_map_of(&repo).expect("it maps");
    assert_eq!(map.placed, vec![(one_line("d3.txt"), d3)]);
    assert_eq!(
        map.stays,
        vec![(one_line("d2.txt"), StaysWhy::AlreadyPushed)]
    );
}

#[test]
fn a_branch_behind_mains_tip_has_no_commits_of_its_own() {
    let repo = on_main(4);
    repo.git(&["branch", "fix", "HEAD~2"]);
    repo.git(&["switch", "-q", "fix"]);
    stage_line_of(&repo, "c2");
    let c1 = oid(&repo, "fix~1");

    let refusal = refused(planned(&repo, &RewriteRequest::Absorb));
    assert!(
        matches!(&refusal, Refusal::NoCommitsOfItsOwn { branch } if branch.as_str().ends_with("fix")),
        "{refusal:?}"
    );
    assert!(matches!(
        refused(planned(&repo, &RewriteRequest::AmendInto { target: c1 })),
        Refusal::NotOnYourBranch { commit, .. } if commit == c1
    ));
}

#[test]
fn absorb_and_a_agree_on_an_orphan_branch() {
    assert_eq!(
        both_reach_the_same(&orphan(3)).len(),
        1,
        "d2 (d1 is the root, d3 HEAD)"
    );
    assert!(
        both_reach_the_same(&pushed_orphan()).is_empty(),
        "d2 pushed"
    );
}
