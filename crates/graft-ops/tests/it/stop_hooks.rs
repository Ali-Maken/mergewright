//! m8-polish-and-ship/13c: a write stopped in its git hook is left where git
//! leaves it — paused, landed, or (git killed before it could pause) aborted
//! with `u` to put it back. Driven through `apply` and `resume` on real
//! repositories with real hooks, the stop fired once the hook is running.
//!
//! Every process a hook here starts records its pid under `.git/test-hooks/`,
//! and [`Hooked`]'s drop signals exactly those pids — never a pattern.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use graft_core::journal::{EntryStatus, JournalEntry, OperationKind, PutBackWhy};
use graft_core::oid::Oid;
use graft_git::run::Stop;
use graft_git::state::InProgress;
use graft_ops::apply::{Applied, ApplyError, Resume, StoppedAt, recover_interrupted, resume};
use graft_ops::journal::in_force;
use graft_ops::operation::{
    FastForward, MergeParams, Operation, PickParams, RevertParams, Source, Tip,
};

use crate::set_aside::{porcelain, resume_with, set_aside_of, stash_commits, whole};
use crate::support::{
    NOW, Never, TempRepo, context, journal_for, run_undo, run_with_stop, state_of, try_run,
};

/// What the user had in his tree, unrelated to the merge.
const HIS_EDIT: &[u8] = b"notes, mid-thought\n";

/// main and feature diverged, feature adds `b.txt`, and `notes.txt` carries
/// his uncommitted edit.
fn diverged() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"base\n");
    repo.write("notes.txt", b"notes\n");
    repo.commit_all("base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("b.txt", b"feature\n");
    repo.commit_all("feature work");
    repo.git(&["checkout", "-q", "main"]);
    repo.write("c.txt", b"main\n");
    repo.commit_all("main work");
    repo.write("notes.txt", HIS_EDIT);
    repo
}

fn id(repo: &TempRepo, rev: &str) -> Oid {
    Oid::from_hex(&repo.git(&["rev-parse", rev])).expect("a full id")
}

fn merge_feature(repo: &TempRepo) -> Operation {
    Operation::Merge(MergeParams {
        source: Source::Branch("refs/heads/feature".to_owned()),
        into: Tip {
            name: "refs/heads/main".to_owned(),
            oid: id(repo, "main"),
        },
        source_tip: id(repo, "feature"),
        ff: FastForward::Default,
    })
}

/// A hook installed under `.git/test-hooks/` (outside the tree `state_of`
/// reads), and the pids its processes wrote there.
pub(crate) struct Hooked {
    dir: PathBuf,
}

impl Hooked {
    /// `body` runs under `/bin/sh`; `PIDS` in it is replaced by the pid
    /// directory. The hook must write `PIDS/hook` (atomically, by rename)
    /// once it is where the test wants the stop.
    pub(crate) fn install(repo: &TempRepo, name: &str, body: &str) -> Hooked {
        let dir = repo.path().join(".git/test-hooks");
        std::fs::create_dir_all(&dir).expect("the hooks dir is made");
        let script = dir.join(name);
        let body = body.replace("PIDS", dir.to_str().expect("utf-8"));
        std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("the hook is written");
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
            .expect("the hook is executable");
        repo.git(&["config", "core.hooksPath", dir.to_str().expect("utf-8")]);
        Hooked { dir }
    }

    /// A hook stuck the ordinary way: a shell waiting on a child.
    pub(crate) fn stuck(repo: &TempRepo, name: &str) -> Hooked {
        Hooked::install(
            repo,
            name,
            "sleep 600 &\necho $! > PIDS/sleep\necho $$ > PIDS/hook.tmp && mv PIDS/hook.tmp PIDS/hook\nwait",
        )
    }

    /// Stuck, and out of git's process group: a hook that daemonises. The
    /// stop finds no helper of git's to end, stops the group — git alone —
    /// and git dies before it can pause the merge. `PIDS/hook` is written
    /// only after the hook has left the group.
    pub(crate) fn stuck_outside_the_group(repo: &TempRepo, name: &str) -> Hooked {
        Hooked::install(
            repo,
            name,
            "exec perl -e 'setpgrp(0,0); open(my $f, \">\", \"PIDS/hook.tmp\") or die; \
             print $f $$; close $f; rename(\"PIDS/hook.tmp\", \"PIDS/hook\") or die; sleep 600'",
        )
    }

    pub(crate) fn running(&self) -> PathBuf {
        self.dir.join("hook")
    }

    /// Where the script `name` was installed, for a helper git runs that is
    /// not a hook (a merge driver).
    pub(crate) fn script(&self, name: &str) -> PathBuf {
        self.dir.join(name)
    }

    fn pid(&self, name: &str) -> Option<String> {
        std::fs::read_to_string(self.dir.join(name))
            .ok()
            .map(|pid| pid.trim().to_owned())
            .filter(|pid| !pid.is_empty())
    }

    /// Every pid this hook recorded is gone, within two seconds: the
    /// processes may be zombies for a moment before their new parent reaps
    /// them, and `kill -0` still finds a zombie.
    pub(crate) fn assert_gone(&self) {
        for name in ["hook", "sleep"] {
            if let Some(pid) = self.pid(name) {
                assert!(gone(&pid), "the hook's {name} ({pid}) was ended");
            }
        }
    }

    /// The hook is removed, so a resume runs without it.
    pub(crate) fn remove(&self, name: &str) {
        std::fs::remove_file(self.dir.join(name)).expect("the hook is removed");
    }
}

impl Drop for Hooked {
    fn drop(&mut self) {
        for name in ["hook", "sleep"] {
            if let Some(pid) = self.pid(name) {
                // Safe to ignore: these are pids this test's own hook wrote,
                // and the usual answer is "no such process" — the stop
                // already ended it. This only cleans up after a failure.
                let _cleanup = Command::new("kill")
                    .args(["-KILL", &pid])
                    .stderr(Stdio::null())
                    .status();
            }
        }
    }
}

fn alive(pid: &str) -> bool {
    Command::new("kill")
        .args(["-0", pid])
        .stderr(Stdio::null())
        .status()
        .expect("kill runs")
        .success()
}

fn gone(pid: &str) -> bool {
    let until = Instant::now() + Duration::from_secs(2);
    while alive(pid) {
        if Instant::now() > until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    true
}

/// The user's stop, fired once the hook is running.
pub(crate) struct StopOnceRunning(pub(crate) PathBuf);

impl Stop for StopOnceRunning {
    fn should_stop(&self) -> bool {
        self.0.exists()
    }
}

fn locks_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("the directory reads") {
        let path = entry.expect("the entry reads").path();
        if path.is_dir() {
            found.extend(locks_under(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

fn entry_now(repo: &TempRepo, id: &graft_core::journal::EntryId) -> JournalEntry {
    journal_for(&repo.open())
        .entries()
        .iter()
        .find(|entry| &entry.id == id)
        .cloned()
        .expect("the entry is in the journal")
}

/// While a merge over his edit is paused, the edit waits in the set-aside
/// copy (19d; ADR-0045 §4, as for a stopped rebase): `S` is listed, found by
/// commit, it holds his edit, and the tree holds `HEAD`'s copy.
fn assert_his_edit_waits_in_s(repo: &TempRepo, entry: &JournalEntry) {
    let stash = set_aside_of(entry);
    assert_eq!(
        stash_commits(repo),
        vec![stash],
        "S listed, found by commit"
    );
    assert_eq!(
        repo.git(&["show", &format!("{stash}:notes.txt")])
            .as_bytes(),
        HIS_EDIT.trim_ascii_end(),
        "his edit is in S"
    );
    assert_eq!(
        repo.read("notes.txt"),
        b"notes\n",
        "the tree holds HEAD's copy"
    );
}

/// A paused merge over a clean tree (an untracked file alone): nothing set
/// aside, no stash, and his untracked file untouched — 13c's claim.
fn assert_nothing_set_aside(repo: &TempRepo, entry: &JournalEntry) {
    assert_eq!(entry.put_back, None, "nothing set aside");
    assert!(stash_commits(repo).is_empty(), "no stash");
    assert_eq!(
        repo.read("scratch.txt"),
        b"his scratch\n",
        "his untracked file is untouched"
    );
}

/// A merge stopped in `hook`: it must come back paused, nothing lost; then
/// `a` (`Resume::Abort`) leaves everything as before the merge.
///
/// Over his edit (`diverged`), the edit waits in `S` while paused
/// ([`assert_his_edit_waits_in_s`]) and the abort puts it back with its
/// split and drops `S`. On a clean tree (`diverged_clean` + an untracked
/// file) nothing is set aside, and the untracked file is untouched — 13c's
/// claim.
fn assert_merge_paused(repo: &TempRepo, hook: &str) {
    let hooked = Hooked::stuck(repo, hook);
    let source = id(repo, "feature");
    let head = id(repo, "HEAD");
    let before = whole(repo);
    let split = porcelain(repo);
    let edited = split.iter().any(|line| !line.starts_with("??"));

    let stopped = run_with_stop(
        repo,
        &merge_feature(repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Err(error) = stopped else {
        panic!("a stopped merge does not succeed");
    };
    assert_eq!(
        error.stopped_at(),
        Some(StoppedAt::Paused(InProgress::Merge))
    );
    let ApplyError::Paused { entry, kind, paths } = error else {
        panic!("paused, not {error:?}");
    };
    assert_eq!(kind, InProgress::Merge);
    assert!(paths.is_empty(), "nothing is unmerged: {paths:?}");
    assert_eq!(entry.status, EntryStatus::Pending, "no outcome line");
    assert_eq!(entry_now(repo, &entry.id).status, EntryStatus::Pending);
    assert_eq!(
        repo.git(&["rev-parse", "MERGE_HEAD"]),
        source.to_string(),
        "MERGE_HEAD is the source tip"
    );
    assert_eq!(id(repo, "HEAD"), head, "nothing committed");
    assert!(
        locks_under(&repo.path().join(".git")).is_empty(),
        "no lock left"
    );
    if edited {
        assert_his_edit_waits_in_s(repo, &entry);
    } else {
        assert_nothing_set_aside(repo, &entry);
    }
    hooked.assert_gone();

    hooked.remove(hook);
    let aborted =
        resume_with(repo, Resume::Abort, NOW + 1).unwrap_or_else(|why| panic!("a aborts: {why}"));
    assert_eq!(aborted.entry.id, entry.id, "the paused entry is closed");
    assert_eq!(porcelain(repo), split, "back, the split kept");
    assert_eq!(whole(repo), before, "as before, S dropped");
}

/// Pure git, no Graft: the contract the pause stands on. A `pre-merge-commit`
/// hook killed by a signal fails like any other failing hook, and git writes
/// the merge down (`abort_commit` → `write_merge_state`) and exits 1.
#[test]
fn git_pauses_a_merge_whose_pre_merge_commit_hook_is_killed() {
    let repo = diverged();
    let hooked = Hooked::install(
        &repo,
        "pre-merge-commit",
        "echo $$ > PIDS/hook.tmp && mv PIDS/hook.tmp PIDS/hook\nexec sleep 600",
    );
    let source = id(&repo, "feature");
    let mut git = Command::new("git")
        .current_dir(repo.path())
        .args(["merge", "--no-edit", "feature"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("git starts");
    let until = Instant::now() + Duration::from_secs(10);
    while hooked.pid("hook").is_none() {
        assert!(Instant::now() < until, "the hook ran");
        std::thread::sleep(Duration::from_millis(10));
    }

    let hook = hooked.pid("hook").expect("the hook's pid");
    let killed = Command::new("kill")
        .args(["-TERM", &hook])
        .status()
        .expect("kill runs");
    let status = git.wait().expect("git ends");

    assert!(killed.success(), "the hook was signalled");
    assert_eq!(
        status.code(),
        Some(1),
        "git failed the merge, it was not killed"
    );
    assert_eq!(repo.git(&["rev-parse", "MERGE_HEAD"]), source.to_string());
    assert!(
        repo.path().join(".git/MERGE_MSG").exists(),
        "MERGE_MSG written"
    );
}

#[test]
fn a_merge_stopped_in_its_pre_merge_commit_hook_is_paused() {
    assert_merge_paused(&diverged(), "pre-merge-commit");
}

#[test]
fn a_merge_stopped_in_its_prepare_commit_msg_hook_is_paused() {
    assert_merge_paused(&diverged(), "prepare-commit-msg");
}

/// A clean tree with an untracked file in it: nothing to set aside.
fn clean_with_scratch() -> TempRepo {
    let repo = diverged_clean();
    repo.write("scratch.txt", b"his scratch\n");
    repo
}

#[test]
fn a_clean_merge_stopped_in_its_pre_merge_commit_hook_is_paused() {
    assert_merge_paused(&clean_with_scratch(), "pre-merge-commit");
}

#[test]
fn a_clean_merge_stopped_in_its_prepare_commit_msg_hook_is_paused() {
    assert_merge_paused(&clean_with_scratch(), "prepare-commit-msg");
}

/// His edit was set aside (19d): the merge landed, the put-back ran, and the
/// answer still says `Landed` — the set-aside close keeps the stop's mark
/// (19d-a2 D1). `u` puts the whole thing back, the stash list included.
#[test]
fn a_merge_stopped_in_its_post_merge_hook_has_landed() {
    let repo = diverged();
    let hooked = Hooked::stuck(&repo, "post-merge");
    let source = id(&repo, "feature");
    let before = whole(&repo);
    let split = porcelain(&repo);

    let landed = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Ok(Applied {
        entry, stopped_at, ..
    }) = landed
    else {
        panic!("a merge committed before its post-merge hook landed: {landed:?}");
    };
    assert_eq!(stopped_at, Some(StoppedAt::Landed));
    assert_eq!(entry.status, EntryStatus::Committed);
    set_aside_of(&entry);
    assert_eq!(id(&repo, "HEAD^2"), source, "the merge commit is made");
    assert!(!repo.path().join(".git/MERGE_HEAD").exists());
    assert_eq!(repo.read("notes.txt"), HIS_EDIT, "his edit is back");
    assert_eq!(porcelain(&repo), split, "with its split");
    assert!(stash_commits(&repo).is_empty(), "S dropped");
    hooked.assert_gone();

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("u puts it back: {why}"));
    assert_eq!(
        whole(&repo),
        before,
        "HEAD, index, tree and stash list as before"
    );
}

/// [`diverged`], and `feature` also rewrites the line his edit changes: the
/// merge itself is clean (his edit is aside while it runs), the put-back of
/// his edit over it clashes on `notes.txt`.
fn diverged_over_his_line() -> TempRepo {
    let repo = TempRepo::init();
    repo.write("a.txt", b"base\n");
    repo.write("notes.txt", b"notes\n");
    repo.commit_all("base");
    repo.git(&["checkout", "-q", "-b", "feature"]);
    repo.write("b.txt", b"feature\n");
    repo.write("notes.txt", b"notes, as feature has them\n");
    repo.commit_all("feature work");
    repo.git(&["checkout", "-q", "main"]);
    repo.write("c.txt", b"main\n");
    repo.commit_all("main work");
    repo.write("notes.txt", HIS_EDIT);
    repo
}

/// The merge landed before its post-merge hook was stopped, and the put-back
/// of his edit then clashed (19d-a3, ticket §R7.3). The answer is the clash —
/// the resolver takes it from there (ADR-0045 §2) — and it still says
/// `Landed`: the merge is committed, not paused. `S` is kept, the clash is
/// live, and `u` puts it all back.
#[test]
fn a_merge_that_landed_before_the_stop_and_clashed_on_put_back_says_landed() {
    let repo = diverged_over_his_line();
    let hooked = Hooked::stuck(&repo, "post-merge");
    let source = id(&repo, "feature");
    let before = whole(&repo);

    let stopped = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Err(error) = stopped else {
        panic!("a put-back that clashed does not succeed");
    };
    assert_eq!(error.stopped_at(), Some(StoppedAt::Landed));
    let ApplyError::Clashed {
        entry,
        clash,
        stopped_at,
    } = error
    else {
        panic!("the clash, not {error:?}");
    };
    assert_eq!(stopped_at, Some(StoppedAt::Landed));
    assert_eq!(entry.status, EntryStatus::Committed);
    assert_eq!(id(&repo, "HEAD^2"), source, "the merge commit is made");
    assert!(!repo.path().join(".git/MERGE_HEAD").exists(), "not paused");
    let stash = set_aside_of(&entry);
    assert_eq!(clash.stash, stash);
    assert_eq!(clash.why, PutBackWhy::SetAside);
    assert_eq!(clash.paths, vec![PathBuf::from("notes.txt")]);
    assert_eq!(stash_commits(&repo), vec![stash], "S kept");
    assert_eq!(
        repo.git(&["diff", "--name-only", "--diff-filter=U"]),
        "notes.txt",
        "the clash is live"
    );
    hooked.assert_gone();

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("u puts it back: {why}"));
    assert_eq!(
        whole(&repo),
        before,
        "HEAD, index, tree and stash list as before"
    );
}

/// git killed before it could pause (here: the hook left git's group, so
/// the stop had no helper to end and stopped git itself). The state 13b saw:
/// the merge's files in the tree, no `MERGE_HEAD`. `Aborted`, and `u` puts
/// everything back, his edit included. His edit was set aside (19d): the
/// set-aside close keeps the reason the stop was read as (19d-a2 D2), puts
/// the edit back and drops `S`.
#[test]
fn a_merge_stopped_before_git_paused_it_is_aborted_and_u_puts_it_back() {
    let repo = diverged();
    let hooked = Hooked::stuck_outside_the_group(&repo, "pre-merge-commit");
    let before = state_of(&repo);
    let stashes_before = whole(&repo).1;

    let stopped = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    let Err(error) = stopped else {
        panic!("a merge git did not pause does not succeed");
    };
    assert_eq!(
        error.stopped_at(),
        Some(StoppedAt::BeforePause { changed: true })
    );
    let ApplyError::Git { entry, .. } = &error else {
        panic!("today's variant, not {error:?}");
    };
    assert_eq!(
        entry.status,
        EntryStatus::Aborted {
            reason: "stopped before git could pause it".to_owned()
        }
    );
    set_aside_of(entry);
    assert!(
        !repo.path().join(".git/MERGE_HEAD").exists(),
        "git did not get to pause it"
    );
    assert_ne!(state_of(&repo), before, "the merge's files are in the tree");
    assert_eq!(repo.read("notes.txt"), HIS_EDIT, "his edit is back");
    assert!(stash_commits(&repo).is_empty(), "S dropped");

    run_undo(&repo, NOW + 1, false).unwrap_or_else(|why| panic!("u puts it back: {why}"));

    assert_eq!(state_of(&repo), before, "HEAD, index and tree as before");
    assert_eq!(whole(&repo).1, stashes_before, "the stash list as before");
    assert_eq!(repo.read("notes.txt"), HIS_EDIT, "his edit, byte for byte");
}

/// Paused, then `a`: everything as before the merge, his edit included.
#[test]
fn a_paused_merge_aborted_by_resume_gives_his_edits_back() {
    let repo = diverged();
    let hooked = Hooked::stuck(&repo, "pre-merge-commit");
    let before = state_of(&repo);
    let Err(ApplyError::Paused { entry, .. }) = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    ) else {
        panic!("paused");
    };
    hooked.remove("pre-merge-commit");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 1, &Never);
    let aborted = resume(&mut ctx, Resume::Abort).unwrap_or_else(|why| panic!("{why}"));

    assert_eq!(
        aborted.entry.id, entry.id,
        "the paused entry is the one closed"
    );
    assert_eq!(
        aborted.entry.status,
        EntryStatus::Aborted {
            reason: "aborted".to_owned()
        }
    );
    assert_eq!(state_of(&repo), before);
    assert_eq!(repo.read("notes.txt"), HIS_EDIT);
}

/// Paused, then `c`: the merge git would have made in one step.
#[test]
fn a_paused_merge_continues_to_the_merge_git_would_have_made() {
    let repo = diverged();
    let twin = repo.copy();
    let hooked = Hooked::stuck(&repo, "pre-merge-commit");
    let Err(ApplyError::Paused { .. }) = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    ) else {
        panic!("paused");
    };
    hooked.remove("pre-merge-commit");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 1, &Never);
    let continued = resume(&mut ctx, Resume::Continue { solved: Vec::new() })
        .unwrap_or_else(|why| panic!("{why}"));
    try_run(&twin, &merge_feature(&twin), NOW).unwrap_or_else(|why| panic!("{why}"));

    assert_eq!(continued.entry.status, EntryStatus::Committed);
    assert_eq!(id(&repo, "HEAD^2"), id(&repo, "feature"));
    assert_eq!(
        repo.git(&["log", "-1", "--format=%B"]),
        twin.git(&["log", "-1", "--format=%B"]),
        "the message a one-step merge writes"
    );
    assert_eq!(repo.read("notes.txt"), HIS_EDIT);
}

/// Paused, then the app is gone: the next launch finds it live, not
/// interrupted.
#[test]
fn a_paused_merge_is_live_at_relaunch() {
    let repo = diverged();
    let hooked = Hooked::stuck(&repo, "pre-merge-commit");
    let Err(ApplyError::Paused { entry, .. }) = run_with_stop(
        &repo,
        &merge_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    ) else {
        panic!("paused");
    };

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let mut ctx = context(&handle, &mut journal, NOW + 1, &Never);
    let recovered = recover_interrupted(&mut ctx).unwrap_or_else(|why| panic!("{why}"));

    assert_eq!(recovered.live, vec![entry.id.clone()]);
    assert!(recovered.closed.is_empty(), "nothing closed as interrupted");
    assert_eq!(entry_now(&repo, &entry.id).status, EntryStatus::Pending);
}

/// The same rule for a cherry-pick. Observed (git 2.x, step 0): of the
/// commit hooks, `git cherry-pick` runs `prepare-commit-msg` and
/// `post-commit` only, and a `prepare-commit-msg` killed by a signal leaves
/// `CHERRY_PICK_HEAD` with `HEAD` unmoved — so it is paused.
#[test]
fn a_cherry_pick_stopped_in_its_hook_is_paused_if_git_pauses_it() {
    let repo = diverged();
    // A clean tree: a cherry-pick sets dirty edits aside, which is not what
    // this is about.
    repo.git(&["checkout", "--", "notes.txt"]);
    let hooked = Hooked::stuck(&repo, "prepare-commit-msg");
    let pick = Operation::CherryPick(PickParams {
        commits: vec![id(&repo, "feature")],
        subject: "feature work".to_owned(),
        head: Tip {
            name: "refs/heads/main".to_owned(),
            oid: id(&repo, "main"),
        },
    });
    let head = id(&repo, "HEAD");

    let stopped = run_with_stop(&repo, &pick, NOW, &StopOnceRunning(hooked.running()));

    let Err(ApplyError::Paused { entry, kind, .. }) = stopped else {
        panic!("paused: {stopped:?}");
    };
    assert_eq!(kind, InProgress::CherryPick);
    assert_eq!(entry.status, EntryStatus::Pending);
    assert!(repo.path().join(".git/CHERRY_PICK_HEAD").exists());
    assert_eq!(id(&repo, "HEAD"), head);
    hooked.assert_gone();
}

/// A clean `diverged` repository: a pick or a revert sets dirty edits aside,
/// which is not what these are about.
fn diverged_clean() -> TempRepo {
    let repo = diverged();
    repo.git(&["checkout", "--", "notes.txt"]);
    repo
}

fn pick_feature(repo: &TempRepo) -> Operation {
    Operation::CherryPick(PickParams {
        commits: vec![id(repo, "feature")],
        subject: "feature work".to_owned(),
        head: Tip {
            name: "refs/heads/main".to_owned(),
            oid: id(repo, "main"),
        },
    })
}

/// A one-commit write stopped in its `post-commit` hook (13d Q2): git had
/// made the commit before the hook ran, so it landed — `Committed`, and the
/// answer says `Landed`. No in-progress state is left, and `HEAD` moved.
fn assert_landed(repo: &TempRepo, hooked: &Hooked, landed: Result<Applied, ApplyError>) {
    let Ok(Applied {
        entry, stopped_at, ..
    }) = landed
    else {
        panic!("a write committed before its post-commit hook landed: {landed:?}");
    };
    assert_eq!(stopped_at, Some(StoppedAt::Landed));
    assert_eq!(entry.status, EntryStatus::Committed);
    for state in ["CHERRY_PICK_HEAD", "REVERT_HEAD", "MERGE_HEAD"] {
        assert!(
            !repo.path().join(".git").join(state).exists(),
            "{state} gone"
        );
    }
    hooked.assert_gone();
}

#[test]
fn a_cherry_pick_stopped_in_its_post_commit_hook_has_landed() {
    let repo = diverged_clean();
    let hooked = Hooked::stuck(&repo, "post-commit");
    let head = id(&repo, "HEAD");

    let landed = run_with_stop(
        &repo,
        &pick_feature(&repo),
        NOW,
        &StopOnceRunning(hooked.running()),
    );

    assert_landed(&repo, &hooked, landed);
    assert_eq!(id(&repo, "HEAD^"), head, "the pick is on top of main");
    assert!(repo.exists("b.txt"), "the picked change is in the tree");
}

#[test]
fn a_revert_stopped_in_its_post_commit_hook_has_landed() {
    let repo = diverged_clean();
    let hooked = Hooked::stuck(&repo, "post-commit");
    let head = id(&repo, "HEAD");
    let revert = Operation::Revert(RevertParams {
        commit: head,
        subject: "main work".to_owned(),
        mainline: None,
        parents: 1,
        parent: Some(id(&repo, "HEAD^")),
        head: Tip {
            name: "refs/heads/main".to_owned(),
            oid: head,
        },
    });

    let landed = run_with_stop(&repo, &revert, NOW, &StopOnceRunning(hooked.running()));

    assert_landed(&repo, &hooked, landed);
    assert_eq!(id(&repo, "HEAD^"), head, "the revert is on top of main");
    assert!(!repo.exists("c.txt"), "main's change is reversed");
}

/// A pick paused by a stop, then `c`, and the commit's `post-commit` hook is
/// stuck and stopped: the `--continue` had committed, so it landed too.
#[test]
fn a_continue_stopped_in_its_post_commit_hook_has_landed() {
    let repo = diverged_clean();
    let head = id(&repo, "HEAD");
    let first = Hooked::stuck(&repo, "prepare-commit-msg");
    let Err(ApplyError::Paused { entry, .. }) = run_with_stop(
        &repo,
        &pick_feature(&repo),
        NOW,
        &StopOnceRunning(first.running()),
    ) else {
        panic!("paused");
    };
    first.remove("prepare-commit-msg");
    first.remove("hook");
    let again = Hooked::stuck(&repo, "post-commit");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let stop = StopOnceRunning(again.running());
    let mut ctx = context(&handle, &mut journal, NOW + 1, &stop);
    let continued = resume(&mut ctx, Resume::Continue { solved: Vec::new() });

    let Ok(Applied {
        entry: closed,
        stopped_at,
        ..
    }) = continued
    else {
        panic!("a continue committed before its post-commit hook landed: {continued:?}");
    };
    assert_eq!(stopped_at, Some(StoppedAt::Landed));
    assert_eq!(closed.id, entry.id, "the paused entry is the one closed");
    assert_eq!(closed.status, EntryStatus::Committed);
    assert_eq!(id(&repo, "HEAD^"), head, "the pick is on top of main");
    assert!(!repo.path().join(".git/CHERRY_PICK_HEAD").exists());
    again.assert_gone();
}

/// A merge begun in the shell (`git merge --no-commit`), finished from Graft
/// (`c` on the banner: `FinishMerge`), and its `pre-commit` hook is stuck and
/// stopped (13d Q4). git fails the commit and the merge is still paused
/// before `c`, as it was: `stopped_at()` says so. The entry has the shape a
/// `FinishMerge` whose commit fails takes (`finish_merge::
/// a_hook_refusal_leaves_the_merge_in_progress`): `Git`, closed `Aborted`,
/// not in force — never a `Pending` entry for a merge Graft did not begin.
#[test]
fn a_finish_merge_stopped_in_its_hook_leaves_the_merge_paused() {
    let repo = diverged();
    repo.git(&["merge", "--no-ff", "--no-commit", "feature"]);
    let source = id(&repo, "feature");
    let head = id(&repo, "HEAD");
    let hooked = Hooked::stuck(&repo, "pre-commit");

    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let stop = StopOnceRunning(hooked.running());
    let mut ctx = context(&handle, &mut journal, NOW, &stop);
    let finished = resume(&mut ctx, Resume::Continue { solved: Vec::new() });

    let Err(error) = finished else {
        panic!("a stopped finish does not commit");
    };
    assert_eq!(
        error.stopped_at(),
        Some(StoppedAt::Paused(InProgress::Merge))
    );
    let ApplyError::Git { entry, .. } = &error else {
        panic!("a failed finish's variant, not {error:?}");
    };
    assert_eq!(entry.kind, OperationKind::FinishMerge);
    assert_eq!(
        entry.status,
        EntryStatus::Aborted {
            reason: "git was stopped".to_owned()
        }
    );
    let entries = journal_for(&repo.open()).entries().to_vec();
    assert!(!in_force(&entries, &entry.id), "not in force");
    assert_eq!(
        repo.git(&["rev-parse", "MERGE_HEAD"]),
        source.to_string(),
        "the merge is still paused"
    );
    assert_eq!(id(&repo, "HEAD"), head, "nothing committed");
    assert!(
        locks_under(&repo.path().join(".git")).is_empty(),
        "no lock left"
    );
    assert_eq!(repo.read("notes.txt"), HIS_EDIT, "his edit is untouched");
    hooked.assert_gone();
}

mod resume;
