//! A merge over unsaved edits sets them aside and puts them back, in one
//! journal entry (m8-polish-and-ship/19d-a, ADR-0045 §4 extended to `Merge`).
//!
//! git refuses a true merge over **any** staged change, even one the merge
//! does not touch, and over an unstaged change to a path the merge changes.
//! The forecast reads only the two commits, so before 19d the preview said
//! `no conflicts expected` and git then refused at `y`. Every claim here is
//! read back from git, the files on disk or the journal file.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

use graft_core::journal::{EntryStatus, JournalEntry, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::state::InProgress;
use graft_ops::apply::{ApplyError, Resume};
use graft_ops::facts::read_facts;
use graft_ops::journal::journal_path;
use graft_ops::operation::{FastForward, MergeParams, Operation, SetAside, Source, Tip};
use graft_ops::preview::{self, ConflictForecast, PreviewPlan};

use crate::set_aside::{
    StopWhen, Tree, edit_is_somewhere, id, journal_entries, last_entry, loose_objects, porcelain,
    resume_with, set_aside_of, staging_lost, stash_commits, step_names, whole,
};
use crate::support::{
    NOW, Never, TempRepo, git_version, run_redo, run_undo, run_with_stop, try_run,
};

// ── repositories and operations ─────────────────────────────────────────

/// Ten lines, so one edit near the top and another near the bottom merge.
const TEN: &[u8] = b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// [`TEN`] with its first line changed: what `feature` brings.
const THEIRS: &[u8] = b"one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// [`TEN`] with its last line changed: his unsaved edit.
const MINE: &[u8] = b"1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";
/// Both: what a clean put-back leaves.
const BOTH: &[u8] = b"one\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";
/// [`TEN`] with its first line changed his own way: clashes with [`THEIRS`].
const MY_FIRST_LINE: &[u8] = b"my first line\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";

/// `main` and `feature` share `base.txt`, `a.txt`, `notes.txt` and
/// `shared.txt` ([`TEN`]); `feature` adds `feature.txt` and changes
/// `shared.txt`'s first line ([`THEIRS`]). With `main_moves`, `main` adds
/// `main.txt` first, so the merge is a true one; without, a fast-forward.
/// On `main`, clean. The round-trip table's files (`base.txt`, `a.txt`) are
/// untouched by both.
fn two_branches(main_moves: bool) -> TempRepo {
    let repo = TempRepo::init();
    repo.write("base.txt", b"base\n");
    repo.write("a.txt", b"a\n");
    repo.write("notes.txt", b"notes\n");
    repo.write("shared.txt", TEN);
    repo.commit_all("one");
    repo.git(&["branch", "feature"]);
    if main_moves {
        repo.write("main.txt", b"main\n");
        repo.commit_all("two on main");
    }
    repo.git(&["switch", "-q", "feature"]);
    repo.write("feature.txt", b"feature\n");
    repo.write("shared.txt", THEIRS);
    repo.commit_all("one on feature");
    repo.git(&["switch", "-q", "main"]);
    repo
}

fn shape(main_moves: bool) -> &'static str {
    if main_moves {
        "a true merge"
    } else {
        "a fast-forward"
    }
}

/// `main` rewrites `shared.txt`'s first line its own way, so merging
/// `feature` stops on it. His edits: `notes.txt` staged, `a.txt` not —
/// neither side touches them, and the split is part of what must come back.
fn stopping_merge() -> (TempRepo, Operation) {
    let repo = two_branches(true);
    repo.write("shared.txt", b"main's one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
    repo.commit_all("main's first line");
    repo.write("notes.txt", b"notes, staged\n");
    repo.git(&["add", "notes.txt"]);
    repo.write("a.txt", b"a, unstaged\n");
    let op = merge_feature(&repo);
    (repo, op)
}

fn head_tip(repo: &TempRepo) -> Tip {
    let name = repo.git(&["symbolic-ref", "HEAD"]);
    Tip {
        oid: id(repo, &name),
        name,
    }
}

/// `m` on `feature`, as the app builds it: the branch, its tip when the
/// preview was drawn, git's default fast-forward rule.
fn merge_feature(repo: &TempRepo) -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch("feature".to_owned()),
        into: head_tip(repo),
        source_tip: id(repo, "refs/heads/feature"),
        ff: FastForward::Default,
    })
}

/// `main`'s commit has two parents: a merge commit was made.
fn is_a_merge_commit(repo: &TempRepo) -> bool {
    repo.git(&["log", "-1", "--format=%P", "main"])
        .split_whitespace()
        .count()
        == 2
}

/// What he staged and what he did not, as diffs (blob ids and modes
/// included). Neither branch touches the round-trip table's files, so these
/// are the same before and after a put-back that keeps the index.
fn split(repo: &TempRepo) -> (String, String) {
    (
        repo.git(&["diff", "--cached", "--no-renames"]),
        repo.git(&["diff", "--no-renames"]),
    )
}

fn plan_of(repo: &TempRepo, op: &Operation) -> PreviewPlan {
    let handle = repo.open();
    let facts = read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("facts");
    preview::plan(&handle, &git_version(), op, &facts, &Never)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
}

// ── went through ────────────────────────────────────────────────────────

/// The tour's case (19c-c NOTES :224-228): a staged change to a file the
/// merge does not touch. git refuses a true merge over it (`Your local
/// changes … would be overwritten by merge`); Graft sets it aside, merges,
/// and puts it back staged. Red before 19d: `ApplyError::Git`.
#[test]
fn a_merge_over_a_staged_change_git_would_refuse_goes_through() {
    let repo = two_branches(true);
    repo.write("notes.txt", b"notes, staged\n");
    repo.git(&["add", "notes.txt"]);
    let stashes_before = stash_commits(&repo);
    let op = merge_feature(&repo);

    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    assert_eq!(applied.entry.status, EntryStatus::Committed);
    set_aside_of(&applied.entry);
    assert!(is_a_merge_commit(&repo), "the merge was committed");
    assert_eq!(porcelain(&repo), ["M  notes.txt"], "still staged");
    assert_eq!(repo.read("notes.txt"), b"notes, staged\n");
    assert_eq!(
        stash_commits(&repo),
        stashes_before,
        "the stash list as before"
    );
    assert_eq!(repo.read("feature.txt"), b"feature\n", "feature came in");
}

/// His unstaged edit to a path the merge also changes, at another line: set
/// aside, merged, put back over theirs, `S` dropped — a true merge and a
/// fast-forward alike, one entry, every step on its Command Log.
#[test]
fn a_merge_puts_unstaged_edits_back_after() {
    for main_moves in [true, false] {
        let what = shape(main_moves);
        let repo = two_branches(main_moves);
        repo.write("shared.txt", MINE);
        let op = merge_feature(&repo);

        let applied = try_run(&repo, &op, NOW)
            .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));

        let entry = &applied.entry;
        assert_eq!(entry.status, EntryStatus::Committed, "{what}");
        set_aside_of(entry);
        assert_eq!(is_a_merge_commit(&repo), main_moves, "{what}");
        if !main_moves {
            assert_eq!(
                id(&repo, "refs/heads/main"),
                id(&repo, "refs/heads/feature"),
                "fast-forwarded"
            );
        }
        assert_eq!(
            repo.read("shared.txt"),
            BOTH,
            "{what}: his edit over theirs"
        );
        assert_eq!(porcelain(&repo), [" M shared.txt"], "{what}: unstaged");
        assert!(stash_commits(&repo).is_empty(), "{what}: S dropped");
        assert_eq!(
            entry.touches.refs,
            ["refs/heads/main", "refs/stash"],
            "{what}"
        );
        assert_eq!(
            step_names(entry),
            [
                "stash create",
                "stash store",
                "reset --hard",
                "the operation",
                "stash apply",
                "stash drop",
            ],
            "{what}: {:?}",
            entry.commands
        );
        assert_eq!(journal_entries(&repo).len(), 1, "{what}: one entry");
    }
}

/// `repo` is ready to merge `feature` with nothing of his to set aside:
/// `sets_aside` is `None`, `touches` names no `refs/stash`, and the entry
/// is today's — no `stash` line, no `put_back`.
fn sets_nothing_aside(repo: &TempRepo, what: &str) {
    let op = merge_feature(repo);
    let handle = repo.open();
    let facts =
        read_facts(&handle, git_version(), &Never, &op.facts_scope()).expect("the facts read");
    assert_eq!(op.sets_aside(&facts), None, "{what}");
    assert_eq!(op.touches(&facts).refs, ["refs/heads/main"], "{what}");

    let applied = try_run(repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.status, EntryStatus::Committed, "{what}");
    assert_eq!(entry.put_back, None, "{what}");
    assert!(
        entry.commands.iter().all(|line| !line.contains("stash")),
        "{what}: {:?}",
        entry.commands
    );
    assert_eq!(entry.touches.refs, ["refs/heads/main"], "{what}");
    assert!(is_a_merge_commit(repo), "{what}: merged");
}

#[test]
fn a_merge_over_a_clean_tree_sets_nothing_aside() {
    let repo = two_branches(true);
    sets_nothing_aside(&repo, "a clean tree");
    assert!(porcelain(&repo).is_empty());
}

/// Untracked files are never set aside (ADR-0045 §4): they stay where they
/// are, and git merges around them.
#[test]
fn untracked_files_alone_set_nothing_aside() {
    let repo = two_branches(true);
    repo.write("junk.tmp", b"untracked\n");
    sets_nothing_aside(&repo, "an untracked file");
    assert_eq!(repo.read("junk.tmp"), b"untracked\n");
    assert_eq!(porcelain(&repo), ["?? junk.tmp"]);
}

/// `feature` is already in `main`, and he has edits (staged and not): git
/// would say "Already up to date." and do nothing, so there is nothing to
/// set aside or put back. The preview is the nothing-to-do box (no ref
/// move, no `HEAD`, no ghost, tree untouched — the app offers only Close)
/// with no set-aside and no put-back in it (19d-a2, NOTES 19d-a question).
#[test]
fn an_up_to_date_merge_over_edits_sets_nothing_aside() {
    let repo = two_branches(true);
    repo.git(&["merge", "-q", "--no-edit", "feature"]);
    repo.write("notes.txt", b"notes, staged\n");
    repo.git(&["add", "notes.txt"]);
    repo.write("a.txt", b"a, unstaged\n");
    let op = merge_feature(&repo);

    let plan = plan_of(&repo, &op);

    assert!(plan.ref_moves.is_empty(), "{:?}", plan.ref_moves);
    assert_eq!(plan.head, None);
    assert!(plan.ghosts.is_empty());
    assert_eq!(plan.worktree, preview::WorktreeEffect::Untouched);
    assert_eq!(plan.set_aside, None, "nothing to set aside for");
    assert_eq!(plan.put_back, None, "nothing to put back");
}

// ── a merge that stops ──────────────────────────────────────────────────

/// Run `op`, which must stop on `shared.txt`, and check the edits wait
/// aside: the entry `Pending` with `put_back` on its line, `S` listed, both
/// edited files as committed.
fn stopped_with_edits_aside(repo: &TempRepo, op: &Operation) -> JournalEntry {
    let entry = match try_run(repo, op, NOW) {
        Err(ApplyError::Conflicted { entry, kind, paths }) => {
            assert_eq!(kind, InProgress::Merge);
            assert_eq!(paths, [PathBuf::from("shared.txt")]);
            *entry
        }
        other => panic!("the merge should stop: {other:?}"),
    };
    assert_eq!(entry.status, EntryStatus::Pending);
    let stash = set_aside_of(&entry);
    assert_eq!(stash_commits(repo), vec![stash], "S waits in the list");
    assert_eq!(
        repo.read("notes.txt"),
        b"notes\n",
        "the staged edit is aside"
    );
    assert_eq!(repo.read("a.txt"), b"a\n", "the unstaged edit is aside");
    assert_eq!(
        set_aside_of(&last_entry(repo)),
        stash,
        "the Pending line has it"
    );
    entry
}

/// The route this ticket is the first to take (19d R1): a set-aside merge
/// that stops, then ends by `how`. Each resume reads a freshly opened
/// journal, so `put_back` comes from the Pending line on disk. After it:
/// one entry, the merge over, the edits back with the split kept, `S`
/// dropped.
fn a_stopped_merge_ends(how: &Resume) {
    let (repo, op) = stopping_merge();
    let before = whole(&repo);
    let status_before = porcelain(&repo);
    assert_eq!(status_before, [" M a.txt", "M  notes.txt"]);
    let original = id(&repo, "refs/heads/main");
    let entry = stopped_with_edits_aside(&repo, &op);
    let aborting = *how == Resume::Abort;
    if !aborting {
        repo.write("shared.txt", RESOLVED);
        repo.git(&["add", "shared.txt"]);
    }

    let applied = resume_with(&repo, how.clone(), NOW + 1)
        .unwrap_or_else(|why| panic!("{how:?}: {}", why.explain(&crate::support::Letters)));

    assert!(!repo.exists(".git/MERGE_HEAD"), "{how:?}: the merge ended");
    let closed = last_entry(&repo);
    assert_eq!(closed.id, entry.id, "{how:?}: one entry");
    assert_eq!(closed, applied.entry, "{how:?}");
    assert_eq!(
        porcelain(&repo),
        status_before,
        "{how:?}: the edits back, the split kept"
    );
    assert_eq!(repo.read("notes.txt"), b"notes, staged\n", "{how:?}");
    assert_eq!(repo.read("a.txt"), b"a, unstaged\n", "{how:?}");
    assert!(stash_commits(&repo).is_empty(), "{how:?}: S dropped");
    assert!(
        closed
            .commands
            .iter()
            .any(|line| line.contains("stash apply")),
        "{how:?}: {:?}",
        closed.commands
    );
    if aborting {
        assert_eq!(
            closed.status,
            EntryStatus::Aborted {
                reason: "aborted".to_owned()
            }
        );
        assert_eq!(id(&repo, "refs/heads/main"), original, "the old tip");
        assert_eq!(whole(&repo), before, "exactly as before the merge");
    } else {
        merged_as_resolved(&repo, &closed, how);
    }
}

/// `shared.txt` as he resolved it.
const RESOLVED: &[u8] = b"resolved\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";

/// A stopped merge `how` finished: committed, a merge commit, his
/// resolution in it.
fn merged_as_resolved(repo: &TempRepo, closed: &JournalEntry, how: &Resume) {
    assert_eq!(closed.status, EntryStatus::Committed, "{how:?}");
    assert!(is_a_merge_commit(repo), "{how:?}: merged");
    assert_eq!(repo.read("shared.txt"), RESOLVED, "{how:?}");
}

/// The finish screen's commit (`ContinueWith`): the message as he left it.
#[test]
fn a_stopped_merge_finished_puts_the_edits_back() {
    a_stopped_merge_ends(&Resume::ContinueWith {
        message: b"Merge feature, by hand\n".to_vec(),
    });
    // The message is the one sent, not git's.
    let (repo, op) = stopping_merge();
    stopped_with_edits_aside(&repo, &op);
    repo.write("shared.txt", THEIRS);
    repo.git(&["add", "shared.txt"]);
    resume_with(
        &repo,
        Resume::ContinueWith {
            message: b"Merge feature, by hand\n".to_vec(),
        },
        NOW + 1,
    )
    .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(
        repo.git(&["log", "-1", "--format=%s"]),
        "Merge feature, by hand"
    );
}

/// `c` on the banner: `git merge --continue`.
#[test]
fn a_stopped_merge_continued_puts_the_edits_back() {
    a_stopped_merge_ends(&Resume::Continue);
}

/// `a`: `git merge --abort`, then the edits back onto the old tip
/// (ADR-0045 §4), the repository exactly as before the merge.
#[test]
fn a_stopped_merge_aborted_puts_the_edits_back() {
    a_stopped_merge_ends(&Resume::Abort);
}

// ── a put-back that clashes ─────────────────────────────────────────────

/// The merge goes through clean; his edit and `feature`'s change the same
/// line, so the put-back clashes: the entry `Committed` with an unmerged
/// index after, `Clashed { SetAside }` on `shared.txt`, `S` kept — the
/// resolver takes it from there, as for a rebase. `u` puts it all back.
#[test]
fn a_merge_put_back_that_clashes_stops_as_a_clash() {
    let repo = two_branches(true);
    repo.write("shared.txt", MY_FIRST_LINE);
    let op = merge_feature(&repo);
    let before = whole(&repo);

    let (entry, clash, stopped_at) = match try_run(&repo, &op, NOW) {
        Err(ApplyError::Clashed {
            entry,
            clash,
            stopped_at,
        }) => (*entry, clash, stopped_at),
        other => panic!("the put-back should clash: {other:?}"),
    };

    // No stop here: only a stop after the commit marks a clash (19d-a3).
    assert_eq!(stopped_at, None, "no stop, no mark");
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("shared.txt")]);
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.index_after.is_some_and(|index| index.is_unmerged()));
    assert!(is_a_merge_commit(&repo), "the merge is done");
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    assert!(
        !repo.git(&["ls-files", "-u"]).is_empty(),
        "git says unmerged"
    );

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "u put it back exactly, S dropped");
}

// ── ADR-0045 §7: undo and redo on the table of trees ────────────────────

/// One tree shape through a true merge: after it, every edited file back,
/// the status, the split and the stash list as before; `u` exactly as
/// before (refs, `HEAD`, index, files, stash list); with `and_redo`, `U`
/// exactly as after.
fn merge_round_trip(tree: Tree, and_redo: bool) {
    let repo = two_branches(true);
    tree.make(&repo);
    let what = format!("merge over {tree:?}");
    let op = merge_feature(&repo);
    let before = whole(&repo);
    let status_before = porcelain(&repo);
    let split_before = split(&repo);

    let applied = try_run(&repo, &op, NOW)
        .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.entry.status, EntryStatus::Committed, "{what}");
    set_aside_of(&applied.entry);
    assert!(is_a_merge_commit(&repo), "{what}: merged");
    let after = whole(&repo);
    assert_eq!(after.1, before.1, "{what}: the stash list is as before");
    for (path, bytes) in tree.edits() {
        match bytes {
            Some(bytes) => assert_eq!(repo.read(path), *bytes, "{what}: {path} is back"),
            None => assert!(!repo.exists(path), "{what}: {path} is gone again"),
        }
    }
    let status_after = porcelain(&repo);
    assert_eq!(status_after, status_before, "{what}: the status as before");
    assert_eq!(split(&repo), split_before, "{what}: the staged split");
    assert_eq!(staging_lost(&status_before, &status_after), 0, "{what}");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{what}: undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "{what}: undo put it back exactly");
    if and_redo {
        run_redo(&repo, NOW + 2, false).unwrap_or_else(|why| {
            panic!("{what}: redo: {}", why.explain(&crate::support::Letters))
        });
        assert_eq!(whole(&repo), after, "{what}: redo put it forward exactly");
    }
}

#[test]
fn undo_of_a_set_aside_merge_round_trips() {
    for tree in Tree::ALL {
        merge_round_trip(tree, false);
    }
}

#[test]
fn redo_of_a_set_aside_merge_round_trips() {
    for tree in Tree::ALL {
        merge_round_trip(tree, true);
    }
}

// ── a stop before each step ─────────────────────────────────────────────

/// Hold `git merge` in its `post-merge` hook — after it has moved `HEAD`,
/// a true merge and a fast-forward alike — until the runner's stop kills
/// it. Bounded at ten seconds, so a stop that never fires fails the test.
fn hold_merge_until_killed(repo: &TempRepo) {
    let hooks = repo.path().join(".git/test-hooks");
    std::fs::create_dir_all(&hooks).expect("the hooks dir is made");
    let hook = hooks.join("post-merge");
    std::fs::write(
        &hook,
        "#!/bin/sh\nrm -f \"$0\"\np=$PPID; n=0\n\
         while kill -0 \"$p\" 2>/dev/null && [ \"$n\" -lt 1000 ]; do sleep 0.01; n=$((n + 1)); done\n",
    )
    .expect("the hook is written");
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))
        .expect("the hook is executable");
    repo.git(&["config", "core.hooksPath", hooks.to_str().expect("utf-8")]);
}

/// A stop just before set-aside step `step` (1–6), keyed to what the step
/// before it leaves on disk (as `set_aside::stop_before`): 2 after `stash
/// create`'s objects, 3 after the Pending line, 4 after `stash store`, 5
/// after `reset --hard` puts `base.txt` back, 6 once the merge moved `HEAD`
/// (git held there until killed).
fn stop_before(repo: &TempRepo, step: u8) -> StopWhen {
    if step >= 6 {
        hold_merge_until_killed(repo);
    }
    let root = repo.path().to_path_buf();
    let git_dir = root.join(".git");
    let journal = journal_path(&repo.open()).expect("the journal path");
    let objects = loose_objects(&git_dir);
    let start_tip = id(repo, "HEAD");
    match step {
        1 => StopWhen::new(|| true),
        2 => StopWhen::new(move || loose_objects(&git_dir) > objects),
        3 => StopWhen::new(move || std::fs::read(&journal).is_ok_and(|bytes| !bytes.is_empty())),
        4 => StopWhen::new(move || git_dir.join("refs/stash").exists()),
        5 => StopWhen::new(move || {
            std::fs::read(root.join("base.txt")).is_ok_and(|bytes| bytes == b"base\n")
        }),
        _ => StopWhen::new(move || {
            std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["rev-parse", "HEAD"])
                .output()
                .is_ok_and(|out| {
                    String::from_utf8_lossy(&out.stdout).trim() != start_tip.to_string()
                })
        }),
    }
}

/// ADR-0045 §5, §7 for a merge, true and fast-forward: a stop before each
/// of steps 1–6 leaves every edit in the tree or in the stash list — read
/// from both — and no entry left open unless git's merge is still live.
#[test]
fn a_stop_anywhere_in_a_set_aside_merge_keeps_the_edits() {
    for main_moves in [true, false] {
        for step in 1..=6 {
            let repo = two_branches(main_moves);
            repo.write("base.txt", b"edited\n");
            repo.write("n.txt", b"new\n");
            repo.git(&["add", "n.txt"]);
            let op = merge_feature(&repo);
            let stop = stop_before(&repo, step);

            let result = run_with_stop(&repo, &op, NOW, &stop);

            let what = format!(
                "{}, stop before step {step}: {:?}",
                shape(main_moves),
                result.as_ref().err()
            );
            assert!(stop.latched.load(Ordering::Relaxed), "{what}: never fired");
            assert!(
                edit_is_somewhere(&repo, "base.txt", b"edited\n"),
                "{what}: base.txt lost"
            );
            assert!(
                edit_is_somewhere(&repo, "n.txt", b"new\n"),
                "{what}: n.txt lost"
            );
            if !repo.exists(".git/MERGE_HEAD") {
                assert!(
                    journal_entries(&repo)
                        .iter()
                        .all(|entry| entry.status != EntryStatus::Pending),
                    "{what}: an entry was left open"
                );
            }
        }
    }
}

// ── the preview ─────────────────────────────────────────────────────────

/// ADR-0045 §8 for a merge: with edits aside the preview forecasts their
/// put-back — `Clean` onto a true merge or a fast-forward the edits don't
/// meet, `Likely` where his edit and the merge change the same line, and
/// `NotChecked` when the merge itself may stop. A clean tree draws neither,
/// and spawns exactly what it spawned before 19d.
#[test]
fn the_merge_preview_forecasts_the_put_back() {
    for main_moves in [true, false] {
        let what = shape(main_moves);
        let repo = two_branches(main_moves);
        repo.write("notes.txt", b"notes, staged\n");
        repo.git(&["add", "notes.txt"]);
        let plan = plan_of(&repo, &merge_feature(&repo));
        assert_eq!(plan.set_aside, Some(SetAside { files: 1 }), "{what}");
        assert_eq!(plan.put_back, Some(ConflictForecast::Clean), "{what}");

        let repo = two_branches(main_moves);
        repo.write("shared.txt", MY_FIRST_LINE);
        let plan = plan_of(&repo, &merge_feature(&repo));
        assert_eq!(plan.conflicts, ConflictForecast::Clean, "{what}");
        assert_eq!(
            plan.put_back,
            Some(ConflictForecast::Likely {
                paths: vec![PathBuf::from("shared.txt")]
            }),
            "{what}: the overlap"
        );
    }

    let (repo, op) = stopping_merge();
    let plan = plan_of(&repo, &op);
    assert_eq!(
        plan.conflicts,
        ConflictForecast::Likely {
            paths: vec![PathBuf::from("shared.txt")]
        }
    );
    assert_eq!(plan.set_aside, Some(SetAside { files: 2 }));
    assert_eq!(
        plan.put_back,
        Some(ConflictForecast::NotChecked(
            "the merge may stop first".to_owned()
        ))
    );

    let repo = two_branches(true);
    let plan = plan_of(&repo, &merge_feature(&repo));
    assert_eq!(plan.set_aside, None, "a clean tree");
    assert_eq!(plan.put_back, None, "a clean tree");
    the_put_back_forecast_costs(&repo);
}

/// What the put-back forecast costs, measured: ADR-0045 §8's `stash create`
/// and `merge-tree`, and between them the landed commit — the merge base,
/// the merged tree, its scratch commit. Five for a true merge, three for a
/// fast-forward; none over a clean tree. `repo` is a clean true merge.
fn the_put_back_forecast_costs(repo: &TempRepo) {
    assert_eq!(
        spawned_by_the_preview(repo),
        CLEAN_TREE_SPAWNS,
        "a clean tree spawns what it did before 19d"
    );
    repo.write("notes.txt", b"notes, staged\n");
    repo.git(&["add", "notes.txt"]);
    let mut dirty = CLEAN_TREE_SPAWNS.to_vec();
    dirty.extend(PUT_BACK_SPAWNS);
    assert_eq!(spawned_by_the_preview(repo), dirty, "with edits aside");

    let repo = two_branches(false);
    assert_eq!(
        spawned_by_the_preview(&repo),
        FF_CLEAN_TREE_SPAWNS,
        "a clean fast-forward spawns what it did before 19d"
    );
    repo.write("notes.txt", b"notes, staged\n");
    repo.git(&["add", "notes.txt"]);
    let mut dirty = FF_CLEAN_TREE_SPAWNS.to_vec();
    dirty.extend(FF_PUT_BACK_SPAWNS);
    assert_eq!(
        spawned_by_the_preview(&repo),
        dirty,
        "a fast-forward with edits aside"
    );
}

// ── counting the preview's spawns ───────────────────────────────────────

/// A true merge's preview over a clean tree, one `git` per entry, measured
/// on this ticket's code before the put-back forecast was added (the merge
/// arm of `sets_aside` spawns nothing; it reads the facts).
const CLEAN_TREE_SPAWNS: &[&str] = &[
    "diff-tree",
    "ls-files",
    "merge-base",
    "var",
    "merge-base",
    "diff-tree",
    "merge-tree",
];

/// The put-back forecast's own, after the merge's (ADR-0045 §8).
const PUT_BACK_SPAWNS: &[&str] = &[
    "stash create",
    "merge-base",
    "merge-tree",
    "commit-tree",
    "merge-tree",
];

/// A fast-forward's preview over a clean tree, measured as
/// [`CLEAN_TREE_SPAWNS`] was.
const FF_CLEAN_TREE_SPAWNS: &[&str] = &["diff-tree", "ls-files", "merge-base", "diff-tree"];

/// A fast-forward lands on the source itself: no merged tree to make.
const FF_PUT_BACK_SPAWNS: &[&str] = &["stash create", "merge-base", "merge-tree"];

/// A `git` first on the child's `PATH`: logs its argv, runs the real one.
const SPAWN_LOG_SHIM: &str = "#!/bin/sh\n\
    printf '%s\\n' \"$*\" >> \"$GRAFT_SPAWN_LOG\"\n\
    exec \"$GRAFT_SPAWN_REAL_GIT\" \"$@\"\n";
const CHILD_REPO: &str = "GRAFT_SPAWN_CHILD_REPO";
const SPAWN_LOG: &str = "GRAFT_SPAWN_LOG";
/// Written by the child just before it asks for the preview.
const MARK: &str = "-- the preview --";

/// The real git, found on this process's `PATH` as `Command` would.
fn real_git() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git is on PATH")
}

/// The subcommand of one logged argv (`-C <dir> --no-pager <subcommand> …`),
/// with `stash`'s own verb.
fn subcommand(argv: &str) -> String {
    let mut words = argv.split_whitespace();
    while let Some(word) = words.next() {
        match word {
            "-C" | "-c" => {
                words.next();
            }
            option if option.starts_with('-') => {}
            "stash" => return format!("stash {}", words.next().unwrap_or("")),
            found => return found.to_owned(),
        }
    }
    String::new()
}

/// `m` on `feature`'s preview, run in a child of this test binary with the
/// logging `git` first on its `PATH`: the subcommands it spawned, in order.
/// The parent's own environment is never changed.
fn spawned_by_the_preview(repo: &TempRepo) -> Vec<String> {
    let bin = repo.path().join(".git/spawn-bin");
    std::fs::create_dir_all(&bin).expect("the shim dir is made");
    let script = bin.join("git");
    std::fs::write(&script, SPAWN_LOG_SHIM).expect("the shim writes");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the shim is executable");
    let log = repo.path().join(".git/spawn-log");
    if log.exists() {
        std::fs::remove_file(&log).expect("the old log goes");
    }
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH is set"),
    )))
    .expect("PATH joins");
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "merge_set_aside::child_previews_the_merge",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PATH", path)
        .env("GRAFT_SPAWN_REAL_GIT", real_git())
        .env(SPAWN_LOG, &log)
        .env(CHILD_REPO, repo.path())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the child runs");
    assert!(status.success(), "the child failed: {status:?}");
    let logged = std::fs::read_to_string(&log).expect("the log reads");
    let (_, previewed) = logged
        .split_once(MARK)
        .expect("the child marked the preview");
    previewed
        .lines()
        .filter(|line| !line.is_empty())
        .map(subcommand)
        .collect()
}

/// `git rev-parse <rev>` in `repo`, through the child's `PATH`.
fn rev_in(repo: &Path, rev: &str) -> Oid {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", rev])
        .output()
        .expect("git runs");
    Oid::from_hex(String::from_utf8_lossy(&out.stdout).trim()).expect("a full id")
}

/// The child's half: the merge of `feature` into `main` previewed in
/// `$GRAFT_SPAWN_CHILD_REPO`, the mark written to `$GRAFT_SPAWN_LOG` just
/// before. Does nothing when run on its own.
#[test]
#[ignore = "the child of the spawn count; runs only under spawned_by_the_preview"]
fn child_previews_the_merge() {
    let (Some(repo), Some(log)) = (std::env::var_os(CHILD_REPO), std::env::var_os(SPAWN_LOG))
    else {
        return;
    };
    let repo = Path::new(&repo);
    let op = Operation::Merge(MergeParams {
        source: Source::Branch("feature".to_owned()),
        into: Tip {
            name: "refs/heads/main".to_owned(),
            oid: rev_in(repo, "refs/heads/main"),
        },
        source_tip: rev_in(repo, "refs/heads/feature"),
        ff: FastForward::Default,
    });
    let handle = RepoHandle::open(repo).expect("the repository opens");
    let git = git_version();
    let facts = read_facts(&handle, git, &Never, &op.facts_scope()).expect("facts");
    let mut marked = std::fs::OpenOptions::new()
        .append(true)
        .open(&log)
        .expect("the log opens");
    std::io::Write::write_all(&mut marked, format!("{MARK}\n").as_bytes()).expect("marked");
    drop(marked);
    preview::plan(&handle, &git, &op, &facts, &Never).expect("the preview");
}
