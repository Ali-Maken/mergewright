//! `c` on a merge **begun in the shell** (m6-conflicts/20): no entry of
//! Graft's behind it, so `resume(Continue)` finishes it as an operation of
//! its own — `Operation::FinishMerge` — whose `u` puts the repository back
//! mid-merge, exactly.
//!
//! Every merge here is started with plain `git`, never through Graft, and
//! every assertion is read back out of git.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use graft_core::journal::{EntryStatus, JournalEntry, MergeState, OperationKind};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::state::InProgress;
use graft_ops::apply::{ApplyError, Resume};
use graft_ops::journal::in_force;
use graft_ops::operation::{MergeEnd, Refusal};
use graft_ops::restore::{self, RestoreError};
use graft_ops::undo::{self, NotInvertible, UndoError};

use crate::conflict_fixtures::{downstream_of, run_resume, two_file_conflict};
use crate::conflict_ops::journal_ids;
use crate::finish::{head_message, refuse_commits, stripspace};
use crate::support::{
    NOW, TempRepo, context, expire_and_prune, holds, journal_for, run_redo, run_undo,
};

// ── fixtures and records ────────────────────────────────────────────────

/// `git merge feature` stopped on `a.txt` and `b.txt`, both resolved in the
/// shell with `checkout --theirs` + `add`. Graft journaled nothing.
fn shell_merge_resolved() -> TempRepo {
    let repo = shell_merge_stopped();
    repo.git(&["checkout", "--theirs", "--", "a.txt", "b.txt"]);
    repo.git(&["add", "--", "a.txt", "b.txt"]);
    repo
}

/// `git merge feature`, stopped, nothing resolved.
fn shell_merge_stopped() -> TempRepo {
    let repo = two_file_conflict();
    assert!(
        repo.try_git(&["merge", "feature"]).is_err(),
        "the merge is meant to stop"
    );
    assert!(journal_ids(&repo).is_empty(), "Graft journaled nothing");
    repo
}

/// `git <args>`'s stdout, byte for byte: no trimming.
fn raw_git(repo: &TempRepo, args: &[&str]) -> Vec<u8> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(repo.path())
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

/// Where git keeps one of its files: `rev-parse --git-path`.
fn git_path(repo: &TempRepo, name: &str) -> PathBuf {
    let printed = repo.git(&["rev-parse", "--git-path", name]);
    let path = Path::new(&printed);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo.path().join(path)
    }
}

/// A file's bytes, or `None` when it is absent.
fn maybe_read(path: &Path) -> Option<Vec<u8>> {
    match std::fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => None,
        Err(why) => panic!("{}: {why}", path.display()),
    }
}

/// The three merge files, each at its `--git-path`.
fn merge_files(repo: &TempRepo) -> Vec<(&'static str, Option<Vec<u8>>)> {
    ["MERGE_HEAD", "MERGE_MSG", "MERGE_MODE"]
        .into_iter()
        .map(|name| (name, maybe_read(&git_path(repo, name))))
        .collect()
}

/// Everything A0 compares: git's own account of the repository, and the
/// bytes on disk.
#[derive(Debug, PartialEq, Eq)]
struct Records {
    status: Vec<u8>,
    stage: Vec<u8>,
    merge_files: Vec<(&'static str, Option<Vec<u8>>)>,
    tracked: Vec<(String, Option<Vec<u8>>)>,
}

fn records(repo: &TempRepo) -> Records {
    let listed = raw_git(repo, &["ls-files", "-z"]);
    let tracked = listed
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|name| {
            let name = String::from_utf8(name.to_vec()).expect("utf-8 test paths");
            let bytes = maybe_read(&repo.path().join(&name));
            (name, bytes)
        })
        .collect();
    Records {
        status: raw_git(repo, &["status", "--porcelain=v2", "--branch"]),
        stage: raw_git(repo, &["ls-files", "--stage"]),
        merge_files: merge_files(repo),
        tracked,
    }
}

/// git's merge state, read straight off the disk: what `merge_before` must
/// equal.
fn merge_state_on_disk(repo: &TempRepo) -> MergeState {
    MergeState {
        merge_head: oid(repo, "MERGE_HEAD"),
        other_heads: Vec::new(),
        merge_msg: maybe_read(&git_path(repo, "MERGE_MSG")).expect("MERGE_MSG"),
        merge_mode: maybe_read(&git_path(repo, "MERGE_MODE")),
    }
}

fn oid(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("git prints an object id")
}

fn merging(repo: &TempRepo) -> bool {
    repo.try_git(&["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .is_ok()
}

fn entries(repo: &TempRepo) -> Vec<JournalEntry> {
    journal_for(&repo.open()).entries().to_vec()
}

fn finish(repo: &TempRepo, how: Resume, at: i64) -> JournalEntry {
    run_resume(repo, how, at)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .entry
}

// ── A0 ──────────────────────────────────────────────────────────────────

/// A0: `c`, then `u`, and every record is byte-equal to its value before
/// `c` — git's status, the index's stages, the three merge files, every
/// tracked file.
#[test]
fn undo_after_c_puts_back_exactly_what_was_there_before_c() {
    let repo = shell_merge_resolved();
    let before = records(&repo);
    assert!(
        before.merge_files.iter().all(|(_, bytes)| bytes.is_some()),
        "the fixture has all three merge files: {:?}",
        before.merge_files
    );

    finish(&repo, Resume::Continue, NOW);
    assert!(!merging(&repo), "c finished the merge");
    assert_ne!(records(&repo), before, "c changed something to undo");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    let after = records(&repo);
    assert_eq!(
        String::from_utf8_lossy(&after.status),
        String::from_utf8_lossy(&before.status),
        "git status"
    );
    assert_eq!(
        String::from_utf8_lossy(&after.stage),
        String::from_utf8_lossy(&before.stage),
        "ls-files --stage"
    );
    assert_eq!(after.merge_files, before.merge_files, "the merge files");
    assert_eq!(after.tracked, before.tracked, "the tracked files");
    assert_eq!(after, before);
}

// ── A1: the route and the entry ─────────────────────────────────────────

#[test]
fn continue_finishes_a_merge_begun_in_the_shell() {
    let repo = shell_merge_resolved();
    let feature = oid(&repo, "feature");
    let state = merge_state_on_disk(&repo);
    assert_eq!(state.merge_head, feature);

    let entry = finish(&repo, Resume::Continue, NOW);

    assert_eq!(
        oid(&repo, "HEAD^2"),
        feature,
        "the merge commit's second parent"
    );
    assert!(!merging(&repo), "MERGE_HEAD is gone");
    assert_eq!(journal_ids(&repo), vec![entry.id.to_string()], "one entry");
    assert_eq!(entry.kind, OperationKind::FinishMerge);
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(entry.description, "finish merge of feature");
    assert_eq!(entry.merge_before, Some(state), "the state read before");
    assert_eq!(entry.merge_after, None);
    assert!(entry.backup_ref.is_some(), "a backup of its own");
    assert!(
        entry
            .commands
            .iter()
            .any(|line| line.ends_with("merge --continue")),
        "the Command Log: {:?}",
        entry.commands
    );
    assert!(in_force(&entries(&repo), &entry.id));
}

#[test]
fn an_unmerged_shell_merge_is_refused_unmerged_not_not_in_progress() {
    let repo = shell_merge_stopped();
    let head = oid(&repo, "HEAD");
    let msg = maybe_read(&git_path(&repo, "MERGE_MSG"));

    for how in [
        Resume::Continue,
        Resume::ContinueWith {
            message: b"mine\n".to_vec(),
        },
    ] {
        let refused = run_resume(&repo, how, NOW).expect_err("unmerged paths");
        match refused {
            ApplyError::Refused(Refusal::Unmerged { paths }) => {
                assert_eq!(paths, vec![PathBuf::from("a.txt"), PathBuf::from("b.txt")]);
            }
            other => panic!("expected Unmerged, got {other:?}"),
        }
    }
    assert!(journal_ids(&repo).is_empty(), "no entry");
    assert_eq!(oid(&repo, "HEAD"), head, "no commit");
    assert!(merging(&repo), "still merging");
    assert_eq!(maybe_read(&git_path(&repo, "MERGE_MSG")), msg);
}

/// What still reaches `NotInProgress`: a skip of a merge with no entry, and
/// nothing in progress at all. An abort of one used to be here too; since
/// m6-conflicts/57 it aborts, as `AbortMerge` (`abort_merge.rs`).
#[test]
fn skip_of_a_shell_merge_stays_not_in_progress() {
    let repo = shell_merge_resolved();
    let refused = run_resume(&repo, Resume::Skip, NOW).expect_err("not Graft's");
    assert!(
        matches!(refused, ApplyError::Refused(Refusal::NotInProgress)),
        "{refused:?}"
    );
    assert!(merging(&repo));
    assert!(journal_ids(&repo).is_empty());

    let clean = two_file_conflict();
    let refused = run_resume(&clean, Resume::Continue, NOW).expect_err("nothing in progress");
    assert!(matches!(
        refused,
        ApplyError::Refused(Refusal::NotInProgress)
    ));
}

#[test]
fn an_edited_message_finishes_a_shell_begun_merge() {
    let repo = shell_merge_resolved();
    let message = b"Subject  \n\n# kept\nbody\n\n\n".to_vec();

    let entry = finish(
        &repo,
        Resume::ContinueWith {
            message: message.clone(),
        },
        NOW,
    );

    let landed = head_message(&repo);
    assert_eq!(
        String::from_utf8_lossy(&landed),
        stripspace(&message),
        "git's whitespace cleanup of what was sent"
    );
    assert_eq!(landed, b"Subject\n\n# kept\nbody\n".to_vec());
    assert_eq!(oid(&repo, "HEAD^2"), oid(&repo, "feature"));
    assert!(!merging(&repo));
    assert_eq!(entry.kind, OperationKind::FinishMerge);
    assert!(
        entry
            .commands
            .iter()
            .any(|line| line.ends_with("commit --cleanup=whitespace -F -")),
        "the Command Log: {:?}",
        entry.commands
    );
}

/// `git stripspace --strip-comments`: what `merge --continue` makes of
/// `MERGE_MSG`.
fn strip_comments(message: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut child = std::process::Command::new("git")
        .args(["stripspace", "--strip-comments"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .expect("stripspace runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(message)
        .expect("written");
    child.wait_with_output().expect("stripspace ends").stdout
}

#[test]
fn an_unedited_finish_keeps_gits_message() {
    let repo = shell_merge_resolved();
    let msg = maybe_read(&git_path(&repo, "MERGE_MSG")).expect("MERGE_MSG");
    assert!(
        String::from_utf8_lossy(&msg).contains("# Conflicts:"),
        "git's comment lines are there to be stripped: {}",
        String::from_utf8_lossy(&msg)
    );

    finish(&repo, Resume::Continue, NOW);

    assert_eq!(
        String::from_utf8_lossy(&head_message(&repo)),
        String::from_utf8_lossy(&strip_comments(&msg))
    );
}

// ── A3: undo, redo, again ───────────────────────────────────────────────

#[test]
fn redo_after_undo_lands_on_the_same_merge_commit() {
    let repo = shell_merge_resolved();
    finish(&repo, Resume::Continue, NOW);
    let merge_commit = oid(&repo, "HEAD");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert!(merging(&repo), "u put the merge back");
    run_redo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("redo: {}", why.explain(&crate::support::Letters)));

    assert_eq!(oid(&repo, "HEAD"), merge_commit, "the same merge commit");
    assert_eq!(
        merge_files(&repo),
        vec![
            ("MERGE_HEAD", None),
            ("MERGE_MSG", None),
            ("MERGE_MODE", None)
        ],
        "no merge files"
    );
    assert!(!merging(&repo));
}

#[test]
fn c_again_after_undo_makes_an_equal_merge() {
    let repo = shell_merge_resolved();
    finish(&repo, Resume::Continue, NOW);
    let tree = repo.git(&["rev-parse", "HEAD^{tree}"]);
    let parents = repo.git(&["log", "-1", "--format=%P"]);

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    let again = finish(&repo, Resume::Continue, NOW + 2);

    assert_eq!(again.kind, OperationKind::FinishMerge);
    assert_eq!(repo.git(&["rev-parse", "HEAD^{tree}"]), tree, "same tree");
    assert_eq!(
        repo.git(&["log", "-1", "--format=%P"]),
        parents,
        "same parents"
    );
    assert!(!merging(&repo));
}

/// A stop that fires, and latches, once a ref reads a given commit.
struct StopWhenRefIs {
    repo: PathBuf,
    name: &'static str,
    at: String,
    fired: AtomicBool,
}

impl Stop for StopWhenRefIs {
    fn should_stop(&self) -> bool {
        if self.fired.load(Ordering::Relaxed) {
            return true;
        }
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(&self.repo)
            .args(["rev-parse", "--verify", "--quiet", self.name])
            .output()
            .expect("git runs");
        let now = String::from_utf8_lossy(&out.stdout).trim_end().to_owned();
        let fire = out.status.success() && now == self.at;
        if fire {
            self.fired.store(true, Ordering::Relaxed);
        }
        fire
    }
}

#[test]
fn a_stop_before_set_merge_state_rolls_back() {
    let repo = shell_merge_resolved();
    let pre_merge = repo.git(&["rev-parse", "HEAD"]);
    finish(&repo, Resume::Continue, NOW);
    let merge_commit = oid(&repo, "HEAD");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let plan = undo::plan_undo(journal.entries()).expect("the finish is undoable");
    assert!(
        matches!(plan.steps.last(), Some(undo::Step::SetMergeState(Some(_)))),
        "the merge state is the last step: {:?}",
        plan.steps
    );
    let stop = StopWhenRefIs {
        repo: repo.path().to_path_buf(),
        name: "refs/heads/main",
        at: pre_merge,
        fired: AtomicBool::new(false),
    };
    let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
    let why = restore::execute(&mut ctx, &plan).expect_err("cancelled");
    assert!(
        stop.fired.load(Ordering::Relaxed),
        "the stop fired on the ref step"
    );
    assert!(
        matches!(why, RestoreError::Cancelled { .. }),
        "{}",
        why.explain(&crate::support::Letters)
    );

    assert_eq!(oid(&repo, "HEAD"), merge_commit, "HEAD is the merge commit");
    assert!(!merging(&repo), "no MERGE_HEAD");
    let all = entries(&repo);
    let last = all.last().expect("the cancelled undo is journaled");
    assert_eq!(
        last.status,
        EntryStatus::Aborted {
            reason: "cancelled".to_owned()
        }
    );
    assert!(!in_force(&all, &last.id), "not in force");
}

/// A merge begun since the entry is no longer drift the user can force
/// past: it is the undo gate's refusal (m6-conflicts/57, decision 3) — the
/// entry was not recorded inside the merge standing now. Before 57 this was
/// `MergeDrift::Begun` behind the force box.
#[test]
fn undo_refuses_when_a_new_merge_was_begun() {
    let repo = shell_merge_resolved();
    finish(&repo, Resume::Continue, NOW);
    // A merge that changes nothing on disk: only git's merge state moves.
    repo.git(&["branch", "empty"]);
    repo.git(&["switch", "-q", "empty"]);
    repo.git(&["commit", "-q", "--allow-empty", "-m", "nothing"]);
    repo.git(&["switch", "-q", "main"]);
    repo.git(&["merge", "--no-ff", "--no-commit", "empty"]);
    assert!(merging(&repo));
    let head = oid(&repo, "HEAD");
    let count = entries(&repo).len();
    let files = merge_files(&repo);

    for force in [false, true] {
        let refused = run_undo(&repo, NOW + 1, force).expect_err("a merge is live");
        assert!(
            matches!(
                refused,
                UndoError::Nothing(NotInvertible::OperationLive {
                    kind: InProgress::Merge
                })
            ),
            "force {force}: {}",
            refused.explain(&crate::support::Letters)
        );
        assert_eq!(
            refused.explain(&crate::support::Letters),
            "finish or abort the merge first"
        );
    }
    assert_eq!(entries(&repo).len(), count, "nothing journaled");
    assert_eq!(oid(&repo, "HEAD"), head, "nothing moved");
    assert_eq!(merge_files(&repo), files, "the new merge is untouched");
}

#[test]
fn a_hook_refusal_leaves_the_merge_in_progress() {
    let repo = shell_merge_resolved();
    refuse_commits(&repo);
    let head = oid(&repo, "HEAD");

    let why = run_resume(&repo, Resume::Continue, NOW).expect_err("the hook refuses");
    let entry = match why {
        ApplyError::Git { entry, .. } => *entry,
        other => panic!("expected the failed git step, got {other:?}"),
    };
    assert!(merging(&repo), "MERGE_HEAD present");
    assert_eq!(oid(&repo, "HEAD"), head, "no commit");
    assert_eq!(entry.kind, OperationKind::FinishMerge);
    assert!(
        matches!(entry.status, EntryStatus::Aborted { .. }),
        "{:?}",
        entry.status
    );
    assert!(!in_force(&entries(&repo), &entry.id), "not in force");
}

// ── A4: pull ────────────────────────────────────────────────────────────

/// `refs/remotes/*`, as `for-each-ref` prints them.
fn remote_refs(repo: &TempRepo) -> String {
    repo.git(&[
        "for-each-ref",
        "--format=%(refname) %(objectname)",
        "refs/remotes",
    ])
}

#[test]
fn a_pull_begun_in_the_shell_finishes_the_same_way() {
    let upstream = two_file_conflict();
    let repo = downstream_of(&upstream, false);
    let url = upstream.path().to_str().expect("utf-8 temp path");
    repo.git(&["remote", "add", "origin", url]);
    repo.git(&["config", "pull.rebase", "false"]);
    assert!(
        repo.try_git(&["pull", "--no-edit", "origin", "feature"])
            .is_err(),
        "the pull stops"
    );
    assert!(merging(&repo));
    let fetched = oid(&repo, "MERGE_HEAD");
    assert_eq!(fetched, oid(&upstream, "feature"));
    repo.git(&["checkout", "--theirs", "--", "a.txt"]);
    repo.git(&["add", "--", "a.txt"]);
    let tracking = remote_refs(&repo);
    assert!(!tracking.is_empty(), "the pull's fetch set a tracking ref");
    let before = records(&repo);

    let entry = finish(&repo, Resume::Continue, NOW);
    assert_eq!(entry.kind, OperationKind::FinishMerge);
    assert_eq!(oid(&repo, "HEAD^2"), fetched, "the fetched tip");
    assert!(!merging(&repo));

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(oid(&repo, "MERGE_HEAD"), fetched, "MERGE_HEAD back");
    assert_eq!(
        remote_refs(&repo),
        tracking,
        "u left the tracking refs alone"
    );
    assert_eq!(records(&repo), before);
}

// ── A2: the record ──────────────────────────────────────────────────────

/// Non-UTF-8 `MERGE_MSG` bytes go through the journal line — written, read
/// back by a fresh `Journal::open` — and back onto the disk by `u`, exact.
#[test]
fn merge_state_round_trips_non_utf8_bytes() {
    let repo = shell_merge_resolved();
    let message = b"Merge \xff\xfe feature\n\n# Conflicts:\n#\ta.txt\n".to_vec();
    std::fs::write(git_path(&repo, "MERGE_MSG"), &message).expect("MERGE_MSG");
    let mode = b"no-ff".to_vec();
    std::fs::write(git_path(&repo, "MERGE_MODE"), &mode).expect("MERGE_MODE");

    let entry = finish(&repo, Resume::Continue, NOW);
    let read_back = entries(&repo)
        .into_iter()
        .find(|found| found.id == entry.id)
        .expect("the entry, parsed from its line");
    let state = read_back.merge_before.expect("merge_before");
    assert_eq!(state.merge_msg, message, "MERGE_MSG, byte for byte");
    assert_eq!(state.merge_mode, Some(mode.clone()), "MERGE_MODE");

    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(maybe_read(&git_path(&repo, "MERGE_MSG")), Some(message));
    assert_eq!(maybe_read(&git_path(&repo, "MERGE_MODE")), Some(mode));
}

/// A pull from a URL leaves a `MERGE_HEAD` only `FETCH_HEAD` reaches. The
/// finish's backup commit has it among its parents, so `u` writes it back
/// after `gc`.
#[test]
fn the_merge_head_commit_is_anchored() {
    let upstream = two_file_conflict();
    let repo = downstream_of(&upstream, false);
    let url = upstream.path().to_str().expect("utf-8 temp path");
    assert!(
        repo.try_git(&["pull", "--no-rebase", "--no-edit", url, "feature"])
            .is_err()
    );
    let fetched = repo.git(&["rev-parse", "MERGE_HEAD"]);
    repo.git(&["checkout", "--theirs", "--", "a.txt"]);
    repo.git(&["add", "--", "a.txt"]);

    let entry = finish(&repo, Resume::Continue, NOW);
    let backup = entry.backup_ref.expect("a backup ref");
    // The backup ref holds the after-commit; its first parent is the
    // before-commit `begin` took.
    let before_parents = repo.git(&["rev-parse", &format!("{backup}^1^@")]);
    assert!(
        before_parents.lines().any(|line| line == fetched),
        "the before-commit's parents {before_parents:?} include {fetched}"
    );

    std::fs::remove_file(git_path(&repo, "FETCH_HEAD")).expect("FETCH_HEAD");
    expire_and_prune(&repo);
    assert!(holds(&repo, &fetched), "gc kept MERGE_HEAD's commit");
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    assert_eq!(repo.git(&["rev-parse", "MERGE_HEAD"]), fetched);
}

// ── observations the ticket asked for ───────────────────────────────────

/// REUC (A3): Graft never writes the resolve-undo extension. Observed: the
/// record `add` made before `c` is still in the index after `c` and after
/// `u` (`git ls-files --resolve-undo` reads the same at all three points),
/// so `git checkout -m <path>` rebuilds the conflict after `u` exactly as it
/// would have before `c`. Recorded in the ticket's NOTES.
#[test]
fn after_u_checkout_m_still_rebuilds_the_conflict() {
    let before_c = shell_merge_resolved();
    before_c.git(&["checkout", "-m", "--", "a.txt"]);
    let rebuilt = before_c.git(&["ls-files", "-u", "--", "a.txt"]);
    assert_eq!(rebuilt.lines().count(), 3, "before c: REUC rebuilds it");

    let repo = shell_merge_resolved();
    let reuc_before = repo.git(&["ls-files", "--resolve-undo"]);
    assert!(!reuc_before.is_empty(), "add recorded resolve-undo");
    finish(&repo, Resume::Continue, NOW);
    let reuc_after_c = repo.git(&["ls-files", "--resolve-undo"]);
    run_undo(&repo, NOW + 1, false)
        .unwrap_or_else(|why| panic!("undo: {}", why.explain(&crate::support::Letters)));
    let reuc_after_u = repo.git(&["ls-files", "--resolve-undo"]);
    assert_eq!(reuc_after_c, reuc_before, "c kept it");
    assert_eq!(reuc_after_u, reuc_before, "u kept it");

    repo.git(&["checkout", "-m", "--", "a.txt"]);
    let unmerged = repo.git(&["ls-files", "-u", "--", "a.txt"]);
    assert_eq!(unmerged, rebuilt, "after u: the same three stages");
}

/// `git merge --autostash feature` over a dirty `notes.txt`, stopped on the
/// conflict and resolved in the shell: `MERGE_AUTOSTASH` holds the edit.
fn autostashed_merge_resolved() -> TempRepo {
    let repo = two_file_conflict();
    repo.write("notes.txt", b"committed\n");
    repo.commit_all("notes");
    repo.write("notes.txt", b"my uncommitted edit\n");
    assert!(
        repo.try_git(&["merge", "--autostash", "feature"]).is_err(),
        "the merge stops"
    );
    assert!(
        git_path(&repo, "MERGE_AUTOSTASH").exists(),
        "git kept the stash"
    );
    assert_eq!(
        repo.read("notes.txt"),
        b"committed\n",
        "the edit is stashed"
    );
    repo.git(&["checkout", "--theirs", "--", "a.txt", "b.txt"]);
    repo.git(&["add", "--", "a.txt", "b.txt"]);
    repo
}

/// The check the ticket asked for, in plain git: does finishing a merge
/// apply `MERGE_AUTOSTASH`?
#[test]
fn git_applies_merge_autostash_when_the_merge_is_finished() {
    for finish_with in [
        ["-c", "core.editor=true", "merge", "--continue"].as_slice(),
        ["commit", "--no-edit"].as_slice(),
    ] {
        let repo = autostashed_merge_resolved();
        repo.git(finish_with);
        assert!(!merging(&repo), "{finish_with:?} finished the merge");
        assert!(
            !git_path(&repo, "MERGE_AUTOSTASH").exists(),
            "{finish_with:?} used the stash up"
        );
        assert_eq!(
            repo.read("notes.txt"),
            b"my uncommitted edit\n",
            "{finish_with:?} applied it"
        );
    }
}

/// So `c` refuses while `MERGE_AUTOSTASH` waits: finishing would apply the
/// stash, and `u` would put the tree back without it. Refused **by name**
/// since m6-conflicts/57 — it used to say `NotInProgress`, "nothing Graft
/// started is in progress", about a merge standing in front of the user.
#[test]
fn finish_refuses_while_git_holds_an_autostash() {
    let repo = autostashed_merge_resolved();
    let head = oid(&repo, "HEAD");

    for how in [
        Resume::Continue,
        Resume::ContinueWith {
            message: b"mine\n".to_vec(),
        },
    ] {
        let refused = run_resume(&repo, how, NOW).expect_err("refused");
        assert!(
            matches!(
                refused,
                ApplyError::Refused(Refusal::AutostashWaits {
                    end: MergeEnd::Finish
                })
            ),
            "{refused:?}"
        );
        assert_eq!(
            refused.explain(&crate::support::Letters),
            "git set your changes aside for this merge (autostash); finish it in the shell: \
             git commit"
        );
    }
    assert!(journal_ids(&repo).is_empty(), "no entry");
    assert_eq!(oid(&repo, "HEAD"), head, "no commit");
    assert!(merging(&repo), "still merging");
    assert!(
        git_path(&repo, "MERGE_AUTOSTASH").exists(),
        "the stash waits"
    );
    assert_eq!(repo.read("notes.txt"), b"committed\n", "not applied");
}

/// The check for the other end, in plain git: `merge --abort` applies
/// `MERGE_AUTOSTASH` too, so an abort from Graft would spend a stash its
/// `u` could not give back.
#[test]
fn git_applies_merge_autostash_when_the_merge_is_aborted() {
    let repo = autostashed_merge_resolved();
    repo.git(&["merge", "--abort"]);
    assert!(!merging(&repo), "aborted");
    assert!(
        !git_path(&repo, "MERGE_AUTOSTASH").exists(),
        "the abort used the stash up"
    );
    assert_eq!(
        repo.read("notes.txt"),
        b"my uncommitted edit\n",
        "and applied it"
    );
}

/// So `a` refuses while `MERGE_AUTOSTASH` waits, by name, before anything
/// is written: the stash is never at risk (m6-conflicts/57).
#[test]
fn abort_refuses_while_git_holds_an_autostash() {
    let repo = autostashed_merge_resolved();
    let head = oid(&repo, "HEAD");
    let files = merge_files(&repo);

    let refused = run_resume(&repo, Resume::Abort, NOW).expect_err("refused");
    assert!(
        matches!(
            refused,
            ApplyError::Refused(Refusal::AutostashWaits {
                end: MergeEnd::Abort
            })
        ),
        "{refused:?}"
    );
    assert_eq!(
        refused.explain(&crate::support::Letters),
        "git set your changes aside for this merge (autostash); abort it in the shell: git merge \
         --abort"
    );
    assert!(journal_ids(&repo).is_empty(), "no entry");
    assert_eq!(oid(&repo, "HEAD"), head, "HEAD unmoved");
    assert!(merging(&repo), "still merging");
    assert_eq!(merge_files(&repo), files, "the merge files untouched");
    assert!(
        git_path(&repo, "MERGE_AUTOSTASH").exists(),
        "the stash waits"
    );
    assert_eq!(repo.read("notes.txt"), b"committed\n", "not applied");
}
