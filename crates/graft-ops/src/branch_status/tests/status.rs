//! `branch_status` against fixture repositories: pushed, merged, behind,
//! who.

use super::{ALICE, Repo, find, stack, stack_candidates};
use crate::branch_status::{
    Behind, BranchStatus, Candidate, CandidateKind, Merged, Pushed, branch_status,
};
use crate::operation::Tip;

fn status(
    repo: &Repo,
    branch: &str,
    candidates: &[Candidate],
    upstream: Option<&Tip>,
) -> BranchStatus {
    let me = find(candidates, branch).clone();
    branch_status(&repo.handle(), &me, candidates, upstream, &()).expect("the status")
}

fn upstream(repo: &Repo, branch: &str, at: &str) -> Tip {
    let name = format!("refs/remotes/origin/{branch}");
    let oid = repo.oid(at);
    repo.git(&["update-ref", &name, &oid.to_string()]);
    Tip { name, oid }
}

#[test]
fn pushed_each_way() {
    let repo = stack();
    let candidates = stack_candidates(&repo);

    let none = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(none.pushed, Pushed::NoUpstream);

    let same = upstream(&repo, "sara/fix-toc", "sara/fix-toc");
    let in_sync = status(&repo, "sara/fix-toc", &candidates, Some(&same));
    assert_eq!(in_sync.pushed, Pushed::InSync);

    let behind_us = upstream(&repo, "sara/fix-toc", "sara/fix-toc~2");
    let to_push = status(&repo, "sara/fix-toc", &candidates, Some(&behind_us));
    assert_eq!(to_push.pushed, Pushed::ToPush(2));

    // The upstream has a commit the branch lacks, and the branch is behind.
    repo.git(&["switch", "-q", "-c", "theirs", "sara/fix-toc"]);
    repo.commit("t1");
    let ahead_of_us = upstream(&repo, "sara/fix-toc", "theirs");
    let to_pull = status(&repo, "sara/fix-toc", &candidates, Some(&ahead_of_us));
    assert_eq!(to_pull.pushed, Pushed::ToPull(1));

    // Both: the branch adds one the upstream lacks.
    repo.git(&["switch", "-q", "sara/fix-toc"]);
    repo.commit("s3");
    let candidates = stack_candidates(&repo);
    let both = status(&repo, "sara/fix-toc", &candidates, Some(&ahead_of_us));
    assert_eq!(both.pushed, Pushed::Both { push: 1, pull: 1 });
}

#[test]
fn merged_into_its_base_after_a_merge_commit() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let before = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        before.merged,
        Merged::NotYet {
            into: "refs/heads/wallet-release".to_owned()
        }
    );

    repo.git(&["switch", "-q", "wallet-release"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge sara", "sara/fix-toc"]);
    let candidates = stack_candidates(&repo);
    let after = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        after.merged,
        Merged::Into("refs/heads/wallet-release".to_owned())
    );
}

/// A teammate's branch made on top of this one holds every commit of it,
/// but that is not "merged".
#[test]
fn a_branch_stacked_on_top_is_not_a_merge() {
    let repo = stack();
    repo.git(&["switch", "-q", "-c", "sara/fix-toc-2", "sara/fix-toc"]);
    repo.commit("t1");
    repo.commit("t2");
    let mut candidates = stack_candidates(&repo);
    candidates.push(repo.cand("sara/fix-toc-2", CandidateKind::Other));
    let sara = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        sara.merged,
        Merged::NotYet {
            into: "refs/heads/wallet-release".to_owned()
        }
    );
}

#[test]
fn behind_counts_the_bases_new_commits() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let before = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        before.behind,
        Behind::UpToDate {
            base: "refs/heads/wallet-release".to_owned()
        }
    );

    repo.git(&["switch", "-q", "wallet-release"]);
    repo.commit("w3");
    repo.commit("w4");
    repo.commit("w5");
    let candidates = stack_candidates(&repo);
    let after = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        after.behind,
        Behind::Behind {
            base: "refs/heads/wallet-release".to_owned(),
            count: 3
        }
    );
}

const BEA: (&str, &str) = ("Bea Brandt", "bea@example.com");
const CAL: (&str, &str) = ("Cal Cruz", "cal@example.com");
const DEV: (&str, &str) = ("Dev Diaz", "dev@example.com");

/// `feature` off `main` with `commits` by the given authors, in order.
fn feature_by(commits: &[(&str, &str)]) -> (Repo, Vec<Candidate>) {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "feature"]);
    for (n, who) in commits.iter().enumerate() {
        repo.commit_as(*who, &format!("f{n}"));
    }
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("feature", CandidateKind::Other),
    ];
    (repo, candidates)
}

fn first_names(status: &BranchStatus) -> Vec<String> {
    status
        .who
        .first
        .iter()
        .map(|n| String::from_utf8_lossy(n).into_owned())
        .collect()
}

#[test]
fn who_one_author() {
    let (repo, candidates) = feature_by(&[ALICE, ALICE]);
    let feature = status(&repo, "feature", &candidates, None);
    assert_eq!(first_names(&feature), ["Alice Ames"]);
    assert_eq!(feature.who.others, 0);
    assert!(!feature.who.everyone);
    // The last of the three commits (m1, f0, f1): 3 minutes in.
    assert_eq!(feature.who.last.seconds, 1_700_000_000 + 180);
}

/// Most commits first; a tie goes to the more recent author.
#[test]
fn who_two_authors() {
    let (repo, candidates) = feature_by(&[BEA, ALICE, BEA, ALICE]);
    let feature = status(&repo, "feature", &candidates, None);
    // Two each; Alice has the newest commit.
    assert_eq!(first_names(&feature), ["Alice Ames", "Bea Brandt"]);
    assert_eq!(feature.who.others, 0);
}

#[test]
fn who_four_authors() {
    let (repo, candidates) = feature_by(&[CAL, DEV, BEA, BEA, BEA, ALICE, ALICE]);
    let feature = status(&repo, "feature", &candidates, None);
    assert_eq!(first_names(&feature), ["Bea Brandt", "Alice Ames"]);
    assert_eq!(feature.who.others, 2);
}

#[test]
fn the_trunk_is_the_main_line() {
    let repo = stack();
    let candidates = stack_candidates(&repo);
    let main = status(&repo, "main", &candidates, None);
    assert_eq!(main.merged, Merged::MainLine);
    assert!(main.route.is_empty());
    assert_eq!(main.behind, Behind::NoBase);
}

/// The trunk is measured against its upstream.
#[test]
fn the_trunk_is_behind_its_upstream() {
    let repo = stack();
    repo.git(&["switch", "-q", "-c", "upstream-main", "main"]);
    repo.commit("u1");
    repo.commit("u2");
    let up = upstream(&repo, "main", "upstream-main");
    let candidates = stack_candidates(&repo);
    let main = status(&repo, "main", &candidates, Some(&up));
    assert_eq!(
        main.behind,
        Behind::Behind {
            base: "refs/remotes/origin/main".to_owned(),
            count: 2
        }
    );
    assert_eq!(main.pushed, Pushed::ToPull(2));
}

#[test]
fn who_on_the_trunk_is_everyone() {
    let (repo, candidates) = feature_by(&[BEA]);
    repo.git(&["switch", "-q", "main"]);
    repo.commit_as(CAL, "m2");
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        candidates[1].clone(),
    ];
    let main = status(&repo, "main", &candidates, None);
    assert!(main.who.everyone);
    assert!(main.who.first.is_empty());
    // main's tip, m2: three minutes in (m1, f0, then m2).
    assert_eq!(main.who.last.seconds, 1_700_000_000 + 180);
}

/// A branch `main` has taken in has no base: every candidate holds it. Its
/// `Who` reads its own history from the tip down (m1 by Alice, f0 and f1
/// by Bea).
#[test]
fn who_of_a_branch_with_no_base() {
    let (repo, _) = feature_by(&[BEA, BEA]);
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge feature", "feature"]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("feature", CandidateKind::Other),
    ];
    let feature = status(&repo, "feature", &candidates, None);
    assert!(feature.route.is_empty());
    assert_eq!(feature.behind, Behind::NoBase);
    assert_eq!(feature.merged, Merged::Into("refs/heads/main".to_owned()));
    assert_eq!(first_names(&feature), ["Bea Brandt", "Alice Ames"]);
    assert_eq!(feature.who.others, 0);
    assert_eq!(feature.who.last.seconds, 1_700_000_000 + 180);
}
