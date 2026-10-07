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
use std::process::{Child, ChildStderr, ChildStdout, ExitStatus};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::{GitArg, RunError, StderrSink};

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
    /// `read`'s `Err`, if any, is the stderr sink refusing to run because an
    /// earlier clone's run already poisoned it — not a pipe I/O failure.
    /// [`apply`] uses this to report [`super::RunError::Sink`] rather than
    /// [`super::RunError::Spawn`] (`KNOWN_GAPS` row 455: the two used to be
    /// the same variant, which reads as "git could not be started" for what
    /// is really an app-side callback bug).
    poisoned_sink: bool,
}

/// Reads one pipe to EOF or to `cap` on its own thread, then reports. With a
/// `sink`, stderr is also handed over segment by segment as it arrives (see
/// [`Segments`]); the sink runs on this thread.
pub(super) fn drain(pipe: Pipe, cap: usize, events: Sender<Event>, sink: Option<StderrSink>) {
    thread::spawn(move || {
        let (stdout, read, poisoned_sink) = match (pipe, sink) {
            (Pipe::Out(mut out), _) => (true, read_capped(&mut out, cap, &mut |_| {}), false),
            (Pipe::Err(mut err), None) => (false, read_capped(&mut err, cap, &mut |_| {}), false),
            (Pipe::Err(mut err), Some(sink)) => {
                let mut poisoned = false;
                let read = read_segmented(&mut err, cap, &sink, &mut poisoned);
                (false, read, poisoned)
            }
        };
        if let Err(unwanted) = events.send(Event {
            stdout,
            read,
            poisoned_sink,
        }) {
            // The receiver is gone only when `run` has already returned
            // (stopped or failed) and no longer wants this output.
            drop(unwanted);
        }
    });
}

/// [`read_capped`], feeding every kept byte through [`Segments`] to `sink`.
/// Sets `*poisoned` before returning `Err` when the failure is the sink
/// mutex itself being poisoned, so the caller can tell that apart from a
/// real pipe read error.
fn read_segmented(
    pipe: &mut impl Read,
    cap: usize,
    sink: &StderrSink,
    poisoned: &mut bool,
) -> io::Result<(Vec<u8>, bool)> {
    let mut sink = sink.0.lock().map_err(|_| {
        // A sink that panicked on an earlier run of this (cloned) invocation.
        // That panic already failed that run; this one says so rather than
        // calling a closure whose state is now unknown.
        *poisoned = true;
        io::Error::other("the stderr sink panicked on an earlier run")
    })?;
    let mut segments = Segments::default();
    let read = read_capped(pipe, cap, &mut |bytes| {
        segments.feed(bytes, &mut |segment| (*sink)(segment));
    });
    segments.finish(&mut |segment| (*sink)(segment));
    read
}

/// Reads up to `cap` bytes; `true` alongside if there was more. Every kept
/// chunk is shown to `on_chunk` as it arrives, in order.
fn read_capped(
    pipe: &mut impl Read,
    cap: usize,
    on_chunk: &mut dyn FnMut(&[u8]),
) -> io::Result<(Vec<u8>, bool)> {
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
            let room = cap - kept.len();
            on_chunk(&chunk[..room]);
            kept.extend_from_slice(&chunk[..room]);
            return Ok((kept, true));
        }
        on_chunk(&chunk[..n]);
        kept.extend_from_slice(&chunk[..n]);
    }
}

/// Splits a byte stream into the segments git writes between `\r` and `\n`
/// — a progress line redrawn in place is a run of `\r`-ended segments —
/// carrying a partial segment across reads. Empty segments (the `\n` after a
/// `\r`, say) are not emitted.
#[derive(Default)]
pub(super) struct Segments {
    partial: Vec<u8>,
}

impl Segments {
    /// Emits every segment `bytes` completes; keeps the unfinished tail.
    pub(super) fn feed(&mut self, bytes: &[u8], emit: &mut dyn FnMut(&[u8])) {
        let mut rest = bytes;
        while let Some(at) = rest.iter().position(|&b| b == b'\r' || b == b'\n') {
            if self.partial.is_empty() {
                if at > 0 {
                    emit(&rest[..at]);
                }
            } else {
                self.partial.extend_from_slice(&rest[..at]);
                emit(&self.partial);
                self.partial.clear();
            }
            rest = &rest[at + 1..];
        }
        self.partial.extend_from_slice(rest);
    }

    /// At end of stream: emits what is left, if anything.
    pub(super) fn finish(&mut self, emit: &mut dyn FnMut(&[u8])) {
        if !self.partial.is_empty() {
            emit(&self.partial);
            self.partial.clear();
        }
    }
}

pub(super) fn apply(event: Event, output: &mut super::GitOutput) -> Result<(), RunError> {
    let poisoned_sink = event.poisoned_sink;
    let (bytes, capped) = event.read.map_err(|error| {
        if poisoned_sink {
            RunError::Sink(error)
        } else {
            RunError::Spawn(error)
        }
    })?;
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
pub(super) const TERM_GRACE: Duration = Duration::from_secs(2);

/// How long a hook-first stop leaves git alone after ending its helpers,
/// for git to end the way it ends when a hook fails (it pauses a merge).
pub(super) const HOOKS_GRACE: Duration = Duration::from_secs(2);

/// How long a helper gets after its `SIGTERM` before `SIGKILL`.
pub(super) const HELPER_TERM_GRACE: Duration = Duration::from_secs(1);

/// How often a hook-first stop reads the group's members again.
const LIST_EVERY: Duration = Duration::from_millis(250);

/// How often the grace period looks to see whether the child has gone.
const TERM_POLL: Duration = Duration::from_millis(2);

/// How a stop ends a git child (m8-polish-and-ship/13c).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum StopOrder {
    /// The whole group at once: [`group_stop`].
    Group,
    /// git's helpers first, git left to end on its own: [`hooks_first`].
    HooksFirst,
}

/// The graces a stop waits through; named so a test can shorten them.
#[derive(Clone, Copy, Debug)]
pub(super) struct Graces {
    pub(super) hooks: Duration,
    pub(super) helper_term: Duration,
    pub(super) term: Duration,
}

impl Graces {
    pub(super) const REAL: Self = Self {
        hooks: HOOKS_GRACE,
        helper_term: HELPER_TERM_GRACE,
        term: TERM_GRACE,
    };
}

/// How a stopped child ended. Only tests read it: the caller of a stop
/// reads what git left on disk, not how git ended.
#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "read by the tests that prove git ended on its own"
    )
)]
pub(super) struct StopEnd {
    /// git's exit status, reaped.
    pub(super) status: ExitStatus,
    /// Whether the stop signalled git itself. `false` when git ended on its
    /// own: before the stop, or after a hook-first stop ended its helpers.
    pub(super) git_signalled: bool,
}

/// Reads a process group's members; [`super::group::members`] outside tests.
pub(super) type Members<'a> = &'a dyn Fn(Pid) -> io::Result<Vec<Pid>>;

/// Stops and reaps the child, in `order`. A child that already exited is
/// only reaped.
///
/// Every child leads its own process group (`spawn` sets it), and a stop
/// signals the group, so a hook and whatever it started end with git. Group
/// signals go only **before** git is reaped: an unreaped leader, even a
/// zombie, keeps the group id reserved, so the number cannot have passed to
/// someone else's group.
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
/// Signals go through `nix` (`killpg`, `kill`). Before 13c this shelled out
/// to `kill(1)` for the `SIGTERM` and reached git's pid alone, so a hung
/// hook outlived the stop.
pub(super) fn stop(
    child: &mut Child,
    order: StopOrder,
    members: Members<'_>,
    graces: Graces,
) -> Result<StopEnd, RunError> {
    match order {
        StopOrder::Group => group_stop(child, graces.term),
        StopOrder::HooksFirst => hooks_first(child, members, graces),
    }
}

/// The default stop, used wherever the runner kills a child for any reason
/// but a hook-first stop: the output cap, a reader that panicked.
pub(super) fn kill(child: &mut Child) -> Result<(), RunError> {
    group_stop(child, TERM_GRACE).map(drop)
}

/// `killpg(SIGTERM)` then `killpg(SIGCONT)` — a member stopped by `SIGTTIN`
/// or `SIGSTOP` acts on the `SIGTERM` only once continued — then
/// `killpg(SIGKILL)` if git is still there after `grace`.
fn group_stop(child: &mut Child, grace: Duration) -> Result<StopEnd, RunError> {
    if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
        return Ok(StopEnd {
            status,
            git_signalled: false,
        });
    }
    // git is alive and unreaped, so the group it leads is still its own.
    let group = pid_of(child)?;
    if signal_group(group, Signal::SIGTERM) {
        let deadline = Instant::now() + grace;
        while Instant::now() < deadline {
            if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
                return Ok(StopEnd {
                    status,
                    git_signalled: true,
                });
            }
            thread::sleep(TERM_POLL);
        }
    }
    // Either `SIGTERM` could not be sent or git sat through it. Both mean
    // the caller's stop has not been honoured yet, and the caller is owed a
    // reaped child more than git is owed its handler. git is still unreaped
    // here (the last `try_wait` said so), so the group is still its own.
    if !signal_group(group, Signal::SIGKILL)
        && let Err(error) = child.kill()
        && child.try_wait().map_err(RunError::Spawn)?.is_none()
    {
        return Err(RunError::Spawn(error));
    }
    let status = child.wait().map_err(RunError::Spawn)?;
    Ok(StopEnd {
        status,
        git_signalled: true,
    })
}

/// End git's helpers — every member of its group but git: hooks, their
/// children, filters, gpg — and leave git to end the way it ends when a hook
/// fails. For a merge that is "Not committing merge": git writes
/// `MERGE_HEAD`, `MERGE_MSG` and `MERGE_MODE` and exits 1, so the merge is
/// paused, not lost (observed on git 2.54 for `pre-merge-commit`,
/// `prepare-commit-msg` and `commit-msg`; 13c NOTES).
///
/// - Each helper gets `SIGTERM` + `SIGCONT`; one that appears later gets the
///   same, the members read at most every [`LIST_EVERY`].
/// - A helper still listed `graces.helper_term` after its `SIGTERM` gets
///   `SIGKILL`.
/// - git is not signalled for `graces.hooks`. Still running then, the whole
///   group gets [`group_stop`].
/// - No helpers, or members that cannot be read: straight to [`group_stop`].
///   **Safe:** that is the stop every write had before 13c, and the caller
///   reads what git left on disk rather than trusting how it ended.
///
/// After git ends on its own, helpers still listed keep the same treatment
/// until none is left, bounded by `graces.hooks + graces.term` from the
/// start — so the whole stop stays within [`super::STOP_WITHIN`].
fn hooks_first(
    child: &mut Child,
    members: Members<'_>,
    graces: Graces,
) -> Result<StopEnd, RunError> {
    let own = |status| StopEnd {
        status,
        git_signalled: false,
    };
    if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
        return Ok(own(status));
    }
    let mut helpers = Helpers {
        git: pid_of(child)?,
        termed: Vec::new(),
    };
    let start = Instant::now();
    let mut ended: Option<ExitStatus> = None;
    let mut next_read = start;
    loop {
        if ended.is_none()
            && let Some(status) = child.try_wait().map_err(RunError::Spawn)?
        {
            ended = Some(status);
            // Read at once: no helper left means the stop is done now.
            next_read = Instant::now();
        }
        let now = Instant::now();
        match ended {
            None if now >= start + graces.hooks => return group_stop(child, graces.term),
            Some(status) if now >= start + graces.hooks + graces.term => return Ok(own(status)),
            _ => {}
        }
        if now >= next_read {
            next_read = now + LIST_EVERY;
            match (helpers.pass(members, now, graces.helper_term), ended) {
                // Safe to fall back: see the doc comment's last bullet.
                (Err(_unreadable), None) => return group_stop(child, graces.term),
                (Err(_) | Ok(false), Some(status)) => {
                    return Ok(own(status));
                }
                // Nothing but git in its group: no hook to end first.
                (Ok(false), None) if helpers.termed.is_empty() => {
                    return group_stop(child, graces.term);
                }
                (Ok(_), _) => {}
            }
        }
        thread::sleep(TERM_POLL);
    }
}

/// The helpers a hook-first stop has signalled, and when.
struct Helpers {
    git: Pid,
    termed: Vec<(Pid, Instant)>,
}

impl Helpers {
    /// One read of the group: `SIGTERM` + `SIGCONT` to each helper not yet
    /// asked, `SIGKILL` to each asked `grace` ago and still listed. `false`
    /// when no helper is listed.
    fn pass(&mut self, members: Members<'_>, now: Instant, grace: Duration) -> io::Result<bool> {
        let git = self.git;
        let listed: Vec<Pid> = members(git)?
            .into_iter()
            .filter(|&pid| pid != git)
            .collect();
        for &helper in &listed {
            match self.termed.iter().find(|(pid, _)| *pid == helper) {
                None => {
                    signal_one(helper, Signal::SIGTERM);
                    signal_one(helper, Signal::SIGCONT);
                    self.termed.push((helper, now));
                }
                Some(&(_, asked)) if now >= asked + grace => signal_one(helper, Signal::SIGKILL),
                Some(_) => {}
            }
        }
        Ok(!listed.is_empty())
    }
}

fn pid_of(child: &Child) -> Result<Pid, RunError> {
    i32::try_from(child.id())
        .map(Pid::from_raw)
        .map_err(|_| RunError::Spawn(io::Error::other("a child pid beyond pid_t")))
}

/// `killpg`; `false` if it could not be delivered.
///
/// A failure here is not swallowed — it is the whole answer this returns,
/// and the caller responds by falling through to the next, harder signal.
/// There is nothing to report to the user: the child gets stopped either
/// way, and the only thing lost is git's chance to tidy up.
fn signal_group(group: Pid, signal: Signal) -> bool {
    nix::sys::signal::killpg(group, signal).is_ok()
}

/// `kill` to one helper of git's group.
///
/// **Safe to ignore a failure:** the helper was listed in git's group a
/// moment ago, so `ESRCH` means it ended in between — what the signal was
/// for. Any other failure (`EPERM`: a setuid helper) leaves it to the group
/// stop that follows if git does not end.
fn signal_one(helper: Pid, signal: Signal) {
    match nix::sys::signal::kill(helper, signal) {
        Ok(()) => {}
        // Safe to ignore: the doc comment above.
        Err(_gone_or_refused) => {}
    }
}

pub(super) fn os_arg(argument: &GitArg) -> OsString {
    match argument {
        GitArg::Flag(flag) => OsString::from(flag),
        GitArg::Count(flag, n) => OsString::from(format!("{flag}{n}")),
        GitArg::Oid(oid) => OsString::from(oid.to_string()),
        GitArg::Ref(bytes) | GitArg::Path(bytes) => os_bytes(bytes),
        GitArg::Glued(flag, bytes) => {
            let mut glued = OsString::from(flag);
            glued.push(os_bytes(bytes));
            glued
        }
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
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    use super::*;

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

        group_stop(&mut child, Duration::from_secs(5)).expect("stopped and reaped");

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
        group_stop(&mut child, Duration::from_millis(100)).expect("stopped and reaped");

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

    /// A reader that hands out `bytes` in pieces cut at `cuts`, one per read.
    struct Scripted<'a> {
        pieces: std::collections::VecDeque<&'a [u8]>,
    }

    impl<'a> Scripted<'a> {
        fn new(bytes: &'a [u8], cuts: &[usize]) -> Self {
            let mut cuts: Vec<usize> = cuts.iter().map(|c| c % (bytes.len() + 1)).collect();
            cuts.sort_unstable();
            let mut pieces = std::collections::VecDeque::new();
            let mut from = 0;
            for cut in cuts.into_iter().chain([bytes.len()]) {
                pieces.push_back(&bytes[from..cut]);
                from = cut;
            }
            Self { pieces }
        }
    }

    impl Read for Scripted<'_> {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            // An empty piece would read as EOF; skip them.
            while let Some(piece) = self.pieces.pop_front() {
                if !piece.is_empty() {
                    buf[..piece.len()].copy_from_slice(piece);
                    return Ok(piece.len());
                }
            }
            Ok(0)
        }
    }

    /// Real stderr shape from `git fetch --progress` on git 2.54.0: `\n`-ended
    /// lines, `\r`-redrawn progress, `remote:` padding, no final separator.
    const FETCH_STDERR: &[u8] = b"remote: Enumerating objects: 90, done.        \n\
remote: Counting objects:   1% (1/90)        \rremote: Counting objects:   2% (2/90)        \r\
remote: Counting objects: 100% (90/90), done.        \n\
Receiving objects:  45% (41/90)\rReceiving objects: 100% (90/90), 7.10 KiB | 7.10 MiB/s, done.\n\
From /tmp/t/bare\n * branch            main       -> FETCH_HEAD";

    fn segments_of(reader: &mut impl Read) -> (Vec<Vec<u8>>, Vec<u8>) {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let into = std::sync::Arc::clone(&seen);
        let sink = StderrSink(std::sync::Arc::new(std::sync::Mutex::new(Box::new(
            move |segment: &[u8]| into.lock().expect("test lock").push(segment.to_vec()),
        ))));
        let (kept, capped) = read_segmented(reader, usize::MAX, &sink, &mut false).expect("reads");
        assert!(!capped);
        let segments = seen.lock().expect("test lock").clone();
        (segments, kept)
    }

    proptest::proptest! {
        #![proptest_config(proptest::prelude::ProptestConfig::with_cases(
            graft_fixtures::proptest_cases()
        ))]

        #[test]
        fn segments_reassemble_across_reads(
            cuts in proptest::collection::vec(0usize..FETCH_STDERR.len() + 1, 0..12)
        ) {
            let (whole, _) = segments_of(&mut Scripted::new(FETCH_STDERR, &[]));
            let (split, kept) = segments_of(&mut Scripted::new(FETCH_STDERR, &cuts));
            proptest::prop_assert_eq!(&split, &whole);
            proptest::prop_assert_eq!(kept.as_slice(), FETCH_STDERR);
            proptest::prop_assert_eq!(whole.len(), 8);
            proptest::prop_assert_eq!(
                whole.last().map(Vec::as_slice),
                Some(&b" * branch            main       -> FETCH_HEAD"[..]),
                "the unterminated tail is flushed at EOF"
            );
        }
    }

    #[test]
    fn a_capped_stream_shows_the_sink_only_the_kept_bytes() {
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let into = std::sync::Arc::clone(&seen);
        let sink = StderrSink(std::sync::Arc::new(std::sync::Mutex::new(Box::new(
            move |segment: &[u8]| into.lock().expect("test lock").push(segment.to_vec()),
        ))));
        let (kept, capped) = read_segmented(
            &mut Scripted::new(b"one\ntwo\nthree\n", &[]),
            6,
            &sink,
            &mut false,
        )
        .expect("reads");
        assert!(capped);
        assert_eq!(kept, b"one\ntw");
        assert_eq!(
            *seen.lock().expect("test lock"),
            vec![b"one".to_vec(), b"tw".to_vec()]
        );
    }

    #[test]
    fn a_child_that_already_exited_is_only_reaped() {
        let mut child = Command::new("true")
            .process_group(0)
            .spawn()
            .expect("true starts");
        child.wait().expect("it exits");
        let end = group_stop(&mut child, Duration::from_millis(0))
            .expect("reaping an exited child is fine");
        assert!(!end.git_signalled, "an exited child is never signalled");
    }
}
