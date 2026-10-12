//! A real kill mid-put-back (30c, review m7-30-06d Question 1): the process
//! running `apply` is killed (`SIGKILL`) between the two applies, or between an apply
//! and the drop. Nothing is lost, recovery says where the edits are, and `u`
//! puts everything back as it was before the switch, the stash list
//! included.
//!
//! The seam is a `git` first on the child's `PATH` (`git_shim.sh`): graft
//! spawns `git` through the inherited `PATH` (`graft-git` `run.rs`). The
//! child is this test binary running [`child_runs_the_switch`], so the
//! parent's own environment is never changed.

#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    reason = "a failing test should panic"
)]

use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};

use graft_core::journal::{EntryId, HeadRef};
use graft_git::repo::RepoHandle;
use graft_ops::EditsAt;
use graft_ops::apply::{apply, recover_interrupted};
use graft_ops::operation::{CheckoutParams, CheckoutTarget, Operation};

use crate::put_back_index::{staged_near_their_change, staged_then_edited_forked};
use crate::set_aside::{journal_entries, set_aside_of, stash_commits, whole};
use crate::support::{NOW, Never, TempRepo, context, journal_for, run_undo};

/// The shim, shared with graft's K4.
const SHIM: &str = include_str!("git_shim.sh");

const CHILD_REPO: &str = "MWR_KILL_CHILD_REPO";
const CHILD_TO: &str = "MWR_KILL_CHILD_TO";
const CHILD_OUT: &str = "MWR_KILL_CHILD_OUT";

/// Which call the shim stops, and how.
#[derive(Clone, Copy)]
pub(crate) enum Shim {
    /// SIGKILL the child on `stash apply` without `--index`.
    KillOnPlainApply,
    /// SIGKILL the child on `stash drop`.
    KillOnDrop,
    /// `stash drop` exits 1.
    FailDrop,
}

impl Shim {
    fn env(self) -> [(&'static str, &'static str); 2] {
        match self {
            Shim::KillOnPlainApply => [("MWR_SHIM_ON", "plain-apply"), ("MWR_SHIM_DO", "kill")],
            Shim::KillOnDrop => [("MWR_SHIM_ON", "drop"), ("MWR_SHIM_DO", "kill")],
            Shim::FailDrop => [("MWR_SHIM_ON", "drop"), ("MWR_SHIM_DO", "fail")],
        }
    }
}

/// The real git, found on this process's `PATH` as `Command` would.
fn real_git() -> PathBuf {
    let path = std::env::var_os("PATH").expect("PATH is set");
    std::env::split_paths(&path)
        .map(|dir| dir.join("git"))
        .find(|candidate| candidate.is_file())
        .expect("git is on PATH")
}

/// The switch to `to`, run in a child of this test binary with the shim
/// first on its `PATH`. Returns whether the child was killed, and the line
/// it wrote (`explain()` of the error, or `ok`) when it was not.
pub(crate) fn run_child(repo: &TempRepo, to: &str, shim: Shim) -> (bool, Option<String>) {
    let bin = repo.path().join(".git/shim-bin");
    std::fs::create_dir_all(&bin).expect("the shim dir is made");
    let script = bin.join("git");
    std::fs::write(&script, SHIM).expect("the shim writes");
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755))
        .expect("the shim is executable");
    let out = repo.path().join(".git/shim-out");
    let path = std::env::join_paths(std::iter::once(bin).chain(std::env::split_paths(
        &std::env::var_os("PATH").expect("PATH is set"),
    )))
    .expect("PATH joins");
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "set_aside_kill::child_runs_the_switch",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("PATH", path)
        .env("MWR_SHIM_REAL_GIT", real_git())
        .envs(shim.env())
        .env(CHILD_REPO, repo.path())
        .env(CHILD_TO, to)
        .env(CHILD_OUT, &out)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .expect("the child runs");
    let killed = status.signal() == Some(9);
    assert!(
        killed || status.success(),
        "the child failed on its own: {status:?}"
    );
    let line = std::fs::read_to_string(&out).ok();
    (killed, line)
}

/// The child's half: the switch to `$MWR_KILL_CHILD_TO` in
/// `$MWR_KILL_CHILD_REPO`, its `explain()` (or `ok`) written to
/// `$MWR_KILL_CHILD_OUT`. Does nothing when run on its own.
#[test]
#[ignore = "the child of the kill tests; runs only under run_child"]
fn child_runs_the_switch() {
    let (Some(repo), Some(to), Some(out)) = (
        std::env::var_os(CHILD_REPO),
        std::env::var(CHILD_TO).ok(),
        std::env::var_os(CHILD_OUT),
    ) else {
        return;
    };
    let handle = RepoHandle::open(Path::new(&repo)).expect("the repository opens");
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW, &never);
    let op = Operation::Checkout(CheckoutParams {
        to: CheckoutTarget::Branch(to),
        head_before: HeadRef::Branch("refs/heads/main".to_owned()),
        on_edits: graft_ops::operation::OnEdits::SetAside,
    });
    let line = match apply(&mut ctx, &op) {
        Ok(_) => "ok".to_owned(),
        Err(error) => error.explain(&crate::support::Letters),
    };
    std::fs::write(out, line).expect("the line writes");
}

/// Recovery, as a relaunch runs it: the entry recovery reports for the
/// set-aside, and the entry itself.
fn recover(repo: &TempRepo) -> Vec<(EntryId, EditsAt)> {
    let handle = repo.open();
    let mut journal = journal_for(&handle);
    let never = Never;
    let mut ctx = context(&handle, &mut journal, NOW + 1, &never);
    recover_interrupted(&mut ctx)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)))
        .set_aside
}

/// The one entry, interrupted, and its set-aside stash.
fn interrupted(repo: &TempRepo) -> (EntryId, graft_core::oid::Oid) {
    let entries = journal_entries(repo);
    let [entry] = entries.as_slice() else {
        panic!("one entry: {entries:?}");
    };
    assert!(entry.is_interrupted(), "{:?}", entry.status);
    (entry.id.clone(), set_aside_of(entry))
}

/// Tracked files equal `HEAD`: nothing staged or unstaged.
fn tracked_at_head(repo: &TempRepo) -> bool {
    repo.git(&["status", "--porcelain", "--untracked-files=no"])
        .is_empty()
}

/// K1. Killed between the two applies: `--index` refused and wrote
/// nothing, the plain apply never ran. The edits are only in `S`, and
/// recovery says so.
#[test]
fn killed_between_the_two_applies_loses_nothing() {
    let repo = staged_near_their_change();
    let before = whole(&repo);
    let (killed, _) = run_child(&repo, "refs/heads/other", Shim::KillOnPlainApply);
    assert!(killed, "the shim killed the child");
    assert!(
        tracked_at_head(&repo),
        "{}",
        repo.git(&["status", "--porcelain"])
    );
    let recovered = recover(&repo);
    let (id, stash) = interrupted(&repo);
    assert_eq!(recovered, [(id, EditsAt::Stash { n: 0 })]);
    assert_eq!(stash_commits(&repo), [stash]);
    assert!(
        repo.git(&["show", &format!("{stash}:f.txt")])
            .contains("\nfive\n"),
        "S holds the edit"
    );
    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "undo exact, stash list included");
}

/// K2. Killed before the drop, the `--index` apply landed whole: the split
/// is back, `S` is a copy, and recovery says so.
#[test]
fn killed_before_the_drop_loses_nothing() {
    let repo = staged_then_edited_forked();
    let before = whole(&repo);
    let (killed, _) = run_child(&repo, "refs/heads/feature", Shim::KillOnDrop);
    assert!(killed, "the shim killed the child");
    let recovered = recover(&repo);
    let (id, _stash) = interrupted(&repo);
    assert_eq!(recovered, [(id, EditsAt::Back { copy: Some(0) })]);
    assert_eq!(repo.git(&["show", ":f.txt"]), "v2", "index v2");
    assert_eq!(repo.read("f.txt"), b"v3\n", "worktree v3");
    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "undo exact, stash list included");
}

/// K2b. Killed before the drop after the fallback: the edits are back
/// without their staging, `S` is a copy. `u` stages line 5 again.
#[test]
fn killed_before_the_drop_after_the_fallback_loses_nothing() {
    let repo = staged_near_their_change();
    let before = whole(&repo);
    let (killed, _) = run_child(&repo, "refs/heads/other", Shim::KillOnDrop);
    assert!(killed, "the shim killed the child");
    let recovered = recover(&repo);
    let (id, _stash) = interrupted(&repo);
    assert_eq!(recovered, [(id, EditsAt::Back { copy: Some(0) })]);
    assert_eq!(repo.git(&["status", "--porcelain"]), " M f.txt");
    run_undo(&repo, NOW + 2, false)
        .unwrap_or_else(|why| panic!("{}", why.explain(&crate::support::Letters)));
    assert_eq!(whole(&repo), before, "undo exact, stash list included");
}
