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

/// Our own `sleep 600`, killed and reaped when the test ends, pass or panic.
/// Until then it is unreaped, so its pid cannot pass to anyone else while a
/// stop is signalling it.
struct OwnSleep(std::process::Child);

impl OwnSleep {
    fn start() -> Self {
        let child = std::process::Command::new("sleep")
            .arg("600")
            .process_group(0)
            .spawn()
            .expect("sleep starts");
        Self(child)
    }

    fn pid(&self) -> Pid {
        Pid::from_raw(i32::try_from(self.0.id()).expect("a pid fits"))
    }
}

impl Drop for OwnSleep {
    fn drop(&mut self) {
        // Safe to ignore: the stop under test may already have ended it
        // (`kill` on an exited child is an error); the `wait` reaps it
        // either way, and a failed reap only leaves a zombie to the runner.
        if let Err(_already_ended) = self.0.kill() {}
        if let Err(_not_reaped) = self.0.wait() {}
    }
}

/// When each read of the group began and came back.
type Reads = std::cell::RefCell<Vec<(Instant, Instant)>>;

/// A read of the group that takes `slow`, listing `helper`, recorded.
fn slow_read(reads: &Reads, slow: Duration, helper: Pid) -> Vec<Pid> {
    let began = Instant::now();
    thread::sleep(slow);
    reads.borrow_mut().push((began, Instant::now()));
    vec![helper]
}

/// Every read starts at least [`stop::LIST_EVERY`] after the previous one
/// came back.
fn assert_spaced(reads: &[(Instant, Instant)], which: &str) {
    assert!(reads.len() >= 3, "{which}: {} reads", reads.len());
    for pair in reads.windows(2) {
        let gap = pair[1].0.saturating_duration_since(pair[0].1);
        assert!(
            gap >= stop::LIST_EVERY,
            "{which}: a read began {gap:?} after the last came back"
        );
    }
}

/// m8/21b-3 (review m1): the next read of the group is a full
/// [`stop::LIST_EVERY`] after the last one came back, not after it began.
/// Timed from the start, a read slower than the interval was followed by
/// another at once, so a slow `ps` ran back to back — in both loops that
/// read the group: `hooks_first`'s while git runs, `after_git_ended`'s after.
#[test]
fn the_next_read_waits_a_full_interval_after_a_slow_read() {
    let slow = Duration::from_millis(150);
    let helper = OwnSleep::start();

    // hooks_first, while git runs: three slow reads list the helper, then
    // the group cannot be read and the stop falls back to the group stop.
    // The hooks grace is long enough that only the reads end the loop.
    let mut git = std::process::Command::new("sh")
        .arg("-c")
        .arg("while :; do sleep 0.02; done")
        .process_group(0)
        .spawn()
        .expect("sh starts");
    let reads = Reads::default();
    let three_then_unreadable = |_: Pid| {
        if reads.borrow().len() < 3 {
            Ok(slow_read(&reads, slow, helper.pid()))
        } else {
            let now = Instant::now();
            reads.borrow_mut().push((now, now));
            Err(io::Error::other("injected"))
        }
    };
    let graces = Graces {
        hooks: Duration::from_secs(10),
        ..Graces::REAL
    };
    let end = stop::hooks_first(&mut git, &three_then_unreadable, graces).expect("stopped");
    assert!(end.git_signalled, "the fallback stopped git: {end:?}");
    // The first four are hooks_first's; a fifth is `after_git_ended`'s own
    // first read once the fallback has ended git.
    assert_spaced(&reads.borrow()[..4], "hooks_first");

    // after_git_ended: the helper stays listed until `until`; the read at
    // `until` is the bound, taken at once, so only the reads before it keep
    // the interval. The "git" pid is only compared, never signalled.
    let reads = Reads::default();
    let listed = |_: Pid| io::Result::Ok(slow_read(&reads, slow, helper.pid()));
    let not_signalled = OwnSleep::start();
    // Five reads fit before it nominally; the test needs three, so a loaded
    // machine stretching each read has room.
    let until = Instant::now() + Duration::from_millis(2000);
    stop::after_git_ended(
        &mut stop::Helpers::of(not_signalled.pid()),
        &listed,
        Graces::REAL,
        until,
    );
    let before_until: Vec<_> = reads
        .borrow()
        .iter()
        .copied()
        .filter(|&(began, _)| began < until)
        .collect();
    assert_spaced(&before_until, "after_git_ended");
}

/// m8/13e (review Minor 8): the guard every sleeping hook's test holds.
#[test]
fn the_hook_guard_kills_its_pid_on_drop() {
    let temp = tempfile::TempDir::with_prefix("mwr-reap-").expect("a temp dir");
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
