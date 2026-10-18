//! What a stop does about git's helpers — hooks, their children, filters —
//! once it reads git's group: the [`Helpers`] it has signalled, and the
//! path both stops take after git ends. Moved out of `run/stop.rs`
//! (m8/21b-3).

use std::io;
use std::thread;
use std::time::{Duration, Instant};

use nix::errno::Errno;
use nix::sys::signal::Signal;
use nix::unistd::Pid;

use super::{Graces, LIST_EVERY, Members, TERM_POLL};

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
pub(in crate::run) fn after_git_ended(
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
pub(in crate::run) struct Helpers {
    git: Pid,
    pub(super) termed: Vec<(Pid, Instant)>,
}

impl Helpers {
    pub(in crate::run) fn of(git: Pid) -> Self {
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
    pub(super) fn first_asked(&self) -> Option<Instant> {
        self.termed.first().map(|&(_, asked)| asked)
    }

    /// One read of the group: `SIGTERM` + `SIGCONT` to each helper not yet
    /// asked, `SIGKILL` to each asked `grace` ago and still listed. `false`
    /// when no helper is listed.
    ///
    /// Each signal is timed from when the read came back, not from when it
    /// was asked for: a slow read must not count against a grace (m8/13k).
    pub(super) fn pass(&mut self, members: Members<'_>, grace: Duration) -> io::Result<bool> {
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
        // Safe to ignore: `EPERM`, a member Mergewright may not signal (a setuid
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
