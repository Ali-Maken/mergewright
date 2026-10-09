//! A fast-forward pull over edits the pull would overwrite sets them aside
//! and puts them back, in one entry (m7-power-tools/03b, ADR-0045 §4
//! extended to Pull). The fetch runs first; what it brought decides.
//!
//! Every claim is read back from git, the files or the journal file. The
//! remote is a plain repository beside the work one (`origin`), so the
//! teammate's commit is made there and only the pull's own fetch brings it
//! — unless a test fetches first on purpose, and says why.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind, PutBackWhy, Touches};
use graft_git::remote::{RemoteFailure, upstream_of};
use graft_ops::apply::ApplyError;
use graft_ops::journal::journal_path;
use graft_ops::operation::{Operation, PullParams};

use crate::set_aside::{
    StopWhen, Tree, edit_is_somewhere, id, journal_entries, loose_objects, porcelain, set_aside_of,
    staging_lost, stash_commits, step_names, whole,
};
use crate::support::{NOW, TempRepo, run_redo, run_undo, run_with_stop, try_run};

// ── repositories ────────────────────────────────────────────────────────

/// Ten lines, so one edit near the top and another near the bottom merge.
const TEN: &[u8] = b"1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// [`TEN`] with its first line changed: what the teammate pushes.
const THEIRS: &[u8] = b"one\n2\n3\n4\n5\n6\n7\n8\n9\n10\n";
/// [`TEN`] with its last line changed: his unsaved edit.
const MINE: &[u8] = b"1\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";
/// Both: what a clean put-back leaves.
const BOTH: &[u8] = b"one\n2\n3\n4\n5\n6\n7\n8\n9\nten\n";

/// His clone and the remote it tracks. `origin` is dropped with it.
pub(crate) struct Behind {
    pub(crate) work: TempRepo,
    pub(crate) origin: TempRepo,
}

/// `origin` holds `shared.txt` ([`TEN`]), `base.txt` and `a.txt` (the
/// round-trip table's files); `work` is on `main` at the same commit,
/// tracking `origin/main`, clean.
fn clone_of_origin() -> Behind {
    let origin = TempRepo::init();
    origin.write("shared.txt", TEN);
    origin.write("base.txt", b"base\n");
    origin.write("a.txt", b"a\n");
    origin.commit_all("one");
    let work = TempRepo::init();
    track(&work, &origin);
    work.git(&["reset", "-q", "--hard", "origin/main"]);
    Behind { work, origin }
}

/// `origin` as `work`'s remote, fetched, and `main`'s upstream.
pub(crate) fn track(work: &TempRepo, origin: &TempRepo) {
    let url = origin.path().to_str().expect("a UTF-8 path").to_owned();
    work.git(&["remote", "add", "origin", &url]);
    work.git(&["fetch", "-q", "origin"]);
    work.git(&["config", "branch.main.remote", "origin"]);
    work.git(&["config", "branch.main.merge", "refs/heads/main"]);
}

/// The teammate's commit on `origin`: `path` becomes `bytes`. Returns it.
fn theirs(behind: &Behind, path: &str, bytes: &[u8]) -> String {
    behind.origin.write(path, bytes);
    behind.origin.commit_all("their change");
    behind.origin.git(&["rev-parse", "HEAD"])
}

/// `p`: a pull of `main` against its upstream as configured now.
pub(crate) fn pull(work: &TempRepo) -> Operation {
    let handle = work.open();
    let upstream = upstream_of(&handle, "refs/heads/main")
        .expect("the upstream reads")
        .expect("main tracks origin/main");
    Operation::Pull(PullParams {
        branch: "refs/heads/main".to_owned(),
        upstream,
    })
}

fn fetch_line() -> String {
    "git fetch --progress origin".to_owned()
}

fn merge_line() -> String {
    "git merge --ff-only --no-overwrite-ignore --no-edit refs/remotes/origin/main".to_owned()
}

fn aborted_reason(entry: &JournalEntry) -> &str {
    match &entry.status {
        EntryStatus::Aborted { reason } => reason,
        other => panic!("expected Aborted, got {other:?}"),
    }
}

// ── the record, as today ────────────────────────────────────────────────

/// Written first, green against the code before 03b and after it: a clean
/// fast-forward pull's entry — its kind, words, scope, the refs before and
/// after, and the Command Log (`fetch`, then `merge`).
#[test]
fn a_pull_journals_as_today() {
    let behind = clone_of_origin();
    let work = &behind.work;
    let before = work.git(&["rev-parse", "HEAD"]);
    let tip = theirs(&behind, "shared.txt", THEIRS);

    let applied = try_run(work, &pull(work), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.kind, OperationKind::Pull);
    assert_eq!(entry.description, "pull origin/main");
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(
        entry.touches,
        Touches {
            refs: vec!["refs/heads/main".to_owned()],
            index: true,
            worktree: true,
        }
    );
    assert_eq!(entry.commands, vec![fetch_line(), merge_line()]);
    assert_eq!(entry.put_back, None);
    assert_eq!(entry.pins, Vec::new());
    assert_eq!(
        entry.refs_before.get("refs/heads/main"),
        Some(&Some(id(work, &before)))
    );
    assert_eq!(
        entry.refs_after.get("refs/heads/main"),
        Some(&Some(id(work, &tip)))
    );
    assert!(entry.backup_ref.is_some());
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip);
    assert_eq!(work.read("shared.txt"), THEIRS);
}

/// Written first, green before and after: a pull whose fetch fails opens
/// the entry and closes it `Aborted` with git's reason, the Command Log
/// still both lines, as before 03b — and over an edit the pull would have
/// overwritten, nothing is set aside: no stash, the tree untouched.
#[test]
fn a_pull_whose_fetch_fails_journals_as_today() {
    let behind = clone_of_origin();
    let work = &behind.work;
    let before = work.git(&["rev-parse", "HEAD"]);
    work.write("shared.txt", MINE);
    let op = pull(work);
    let gone = behind.origin.path().join("gone");
    work.git(&["remote", "set-url", "origin", gone.to_str().unwrap()]);

    let Err(ApplyError::Remote { entry, .. }) = try_run(work, &op, NOW) else {
        panic!("a fetch from a missing remote fails as a remote failure");
    };

    assert_eq!(entry.kind, OperationKind::Pull);
    assert_eq!(entry.description, "pull origin/main");
    assert!(
        aborted_reason(&entry).starts_with("no repository at '")
            && aborted_reason(&entry).ends_with("', or no access to it"),
        "{:?}",
        entry.status
    );
    assert_eq!(
        entry.touches,
        Touches {
            refs: vec!["refs/heads/main".to_owned()],
            index: true,
            worktree: true,
        }
    );
    assert_eq!(entry.commands, vec![fetch_line(), merge_line()]);
    assert_eq!(entry.put_back, None);
    assert_eq!(entry.refs_before, entry.refs_after);
    assert_eq!(journal_entries(work).len(), 1, "one entry");
    assert_eq!(work.git(&["rev-parse", "HEAD"]), before);
    assert_eq!(work.read("shared.txt"), MINE, "the edit untouched");
    assert!(stash_commits(work).is_empty(), "nothing set aside");
}

// ── the set-aside ───────────────────────────────────────────────────────

/// :805. His unsaved edit to a file the pull changes, at another line: the
/// fetch brings the change (not fetched before — the order is the point),
/// the edit is set aside, the pull fast-forwards, the edit comes back
/// merged over theirs, `S` dropped. Red before 03b: git's dirty-tree
/// refusal.
#[test]
fn a_ff_pull_over_overlapping_edits_sets_them_aside() {
    let behind = clone_of_origin();
    let work = &behind.work;
    work.write("shared.txt", MINE);
    let tip = theirs(&behind, "shared.txt", THEIRS);

    let applied = try_run(work, &pull(work), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.status, EntryStatus::Committed);
    set_aside_of(entry);
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip, "fast-forwarded");
    assert_eq!(work.read("shared.txt"), BOTH, "his edit back over theirs");
    assert_eq!(
        work.git(&["status", "--porcelain"]),
        " M shared.txt",
        "unstaged, as it was"
    );
    assert!(stash_commits(work).is_empty(), "S dropped");
    assert_eq!(
        entry.touches.refs,
        vec!["refs/heads/main".to_owned(), "refs/stash".to_owned()]
    );
    assert_eq!(
        step_names(entry),
        vec![
            "the operation",
            "stash create",
            "stash store",
            "reset --hard",
            "the operation",
            "stash apply",
            "stash drop",
        ],
        "{:?}",
        entry.commands
    );
    assert_eq!(entry.commands[0], fetch_line(), "the fetch ran first");
    assert_eq!(entry.commands[4], merge_line());
}

/// The put-back clashes (his edit and theirs on the same line): the pull
/// is done, the entry `Committed` with an unmerged index after, `S` kept —
/// a clash, as a set-aside checkout's is.
#[test]
fn a_ff_pull_whose_put_back_clashes_stops_as_a_clash() {
    let behind = clone_of_origin();
    let work = &behind.work;
    work.write("shared.txt", b"my first line\n2\n3\n4\n5\n6\n7\n8\n9\n10\n");
    let tip = theirs(&behind, "shared.txt", THEIRS);

    let (entry, clash) = match try_run(work, &pull(work), NOW) {
        Err(ApplyError::Clashed { entry, clash, .. }) => (*entry, clash),
        other => panic!("the put-back should clash: {other:?}"),
    };

    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("shared.txt")]);
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert!(entry.index_after.is_some_and(|index| index.is_unmerged()));
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip, "the pull is done");
    assert_eq!(stash_commits(work), vec![stash], "S kept");
}

/// Edits the pull does not touch: nothing set aside — no stash made, the
/// Command Log as today — and his staging kept byte for byte.
#[test]
fn a_ff_pull_over_unrelated_edits_is_unchanged() {
    let behind = clone_of_origin();
    let work = &behind.work;
    work.write("base.txt", b"staged\n");
    work.git(&["add", "base.txt"]);
    work.write("base.txt", b"staged\nand unstaged\n");
    let staged = work.git(&["ls-files", "-s", "--", "base.txt"]);
    let tip = theirs(&behind, "shared.txt", THEIRS);

    let applied = try_run(work, &pull(work), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(entry.put_back, None, "nothing set aside");
    assert_eq!(entry.commands, vec![fetch_line(), merge_line()]);
    assert_eq!(entry.touches.refs, vec!["refs/heads/main".to_owned()]);
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip);
    assert_eq!(
        work.git(&["ls-files", "-s", "--", "base.txt"]),
        staged,
        "the index entry byte-equal"
    );
    assert_eq!(work.read("base.txt"), b"staged\nand unstaged\n");
    assert!(stash_commits(work).is_empty(), "no stash");
}

/// The probe 19d asked for (m8-polish-and-ship/19d R1, "Pull checked"):
/// a staged change alone, to a file the fast-forward does not touch. git
/// refuses a *true* merge over any staged change; a fast-forward pull goes
/// through it, so the pull keeps its own rule (03b) and needs no other.
#[test]
fn an_ff_pull_over_an_untouched_staged_change_goes_through() {
    let behind = clone_of_origin();
    let work = &behind.work;
    work.write("base.txt", b"staged\n");
    work.git(&["add", "base.txt"]);
    let staged = work.git(&["ls-files", "-s", "--", "base.txt"]);
    let tip = theirs(&behind, "shared.txt", THEIRS);

    let applied = try_run(work, &pull(work), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));

    let entry = &applied.entry;
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(entry.put_back, None, "nothing set aside");
    assert_eq!(entry.commands, vec![fetch_line(), merge_line()]);
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip, "fast-forwarded");
    assert_eq!(work.read("shared.txt"), THEIRS, "theirs came in");
    assert_eq!(
        work.git(&["ls-files", "-s", "--", "base.txt"]),
        staged,
        "still staged, byte-equal"
    );
    assert_eq!(porcelain(work), ["M  base.txt"]);
    assert!(stash_commits(work).is_empty(), "no stash");
}

/// Diverged: not a fast-forward, so nothing is set aside — git's refusal
/// and today's words, the edit untouched, no stash.
#[test]
fn a_diverged_pull_over_edits_sets_nothing_aside() {
    let behind = clone_of_origin();
    let work = &behind.work;
    work.write("a.txt", b"mine, committed\n");
    work.commit_all("ours");
    let ours = work.git(&["rev-parse", "HEAD"]);
    work.write("shared.txt", MINE);
    theirs(&behind, "shared.txt", THEIRS);

    let Err(ApplyError::Remote { entry, failure, .. }) = try_run(work, &pull(work), NOW) else {
        panic!("a diverged pull fails as a remote failure");
    };

    assert_eq!(failure, RemoteFailure::NotFastForward);
    assert_eq!(
        aborted_reason(&entry),
        "diverged: not possible to fast-forward"
    );
    assert_eq!(entry.put_back, None);
    assert_eq!(work.git(&["rev-parse", "HEAD"]), ours);
    assert_eq!(work.read("shared.txt"), MINE);
    assert!(stash_commits(work).is_empty(), "no stash");
}

/// One `p`, one entry; `u` puts back HEAD, the staged split (from the
/// backup) and every file, and drops nothing of his. The tracking ref is
/// fetched before, on purpose: a pull's undo never moves it back (the
/// remote still has those commits), so "exactly as before" holds only with
/// it already there.
#[test]
fn the_ff_pull_entry_is_one_entry_and_u_restores_edits() {
    let behind = clone_of_origin();
    let work = &behind.work;
    theirs(&behind, "shared.txt", THEIRS);
    work.git(&["fetch", "-q", "origin"]);
    work.write("shared.txt", b"1\n2\n3\n4\n5\n6\n7\n8\n9\nstaged ten\n");
    work.git(&["add", "shared.txt"]);
    work.write("shared.txt", MINE);
    let before = whole(work);

    let applied = try_run(work, &pull(work), NOW)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    set_aside_of(&applied.entry);
    assert_eq!(journal_entries(work).len(), 1, "one entry for one p");
    assert_eq!(
        work.git(&["show", ":shared.txt"]),
        "one\n2\n3\n4\n5\n6\n7\n8\n9\nstaged ten"
    );
    assert_eq!(work.read("shared.txt"), BOTH);
    let after = whole(work);

    run_undo(work, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(work), before, "u: everything as before");
    assert_eq!(
        work.git(&["show", ":shared.txt"]),
        "1\n2\n3\n4\n5\n6\n7\n8\n9\nstaged ten",
        "the staged half back"
    );
    run_redo(work, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(work), after, "redo: as after");
}

// ── ADR-0045 §7's table ─────────────────────────────────────────────────

/// One tree shape, plus the unsaved edit to `shared.txt` that makes the
/// pull set aside. The tracking ref is fetched before, as in the undo test.
fn round_trip(tree: Tree) {
    let behind = clone_of_origin();
    let work = &behind.work;
    let tip = theirs(&behind, "shared.txt", THEIRS);
    work.git(&["fetch", "-q", "origin"]);
    tree.make(work);
    work.write("shared.txt", MINE);
    let what = format!("pull over {tree:?}");
    let op = pull(work);
    let before = whole(work);
    let status_before = porcelain(work);
    let split_before = split_but_shared(work);

    let applied = try_run(work, &op, NOW)
        .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));
    assert_eq!(applied.entry.status, EntryStatus::Committed, "{what}");
    set_aside_of(&applied.entry);
    assert_eq!(work.git(&["rev-parse", "HEAD"]), tip, "{what}");
    let after = whole(work);
    assert_eq!(after.1, before.1, "{what}: the stash list is as before");
    for (path, bytes) in tree.edits() {
        match bytes {
            Some(bytes) => assert_eq!(work.read(path), *bytes, "{what}: {path} is back"),
            None => assert!(!work.exists(path), "{what}: {path} is gone again"),
        }
    }
    assert_eq!(work.read("shared.txt"), BOTH, "{what}");
    let status_after = porcelain(work);
    assert_eq!(status_after, status_before, "{what}: the status as before");
    assert_eq!(split_but_shared(work), split_before, "{what}: the split");
    assert_eq!(staging_lost(&status_before, &status_after), 0, "{what}");

    run_undo(work, NOW + 1, false)
        .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(work), before, "{what}: undo exact");
    run_redo(work, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{what}: {}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(work), after, "{what}: redo exact");
}

/// The staged and unstaged diffs of everything but `shared.txt`, whose
/// blob ids the pull changes.
fn split_but_shared(repo: &TempRepo) -> (String, String) {
    (
        repo.git(&[
            "diff",
            "--cached",
            "--no-renames",
            "--",
            ".",
            ":!shared.txt",
        ]),
        repo.git(&["diff", "--no-renames", "--", ".", ":!shared.txt"]),
    )
}

/// ADR-0045 §7's tree table through a pull, one tree at a time (the set
/// runs in one test: each is a small repository pair).
#[test]
fn the_ff_pull_set_aside_round_trips() {
    for tree in Tree::ALL {
        round_trip(tree);
    }
}

// ── a stop before each step ─────────────────────────────────────────────

/// Hold `git merge` in its `post-merge` hook — after it has moved `HEAD` —
/// until the runner's stop kills it. Bounded at ten seconds.
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
/// before it leaves on disk (as `set_aside::stop_before`). The tracking ref
/// is fetched before, so the pull's own fetch writes no objects and step
/// 2's key is `stash create`'s alone.
fn stop_before(work: &TempRepo, step: u8) -> StopWhen {
    if step >= 6 {
        hold_merge_until_killed(work);
    }
    let root = work.path().to_path_buf();
    let git_dir = root.join(".git");
    let journal = journal_path(&work.open()).expect("the journal path");
    let objects = loose_objects(&git_dir);
    let start_tip = id(work, "HEAD");
    match step {
        1 => StopWhen::new(|| true),
        2 => StopWhen::new(move || loose_objects(&git_dir) > objects),
        3 => StopWhen::new(move || std::fs::read(&journal).is_ok_and(|bytes| !bytes.is_empty())),
        4 => StopWhen::new(move || git_dir.join("refs/stash").exists()),
        // `reset --hard` has put `shared.txt` back to `HEAD`'s.
        5 => StopWhen::new(move || std::fs::read(root.join("shared.txt")).is_ok_and(|b| b == TEN)),
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

/// ADR-0045 §5, §7 for a pull: a stop before each of steps 1–6 leaves his
/// edits in the tree or in the stash list — read from both — and never an
/// entry left open.
#[test]
fn a_ff_pull_stopped_before_each_step_loses_nothing() {
    for step in 1..=6 {
        let behind = clone_of_origin();
        let work = &behind.work;
        theirs(&behind, "shared.txt", THEIRS);
        work.git(&["fetch", "-q", "origin"]);
        work.write("shared.txt", MINE);
        work.write("n.txt", b"new\n");
        work.git(&["add", "n.txt"]);
        let op = pull(work);
        let stop = stop_before(work, step);

        let result = run_with_stop(work, &op, NOW, &stop);

        let what = format!("stop before step {step}: {:?}", result.as_ref().err());
        assert!(stop.latched.load(Ordering::Relaxed), "{what}: never fired");
        let mine_somewhere = edit_is_somewhere(work, "shared.txt", MINE)
            || edit_is_somewhere(work, "shared.txt", BOTH);
        assert!(mine_somewhere, "{what}: shared.txt's edit lost");
        assert!(
            edit_is_somewhere(work, "n.txt", b"new\n"),
            "{what}: n.txt lost"
        );
        assert!(
            journal_entries(work)
                .iter()
                .all(|entry| entry.status != EntryStatus::Pending),
            "{what}: an entry was left open"
        );
    }
}
