//! The merge preview's look (ease-pass 10), against real git: what comes
//! in, what clashes, and one forecast for both the box and `conflicts`.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::history_reads::{MergeCheck, UNRELATED};
use graft_ops::facts::read_facts;
use graft_ops::operation::{FastForward, MergeParams, Operation, Source, Tip};
use graft_ops::preview::{self, ConflictForecast, PreviewPlan};

use crate::support::{Never, TempRepo, git_version};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn merge_of(repo: &TempRepo, source: &str) -> Operation {
    let head = repo.git(&["symbolic-ref", "HEAD"]);
    Operation::Merge(MergeParams {
        source: Source::Branch(format!("refs/heads/{source}")),
        into: Tip {
            name: head.clone(),
            oid: id(repo, &head),
        },
        source_tip: id(repo, source),
        ff: FastForward::Default,
    })
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("preview: {}", why.explain(&crate::support::Letters)))
}

/// `main`, and `topic` off its first commit with three commits of its
/// own: one touching `shared.txt` in two places `main` also changed.
fn clashing() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"a\nb\nc\nd\ne\nf\ng\n");
    repo.commit_all("one");
    repo.git(&["branch", "topic"]);
    repo.write("shared.txt", b"A main\nb\nc\nd\ne\nf\nG main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "topic"]);
    repo.write("new.txt", b"new\n");
    repo.commit_all("add new");
    repo.write("shared.txt", b"A topic\nb\nc\nd\ne\nf\nG topic\n");
    repo.commit_all("change shared");
    repo.write("other.txt", b"other\n");
    repo.commit_all("add other");
    repo.git(&["switch", "-q", "main"]);
    repo
}

#[test]
fn a_clashing_merge_looks_once_and_names_the_clash_with_its_places() {
    let repo = clashing();
    let plan = plan_of(&repo, &merge_of(&repo, "topic"));
    let look = plan.merge.as_ref().expect("a merge look");
    assert_eq!(look.incoming.commits.len(), 3, "three come in");
    assert_eq!(
        look.incoming
            .commits
            .iter()
            .map(|commit| commit.subject.as_str())
            .collect::<Vec<_>>(),
        ["add new", "change shared", "add other"],
        "oldest first"
    );
    let MergeCheck::Checked(forecast) = &look.check else {
        panic!("checked: {:?}", look.check);
    };
    assert_eq!(forecast.changed.len(), 3);
    assert_eq!(forecast.clashes.len(), 1);
    assert_eq!(forecast.clashes[0].path, PathBuf::from("shared.txt"));
    assert_eq!(forecast.clashes[0].places, Some(2));
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: vec![PathBuf::from("shared.txt")]
        },
        "derived from the same look"
    );
    assert!(!look.unrelated());
}

#[test]
fn a_clean_merge_is_clean_from_the_look() {
    let repo = clashing();
    repo.git(&["switch", "-q", "-c", "side", "main~1"]);
    repo.write("side.txt", b"side\n");
    repo.commit_all("on side");
    let plan = plan_of(&repo, &merge_of(&repo, "main"));
    let look = plan.merge.as_ref().expect("a merge look");
    let MergeCheck::Checked(forecast) = &look.check else {
        panic!("checked: {:?}", look.check);
    };
    assert!(forecast.clashes.is_empty());
    assert_eq!(plan.conflicts, ConflictForecast::Clean);
}

/// The one-forecast proof: `forecast_merge` (what `forecast_one` reads)
/// answers unrelated tips with merge-tree's own refusal words, while
/// `merge_forecast` answers [`UNRELATED`]. `conflicts` carrying
/// [`UNRELATED`] shows it came from the look, not a second forecast.
#[test]
fn an_unrelated_merge_is_not_checked_with_the_unrelated_reason() {
    let repo = clashing();
    repo.git(&["switch", "-q", "--orphan", "stray"]);
    repo.write("stray.txt", b"stray\n");
    repo.git(&["add", "stray.txt"]);
    repo.git(&["commit", "-qm", "stray root"]);
    repo.git(&["switch", "-qf", "main"]);
    let plan = plan_of(&repo, &merge_of(&repo, "stray"));
    let look = plan.merge.as_ref().expect("a merge look");
    assert!(look.unrelated());
    assert_eq!(look.incoming.commits.len(), 1);
    assert_eq!(
        plan.conflicts,
        ConflictForecast::NotChecked(UNRELATED.to_owned())
    );
}

#[test]
fn an_up_to_date_merge_has_no_look() {
    let repo = clashing();
    repo.git(&["branch", "behind", "main~1"]);
    let plan = plan_of(&repo, &merge_of(&repo, "behind"));
    assert_eq!(plan.merge, None);
}

/// A fast-forward makes no merge commit: the look says so, and a true
/// merge's does not.
#[test]
fn a_fast_forward_plan_says_so_and_a_true_merge_does_not() {
    let repo = clashing();
    repo.git(&["switch", "-q", "-c", "behind", "main~1"]);
    let plan = plan_of(&repo, &merge_of(&repo, "main"));
    let look = plan.merge.as_ref().expect("a fast-forward has a look");
    assert!(look.fast_forward, "behind moves up to main");
    assert_eq!(look.incoming.commits.len(), 1, "main's one commit comes in");

    repo.git(&["switch", "-q", "main"]);
    let plan = plan_of(&repo, &merge_of(&repo, "topic"));
    let look = plan.merge.as_ref().expect("a merge look");
    assert!(!look.fast_forward, "topic and main diverged");
}
