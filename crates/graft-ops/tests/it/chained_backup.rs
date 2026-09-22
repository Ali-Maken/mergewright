//! The chained backup a folded step takes (m6-conflicts/08 Decision 2).
//!
//! A step inside a live operation still writes no journal line — one entry
//! from the stop to the finish, M5 hard choice 2 — but it is no longer
//! unrecorded: the repository as it stands is committed onto the live
//! entry's own backup ref first. That is what makes the conflict's stages
//! readable after `⏎`, and what makes a hand edit recoverable after the
//! other side is taken over it.
//!
//! Every assertion is read back out of the repository with `git`.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::PathBuf;

use graft_core::journal::{EntryStatus, JournalEntry};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::status::UnmergedHow;
use graft_ops::apply::{Applied, ApplyError, Resume, apply, resume};
use graft_ops::operation::{
    FastForward, MergeParams, Operation, ResolveWholeParams, Side, Source, Tip,
};

use crate::support::{NOW, Never, TempRepo, context, journal_for};

// ── the stopped span ────────────────────────────────────────────────────

/// A merge Graft ran that stopped on `shared.txt` (`UU`) and `gone.txt`
/// (`UD` — ours modified it, theirs deleted it).
///
/// Two shapes on purpose: the `UU` one is what `d` restarts, the `UD` one is
/// the reviewer's data-loss path, where taking theirs is `git rm --quiet`.
fn a_stopped_graft_merge() -> (TempRepo, JournalEntry) {
    let repo = TempRepo::init();
    repo.write("shared.txt", b"base\n");
    repo.write("gone.txt", b"base\n");
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    repo.write("shared.txt", b"main's line\n");
    repo.write("gone.txt", b"main's line\n");
    repo.commit_all("two on main");
    repo.git(&["switch", "-q", "feature"]);
    repo.write("shared.txt", b"feature's line\n");
    repo.git(&["rm", "-q", "gone.txt"]);
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);

    let branch = repo.git(&["symbolic-ref", "HEAD"]);
    let op = Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/feature".to_owned()),
        into: Tip {
            name: branch.clone(),
            oid: oid_at(&repo, &branch),
        },
        source_tip: oid_at(&repo, "refs/heads/feature"),
        ff: FastForward::Never,
    });

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    let entry = match apply(&mut ctx, &op) {
        Err(ApplyError::Conflicted { entry, .. }) => *entry,
        other => panic!("the merge should have stopped: {other:?}"),
    };
    drop(journal);

    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(repo.exists(".git/MERGE_HEAD"), "the merge is live");
    assert!(entry.backup_ref.is_some(), "and it took a backup");
    (repo, entry)
}

fn oid_at(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

/// Where the entry's backup ref points now.
fn backup_tip(repo: &TempRepo, entry: &JournalEntry) -> Oid {
    let name = entry.backup_ref.as_deref().expect("a backup ref");
    oid_at(repo, name)
}

/// Run one operation with a stop of the caller's choosing.
fn step(repo: &TempRepo, op: &Operation, at: i64, stop: &dyn Stop) -> Result<Applied, ApplyError> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, at, stop);
    apply(&mut ctx, op)
}

fn resolve(path: &str, side: Side, how: UnmergedHow) -> Operation {
    Operation::ResolveWholeFile(ResolveWholeParams {
        path: PathBuf::from(path),
        side,
        how,
    })
}

fn journal_lines(repo: &TempRepo) -> usize {
    std::fs::read_to_string(repo.path().join(".git/graft/undo-journal.jsonl"))
        .expect("the journal reads")
        .lines()
        .count()
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    let handle = repo.open();
    journal_for(&handle).entries().to_vec()
}

// ── the tests ───────────────────────────────────────────────────────────

/// A folded step moves the entry's backup ref to a new commit whose first
/// parent is the old target, whose `unmerged/` holds the path's three
/// stages — and writes no journal line.
#[test]
fn folded_step_chains_a_backup() {
    let (repo, entry) = a_stopped_graft_merge();
    let before_ref = backup_tip(&repo, &entry);
    let before_lines = journal_lines(&repo);

    let applied = step(
        &repo,
        &resolve("shared.txt", Side::Ours, UnmergedHow::BothModified),
        NOW + 1,
        &Never,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain()));

    let chained = applied.step_backup.expect("the step chained a backup");
    assert_eq!(
        backup_tip(&repo, &entry),
        chained,
        "the ref moved to the chained commit"
    );
    assert_eq!(
        oid_at(&repo, &format!("{chained}^1")),
        before_ref,
        "whose first parent is what the ref held before"
    );

    // The stages the step was about to drop are in it.
    let held = repo.git(&[
        "ls-tree",
        "-r",
        "--name-only",
        &format!("{chained}:unmerged"),
    ]);
    for stage in 1..=3 {
        assert!(
            held.lines()
                .any(|line| line == format!("{stage}/shared.txt")),
            "stage {stage} of shared.txt is kept:\n{held}"
        );
    }

    assert_eq!(
        journal_lines(&repo),
        before_lines,
        "and the journal file is untouched: a folded step is still one entry"
    );
}

/// The reviewer's scenario, end to end (m6 g03 review, M2).
///
/// `e` opens a `UD` file in `$EDITOR`, the user writes something, then `t`
/// takes theirs — which for `deleted by them` is `git rm --quiet`, with no
/// confirm, no journal line and a span backup that predates the edit. Before
/// the chain, those bytes were gone for good.
#[test]
fn a_hand_edit_is_recoverable_after_the_other_side_is_taken() {
    let (repo, _entry) = a_stopped_graft_merge();
    let edit = b"main's line\nand the paragraph I wrote by hand\n";
    repo.write("gone.txt", edit);

    let applied = step(
        &repo,
        &resolve("gone.txt", Side::Theirs, UnmergedHow::DeletedByThem),
        NOW + 1,
        &Never,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain()));

    assert!(
        !repo.exists("gone.txt"),
        "taking theirs removed the file, edit and all"
    );

    let chained = applied.step_backup.expect("the step chained a backup");
    assert_eq!(
        repo.git(&["cat-file", "blob", &format!("{chained}:worktree/gone.txt")])
            .into_bytes(),
        // `git` trims the trailing newline on the way back through the
        // helper, so compare against the same trimming.
        String::from_utf8_lossy(edit).trim_end().as_bytes(),
        "the hand edit is in the chained commit's working tree"
    );
}

/// A stop raised before the chained snapshot: `Stopped`, the ref unmoved,
/// the step not run.
#[test]
fn stop_before_chain_writes_nothing() {
    struct Always;
    impl Stop for Always {
        fn should_stop(&self) -> bool {
            true
        }
    }

    let (repo, entry) = a_stopped_graft_merge();
    let before_ref = backup_tip(&repo, &entry);
    let before_bytes = repo.read("shared.txt");

    let stopped = step(
        &repo,
        &resolve("shared.txt", Side::Ours, UnmergedHow::BothModified),
        NOW + 1,
        &Always,
    );
    assert!(
        matches!(stopped, Err(ApplyError::Stopped)),
        "a stop is a stop, not a git failure: {stopped:?}"
    );
    assert_eq!(
        backup_tip(&repo, &entry),
        before_ref,
        "the ref did not move"
    );
    assert_eq!(
        repo.read("shared.txt"),
        before_bytes,
        "and neither did the file"
    );
    assert_eq!(
        repo.git(&["ls-files", "-u", "--", "shared.txt"])
            .lines()
            .count(),
        3,
        "the path is still unmerged"
    );
}

/// Three chained commits change nothing the journal answers: the same entry
/// count, the same `next_undo`, the same line count. The app's "`u` is
/// refused while an operation is live" is untouched by any of it.
#[test]
fn chain_adds_no_entries() {
    let (repo, _entry) = a_stopped_graft_merge();
    let before_lines = journal_lines(&repo);
    let before_entries = entries(&repo);
    let before_next = graft_ops::journal::next_undo(&before_entries).map(|entry| entry.id.clone());

    for (at, op) in [
        (
            NOW + 1,
            resolve("shared.txt", Side::Ours, UnmergedHow::BothModified),
        ),
        (
            NOW + 2,
            resolve("gone.txt", Side::Ours, UnmergedHow::DeletedByThem),
        ),
    ] {
        step(&repo, &op, at, &Never).unwrap_or_else(|why| panic!("{}", why.explain()));
    }

    let after_entries = entries(&repo);
    assert_eq!(journal_lines(&repo), before_lines);
    assert_eq!(after_entries.len(), before_entries.len());
    assert_eq!(
        graft_ops::journal::next_undo(&after_entries).map(|entry| entry.id.clone()),
        before_next,
        "the chain is invisible to undo"
    );
}

/// `resume(Continue)` chains one of its own first, and the closing
/// after-commit parents the chain tip — so nothing in the chain becomes
/// unreachable when the ref moves to the after-commit.
#[test]
fn resume_chains_then_after_snapshot_parents_the_chain() {
    let (repo, entry) = a_stopped_graft_merge();
    for op in [
        resolve("shared.txt", Side::Ours, UnmergedHow::BothModified),
        resolve("gone.txt", Side::Ours, UnmergedHow::DeletedByThem),
    ] {
        step(&repo, &op, NOW + 1, &Never).unwrap_or_else(|why| panic!("{}", why.explain()));
    }
    let after_steps = backup_tip(&repo, &entry);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 3, &never);
    resume(&mut ctx, Resume::Continue).unwrap_or_else(|why| panic!("{}", why.explain()));
    drop(journal);

    let sealed = backup_tip(&repo, &entry);
    assert_ne!(sealed, after_steps, "the ref moved to the after-commit");
    let chained_by_resume = oid_at(&repo, &format!("{sealed}^1"));
    assert_ne!(
        chained_by_resume, after_steps,
        "resume took a chained snapshot of its own before running git"
    );
    assert_eq!(
        oid_at(&repo, &format!("{chained_by_resume}^1")),
        after_steps,
        "which is parented on the last folded step's commit"
    );
    // Exits non-zero when it is not an ancestor, and `git` panics on that.
    repo.git(&[
        "merge-base",
        "--is-ancestor",
        &after_steps.to_string(),
        &sealed.to_string(),
    ]);

    // And the span really did finish.
    assert!(!repo.exists(".git/MERGE_HEAD"));
    assert_eq!(
        entries(&repo)[0].status,
        EntryStatus::Committed,
        "one entry, closed once"
    );
    assert_eq!(journal_lines(&repo), 2, "and exactly its two lines");
}
