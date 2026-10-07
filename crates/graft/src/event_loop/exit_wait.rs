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
//!
//! Each write stopped on the way out leaves one line saying where git was
//! left (`update::quit::exit_line`, 13d): its own answer, waited for up to
//! `jobs::FORCED_WAIT`.

use std::io::{self, Write};
use std::time::{Duration, Instant};

use graft_core::job::JobId;
use graft_term::sanitize_line;

use super::signals::Watch;
use crate::jobs::{EXIT_WRITE_WAIT, JobRunner, SHUTDOWN_GRACE, Shutdown};
use crate::update::quit::{exit_line, not_started_line};

/// The binary's own name, as the man page and the shell know it.
const BIN: &str = env!("CARGO_PKG_NAME");

/// What a repository's run leaves running once its loop is over: the job
/// pool and the watchers, stopped by [`Pool::stop`] once the terminal is
/// back.
pub struct Pool {
    pub(super) jobs: JobRunner<std::sync::mpsc::Sender<super::Wake>>,
    pub(super) signals: Watch,
    pub(super) resizes: signal_hook::iterator::Handle,
    /// The process keymap's spelling of undo (ADR-0055: one keymap per
    /// process), for the exit line of a write git could not pause.
    pub(super) undo: String,
}

/// The write the stop box's `y` stopped (`Exit::StoppedHalfWay`): its line
/// is printed whether or not its answer came, counted from the loop's wait.
pub(super) struct Boxed<'a> {
    pub job: JobId,
    pub what: &'a str,
    /// How long Graft had waited on it when he pressed `y`.
    pub after: Duration,
}

impl Pool {
    /// The pool and both watchers, held for [`Pool::stop`].
    pub(super) const fn new(
        jobs: JobRunner<std::sync::mpsc::Sender<super::Wake>>,
        signals: Watch,
        resizes: signal_hook::iterator::Handle,
        undo: String,
    ) -> Self {
        Self {
            jobs,
            signals,
            resizes,
            undo,
        }
    }

    /// Stop the pool now: cancel what can stop, wait up to
    /// [`EXIT_WRITE_WAIT`] for a write that cannot — less if the signal
    /// comes again — then stop it half-way, and close the watchers. Each
    /// write stopped leaves its exit line: `boxed`, the stop box's, always.
    ///
    /// `signalled`: the loop ended on a signal, so a write still running gets
    /// the waiting line first (item 8 of the review fix). Lines go to
    /// `stderr`; see [`Pool::stop_to`].
    pub(super) fn stop(self, signalled: bool, boxed: Option<&Boxed<'_>>) -> io::Result<Shutdown> {
        self.stop_to(&mut io::stderr(), signalled, boxed)
    }

    /// [`Pool::stop`], writing to `stderr` instead of the real one — the
    /// seam a test reads.
    pub(super) fn stop_to(
        self,
        stderr: &mut impl Write,
        signalled: bool,
        boxed: Option<&Boxed<'_>>,
    ) -> io::Result<Shutdown> {
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
        if let Some(boxed) = boxed {
            let entry = shutdown
                .forced
                .iter()
                .find(|forced| forced.job == boxed.job);
            let line = match entry {
                Some(forced) if !forced.started => not_started_line(BIN, boxed.what),
                _ => exit_line(
                    BIN,
                    boxed.what,
                    boxed.after,
                    entry.and_then(|forced| forced.at),
                    &self.undo,
                ),
            };
            match writeln!(stderr, "{line}").and_then(|()| stderr.flush()) {
                // A failed write is ignored on purpose: he asked for this
                // stop in the box, with stderr gone there is nowhere left to
                // say it, and the next launch shows where git was left
                // anyway (as 13b's line in `main` did).
                Ok(()) | Err(_) => {}
            }
        }
        let stopped_after = began.elapsed();
        for forced in &shutdown.forced {
            if boxed.is_some_and(|boxed| boxed.job == forced.job) {
                continue;
            }
            let line = if forced.started {
                exit_line(BIN, &forced.what, stopped_after, forced.at, &self.undo)
            } else {
                not_started_line(BIN, &forced.what)
            };
            let written = writeln!(stderr, "{line}").and_then(|()| stderr.flush());
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
