//! How a stop ends a git child and its process group (m8-polish-and-ship/13c):
//! the default group stop and the hook-first stop. Moved out of `exec.rs` in
//! m8/13e; [`super`] decides *when* to stop, this is how.

use std::io;
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::RunError;

/// How long a child gets to clean up after `SIGTERM` before `SIGKILL`.
pub(super) const TERM_GRACE: Duration = Duration::from_secs(2);

/// How long a hook-first stop leaves git alone after ending its helpers,
/// for git to end the way it ends when a hook fails (it pauses a merge).
pub(super) const HOOKS_GRACE: Duration = Duration::from_secs(2);

/// How long a helper gets after its `SIGTERM` before `SIGKILL`.
pub(super) const HELPER_TERM_GRACE: Duration = Duration::from_secs(1);

/// How often a hook-first stop reads the group's members again.
pub(super) const LIST_EVERY: Duration = Duration::from_millis(250);

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
        StopOrder::Group => group_stop(child, members, graces),
        StopOrder::HooksFirst => hooks_first(child, members, graces),
    }
}

/// The default stop, used wherever the runner kills a child for any reason
/// but a hook-first stop: the output cap, a reader that panicked.
pub(super) fn kill(child: &mut Child) -> Result<(), RunError> {
    group_stop(child, &super::group::members, Graces::REAL).map(drop)
}

/// `killpg(SIGTERM)` then `killpg(SIGCONT)` — a member stopped by `SIGTTIN`
/// or `SIGSTOP` acts on the `SIGTERM` only once continued — then
/// `killpg(SIGKILL)` if git is still there after `graces.term`.
///
/// git ending of the `SIGTERM` does not end the stop while helpers are
/// still listed (a hook that ignores `SIGTERM`): they get [`after_git_ended`]
/// within the same `graces.term`, so a default stop still ends within
/// `POLL + TERM_GRACE` (m8/13e, review Minor 2).
pub(super) fn group_stop(
    child: &mut Child,
    members: Members<'_>,
    graces: Graces,
) -> Result<StopEnd, RunError> {
    if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
        return Ok(StopEnd {
            status,
            git_signalled: false,
        });
    }
    // git is alive and unreaped, so the group it leads is still its own.
    let group = pid_of(child)?;
    if signal_group(group, Signal::SIGTERM) {
        continue_group(group);
        let deadline = Instant::now() + graces.term;
        while Instant::now() < deadline {
            if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
                after_git_ended(&mut Helpers::of(group), members, graces, deadline);
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
/// - git is not signalled for `graces.hooks` **from the first helper's
///   `SIGTERM`**. Still running then, the whole group gets [`group_stop`].
/// - No helpers, or members that cannot be read: straight to [`group_stop`].
///   **Safe:** that is the stop every write had before 13c, and the caller
///   reads what git left on disk rather than trusting how it ended.
///
/// Why the grace starts at the `SIGTERM` and not at the call (m8/13k): git's
/// time to pause begins when its hook ends. Reading the group is one `ps` on
/// macOS, which took 1.9–2.3 s on a loaded machine — the whole grace. Counted
/// from the call, git was shot while it was still writing `MERGE_HEAD`, and
/// the merge he asked to leave paused was not paused.
///
/// After git ends on its own, helpers still listed get [`after_git_ended`],
/// bounded by `graces.hooks + graces.term` from the first `SIGTERM` — so the
/// whole stop stays within [`super::STOP_WITHIN`] plus that first read of
/// the group.
pub(super) fn hooks_first(
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
    let mut helpers = Helpers::of(pid_of(child)?);
    let start = Instant::now();
    let mut next_read = start;
    loop {
        if let Some(status) = child.try_wait().map_err(RunError::Spawn)? {
            let until = helpers.first_asked().unwrap_or(start) + graces.hooks + graces.term;
            after_git_ended(&mut helpers, members, graces, until);
            return Ok(own(status));
        }
        let now = Instant::now();
        if helpers
            .first_asked()
            .is_some_and(|asked| now >= asked + graces.hooks)
        {
            return group_stop(child, members, graces);
        }
        if now >= next_read {
            let read = helpers.pass(members, graces.helper_term);
            // From when the read came back: a slow `ps` must not make the
            // next one follow at once (m8/21b-3).
            next_read = Instant::now() + LIST_EVERY;
            match read {
                // Safe to fall back: see the doc comment's last bullet.
                Err(_unreadable) => return group_stop(child, members, graces),
                // Nothing but git in its group: no hook to end first.
                Ok(false) if helpers.termed.is_empty() => {
                    return group_stop(child, members, graces);
                }
                Ok(_) => {}
            }
        }
        thread::sleep(TERM_POLL);
    }
}

/// git has ended and been reaped; its helpers may not have (a hook that
/// ignores `SIGTERM`, one stopped by job control). The one helper path both
/// stops take after git ends:
///
/// - read the group at once, then at most every [`LIST_EVERY`];
/// - each helper gets `SIGTERM` + `SIGCONT`, and `SIGKILL` once still listed
///   `graces.helper_term` after that;
/// - none listed: done. Still listed at `until`: `SIGKILL` to each, so the
///   stop's bound holds;
/// - the group cannot be read: [`kill_reaped_group`] (review Minor 4).
pub(super) fn after_git_ended(
    helpers: &mut Helpers,
    members: Members<'_>,
    graces: Graces,
    until: Instant,
) {
    let mut next_read = Instant::now();
    loop {
        let now = Instant::now();
        let read = if now >= until {
            helpers.end_all(members).map(|()| false)
        } else if now >= next_read {
            let read = helpers.pass(members, graces.helper_term);
            // From when the read came back, as in `hooks_first`.
            next_read = Instant::now() + LIST_EVERY;
            read
        } else {
            Ok(true)
        };
        match read {
            Ok(true) => thread::sleep(TERM_POLL),
            Ok(false) => return,
            // Not dropped: the failed read is answered by ending the group.
            Err(_unreadable) => return kill_reaped_group(helpers.git),
        }
    }
}

/// The helpers a hook-first stop has signalled, and when.
pub(super) struct Helpers {
    git: Pid,
    termed: Vec<(Pid, Instant)>,
}

impl Helpers {
    pub(super) fn of(git: Pid) -> Self {
        Self {
            git,
            termed: Vec::new(),
        }
    }

    /// Every helper still listed gets `SIGKILL`.
    fn end_all(&self, members: Members<'_>) -> io::Result<()> {
        let git = self.git;
        for helper in members(git)?.into_iter().filter(|&pid| pid != git) {
            signal_one(helper, Signal::SIGKILL);
        }
        Ok(())
    }

    /// When the first helper was sent `SIGTERM`: where git's grace starts.
    fn first_asked(&self) -> Option<Instant> {
        self.termed.first().map(|&(_, asked)| asked)
    }

    /// One read of the group: `SIGTERM` + `SIGCONT` to each helper not yet
    /// asked, `SIGKILL` to each asked `grace` ago and still listed. `false`
    /// when no helper is listed.
    ///
    /// Each signal is timed from when the read came back, not from when it
    /// was asked for: a slow read must not count against a grace (m8/13k).
    fn pass(&mut self, members: Members<'_>, grace: Duration) -> io::Result<bool> {
        let git = self.git;
        let listed: Vec<Pid> = members(git)?
            .into_iter()
            .filter(|&pid| pid != git)
            .collect();
        let now = Instant::now();
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

/// `killpg(SIGCONT)`: a member stopped by job control (`SIGSTOP`, or
/// `SIGTTIN` from a hook reading the terminal) acts on its `SIGTERM` only
/// once continued (m8/13e, review Major 3).
///
/// **Safe to ignore a failure:** git is unreaped, so the group is its own;
/// a refused `SIGCONT` leaves a stopped git to the `SIGKILL` after the grace,
/// and a stopped helper to the per-helper `SIGCONT` once git has ended.
fn continue_group(group: Pid) {
    match nix::sys::signal::killpg(group, Signal::SIGCONT) {
        Ok(()) => {}
        // Safe to ignore: the doc comment above.
        Err(_refused) => {}
    }
}

/// `killpg(SIGKILL)` to the group of a git already reaped, whose members
/// could not be read (m8/13e, review Minor 4).
///
/// **Safe:** git's pid no longer reserves the group id, but a group id stays
/// reserved while any member lives — so while a helper is left this number
/// is still git's group, and once none is, the kill is `ESRCH` and reaches
/// no one. (A new process taking the freed number as its group within
/// milliseconds is review Minor 3, filed apart.)
fn kill_reaped_group(group: Pid) {
    match nix::sys::signal::killpg(group, Signal::SIGKILL) {
        // Sent; or the group is empty (`ESRCH`): nothing of git's is left
        // to end.
        Ok(()) | Err(Errno::ESRCH) => {}
        // Safe to ignore: `EPERM`, a member Graft may not signal (a setuid
        // helper). git is reaped and the caller reads what it left on disk;
        // there is no harder signal left to send.
        Err(_refused) => {}
    }
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
