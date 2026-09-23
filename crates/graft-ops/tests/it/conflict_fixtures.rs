//! Stopped operations for `conflict_set.rs` and `finish.rs`: a two-file
//! conflict every kind can stop on, and Graft's own merge and rebase over it.

// Helpers, not `#[test]` functions: see `support.rs` for why `expect` is
// fine here.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_ops::apply::{Applied, ApplyError, Resume, resume};
use graft_ops::conflict::{ConflictSet, operation_conflicts};
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    FactsScope, FastForward, MergeParams, Operation, RebaseParams, Source, Target, Tip,
};

use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, try_run};

/// `a.txt` and `b.txt` conflict between `main` and `feature`; `clean.txt`
/// changes on `feature` alone, so git combines it on its own.
pub fn two_file_conflict() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"1\n2\n3\n");
    repo.write("b.txt", b"x\n");
    repo.write("clean.txt", b"k\nl\n");
    repo.commit_all("base");
    repo.git(&["branch", "feature"]);
    repo.write("a.txt", b"1\nM\n3\n");
    repo.write("b.txt", b"M\n");
    repo.commit_all("main's side");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("a.txt", b"1\nF\n3\n");
    repo.write("b.txt", b"F\n");
    repo.write("clean.txt", b"k2\nl\nnew\n");
    repo.commit_all("feature's side");
    repo.git(&["switch", "-q", "main"]);
    repo
}

/// A second repository that shares `upstream`'s history — fetched, not
/// rebuilt: two repositories built alike share commit ids only when they
/// were built within the same second, so a rebuilt one made `pull` refuse
/// "unrelated histories" whenever a second ticked over. `main` sits on
/// `upstream`'s base with `a.txt` (and `b.txt` when `both`) changed, so a
/// pull of `feature` stops on them.
pub fn downstream_of(upstream: &TempRepo, both: bool) -> TempRepo {
    let repo = TempRepo::init();
    let from = upstream.path().to_str().expect("utf-8 temp path");
    repo.git(&["fetch", "-q", from, "main"]);
    repo.git(&["reset", "-q", "--hard", "FETCH_HEAD~1"]);
    repo.write("a.txt", b"1\nD\n3\n");
    if both {
        repo.write("b.txt", b"D\n");
    }
    repo.commit_all("downstream");
    repo
}

pub fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn head_tip(repo: &TempRepo) -> Tip {
    let name = repo.git(&["symbolic-ref", "HEAD"]);
    Tip {
        oid: id(repo, &name),
        name,
    }
}

/// Graft's merge of `feature` into `main`, which stops on both files.
pub fn graft_merge_stops(repo: &TempRepo) {
    let op = Operation::Merge(MergeParams {
        source: Source::Branch("feature".to_owned()),
        into: head_tip(repo),
        source_tip: id(repo, "feature"),
        ff: FastForward::Default,
    });
    let stopped = try_run(repo, &op, NOW);
    assert!(
        matches!(stopped, Err(ApplyError::Conflicted { .. })),
        "the merge stops on a conflict"
    );
}

/// Graft's rebase of `feature` onto `main`, which stops on both files.
pub fn graft_rebase_stops(repo: &TempRepo) {
    repo.git(&["switch", "-q", "feature"]);
    let op = Operation::Rebase(RebaseParams {
        branch: head_tip(repo),
        onto: Source::Branch("main".to_owned()),
        onto_tip: id(repo, "main"),
    });
    let stopped = try_run(repo, &op, NOW);
    assert!(
        matches!(stopped, Err(ApplyError::Conflicted { .. })),
        "the rebase stops on a conflict"
    );
}

/// Marking a file done the way the resolver does: Graft's own `Stage`,
/// which chains a backup of the unmerged index before `add` drops it.
pub fn graft_stage(repo: &TempRepo, path: &str, at: i64) {
    try_run(
        repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from(path)])),
        at,
    )
    .unwrap_or_else(|why| panic!("staging {path}: {}", why.explain()));
}

/// The conflict set, read with a fresh handle and fresh facts.
pub fn conflict_set(repo: &TempRepo) -> ConflictSet {
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &FactsScope::default()).expect("the facts read");
    operation_conflicts(&handle, &facts).expect("the conflict set reads")
}

pub fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

pub fn run_resume(repo: &TempRepo, how: Resume, at: i64) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, at, &never);
    resume(&mut ctx, how)
}
