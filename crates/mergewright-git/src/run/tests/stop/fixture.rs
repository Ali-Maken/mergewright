//! The stop tests' fixture: a merge whose hook runs a script, the stop
//! fired at it, and the guard that cleans up after it (m8/13e: moved out of
//! `run/tests/stop.rs`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, kill as signal};
use nix::unistd::{Pid, getpgid, getpgrp};

use crate::fixture::Fixture;
use crate::run::{GitArg, GitInvocation, GitOutput, RunError, group};

/// `main` and `feature` each one commit past a shared base, his
/// unrelated `notes.txt` left dirty, and `.git/hooks/<hook>` running
/// `body` (which may use `$PIDFILE`).
pub(super) struct Merging {
    pub(super) repo: Fixture,
    pub(super) source: String,
    pub(super) pidfile: PathBuf,
    pub(super) notes: Vec<u8>,
}

pub(super) fn merging(hook: &str, body: &str) -> Merging {
    let repo = Fixture::empty_repo();
    let write = |name: &str, text: &str| {
        std::fs::write(repo.root().join(name), text).expect("a fixture file writes");
    };
    write("a.txt", "base\n");
    write("notes.txt", "notes\n");
    repo.git(&["add", "."]);
    repo.git(&["commit", "-m", "base"]);
    repo.git(&["checkout", "-b", "feature"]);
    write("b.txt", "feature\n");
    repo.git(&["add", "b.txt"]);
    repo.git(&["commit", "-m", "feature"]);
    repo.git(&["checkout", "main"]);
    write("c.txt", "main\n");
    repo.git(&["add", "c.txt"]);
    repo.git(&["commit", "-m", "main side"]);
    write("notes.txt", "notes\nhis edit\n");
    let script = repo.root().join(".git/hooks").join(hook);
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("the hook writes");
    let mut mode = std::fs::metadata(&script)
        .expect("hook metadata")
        .permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut mode, 0o755);
    std::fs::set_permissions(&script, mode).expect("the hook is executable");
    let source = repo.rev_parse("feature");
    let pidfile = repo.root().join(".git/hook-pid");
    let notes = std::fs::read(repo.root().join("notes.txt")).expect("notes reads");
    Merging {
        repo,
        source,
        pidfile,
        notes,
    }
}

/// `git merge --no-ff feature`, hermetic: no global or system config
/// (a user's `core.hooksPath` or `commit.gpgsign` would change the run).
pub(super) fn merge(m: &Merging) -> GitInvocation {
    GitInvocation::in_directory(m.repo.root())
        .args([
            GitArg::Flag("merge"),
            GitArg::Flag("--no-ff"),
            GitArg::Flag("feature"),
        ])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture Maker")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture Maker")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("PIDFILE", &m.pidfile)
}

/// The pid the hook wrote, once it has.
pub(super) fn hook_pid(pidfile: &Path) -> Pid {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Ok(text) = std::fs::read_to_string(pidfile)
            && let Ok(pid) = text.trim().parse::<i32>()
        {
            return Pid::from_raw(pid);
        }
        assert!(Instant::now() < deadline, "the hook never wrote its pid");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Test cleanup (m8/13e, review Minor 8): on drop, `SIGKILL` to the pid a
/// hook wrote and to every process still in its group, so an assert that
/// fails before the stop has ended them never leaves a `sleep 600` behind.
/// Hold one in every test whose hook sleeps.
pub(super) struct Reap {
    /// The pid the hook wrote.
    pub(super) pid: Pid,
    /// Its process group (git's), read while it was alive.
    pub(super) group: Option<Pid>,
}

impl Reap {
    /// Waits for the pid the hook writes to `pidfile`, and guards it.
    pub(super) fn hook(pidfile: &Path) -> Self {
        let pid = hook_pid(pidfile);
        Self {
            pid,
            group: getpgid(Some(pid)).ok(),
        }
    }

    /// The hook's group, which it must have had when it was guarded.
    pub(super) fn group(&self) -> Pid {
        self.group.expect("the hook was alive when it was guarded")
    }
}

impl Drop for Reap {
    fn drop(&mut self) {
        let mut pids = vec![self.pid];
        // Never our own group: that is the test runner.
        if let Some(group) = self.group.filter(|&group| group != getpgrp())
            && let Ok(members) = group::members(group)
        {
            // A failed read leaves only the pid itself to end: this is
            // cleanup after a test, with nothing to report to.
            pids.extend(members);
        }
        for pid in pids {
            // Safe to ignore: pids this test's own hook wrote or its group
            // holds, and the usual answer is "no such process" — the stop
            // already ended it. This only cleans up after a failure.
            if let Err(_already_gone) = signal(pid, Signal::SIGKILL) {}
        }
    }
}

/// Runs `invocation` on a thread, stops it once the hook has written its
/// pid, and says what `run` returned and how long the stop took, with the
/// hook guarded.
pub(super) fn stop_at_the_hook(
    invocation: GitInvocation,
    pidfile: &Path,
) -> (Result<GitOutput, RunError>, Duration, Reap) {
    stop_when(invocation, pidfile, |_| true)
}

/// [`stop_at_the_hook`], once the hook has stopped itself (`ps` reads `T`).
pub(super) fn stop_once_stopped(
    invocation: GitInvocation,
    pidfile: &Path,
) -> (Result<GitOutput, RunError>, Duration, Reap) {
    stop_when(invocation, pidfile, is_stopped)
}

pub(super) fn stop_when(
    invocation: GitInvocation,
    pidfile: &Path,
    ready: fn(Pid) -> bool,
) -> (Result<GitOutput, RunError>, Duration, Reap) {
    let stop = Arc::new(AtomicBool::new(false));
    let asked = Arc::clone(&stop);
    let running = thread::spawn(move || invocation.run(&*asked));
    let hook = Reap::hook(pidfile);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready(hook.pid) {
        assert!(Instant::now() < deadline, "the hook never got ready");
        thread::sleep(Duration::from_millis(5));
    }
    let at = Instant::now();
    stop.store(true, Ordering::Relaxed);
    let result = running.join().expect("the run thread ends");
    (result, at.elapsed(), hook)
}

/// `kill -0` fails: the process is gone. Polled briefly, because a hook
/// orphaned by git's end is reaped by init a moment later, and a zombie
/// still answers `kill -0`.
pub(super) fn gone(pid: Pid) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    while signal(pid, None).is_ok() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
    true
}

/// No member of `group` is left (polled, as [`gone`]).
pub(super) fn group_empty(group: Pid) -> bool {
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let left = group::members(group).expect("the table reads");
        if left.is_empty() {
            return true;
        }
        if Instant::now() >= deadline {
            eprintln!("left in group {group}: {left:?}");
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

pub(super) fn locks_under(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for entry in std::fs::read_dir(dir).expect("a readable directory") {
        let path = entry.expect("an entry").path();
        if path.is_dir() {
            found.extend(locks_under(&path));
        } else if path.extension().is_some_and(|ext| ext == "lock") {
            found.push(path);
        }
    }
    found
}

pub(super) fn assert_paused(m: &Merging) {
    let git_dir = m.repo.root().join(".git");
    let merge_head = std::fs::read_to_string(git_dir.join("MERGE_HEAD"))
        .expect("git paused the merge: MERGE_HEAD is there");
    assert_eq!(merge_head.trim(), m.source, "MERGE_HEAD is the source tip");
    assert!(git_dir.join("MERGE_MSG").exists(), "MERGE_MSG is there");
    assert_eq!(locks_under(&git_dir), Vec::<PathBuf>::new(), "no lock left");
    assert_eq!(
        std::fs::read(m.repo.root().join("notes.txt")).expect("notes reads"),
        m.notes,
        "his dirty file is byte-equal"
    );
}

/// `ps -o stat=` starts with `T`: stopped by a signal.
pub(super) fn is_stopped(pid: Pid) -> bool {
    let stat = std::process::Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("ps runs");
    String::from_utf8_lossy(&stat.stdout)
        .trim()
        .starts_with('T')
}
