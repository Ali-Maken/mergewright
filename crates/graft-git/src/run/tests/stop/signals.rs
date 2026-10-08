//! The stop's own signal order, against plain `sh` children rather than git:
//! moved out of `run/stop.rs`'s inline tests (m8/21b-3).

use std::io::{self, Read};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::unistd::Pid;

use crate::run::stop::{Graces, group_stop, hooks_first};

/// A shell that traps `SIGTERM`, records that it arrived, and exits. It
/// leads its own process group, as every child `spawn` starts does.
fn trapping_child(marker: &std::path::Path, trap: &str) -> Child {
    Command::new("sh")
        .arg("-c")
        .arg(format!(
            "trap '{trap}' TERM; echo ready; while :; do sleep 0.02; done"
        ))
        .env("MARKER", marker)
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("sh starts")
}

/// A group with nothing in it but the child itself.
#[expect(
    clippy::unnecessary_wraps,
    reason = "the shape of `Members`, which can fail"
)]
fn no_helpers(_: Pid) -> io::Result<Vec<Pid>> {
    Ok(Vec::new())
}

/// The real graces with the `SIGTERM` grace set to `term`.
fn graces(term: Duration) -> Graces {
    Graces {
        term,
        ..Graces::REAL
    }
}

/// Waits for the child's first line, so the trap is certainly installed.
fn wait_until_ready(child: &mut Child) {
    let mut out = child.stdout.take().expect("stdout is piped");
    let mut byte = [0u8; 1];
    loop {
        let n = out.read(&mut byte).expect("the child writes a line");
        assert_ne!(n, 0, "the child exited before saying it was ready");
        if byte[0] == b'\n' {
            return;
        }
    }
}

/// The property the lock-file argument rests on: git is *asked* to stop,
/// so its own handler runs, rather than being shot where it stands.
#[test]
fn a_stopped_child_is_asked_first_so_its_cleanup_can_run() {
    let temp = tempfile::TempDir::with_prefix("graft-kill-").expect("a temp dir");
    let marker = temp.path().join("caught-term");
    let mut child = trapping_child(&marker, "touch \"$MARKER\"; exit 0");
    wait_until_ready(&mut child);

    group_stop(&mut child, &no_helpers, graces(Duration::from_secs(5)))
        .expect("stopped and reaped");

    assert!(
        marker.exists(),
        "the child's SIGTERM handler ran; SIGKILL would have left no marker"
    );
}

/// And a child that sits through `SIGTERM` is still stopped, so a cancel
/// the user asked for cannot hang on a hook that ignores the signal.
#[test]
fn a_child_that_ignores_the_ask_is_killed_anyway() {
    let temp = tempfile::TempDir::with_prefix("graft-kill-").expect("a temp dir");
    let marker = temp.path().join("never-written");
    let mut child = trapping_child(&marker, "");
    wait_until_ready(&mut child);

    let start = Instant::now();
    group_stop(&mut child, &no_helpers, graces(Duration::from_millis(100)))
        .expect("stopped and reaped");

    assert!(!marker.exists(), "the trap was empty, so nothing ran");
    assert!(
        start.elapsed() < Duration::from_secs(2),
        "the grace period bounded the wait, not the child"
    );
    assert!(
        child.try_wait().expect("try_wait").is_some(),
        "the child has been reaped"
    );
}

/// m8/13k: git's grace begins when its helpers are ended, not before the
/// group is read. On a loaded Mac one `ps` took 1.9–2.3 s, the whole
/// `HOOKS_GRACE`; git, still writing its pause when the read came back,
/// was then shot (`SIGTERM` to the group) and the merge was not paused.
///
/// The "git" here waits on its helper as git waits on a hook, then takes
/// 100 ms to end once the helper is gone, as git takes to write
/// `MERGE_HEAD` and exit 1. The read of the group takes longer than the
/// whole hooks grace.
#[test]
fn a_slow_read_of_the_group_does_not_spend_gits_grace() {
    let mut child = Command::new("sh")
        .arg("-c")
        .arg("sleep 600 & echo ready; wait; sleep 0.1; exit 1")
        .process_group(0)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("sh starts");
    wait_until_ready(&mut child);
    // The real graces: git's 100 ms exit has the same 1.9 s margin it has
    // in the app, so a loaded machine does not make this test flake.
    let graces = Graces::REAL;
    let slow = |group: Pid| {
        thread::sleep(graces.hooks + Duration::from_millis(100));
        crate::run::group::members(group)
    };

    let end = hooks_first(&mut child, &slow, graces).expect("stopped and reaped");

    assert!(
        !end.git_signalled,
        "git was left to end on its own: {end:?}"
    );
    assert_eq!(end.status.code(), Some(1), "{end:?}");
}

#[test]
fn a_child_that_already_exited_is_only_reaped() {
    let mut child = Command::new("true")
        .process_group(0)
        .spawn()
        .expect("true starts");
    child.wait().expect("it exits");
    let end = group_stop(&mut child, &no_helpers, graces(Duration::from_millis(0)))
        .expect("reaping an exited child is fine");
    assert!(!end.git_signalled, "an exited child is never signalled");
}
