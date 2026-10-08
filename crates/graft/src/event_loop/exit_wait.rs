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
    /// `ending`: when the process is ending, however it ends, a write still
    /// running gets the waiting line first (item 8 of the 13b review fix;
    /// any exit since 13g, review m8-13cd Minor 6). Lines go to `stderr`;
    /// see [`Pool::stop_to`].
    pub(super) fn stop(self, ending: Ending, boxed: Option<&Boxed<'_>>) -> io::Result<Shutdown> {
        self.stop_to(&mut io::stderr(), ending, boxed)
    }

    /// [`Pool::stop`], writing to `stderr` instead of the real one — the
    /// seam a test reads.
    pub(super) fn stop_to(
        self,
        stderr: &mut impl Write,
        ending: Ending,
        boxed: Option<&Boxed<'_>>,
    ) -> io::Result<Shutdown> {
        let began = Instant::now();
        let signalled = ending == Ending::Signalled;
        // Counted from before the waiting line: a signal sent once the line
        // is read is the signal again, however soon it comes.
        let again = if signalled {
            self.signals.again_after_one()
        } else {
            self.signals.again()
        };
        if ending != Ending::NextPlace {
            say_waiting(stderr, &self.jobs.writes_running());
        }
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

/// How the run that is stopping its pool ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Ending {
    /// The loop ended on a signal, or on the terminal hanging up (13g): the
    /// process dies of it once the pool is down.
    Signalled,
    /// Any other end of the process: `q`, the stop box, a failure.
    Exited,
    /// Another place takes the screen next (`Exit::Open`, `Exit::Practise`,
    /// `Exit::LeaveTour`): nothing may be
    /// printed over it, so no waiting line.
    NextPlace,
}

/// One waiting line per write still running.
fn say_waiting(stderr: &mut impl Write, writes: &[String]) {
    for what in writes {
        match writeln!(stderr, "{}", waiting_line(what)).and_then(|()| stderr.flush()) {
            Ok(()) => {}
            // Safe to drop: after a hang-up (or with stderr closed) these
            // words have nowhere to go. Losing the line loses words, not the
            // wait or the stop, and the next launch shows where git was left.
            Err(_nowhere_to_go) => {}
        }
    }
}

/// `<bin>: waiting for <op> to finish (up to 60 s) · send the signal again to
/// stop it half-way` — the exit's word before its wait, on any exit that
/// leaves a write running (13g).
#[must_use]
pub fn waiting_line(what: &str) -> String {
    format!(
        "{BIN}: waiting for {} to finish (up to {} s) \u{b7} send the signal again to stop it half-way",
        sanitize_line(what).as_str(),
        EXIT_WRITE_WAIT.as_secs()
    )
}

#[cfg(test)]
mod tests {
    use std::io::{self, Write};
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    use signal_hook::iterator::Signals;

    use super::{Ending, Pool, waiting_line};
    use crate::jobs::{Dispatch, JobRunner, Outcome};
    use crate::repos::RepoStore;
    use crate::update::Command;

    /// What the write's line names.
    const MERGE: &str = "merge main into feature";

    /// A pool with one unstoppable write running, which lands on its own
    /// after `lands_after`, and watchers that watch nothing.
    fn pool_with_a_write(lands_after: Duration) -> Pool {
        let (sender, _wakes) = mpsc::channel();
        let store = Arc::new(RepoStore::new(env!("CARGO_MANIFEST_DIR").into(), None));
        let jobs = JobRunner::start(sender, 1, store).expect("a test machine starts a thread");
        let (started, has_started) = mpsc::channel();
        let id = jobs.submit(move |_cancel| {
            started.send(()).expect("the test is listening");
            std::thread::sleep(lands_after);
            Outcome::Done
        });
        jobs.dispatch(Command::Unstoppable(id, MERGE.to_owned()));
        has_started
            .recv_timeout(Duration::from_secs(5))
            .expect("the write starts");
        assert_eq!(jobs.writes_running(), vec![MERGE.to_owned()]);
        let signals = super::super::signals::unwatched().expect("a self-pipe");
        let resizes = Signals::new::<[i32; 0], i32>([])
            .expect("a self-pipe")
            .handle();
        Pool::new(jobs, signals, resizes, "u".to_owned())
    }

    /// Review m8-13cd Minor 6: the waiting line is the shell's word on any
    /// exit that leaves a write running, not only a signalled one.
    #[test]
    fn a_fatal_exit_with_a_write_running_prints_the_waiting_line() {
        let pool = pool_with_a_write(Duration::from_millis(100));
        let mut stderr = Vec::new();

        let shutdown = pool
            .stop_to(&mut stderr, Ending::Exited, None)
            .expect("the pool stops");

        assert_eq!(
            String::from_utf8(stderr).expect("utf-8"),
            format!("{}\n", waiting_line(MERGE))
        );
        assert!(shutdown.forced.is_empty(), "the write landed");
    }

    /// A stderr that is the dead terminal: every write fails with `EIO`.
    struct Closed;

    impl Write for Closed {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from_raw_os_error(5))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::from_raw_os_error(5))
        }
    }

    /// m8/13g: after a hang-up the waiting line has nowhere to go; the wait
    /// goes on, and the stop is not a failure.
    #[test]
    fn the_waiting_line_to_a_closed_stderr_is_not_fatal() {
        for ending in [Ending::Exited, Ending::Signalled] {
            let pool = pool_with_a_write(Duration::from_millis(50));

            let shutdown = pool
                .stop_to(&mut Closed, ending, None)
                .unwrap_or_else(|error| panic!("{ending:?}: {error}"));

            assert!(shutdown.forced.is_empty(), "{ending:?}: the write landed");
        }
    }
}
