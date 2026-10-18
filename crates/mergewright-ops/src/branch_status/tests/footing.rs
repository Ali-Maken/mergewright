//! Where a branch stands (01r's R2), as `branch_status` answers it: merged
//! into a long-lived line, merged into an ordinary base, on a line with no
//! commits of its own, or its own; and an orphan.

use super::status::{authors, status};
use super::{Repo, stack};
use crate::branch_status::{Behind, CandidateKind, Merged, Who};

const SARA: (&str, &str) = ("Sara Sahin", "sara@example.com");
const WENDY: (&str, &str) = ("Wendy Wu", "wendy@example.com");

/// Major 2: `sara` merged into main, then `bob/y` cut from main, then main
/// moves on. bob/y holds sara and is nearer, but main took it in.
#[test]
fn merged_into_main_not_into_a_later_branch() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "sara"]);
    repo.commit("s1");
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge sara", "sara"]);
    repo.git(&["switch", "-q", "-c", "bob/y"]);
    repo.commit("y1");
    repo.git(&["switch", "-q", "main"]);
    repo.commit("m2");
    repo.commit("m3");
    repo.commit("m4");
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("sara", CandidateKind::Other),
        repo.cand("bob/y", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara", &candidates, None);
    assert_eq!(sara.merged, Merged::Into("refs/heads/main".to_owned()));
}

/// Major 3: `bob/x` was cut from sara@s1, sara added s2, and bob ran
/// `git merge sara` to catch up. That merges nothing of sara's anywhere.
#[test]
fn a_teammate_syncing_does_not_merge_the_base() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "sara"]);
    repo.commit("s1");
    repo.git(&["switch", "-q", "-c", "bob/x"]);
    repo.commit("x1");
    repo.git(&["switch", "-q", "sara"]);
    repo.commit("s2");
    repo.git(&["switch", "-q", "bob/x"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge sara", "sara"]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("sara", CandidateKind::Other),
        repo.cand("bob/x", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara", &candidates, None);
    assert_eq!(
        sara.merged,
        Merged::NotYet {
            into: Some("refs/heads/main".to_owned())
        }
    );
}

/// Major 4 via integration: sara was cut from `wallet-release` (Wendy's
/// w1 w2) and merged back into it; main holds neither. `Who` is Sara only,
/// not the integration branch's own work.
#[test]
fn who_ignores_the_integration_branchs_own_work() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "wallet-release"]);
    repo.commit_as(WENDY, "w1");
    repo.commit_as(WENDY, "w2");
    repo.git(&["switch", "-q", "-c", "sara"]);
    repo.commit_as(SARA, "s1");
    repo.commit_as(SARA, "s2");
    repo.git(&["switch", "-q", "wallet-release"]);
    repo.git_as(
        WENDY,
        &["merge", "-q", "--no-ff", "-m", "merge sara", "sara"],
    );
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("wallet-release", CandidateKind::Integration),
        repo.cand("sara", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara", &candidates, None);
    assert_eq!(
        sara.merged,
        Merged::Into("refs/heads/wallet-release".to_owned())
    );
    // s2: 5 minutes in (m1 w1 w2 s1 s2).
    assert_eq!(
        authors(&sara),
        (vec!["Sara Sahin".to_owned()], 0, 1_700_000_000 + 300)
    );
}

/// Transitive: sara → wallet-release → main, both `--no-ff`. The trunk
/// decides "merged into main"; `Who` still reads from the nearest holder's
/// entry (wallet's merge of sara), so Wendy's work stays out.
#[test]
fn who_of_a_branch_merged_through_integration_reads_the_nearest_entry() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "wallet-release"]);
    repo.commit_as(WENDY, "w1");
    repo.git(&["switch", "-q", "-c", "sara"]);
    repo.commit_as(SARA, "s1");
    repo.git(&["switch", "-q", "wallet-release"]);
    repo.commit_as(WENDY, "w2");
    repo.git_as(
        WENDY,
        &["merge", "-q", "--no-ff", "-m", "merge sara", "sara"],
    );
    repo.commit_as(WENDY, "w3");
    repo.git(&["switch", "-q", "main"]);
    repo.git(&[
        "merge",
        "-q",
        "--no-ff",
        "-m",
        "merge wallet",
        "wallet-release",
    ]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("wallet-release", CandidateKind::Integration),
        repo.cand("sara", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara", &candidates, None);
    assert_eq!(sara.merged, Merged::Into("refs/heads/main".to_owned()));
    // s1: 3 minutes in (m1 w1 s1).
    assert_eq!(
        authors(&sara),
        (vec!["Sara Sahin".to_owned()], 0, 1_700_000_000 + 180)
    );
}

/// `bob/z` was cut from sara's tip, then merged `ops/ci` in. Its line runs
/// straight through sara: that merge took in ops/ci, not sara.
#[test]
fn a_branch_on_the_tip_that_merges_something_else_has_not_merged_it() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.git(&["switch", "-q", "-c", "ops/ci"]);
    repo.commit("o1");
    repo.git(&["switch", "-q", "-c", "sara", "main"]);
    repo.commit("s1");
    repo.git(&["switch", "-q", "-c", "bob/z"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge ops", "ops/ci"]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("sara", CandidateKind::Other),
        repo.cand("bob/z", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara", &candidates, None);
    assert_eq!(
        sara.merged,
        Merged::NotYet {
            into: Some("refs/heads/main".to_owned())
        }
    );
}

/// Major 4, fresh: `foo` made at main@m2, then main gets m3 m4. It has no
/// commits of its own; it sits on main's line, two behind.
#[test]
fn a_fresh_branch_is_on_its_line() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.commit("m2");
    repo.git(&["branch", "foo"]);
    repo.commit("m3");
    repo.commit("m4");
    repo.git(&["branch", "bar"]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("foo", CandidateKind::Other),
        repo.cand("bar", CandidateKind::Other),
    ];
    let foo = status(&repo, "foo", &candidates, None);
    assert_eq!(foo.route, [candidates[0].clone()]);
    assert_eq!(
        foo.behind,
        Behind::Behind {
            base: "refs/heads/main".to_owned(),
            count: 2
        }
    );
    assert_eq!(foo.merged, Merged::NothingToMerge);
    assert_eq!(foo.who, Who::NoOwnCommits);

    // Made at main's tip: on its line, up to date.
    let bar = status(&repo, "bar", &candidates, None);
    assert_eq!(bar.route, [candidates[0].clone()]);
    assert_eq!(
        bar.behind,
        Behind::UpToDate {
            base: "refs/heads/main".to_owned()
        }
    );
    assert_eq!(bar.merged, Merged::NothingToMerge);
    assert_eq!(bar.who, Who::NoOwnCommits);
}

/// `gh-pages` shares no history with main: no base, nowhere to merge.
#[test]
fn an_unrelated_branch_has_no_base() {
    let repo = Repo::new();
    repo.commit("m1");
    repo.commit("m2");
    repo.git(&["switch", "-q", "--orphan", "gh-pages"]);
    repo.commit("p1");
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("gh-pages", CandidateKind::Other),
    ];
    let pages = status(&repo, "gh-pages", &candidates, None);
    assert!(pages.route.is_empty(), "{:?}", pages.route);
    assert_eq!(pages.behind, Behind::NoBase);
    assert_eq!(pages.merged, Merged::NotYet { into: None });
    assert_eq!(authors(&pages).0, ["Alice Ames"]);
}

/// Regression: an ordinary base that has not moved on merges sara
/// `--no-ff`: that is a merge into it.
#[test]
fn merged_into_an_unmoved_ordinary_base() {
    let repo = stack();
    repo.git(&["switch", "-q", "wallet-release"]);
    repo.git(&["merge", "-q", "--no-ff", "-m", "merge sara", "sara/fix-toc"]);
    let candidates = vec![
        repo.cand("main", CandidateKind::Trunk),
        repo.cand("wallet-release", CandidateKind::Other),
        repo.cand("sara/fix-toc", CandidateKind::Other),
    ];
    let sara = status(&repo, "sara/fix-toc", &candidates, None);
    assert_eq!(
        sara.merged,
        Merged::Into("refs/heads/wallet-release".to_owned())
    );
}
