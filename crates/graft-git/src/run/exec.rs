//! The child process machinery [`super::GitInvocation::supervise`] and
//! [`super::GitInvocation::spawn`] are built on: draining stdout/stderr on
//! their own threads without deadlocking on either, killing and reaping the
//! child, and encoding a [`super::GitArg`] the two ways it is needed (as an
//! [`OsString`] to actually spawn, as shell-quoted text for
//! [`super::GitInvocation::command_line`]). Split out of `run.rs` (m2 review
//! M8) — nothing here decides *when* to kill or read; [`super`] does that,
//! this is just how.

use std::ffi::OsString;
use std::io::{self, Read};
use std::process::{Child, ChildStderr, ChildStdout, Stdio};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use super::{GitArg, RunError};

/// How much one read from a pipe takes at a time.
const READ_CHUNK: usize = 64 * 1024;

pub(super) enum Ending {
    Exited(std::process::ExitStatus, super::GitOutput),
    Capped(super::GitOutput),
}

pub(super) enum Pipe {
    Out(ChildStdout),
    Err(ChildStderr),
}

/// What a reader thread reports, once, when it stops reading.
pub(super) struct Event {
    stdout: bool,
    read: io::Result<(Vec<u8>, bool)>,
}

/// Reads one pipe to EOF or to `cap` on its own thread, then reports.
pub(super) fn drain(pipe: Pipe, cap: usize, events: Sender<Event>) {
    thread::spawn(move || {
        let (stdout, read) = match pipe {
            Pipe::Out(mut out) => (true, read_capped(&mut out, cap)),
            Pipe::Err(mut err) => (false, read_capped(&mut err, cap)),
        };
        if let Err(unwanted) = events.send(Event { stdout, read }) {
            // The receiver is gone only when `run` has already returned
            // (stopped or failed) and no longer wants this output.
            drop(unwanted);
        }
    });
}

/// Reads up to `cap` bytes; `true` alongside if there was more.
fn read_capped(pipe: &mut impl Read, cap: usize) -> io::Result<(Vec<u8>, bool)> {
    let mut kept = Vec::new();
    let mut chunk = vec![0; READ_CHUNK];
    loop {
        let n = match pipe.read(&mut chunk) {
            Ok(0) => return Ok((kept, false)),
            Ok(n) => n,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if kept.len() + n > cap {
            kept.extend_from_slice(&chunk[..cap - kept.len()]);
            return Ok((kept, true));
        }
        kept.extend_from_slice(&chunk[..n]);
    }
}

pub(super) fn apply(event: Event, output: &mut super::GitOutput) -> Result<(), RunError> {
    let (bytes, capped) = event.read.map_err(RunError::Spawn)?;
    if event.stdout {
        output.stdout = bytes;
    } else {
        output.stderr = bytes;
    }
    output.truncated |= capped;
    Ok(())
}

/// After a kill for the cap: gathers the other pipe, if it reports promptly.
/// A grandchild git started (an external diff tool) can hold a pipe open
/// past the kill; its output is then simply not collected.
pub(super) fn collect_rest(
    received: &Receiver<Event>,
    output: &mut super::GitOutput,
    mut open: usize,
) {
    while open > 0 {
        match received.recv_timeout(super::POLL) {
            Ok(event) => {
                open -= 1;
                if let Err(error) = apply(event, output) {
                    // The capped result is still good; the other pipe's read
                    // failing after the kill is a consequence of the kill.
                    drop(error);
                }
            }
            Err(_) => return,
        }
    }
}

/// How long a child gets to clean up after `SIGTERM` before `SIGKILL`.
const TERM_GRACE: Duration = Duration::from_secs(2);

/// How often the grace period looks to see whether the child has gone.
const TERM_POLL: Duration = Duration::from_millis(2);

/// Stops and reaps the child. A child that already exited is only reaped.
///
/// `SIGTERM` first, `SIGKILL` only if the grace runs out. The difference is
/// not politeness: git removes its lock files from a signal handler
/// (`SIGTERM`/`SIGINT`/`SIGHUP`/`SIGQUIT`) and from `atexit`, and neither
/// runs on `SIGKILL`. `Child::kill` is `SIGKILL`, so a cancelled `git
/// commit` could leave `.git/index.lock`, `.git/HEAD.lock` or
/// `.git/refs/heads/<branch>.lock` behind — after which every git command in
/// that repository, Graft's and every other tool's, fails until the user
/// deletes a file they have never heard of.
///
/// Measured on git 2.54 over a 20,000-file index, sweeping the kill instant
/// across the commit in 250 µs steps: **43 stranded locks in 267 `SIGKILL`s
/// that landed while git was running** — near every kill in the ~7 ms git
/// holds `index.lock`. The same sweep with `SIGTERM`: 6 in 288, and those
/// six are a race inside git itself (`create_tempfile` opens the lock file
/// before it marks the tempfile active, so a signal arriving in between
/// finds nothing to remove). That residue is exactly what a user's `Ctrl+C`
/// in a terminal already risks; it is not something a caller can close.
///
/// There is no `SIGTERM` in `std`, and `graft-git` has no `libc`, so the
/// signal goes through `kill(1)` — the same dependency-free route `graft`
/// already uses for `SIGTSTP`.
pub(super) fn kill(child: &mut Child) -> Result<(), RunError> {
    kill_within(child, TERM_GRACE)
}

/// [`kill`] with the grace period named, so a test need not wait two seconds.
pub(super) fn kill_within(child: &mut Child, grace: Duration) -> Result<(), RunError> {
    if child.try_wait().map_err(RunError::Spawn)?.is_some() {
        return child.wait().map(drop).map_err(RunError::Spawn);
    }
    // The child is alive and unreaped, so its pid is still reserved to it:
    // nothing else can have taken the number between the check and the
    // signal.
    if terminate(child.id()) {
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if child.try_wait().map_err(RunError::Spawn)?.is_some() {
                return child.wait().map(drop).map_err(RunError::Spawn);
            }
            thread::sleep(TERM_POLL);
        }
    }
    // Either `SIGTERM` could not be sent or the child sat through it. Both
    // mean the caller's stop has not been honoured yet, and the caller is
    // owed a reaped child more than git is owed its handler.
    if let Err(error) = child.kill()
        && child.try_wait().map_err(RunError::Spawn)?.is_none()
    {
        return Err(RunError::Spawn(error));
    }
    child.wait().map(drop).map_err(RunError::Spawn)
}

/// Sends `SIGTERM` to `pid`; `false` if it could not be delivered.
///
/// A failure here is not swallowed — it is the whole answer this returns,
/// and the one caller responds by falling through to `SIGKILL`. There is
/// nothing to report to the user: the child gets stopped either way, and
/// the only thing lost is git's chance to tidy up.
fn terminate(pid: u32) -> bool {
    std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub(super) fn os_arg(argument: &GitArg) -> OsString {
    match argument {
        GitArg::Flag(flag) => OsString::from(flag),
        GitArg::Count(flag, n) => OsString::from(format!("{flag}{n}")),
        GitArg::Oid(oid) => OsString::from(oid.to_string()),
        GitArg::Ref(bytes) | GitArg::Path(bytes) => os_bytes(bytes),
        GitArg::Separator => OsString::from("--"),
        GitArg::Word(word) => OsString::from(word),
    }
}

#[cfg(unix)]
fn os_bytes(bytes: &[u8]) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(bytes.to_vec())
}

#[cfg(not(unix))]
fn os_bytes(bytes: &[u8]) -> OsString {
    // Windows arguments are UTF-16; git for Windows reads paths as UTF-8.
    OsString::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Quotes `text` for a POSIX shell when it holds anything beyond the plainly
/// safe characters.
pub(super) fn shell_quote(text: &str) -> String {
    let plain = !text.is_empty()
        && text
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_./=:@%+,^~".contains(&b));
    if plain {
        text.to_owned()
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::*;

    /// A shell that traps `SIGTERM`, records that it arrived, and exits.
    fn trapping_child(marker: &std::path::Path, trap: &str) -> Child {
        Command::new("sh")
            .arg("-c")
            .arg(format!(
                "trap '{trap}' TERM; echo ready; while :; do sleep 0.02; done"
            ))
            .env("MARKER", marker)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("sh starts")
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

        kill_within(&mut child, Duration::from_secs(5)).expect("stopped and reaped");

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
        kill_within(&mut child, Duration::from_millis(100)).expect("stopped and reaped");

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

    #[test]
    fn a_child_that_already_exited_is_only_reaped() {
        let mut child = Command::new("true").spawn().expect("true starts");
        child.wait().expect("it exits");
        kill_within(&mut child, Duration::from_millis(0)).expect("reaping an exited child is fine");
    }
}
