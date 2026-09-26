//! m7-power-tools/28a (ADR-0049): a file git doesn't keep, where a write
//! would put one of its own, is refused before the write — in the preview
//! and again at `y` — and never journaled.
//!
//! Each test puts his bytes (`MINE`) in a file git doesn't keep, usually
//! ignored through `.git/info/exclude`, then reads the outcome back from
//! the disk and the journal, never from the value the run returned alone.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use graft_core::journal::{EntryStatus, HeadRef};
use graft_core::oid::Oid;
use graft_fixtures::Shape;
use graft_fixtures::remote::LocalRemote;
use graft_git::remote::{RemoteFailure, upstream_of};
use graft_git::repo::RepoHandle;
use graft_ops::apply::{Applied, ApplyError, apply};
use graft_ops::facts::read_facts;
use graft_ops::operation::{
    BranchCreateParams, BranchFromStashParams, CheckoutParams, CheckoutTarget, FastForward,
    MergeParams, Operation, PullParams, RecoverParams, RecoverTarget, Refusal, ResetMode,
    ResetParams, ResetTarget, Source, Tip,
};
use graft_ops::preview::{self, PreviewError, PreviewPlan};

use crate::support::{NOW, Never, TempRepo, context, git_version, journal_for, try_run};

const MINE: &[u8] = b"MINE\n";

// ── the repository ──────────────────────────────────────────────────────

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("an object id")
}

fn sha7(oid: Oid) -> String {
    oid.to_string()[..7].to_owned()
}

/// `main` holds `a.txt`; `other` adds `c.txt` = `theirs` on top. Checked
/// out on `main`, with `c.txt` ignored and holding his bytes.
fn main_and_other() -> TempRepo {
    let repo = main_and_other_unignored();
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_eq!(
        repo.git(&["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
    repo
}

/// The same, with no ignore rule and no `c.txt` yet.
fn main_and_other_unignored() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("c.txt", b"theirs\n");
    repo.commit_all("two on other");
    repo.git(&["switch", "-q", "main"]);
    repo
}

fn switch_to(name: &str) -> Operation {
    Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(format!("refs/heads/{name}")),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    })
}

fn main_tip(repo: &TempRepo) -> Tip {
    Tip {
        name: "refs/heads/main".to_owned(),
        oid: id(repo, "main"),
    }
}

fn merge_other(repo: &TempRepo, ff: FastForward) -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/other".to_owned()),
        into: main_tip(repo),
        source_tip: id(repo, "other"),
        ff,
    })
}

fn reset_main(repo: &TempRepo, mode: ResetMode) -> Operation {
    Operation::Reset(ResetParams {
        target: ResetTarget::Head {
            branch: main_tip(repo),
            mode,
        },
        to: id(repo, "other"),
    })
}

// ── the four checks ─────────────────────────────────────────────────────

/// What one attempt did: the preview, the run, the file's bytes after, and
/// the journal's entry count before and after.
struct Outcome {
    preview: Result<PreviewPlan, PreviewError>,
    applied: Result<Applied, ApplyError>,
    bytes: Vec<u8>,
    entries: (usize, usize),
}

fn entry_count(repo: &TempRepo) -> usize {
    journal_for(&repo.open()).entries().len()
}

fn preview_in(handle: &RepoHandle, op: &Operation) -> Result<PreviewPlan, PreviewError> {
    let facts =
        read_facts(handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    preview::plan(handle, &git_version(), op, &facts, &Never)
}

fn attempt(repo: &TempRepo, op: &Operation, path: &str) -> Outcome {
    let before = entry_count(repo);
    let preview = preview_in(&repo.open(), op);
    let applied = try_run(repo, op, NOW);
    Outcome {
        preview,
        applied,
        bytes: std::fs::read(repo.path().join(path)).unwrap_or_default(),
        entries: (before, entry_count(repo)),
    }
}

fn preview_words(preview: &Result<PreviewPlan, PreviewError>) -> String {
    match preview {
        Err(PreviewError::Refused(refusal)) => refusal.explain(),
        Err(other) => format!("preview failed: {}", other.explain()),
        Ok(_) => "preview drew a plan".to_owned(),
    }
}

fn applied_words(applied: &Result<Applied, ApplyError>) -> String {
    match applied {
        Err(ApplyError::Refused(refusal)) => refusal.explain(),
        Err(other) => format!("apply failed: {}", other.explain()),
        Ok(_) => "apply ran".to_owned(),
    }
}

/// All four of the ticket's checks, asserted together so a red run says
/// every one that failed.
fn assert_refused(repo: &TempRepo, op: &Operation, path: &str, words: &str) {
    let outcome = attempt(repo, op, path);
    let seen = (
        preview_words(&outcome.preview),
        applied_words(&outcome.applied),
        String::from_utf8_lossy(&outcome.bytes).into_owned(),
        outcome.entries.1 - outcome.entries.0,
    );
    let wanted = (
        words.to_owned(),
        words.to_owned(),
        String::from_utf8_lossy(MINE).into_owned(),
        0,
    );
    assert_eq!(
        seen, wanted,
        "(preview, apply, {path}'s bytes, new journal entries)"
    );
}

// ── one-shot writes ─────────────────────────────────────────────────────

#[test]
fn switch_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    assert_refused(
        &repo,
        &switch_to("other"),
        "c.txt",
        "c.txt (ignored) is in the way of switching to other · move it aside first",
    );
}

#[test]
fn switch_detached_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let other = id(&repo, "other");
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Detached(other),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of switching to {} · move it aside first",
            sha7(other)
        ),
    );
}

#[test]
fn new_branch_switch_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = Operation::BranchCreate(BranchCreateParams {
        name: "fresh".to_owned(),
        at: id(&repo, "other"),
        checkout: true,
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of switching to fresh · move it aside first",
    );
    assert!(
        repo.try_git(&["rev-parse", "--verify", "-q", "refs/heads/fresh"])
            .is_err(),
        "no branch made"
    );
}

/// The stash's base tracks `c.txt` and `HEAD` doesn't: `stash branch`
/// switches to that base first.
#[test]
fn branch_from_stash_over_an_ignored_file_is_refused() {
    let repo = main_and_other_unignored();
    repo.git(&["switch", "-q", "other"]);
    repo.write("a.txt", b"a, edited on other\n");
    repo.git(&["stash", "push", "-q"]);
    let stash = id(&repo, "stash@{0}");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    let op = Operation::BranchFromStash(BranchFromStashParams {
        name: "from-stash".to_owned(),
        index: 0,
        commit: stash,
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of switching to from-stash · move it aside first",
    );
}

#[test]
fn merge_fast_forward_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = merge_other(&repo, FastForward::Only);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of merging other · move it aside first",
    );
}

#[test]
fn merge_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = merge_other(&repo, FastForward::Never);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        "c.txt (ignored) is in the way of merging other · move it aside first",
    );
}

#[test]
fn hard_reset_over_an_ignored_file_is_refused() {
    let repo = main_and_other();
    let op = reset_main(&repo, ResetMode::Hard);
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of resetting main to {} · move it aside first",
            sha7(id(&repo, "other"))
        ),
    );
}

/// No false refusal: a soft or mixed reset writes no file.
#[test]
fn soft_and_mixed_resets_are_not_refused() {
    for mode in [ResetMode::Soft, ResetMode::Mixed] {
        let repo = main_and_other();
        let op = reset_main(&repo, mode);
        let outcome = attempt(&repo, &op, "c.txt");
        assert!(
            outcome.preview.is_ok(),
            "{mode:?}: {}",
            preview_words(&outcome.preview)
        );
        assert!(
            outcome.applied.is_ok(),
            "{mode:?}: {}",
            applied_words(&outcome.applied)
        );
        assert_eq!(outcome.bytes, MINE, "{mode:?}: his bytes");
        assert_eq!(
            id(&repo, "main"),
            id(&repo, "other"),
            "{mode:?}: main moved"
        );
    }
}

/// 23b's C1 case: a checked-out branch put back is a `reset --hard`.
#[test]
fn put_back_over_an_ignored_file_is_refused_in_the_way() {
    let repo = main_and_other();
    let main = id(&repo, "main");
    let other = id(&repo, "other");
    let op = Operation::Recover(RecoverParams {
        target: RecoverTarget::MoveRef {
            name: "main".to_owned(),
            to: other,
            expected: main,
        },
        because: "the shell's reset".to_owned(),
    });
    assert_refused(
        &repo,
        &op,
        "c.txt",
        &format!(
            "c.txt (ignored) is in the way of putting main back to {} · move it aside first",
            sha7(other)
        ),
    );
}

#[test]
fn untracked_file_in_the_way_is_refused_before_git() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "c.txt",
        "c.txt is in the way of switching to other · S stashes it first",
    );
}

#[test]
fn a_directory_or_a_file_above_is_in_the_way() {
    // `other` tracks a file `dist`; an ignored directory `dist/` holds his.
    let repo = TempRepo::init();
    repo.write("a.txt", b"a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("dist", b"theirs\n");
    repo.commit_all("dist on other");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"dist/\n");
    repo.write("dist/app.js", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "dist/app.js",
        "dist (ignored) is in the way of switching to other · move it aside first",
    );

    // `other` tracks `a/b`; an ignored file `a` holds his.
    let repo = TempRepo::init();
    repo.write("x.txt", b"x\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.write("a/b", b"theirs\n");
    repo.commit_all("a/b on other");
    repo.git(&["switch", "-q", "main"]);
    repo.write(".git/info/exclude", b"a\n");
    repo.write("a", MINE);
    assert_refused(
        &repo,
        &switch_to("other"),
        "a",
        "a (ignored) is in the way of switching to other · move it aside first",
    );
}

/// No false refusal: ignored build output no commit writes.
#[test]
fn ignored_build_output_elsewhere_is_not_in_the_way() {
    let repo = main_and_other_unignored();
    repo.write(".git/info/exclude", b"target/\n");
    repo.write("target/out.bin", MINE);
    let outcome = attempt(&repo, &switch_to("other"), "target/out.bin");
    assert!(
        outcome.preview.is_ok(),
        "{}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, MINE);
}

/// A tracked file the target replaces with a directory is git's to remove,
/// not his: `a` tracked on `main`, `a/b` on `other`.
#[test]
fn a_tracked_file_above_is_not_in_the_way() {
    let repo = TempRepo::init();
    repo.write("a", b"tracked a\n");
    repo.commit_all("one");
    repo.git(&["switch", "-q", "-c", "other"]);
    repo.git(&["rm", "-q", "a"]);
    repo.write("a/b", b"theirs\n");
    repo.commit_all("a/b on other");
    repo.git(&["switch", "-q", "main"]);
    let outcome = attempt(&repo, &switch_to("other"), "a/b");
    assert!(
        outcome.preview.is_ok(),
        "{}",
        preview_words(&outcome.preview)
    );
    assert!(
        outcome.applied.is_ok(),
        "{}",
        applied_words(&outcome.applied)
    );
    assert_eq!(outcome.bytes, b"theirs\n");
}

// ── stashes ─────────────────────────────────────────────────────────────

fn stash_apply(repo: &TempRepo) -> Operation {
    Operation::StashApply {
        index: 0,
        commit: id(repo, "stash@{0}"),
    }
}

fn stash_words() -> Refusal {
    Refusal::StashInTheWay {
        unsaved: Vec::new(),
        staged: Vec::new(),
        untracked: vec![PathBuf::from("c.txt")],
    }
}

/// The stash holds an untracked `c.txt`; since then `c.txt` is ignored and
/// holds his bytes. The preview's words are unchanged; `apply` now refuses
/// before `begin`.
#[test]
fn stash_apply_over_an_ignored_file_is_refused_at_y() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", b"stashed\n");
    repo.git(&["stash", "push", "-q", "-u"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &stash_apply(&repo),
        "c.txt",
        &stash_words().explain(),
    );
}

/// The stash's index adds `c.txt`; since then `c.txt` is ignored on disk.
#[test]
fn stash_whose_staged_new_file_meets_an_ignored_file() {
    let repo = main_and_other_unignored();
    repo.write("c.txt", b"staged\n");
    repo.git(&["add", "c.txt"]);
    repo.git(&["stash", "push", "-q"]);
    repo.write(".git/info/exclude", b"c.txt\n");
    repo.write("c.txt", MINE);
    assert_refused(
        &repo,
        &stash_apply(&repo),
        "c.txt",
        &stash_words().explain(),
    );
}

// ── pull ────────────────────────────────────────────────────────────────

fn remote_git(remote: &LocalRemote, args: &[&str]) -> String {
    remote
        .git(remote.work(), args)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"))
}

fn pull(remote: &LocalRemote) -> Operation {
    let branch = format!("refs/heads/{}", remote.default_branch());
    let repo = RepoHandle::open(remote.work()).expect("the work clone opens");
    let upstream = upstream_of(&repo, &branch)
        .expect("the upstream reads")
        .expect("the work clone tracks origin");
    Operation::Pull(PullParams { branch, upstream })
}

fn run_in(dir: &Path, op: &Operation) -> Result<Applied, ApplyError> {
    let repo = RepoHandle::open(dir).expect("the repository opens");
    let mut journal = journal_for(&repo);
    let mut ctx = context(&repo, &mut journal, NOW, &Never);
    apply(&mut ctx, op)
}

/// The file the remote's new commit adds, ignored in the work clone and
/// holding his bytes.
fn ignore_what_it_adds(remote: &LocalRemote, tip: &str) -> String {
    // Read from the bare remote: the work clone may not have fetched it.
    let added = remote
        .git(
            remote.bare(),
            &["diff-tree", "--no-commit-id", "--name-only", "-r", tip],
        )
        .unwrap_or_else(|why| panic!("diff-tree: {why}"));
    std::fs::write(
        remote.work().join(".git/info/exclude"),
        format!("{added}\n"),
    )
    .unwrap();
    std::fs::write(remote.work().join(&added), MINE).unwrap();
    assert_eq!(remote_git(remote, &["status", "--porcelain"]), "");
    added
}

/// The tracking tip already tracks the file: refused before the fetch.
#[test]
fn pull_over_an_ignored_file_is_refused() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let tip = remote.commit_on_remote("adds a file").unwrap();
    remote_git(&remote, &["fetch", "-q", "origin"]);
    let added = ignore_what_it_adds(&remote, &tip);
    // A second commit the fetch would bring: the tracking ref still at
    // `tip` afterwards proves the fetch never ran.
    remote.commit_on_remote("after").unwrap();
    let tracking = format!("refs/remotes/origin/{}", remote.default_branch());
    let op = pull(&remote);
    let words = format!(
        "{added} (ignored) is in the way of pulling origin/{} · move it aside first",
        remote.default_branch()
    );
    let handle = RepoHandle::open(remote.work()).unwrap();
    let before = journal_for(&handle).entries().len();

    let preview = preview_in(&handle, &op);
    let applied = run_in(remote.work(), &op);

    assert_eq!(
        (
            preview_words(&preview),
            applied_words(&applied),
            std::fs::read(remote.work().join(&added)).unwrap(),
            journal_for(&handle).entries().len() - before,
            remote_git(&remote, &["rev-parse", &tracking]),
        ),
        (words.clone(), words, MINE.to_vec(), 0, tip),
        "(preview, apply, his bytes, new entries, the tracking ref)"
    );
}

/// The tracking tip lacks the file and the remote has it: Graft's read
/// can't see it, the fetch brings it, and git's `--no-overwrite-ignore`
/// refuses the merge.
#[test]
fn pull_whose_fetch_brings_an_ignored_path_is_refused_by_git() {
    let remote = LocalRemote::new(Shape::Linear).expect("a local remote");
    let before_head = remote_git(&remote, &["rev-parse", "HEAD"]);
    let tip = remote.commit_on_remote("adds a file").unwrap();
    let added = ignore_what_it_adds(&remote, &tip);

    let applied = run_in(remote.work(), &pull(&remote));

    assert_eq!(
        std::fs::read(remote.work().join(&added)).unwrap(),
        MINE,
        "his ignored file was overwritten"
    );
    assert_eq!(remote_git(&remote, &["rev-parse", "HEAD"]), before_head);
    match applied {
        Err(ApplyError::Remote { failure, entry, .. }) => {
            assert_eq!(
                failure,
                RemoteFailure::WouldOverwrite { paths: vec![added] }
            );
            assert!(matches!(entry.status, EntryStatus::Aborted { .. }));
        }
        other => panic!("not git's refusal: {}", applied_words(&other)),
    }
}
