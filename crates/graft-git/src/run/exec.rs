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
use std::process::{Child, ChildStderr, ChildStdout};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

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

/// Kills and reaps the child. A child that already exited is only reaped.
pub(super) fn kill(child: &mut Child) -> Result<(), RunError> {
    if let Err(error) = child.kill()
        && child.try_wait().map_err(RunError::Spawn)?.is_none()
    {
        return Err(RunError::Spawn(error));
    }
    child.wait().map(drop).map_err(RunError::Spawn)
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
