//! m8-polish-and-ship/13c: the process group, and how a stop ends git's
//! hook. Moved out of `run/tests.rs` (m8/13e).

use std::os::unix::process::CommandExt;
use std::time::Instant;

use nix::errno::Errno;
use nix::sys::signal::kill as signal;
use nix::unistd::{Pid, getpgid, getpgrp};

use super::*;
use crate::run::group;
use crate::run::stop::{self, Graces, StopOrder};

mod fixture;
mod signals;
use fixture::{
    Reap, assert_paused, gone, group_empty, merge, merging, stop_at_the_hook, stop_once_stopped,
};

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
    let (result, took, hook) = stop_at_the_hook(merge(&m), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(gone(hook.pid), "the hook is gone");
    assert!(group_empty(hook.group()), "the hook's sleep is gone too");
}

#[test]
fn hooks_first_lets_git_pause_a_merge() {
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let (result, took, hook) = stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(
        gone(hook.pid) && group_empty(hook.group()),
        "nothing of the group left"
    );
    assert_paused(&m);

    // git itself was never signalled: the stop `wait` calls reports
    // git's own exit, 1 ("Not committing merge").
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let mut child = merge(&m).spawn().expect("git starts");
    let _hook = Reap::hook(&m.pidfile);
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
    let (result, took, hook) = stop_at_the_hook(merge(&m).stop_hooks_first(), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(
        took >= HELPER_TERM_GRACE_FOR_TESTS,
        "the hook was given its grace first: {took:?}"
    );
    assert!(
        gone(hook.pid) && group_empty(hook.group()),
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
    let (result, took, hook) = stop_once_stopped(merge(&m).stop_hooks_first(), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(gone(hook.pid), "the stopped hook was continued and ended");
    assert_paused(&m);
}

#[test]
fn hooks_first_falls_back_to_the_group_stop_when_members_cannot_be_listed() {
    let m = merging("pre-merge-commit", r#"echo $$ > "$PIDFILE"; sleep 600"#);
    let mut child = merge(&m).spawn().expect("git starts");
    let hook = Reap::hook(&m.pidfile);
    let unreadable = |_: Pid| -> io::Result<Vec<Pid>> { Err(io::Error::other("injected")) };
    let end = stop::stop(&mut child, StopOrder::HooksFirst, &unreadable, Graces::REAL)
        .expect("stopped and reaped");
    assert!(end.git_signalled, "the group stop signalled git: {end:?}");
    assert!(
        gone(hook.pid) && group_empty(hook.group()),
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
    let job = Reap::hook(&m.pidfile);
    assert!(
        signal(job.pid, None).is_ok(),
        "the hook's background job was left running"
    );
}

/// m8/13e (review Major 3): the default stop — a rebase's, a remote op's —
/// continues a hook stopped by job control, so it acts on its `SIGTERM`.
#[test]
fn the_group_stop_continues_and_ends_a_hook_stopped_by_sigstop() {
    group_stop_ends_a_hook_that_stopped_itself("STOP");
}

/// The `/dev/tty`-reading hook 13c's decision 2 names: background reads
/// stop it with `SIGTTIN`.
#[test]
fn the_group_stop_continues_and_ends_a_hook_stopped_by_sigttin() {
    group_stop_ends_a_hook_that_stopped_itself("TTIN");
}

fn group_stop_ends_a_hook_that_stopped_itself(stop_signal: &str) {
    let m = merging(
        "pre-merge-commit",
        &format!(r#"echo $$ > "$PIDFILE"; kill -{stop_signal} $$; sleep 600"#),
    );
    // `stop_once_stopped` stops git only once `ps` reads `T` for the hook.
    let (result, took, hook) = stop_once_stopped(merge(&m), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(gone(hook.pid), "the stopped hook was continued and ended");
    assert!(group_empty(hook.group()), "nothing of the group left");
}

/// m8/13e (review Minor 2): git dies of the group's `SIGTERM`, the hook
/// does not; the default stop still ends it.
#[test]
fn the_group_stop_ends_a_hook_that_ignores_term() {
    let m = merging(
        "pre-merge-commit",
        r#"trap '' TERM; echo $$ > "$PIDFILE"; sleep 600"#,
    );
    let (result, took, hook) = stop_at_the_hook(merge(&m), &m.pidfile);
    assert!(matches!(result, Err(RunError::Stopped)), "{result:?}");
    assert!(took <= STOP_WITHIN, "stopped in {took:?}");
    assert!(gone(hook.pid), "the hook is gone");
    assert!(group_empty(hook.group()), "nothing of the group left");
}

/// m8/13e (review Minor 4): git ends on its own once its hook is gone, and
/// then the group cannot be read. The helper the hook left — one that
/// ignores `SIGTERM` — is still ended.
#[test]
fn hooks_first_ends_helpers_when_members_cannot_be_read_after_git_ended() {
    let m = merging(
        "pre-merge-commit",
        r#"(trap '' TERM; exec sleep 600) & echo $! > "$PIDFILE"; wait"#,
    );
    let mut child = merge(&m).spawn().expect("git starts");
    let git = Pid::from_raw(i32::try_from(child.id()).expect("a pid fits"));
    let helper = Reap::hook(&m.pidfile);
    // Readable while git is ours to reap; unreadable once the stop has
    // reaped it (`kill -0` on our own reaped child: no such process).
    let unreadable_after_git = |group: Pid| -> io::Result<Vec<Pid>> {
        match signal(git, None) {
            Err(Errno::ESRCH) => Err(io::Error::other("injected")),
            _ => group::members(group),
        }
    };
    let end = stop::stop(
        &mut child,
        StopOrder::HooksFirst,
        &unreadable_after_git,
        Graces::REAL,
    )
    .expect("stopped and reaped");
    assert!(!end.git_signalled, "git ended on its own: {end:?}");
    assert!(gone(helper.pid), "the helper that ignores TERM is gone");
    assert!(group_empty(helper.group()), "nothing of the group left");
}

/// m8/13e (review Minor 8): the guard every sleeping hook's test holds.
#[test]
fn the_hook_guard_kills_its_pid_on_drop() {
    let temp = tempfile::TempDir::with_prefix("graft-reap-").expect("a temp dir");
    let pidfile = temp.path().join("pid");
    let mut sleeper = std::process::Command::new("sleep")
        .arg("600")
        .process_group(0)
        .spawn()
        .expect("sleep starts");
    std::fs::write(&pidfile, sleeper.id().to_string()).expect("the pid writes");
    drop(Reap::hook(&pidfile));
    let deadline = Instant::now() + Duration::from_secs(2);
    let ended = loop {
        if let Some(status) = sleeper.try_wait().expect("try_wait") {
            break Some(status);
        }
        if Instant::now() >= deadline {
            break None;
        }
        thread::sleep(Duration::from_millis(5));
    };
    if ended.is_none() {
        // Our own child: end it before failing.
        sleeper.kill().expect("our own sleep can be ended");
        sleeper.wait().expect("and reaped");
    }
    let status = ended.expect("the guard ended the sleep");
    assert_eq!(
        std::os::unix::process::ExitStatusExt::signal(&status),
        Some(9),
        "SIGKILL ended it: {status:?}"
    );
}
