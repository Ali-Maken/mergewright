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

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::PathBuf;

use mergewright_core::journal::{EntryStatus, JournalEntry};
use mergewright_core::oid::Oid;
use mergewright_git::run::Stop;
use mergewright_git::status::UnmergedHow;
use mergewright_ops::apply::{Applied, ApplyError, Resume, apply, resume};
use mergewright_ops::operation::{
    FastForward, MergeParams, Operation, Refusal, ResolveWholeParams, RestartKind, RestartParams,
    Side, Source, Target, Tip,
};

use crate::support::{NOW, Never, TempRepo, context, journal_for};
use mergewright_core::on_disk::CURRENT_DIR;

// ── the stopped span ────────────────────────────────────────────────────

/// A merge Mergewright ran that stopped on `shared.txt` (`UU`) and `gone.txt`
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
    std::fs::read_to_string(
        repo.path()
            .join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl")),
    )
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
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

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
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

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
    let before_next =
        mergewright_ops::journal::next_undo(&before_entries).map(|entry| entry.id.clone());

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
        step(&repo, &op, at, &Never)
            .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    }

    let after_entries = entries(&repo);
    assert_eq!(journal_lines(&repo), before_lines);
    assert_eq!(after_entries.len(), before_entries.len());
    assert_eq!(
        mergewright_ops::journal::next_undo(&after_entries).map(|entry| entry.id.clone()),
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
        step(&repo, &op, NOW + 1, &Never)
            .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    }
    let after_steps = backup_tip(&repo, &entry);

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 3, &never);
    resume(&mut ctx, Resume::Continue { solved: Vec::new() })
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
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

// ── `d` after `⏎` (m6-conflicts/08 scope 5(c)) ──────────────────────────

/// `git ls-files -u -- <path>`, as git prints it.
fn unmerged_lines(repo: &TempRepo, path: &str) -> String {
    repo.git(&["--literal-pathspecs", "ls-files", "-u", "--", path])
}

/// `git ls-files --resolve-undo`: git's own resolve-undo (REUC) extension.
fn resolve_undo(repo: &TempRepo) -> String {
    repo.git(&["ls-files", "--resolve-undo"])
}

/// `⏎` on a path: the resolver writes the resolved bytes, then stages them.
fn mark_resolved(repo: &TempRepo, path: &str, bytes: &[u8], at: i64) {
    repo.write(path, bytes);
    step(
        repo,
        &Operation::Stage(Target::Paths(vec![PathBuf::from(path)])),
        at,
        &Never,
    )
    .unwrap_or_else(|why| panic!("staging {path}: {}", why.explain(&crate::support::Letters)));
}

fn restart(path: &str) -> Operation {
    Operation::RestartConflict(RestartParams {
        path: PathBuf::from(path),
        kind: RestartKind::Markers,
    })
}

/// Whether the recorded command lines include the index restore this group
/// added — which is what says **Mergewright** put the stages back.
fn put_stages_back(applied: &Applied) -> bool {
    applied
        .entry
        .commands
        .iter()
        .any(|line| line.contains("update-index") && line.contains("--index-info"))
}

/// `d` after `⏎`: the path is at stage 0 and `check` has no `UnmergedHow` to
/// read at all, so the stages come back out of the chained backup and
/// `checkout --merge` regenerates the markers from them.
///
/// **The outcome here does not discriminate, and is not meant to.** Measured
/// by disabling the restore and re-running: the stages and the markers still
/// came back, because `git add` had recorded them in git's own resolve-undo
/// extension and bare `checkout --merge` restored from there (git 2.54.0).
/// Only the last assertion — `update-index --index-info` in the recorded
/// command lines — fails. `restart_survives_a_relaunch` is the test that
/// takes REUC away and makes the outcome itself the proof.
#[test]
fn restart_after_stage_brings_the_conflict_back() {
    let (repo, _entry) = a_stopped_graft_merge();
    let stages_before = unmerged_lines(&repo, "shared.txt");
    assert_eq!(
        stages_before.lines().count(),
        3,
        "the merge left three stages"
    );

    mark_resolved(&repo, "shared.txt", b"a hand-written merge\n", NOW + 1);
    assert_eq!(
        unmerged_lines(&repo, "shared.txt"),
        String::new(),
        "`⏎` moved it to stage 0"
    );

    let applied = step(&repo, &restart("shared.txt"), NOW + 2, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(
        unmerged_lines(&repo, "shared.txt"),
        stages_before,
        "the same three stage oids and modes, byte for byte"
    );
    let back = String::from_utf8_lossy(&repo.read("shared.txt")).into_owned();
    assert!(back.contains("<<<<<<<"), "the markers are back: {back}");
    assert!(back.contains("feature's line"), "theirs is back: {back}");
    assert!(
        put_stages_back(&applied),
        "the stages were put back from the recorded tree: {:?}",
        applied.entry.commands
    );
}

/// The same, with git's resolve-undo **cleared** — the only control that
/// tells Mergewright's stage tree apart from git's own bookkeeping.
///
/// `git add` on a conflicted path records its three stages in the index's
/// REUC extension, and bare `git checkout --merge` restores from there
/// (measured, git 2.54.0). So a green
/// `restart_after_stage_brings_the_conflict_back` proves nothing on its own.
/// Here the index is rebuilt from its own tree, which drops REUC: the same
/// bare command then exits 0 and does nothing — asserted below, so the
/// control is part of the test rather than a claim about it — and the only
/// thing left that can bring the conflict back is the chain commit. That
/// lives in the ref, not in memory, which is the relaunch: every `step` here
/// opens a fresh `Context` over a freshly read journal.
///
/// Checked by disabling the restore and re-running: `ls-files -u` comes back
/// **empty**, not wrong. This test is answered by the recorded stage tree and
/// by nothing else.
#[test]
fn restart_survives_a_relaunch() {
    let (repo, _entry) = a_stopped_graft_merge();
    let stages_before = unmerged_lines(&repo, "shared.txt");

    // Both paths resolved, so the index has a tree to be rebuilt from. The
    // `UD` one goes first, so the newest chain commit is the one taken before
    // `shared.txt` was staged — the one still holding its stages.
    step(
        &repo,
        &resolve("gone.txt", Side::Ours, UnmergedHow::DeletedByThem),
        NOW + 1,
        &Never,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    mark_resolved(&repo, "shared.txt", b"a hand-written merge\n", NOW + 2);
    assert!(
        !resolve_undo(&repo).is_empty(),
        "git recorded resolve-undo for the staged paths"
    );

    // The relaunch, and the control: an index rebuilt from its own tree is
    // the same index without REUC.
    let tree = repo.git(&["write-tree"]);
    std::fs::remove_file(repo.path().join(".git/index")).expect("the index is removed");
    repo.git(&["read-tree", &tree]);
    assert_eq!(
        resolve_undo(&repo),
        String::new(),
        "resolve-undo is gone, so git alone has nothing to restore from"
    );
    assert!(repo.exists(".git/MERGE_HEAD"), "the merge is still live");

    repo.git(&[
        "--literal-pathspecs",
        "checkout",
        "--merge",
        "--",
        "shared.txt",
    ]);
    assert_eq!(
        unmerged_lines(&repo, "shared.txt"),
        String::new(),
        "bare `checkout --merge` exits 0 and does nothing once REUC is gone"
    );

    let applied = step(&repo, &restart("shared.txt"), NOW + 3, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    // The outcome first, then the mechanism: with REUC gone, the outcome
    // alone already fails if the stage tree is not what answered.
    assert_eq!(
        unmerged_lines(&repo, "shared.txt"),
        stages_before,
        "the same three stage oids and modes, byte for byte"
    );
    let back = String::from_utf8_lossy(&repo.read("shared.txt")).into_owned();
    assert!(back.contains("<<<<<<<"), "the markers are back: {back}");
    assert!(
        put_stages_back(&applied),
        "the stages came from the recorded tree: {:?}",
        applied.entry.commands
    );
}

/// No chain commit holds the path, so `d` says so in words and writes
/// nothing — rather than running a `checkout --merge` that would exit 0 and
/// silently do nothing.
#[test]
fn restart_refuses_without_stages() {
    let (repo, entry) = a_stopped_graft_merge();
    let pre_op = backup_tip(&repo, &entry);

    mark_resolved(&repo, "shared.txt", b"a hand-written merge\n", NOW + 1);
    // The chain thrown away: the ref back at the pre-operation snapshot,
    // whose index was merged and holds no stages for anything.
    let name = entry.backup_ref.as_deref().expect("a backup ref");
    repo.git(&["update-ref", name, &pre_op.to_string()]);

    let index_before = repo.git(&["--literal-pathspecs", "ls-files", "-s", "--", "shared.txt"]);
    let bytes_before = repo.read("shared.txt");
    let lines_before = journal_lines(&repo);

    match step(&repo, &restart("shared.txt"), NOW + 2, &Never) {
        Err(ApplyError::Refused(Refusal::NoStagesKept { path })) => {
            assert_eq!(path, PathBuf::from("shared.txt"));
        }
        other => panic!("expected NoStagesKept, got {other:?}"),
    }

    assert_eq!(
        repo.git(&["--literal-pathspecs", "ls-files", "-s", "--", "shared.txt"]),
        index_before,
        "the index is untouched"
    );
    assert_eq!(repo.read("shared.txt"), bytes_before, "and the file");
    assert_eq!(journal_lines(&repo), lines_before, "and the journal");
}
