//! m8-polish-and-ship/13c: the process group, and how a stop ends git's
//! hook. Moved out of `run/tests.rs` (m8/13e).

use std::path::PathBuf;
use std::time::Instant;

use nix::sys::signal::kill as signal;
use nix::unistd::{Pid, getpgid, getpgrp};

use super::*;
use crate::run::group;
use crate::run::stop::{self, Graces, StopOrder};

/// `main` and `feature` each one commit past a shared base, his
/// unrelated `notes.txt` left dirty, and `.git/hooks/<hook>` running
/// `body` (which may use `$PIDFILE`).
struct Merging {
    repo: Fixture,
    source: String,
    pidfile: PathBuf,
    notes: Vec<u8>,
}

fn merging(hook: &str, body: &str) -> Merging {
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
fn merge(m: &Merging) -> GitInvocation {
    GitInvocation::in_directory(m.repo.root())
        .args([
            GitArg::Flag("merge"),
            GitArg::Flag("--no-ff"),
            GitArg::Flag("feature"),
        ])
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Graft Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Graft Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("PIDFILE", &m.pidfile)
}

/// The pid the hook wrote, once it has.
fn hook_pid(pidfile: &Path) -> Pid {
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

/// Runs `invocation` on a thread, stops it once the hook has written its
/// pid, and says what `run` returned, how long the stop took, and the
/// hook's pid and process group.
fn stop_at_the_hook(
    invocation: GitInvocation,
    pidfile: &Path,
) -> (Result<GitOutput, RunError>, Duration, Pid, Pid) {
    let stop = Arc::new(AtomicBool::new(false));
    let asked = Arc::clone(&stop);
    let running = thread::spawn(move || invocation.run(&*asked));
    let hook = hook_pid(pidfile);
    let group = getpgid(Some(hook)).expect("the hook is alive");
    let at = Instant::now();
    stop.store(true, Ordering::Relaxed);
    let result = running.join().expect("the run thread ends");
    (result, at.elapsed(), hook, group)
}

/// `kill -0` fails: the process is gone. Polled briefly, because a hook
/// orphaned by git's end is reaped by init a moment later, and a zombie
/// still answers `kill -0`.
fn gone(pid: Pid) -> bool {
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
fn group_empty(group: Pid) -> bool {
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

fn locks_under(dir: &Path) -> Vec<PathBuf> {
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

fn assert_paused(m: &Merging) {
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

#[test]
fn git_runs_in_its_own_process_group() {
    let repo = Fixture::with_two_commits();
    let invocation = GitInvocation::in_directory(repo.root())
        .args([GitArg::Flag("cat-file"), GitArg::Flag("--batch")])
        .stdin(Vec::new());
    let mut child = invocation.spawn().expect("git starts");
    // stdin stays open, so git waits on it and is certainly alive.
    let pid = Pid::from_raw(i32::try_from(child.id()).expect("a pid fits"));
    let group = getpgid(Some(pid));
    drop(child.stdin.take());
    child.wait().expect("git ends on EOF");
    let group = group.expect("git's group reads");
    assert_eq!(group, pid, "git leads its own group");
    assert_ne!(group, getpgrp(), "and it is not ours");
}

#[test]
fn a_stop_leaves_no_process_of_the_group() {
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let (result, took, hook, group) = stop_at_the_hook(merge(&m), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(gone(hook), "the hook is gone");
    assert!(group_empty(group), "the hook's sleep is gone too");
}

#[test]
fn hooks_first_lets_git_pause_a_merge() {
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let (result, took, hook, group) = stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(
        gone(hook) && group_empty(group),
        "nothing of the group left"
    );
    assert_paused(&m);

    // git itself was never signalled: the stop `wait` calls reports
    // git's own exit, 1 ("Not committing merge").
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let mut child = merge(&m).spawn().expect("git starts");
    hook_pid(&m.pidfile);
    let end = stop::stop(
        &mut child,
        StopOrder::HooksFirst,
        &group::members,
        Graces::REAL,
    )
    .expect("stopped and reaped");
    assert!(!end.git_signalled, "{end:?}");
    assert_eq!(end.status.code(), Some(1), "git's own exit: {end:?}");
    assert_paused(&m);
}

#[test]
fn hooks_first_kills_a_hook_that_ignores_term() {
    let m = merging(
        "pre-merge-commit",
        r#"trap '' TERM; echo $$ > "$PIDFILE"; sleep 600"#,
    );
    let (result, took, hook, group) = stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(
        took >= HELPER_TERM_GRACE_FOR_TESTS,
        "the hook was given its grace first: {took:?}"
    );
    assert!(
        gone(hook) && group_empty(group),
        "nothing of the group left"
    );
    assert_paused(&m);
}

const HELPER_TERM_GRACE_FOR_TESTS: Duration = stop::HELPER_TERM_GRACE;

#[test]
fn a_stopped_hook_is_continued_and_ended() {
    let m = merging(
        "pre-merge-commit",
        r#"echo $$ > "$PIDFILE"; kill -STOP $$; sleep 600"#,
    );
    let stop = Arc::new(AtomicBool::new(false));
    let asked = Arc::clone(&stop);
    let invocation = merge(&m).stop_hooks_first();
    let running = thread::spawn(move || invocation.run(&*asked));
    let hook = hook_pid(&m.pidfile);
    let deadline = Instant::now() + Duration::from_secs(10);
    while !is_stopped(hook) {
        assert!(Instant::now() < deadline, "the hook never stopped itself");
        thread::sleep(Duration::from_millis(5));
    }
    stop.store(true, Ordering::Relaxed);
    let at = Instant::now();
    let result = running.join().expect("the run thread ends");
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(at.elapsed() <= STOP_WITHIN, "stopped in {:?}", at.elapsed());
    assert!(gone(hook), "the stopped hook was continued and ended");
    assert_paused(&m);
}

/// `ps -o stat=` starts with `T`: stopped by a signal.
fn is_stopped(pid: Pid) -> bool {
    let stat = std::process::Command::new("/bin/ps")
        .args(["-o", "stat=", "-p", &pid.to_string()])
        .output()
        .expect("ps runs");
    String::from_utf8_lossy(&stat.stdout)
        .trim()
        .starts_with('T')
}

#[test]
fn hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed() {
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let mut child = merge(&m).spawn().expect("git starts");
    let hook = hook_pid(&m.pidfile);
    let group = getpgid(Some(hook)).expect("the hook is alive");
    let unreadable = |_: Pid| -> io::Result<Vec<Pid>> { Err(io::Error::other("injected")) };
    let end = stop::stop(&mut child, StopOrder::HooksFirst, &unreadable, Graces::REAL)
        .expect("stopped and reaped");
    assert!(end.git_signalled, "the group stop signalled git: {end:?}");
    assert!(
        gone(hook) && group_empty(group),
        "nothing of the group left"
    );
}

#[test]
fn an_unstopped_run_leaves_a_hooks_background_job_alone() {
    let m = merging(
        "post-merge",
        r#"sleep 600 </dev/null >/dev/null 2>&1 & echo $! > "$PIDFILE""#,
    );
    merge(&m).run(&()).expect("the merge lands");
    let job = hook_pid(&m.pidfile);
    let alive = signal(job, None).is_ok();
    // Our own test started it; end it before asserting.
    signal(job, nix::sys::signal::Signal::SIGKILL).expect("our own job can be ended");
    assert!(alive, "the hook's background job was left running");
}
