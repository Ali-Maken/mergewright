//! The end of a repository's run, **after** the terminal is given back
//! (m8/13b review fix): stop the job pool, waiting a bounded time for a write
//! that cannot stop half-way, and say in the shell what happened.
//!
//! Why after: the pool may have to wait for a merge or rebase to land — up to
//! [`EXIT_WRITE_WAIT`] — and a terminal left in raw mode on the alternate
//! screen for that long looks frozen, and is lost for good if the wait is
//! killed. So the order on every exit is: graphics freed and the screen torn
//! down (the loop, `startup::run_repo`), the terminal restored
//! (`startup::wind_down`), then [`Pool::stop`].

use std::io::{self, Write};
use std::time::{Duration, Instant};

use graft_term::sanitize_line;

use super::signals::Watch;
use crate::jobs::{EXIT_WRITE_WAIT, JobRunner, SHUTDOWN_GRACE, Shutdown};

/// The binary's own name, as the man page and the shell know it.
const BIN: &str = env!("CARGO_PKG_NAME");

/// What a repository's run leaves running once its loop is over: the job
/// pool and the watchers, stopped by [`Pool::stop`] once the terminal is
/// back.
pub struct Pool {
    pub(super) jobs: JobRunner<std::sync::mpsc::Sender<super::Wake>>,
    pub(super) signals: Watch,
    pub(super) resizes: signal_hook::iterator::Handle,
}

impl Pool {
    /// The pool and both watchers, held for [`Pool::stop`].
    pub(super) const fn new(
        jobs: JobRunner<std::sync::mpsc::Sender<super::Wake>>,
        signals: Watch,
        resizes: signal_hook::iterator::Handle,
    ) -> Self {
        Self {
            jobs,
            signals,
            resizes,
        }
    }

    /// Stop the pool now: cancel what can stop, wait up to
    /// [`EXIT_WRITE_WAIT`] for a write that cannot — less if the signal
    /// comes again — then stop it half-way, and close the watchers.
    ///
    /// `signalled`: the loop ended on a signal, so a write still running gets
    /// the waiting line first (item 8 of the review fix). Lines go to
    /// `stderr`; see [`stop_to`].
    pub(super) fn stop(self, signalled: bool) -> io::Result<Shutdown> {
        self.stop_to(&mut io::stderr(), signalled)
    }

    /// [`Pool::stop`], writing to `stderr` instead of the real one — the
    /// seam a test reads.
    pub(super) fn stop_to(self, stderr: &mut impl Write, signalled: bool) -> io::Result<Shutdown> {
        let began = Instant::now();
        let writes = self.jobs.writes_running();
        if signalled {
            for what in &writes {
                // Best-effort: after a hang-up there is no terminal left to
                // read stderr, and the write fails. Losing the line loses
                // words, not the wait or the stop: the exit status still
                // carries the signal (`main` re-raises it).
                match writeln!(stderr, "{}", waiting_line(what)).and_then(|()| stderr.flush()) {
                    Ok(()) | Err(_) => {}
                }
            }
        }
        let again = self.signals.again();
        let shutdown = self.jobs.shutdown(SHUTDOWN_GRACE, EXIT_WRITE_WAIT, &again);
        // Let the signal threads end. The mux's reader is the process's, not
        // this run's (it is blocked waiting on the terminal until the user
        // types), so it is left to die with the process. It holds nothing
        // that matters.
        self.signals.close();
        self.resizes.close();
        for what in &shutdown.forced {
            let written = writeln!(stderr, "{}", stopped_line(what, began.elapsed()))
                .and_then(|()| stderr.flush());
            match written {
                Ok(()) => {}
                // As the waiting line: after a signal the status says it.
                Err(_) if signalled => {}
                // Otherwise the line is the only word that git was stopped
                // half-way: the run's outcome carries the failure.
                Err(error) => return Err(error),
            }
        }
        Ok(shutdown)
    }
}

/// `<bin>: stopped <op> half-way after <n> s · <bin> here shows where it
/// stopped` — the one line a stopped write leaves in the shell (keymap §2.2,
/// "a stuck write"). `<n>` is whole seconds.
#[must_use]
pub fn stopped_line(what: &str, after: Duration) -> String {
    format!(
        "{BIN}: stopped {} half-way after {} s \u{b7} {BIN} here shows where it stopped",
        sanitize_line(what).as_str(),
        after.as_secs()
    )
}

/// `<bin>: waiting for <op> to finish (up to 60 s) · send the signal again to
/// stop it half-way` — the signal path's word before its wait.
#[must_use]
pub fn waiting_line(what: &str) -> String {
    format!(
        "{BIN}: waiting for {} to finish (up to {} s) \u{b7} send the signal again to stop it half-way",
        sanitize_line(what).as_str(),
        EXIT_WRITE_WAIT.as_secs()
    )
}
