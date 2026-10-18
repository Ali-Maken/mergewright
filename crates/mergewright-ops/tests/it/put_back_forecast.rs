//! The preview forecasts the put-back of edits set aside around a rebase or
//! a switch (ADR-0045 §8, m6-conflicts/84): `PreviewPlan::put_back`.
//!
//! Every repository here is real. The "writes nothing" test reads the index
//! bytes, the refs, the stash list and the status before and after `plan`.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::run::Stop;
use mergewright_git::version::GitVersion;
use mergewright_ops::facts::read_facts;
use mergewright_ops::operation::{CheckoutParams, CheckoutTarget, Operation, RebaseParams, Source, Tip};
use mergewright_ops::preview::{self, ConflictForecast, PreviewError, PreviewPlan};

use crate::support::{Never, TempRepo, git_version};

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn checkout_to(repo: &TempRepo, to: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(to.to_owned()),
        head_before: HeadRef::Branch(repo.git(&["symbolic-ref", "HEAD"])),
        on_edits: mergewright_ops::operation::OnEdits::SetAside,
    })
}

fn rebase_onto(repo: &TempRepo, onto: &str) -> Operation {
    let name = repo.git(&["symbolic-ref", "HEAD"]);
    Operation::Rebase(RebaseParams::new(
        Tip {
            oid: id(repo, &name),
            name,
        },
        Source::Branch(onto.to_owned()),
        id(repo, onto),
    ))
}

fn plan_with(
    repo: &TempRepo,
    op: &Operation,
    git: &GitVersion,
    stop: &dyn Stop,
) -> Result<PreviewPlan, PreviewError> {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, git, op, &facts, stop)
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    plan_with(repo, op, &git_version(), &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

/// `main` and `feature` both change `base.txt`; the user has edited it on
/// `main`. The same shape as mergewright-ops' `set_aside::clashing_checkout` and
/// the app's clash walk: switching to `feature` clashes on the put-back.
fn switch_that_clashes() -> (TempRepo, Operation) {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("base.txt", b"feature's line\n");
    repo.commit_all("feature changes base");
    repo.git(&["switch", "-q", "main"]);
    repo.write("base.txt", b"my edit\n");
    let op = checkout_to(&repo, "refs/heads/feature");
    (repo, op)
}

/// `main` and `feature` hold `base.txt` and `a.txt` alike and differ in one
/// file each; the user's edit to `a.txt` goes back cleanly anywhere.
fn diverged() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("main.txt", b"main\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// `feature` is rebased onto `main`, which changed `base.txt`; the replay is
/// clean (feature's own commit touches `feature.txt`), and the user's
/// uncommitted edit to `base.txt` clashes with main's line when put back.
fn rebase_that_clashes() -> (TempRepo, Operation) {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("base.txt", b"main's line\n");
    repo.commit_all("main changes base");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.commit_all("one on feature");
    repo.write("base.txt", b"my edit\n");
    let op = rebase_onto(&repo, "refs/heads/main");
    (repo, op)
}

#[test]
fn a_switch_whose_put_back_clashes_forecasts_the_clash() {
    let (repo, op) = switch_that_clashes();
    let plan = plan_of(&repo, &op);
    assert!(plan.set_aside.is_some(), "the edit is set aside");
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Likely {
            paths: vec![PathBuf::from("base.txt")]
        })
    );
    assert_eq!(plan.conflicts, ConflictForecast::Clean, "the switch itself");
}

#[test]
fn a_switch_whose_put_back_is_clean_forecasts_clean() {
    let repo = diverged();
    repo.write("a.txt", b"my edit\n");
    let op = checkout_to(&repo, "refs/heads/feature");
    let plan = plan_of(&repo, &op);
    assert_eq!(plan.put_back, Some(ConflictForecast::Clean));
}

#[test]
fn a_rebase_whose_put_back_clashes_forecasts_the_clash() {
    let (repo, op) = rebase_that_clashes();
    let plan = plan_of(&repo, &op);
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Clean,
        "the replay is clean"
    );
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Likely {
            paths: vec![PathBuf::from("base.txt")]
        })
    );
}

#[test]
fn a_rebase_whose_put_back_is_clean_forecasts_clean() {
    let repo = diverged();
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"my edit\n");
    let op = rebase_onto(&repo, "refs/heads/main");
    let plan = plan_of(&repo, &op);
    assert_eq!(plan.put_back, Some(ConflictForecast::Clean));
}

/// The replay itself may stop (both sides rewrite `shared.txt`): the put-back
/// is then not forecast — it would be onto a tree nobody can predict.
#[test]
fn a_rebase_that_may_stop_leaves_the_put_back_not_checked() {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.write("other.txt", b"other\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("shared.txt", b"main's line\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("shared.txt", b"feature's line\n");
    repo.commit_all("one on feature");
    repo.write("other.txt", b"my edit\n");
    let op = rebase_onto(&repo, "refs/heads/main");
    let plan = plan_of(&repo, &op);
    assert!(matches!(plan.conflicts, ConflictForecast::StopsAt { .. }));
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::NotChecked(
            "the rebase may stop first".to_owned()
        ))
    );
}

#[test]
fn a_clean_tree_has_no_put_back_forecast() {
    let repo = diverged();
    let checkout = checkout_to(&repo, "refs/heads/feature");
    assert_eq!(plan_of(&repo, &checkout).put_back, None);
    repo.git(&["switch", "-q", "feature"]);
    let rebase = rebase_onto(&repo, "refs/heads/main");
    assert_eq!(plan_of(&repo, &rebase).put_back, None);
}

/// Everything `plan` could have written, read from git: the index file's
/// bytes, every ref, the stash list, the status, and the git dir's top-level
/// names (no temporary index left behind).
fn written(repo: &TempRepo) -> (Vec<u8>, String, String, String, Vec<String>) {
    let mut names: Vec<String> = std::fs::read_dir(repo.path().join(".git"))
        .expect("the git dir reads")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    (
        std::fs::read(repo.path().join(".git/index")).expect("the index reads"),
        repo.git(&["for-each-ref"]),
        repo.git(&["stash", "list"]),
        // `--no-optional-locks`: a plain status refreshes `.git/index`.
        repo.git(&["--no-optional-locks", "status", "--porcelain=v2"]),
        names,
    )
}

#[test]
fn the_put_back_forecast_writes_no_ref_index_or_file() {
    for (repo, op) in [switch_that_clashes(), rebase_that_clashes()] {
        // A clean file whose stat no longer matches the index: `stash
        // create` refreshes the index it reads, which would rewrite
        // `.git/index` if it ran on the real one.
        let clean = if repo.exists("feature.txt") {
            "feature.txt"
        } else {
            "base.txt"
        };
        std::process::Command::new("touch")
            .args(["-t", "200001010000"])
            .arg(repo.path().join(clean))
            .status()
            .expect("touch runs");
        let before = written(&repo);
        let plan = plan_of(&repo, &op);
        assert!(plan.put_back.is_some());
        assert_eq!(written(&repo), before, "{op:?}");
    }
}

/// A `Stop` that lets the first `n` polls through, then stops.
struct StopAfter {
    left: AtomicUsize,
}

impl Stop for StopAfter {
    fn should_stop(&self) -> bool {
        self.left
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |left| {
                left.checked_sub(1)
            })
            .is_err()
    }
}

/// `plan`'s own first poll passes; a checkout's kind plan polls no more, so
/// the next poll is the put-back forecast's, which must answer `Stopped`, not
/// a forecast with a hole in it.
#[test]
fn the_put_back_forecast_honours_stop() {
    let (repo, op) = switch_that_clashes();
    let stop = StopAfter {
        left: AtomicUsize::new(1),
    };
    let answer = plan_with(&repo, &op, &git_version(), &stop);
    assert!(matches!(answer, Err(PreviewError::Stopped)), "{answer:?}");
}

/// On a git older than `merge-tree --merge-base` (2.40; Mergewright's minimum is
/// 2.38, PROMPT.md §1) the put-back is not checked, and says why: a hole,
/// never a clean bill and never a failed preview.
#[test]
fn an_old_git_leaves_the_put_back_not_checked() {
    let (repo, op) = switch_that_clashes();
    let old = GitVersion {
        major: 2,
        minor: 39,
        patch: 0,
    };
    let plan = plan_with(&repo, &op, &old, &Never).expect("the preview still draws");
    match plan.put_back {
        Some(ConflictForecast::NotChecked(why)) => assert!(why.contains("2.40"), "{why}"),
        other => panic!("not checked on git 2.39: {other:?}"),
    }
}

/// 05b: `feature`'s two commits (`x.txt`, `y.txt`) are squash-merged into
/// `main`, which then rewrites `x.txt`; the user has an edit to `x.txt`.
/// The completed rebase drops both commits, so `feature` lands on `main`
/// itself, and the edit goes back onto main's `x.txt`: a clash on `x.txt`.
/// Measured by one merge of `HEAD` and `main` instead (dropped commits
/// included), the landing would conflict on `x.txt` itself and the
/// put-back would read "the rebase may stop first".
#[test]
fn a_rebase_with_drops_measures_the_put_back_from_where_it_lands() {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "feature"]);
    repo.write("x.txt", b"feature\n");
    repo.commit_all("x");
    repo.write("y.txt", b"y\n");
    repo.commit_all("y");
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["merge", "-q", "--squash", "feature"]);
    repo.git(&["commit", "-q", "-m", "feature (#7)"]);
    repo.write("x.txt", b"main later\n");
    repo.commit_all("main rewrites x");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("x.txt", b"my edit\n");

    let handle = repo.open();
    let op = rebase_onto(&repo, "refs/heads/main");
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    let (op, plan) = preview::plan_as_run(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    let Operation::Rebase(params) = &op else {
        panic!("a rebase");
    };
    assert_eq!(params.drops.len(), 2, "both commits dropped");
    assert_eq!(plan.conflicts, ConflictForecast::Clean, "the replay itself");
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::Likely {
            paths: vec![PathBuf::from("x.txt")]
        })
    );
}
