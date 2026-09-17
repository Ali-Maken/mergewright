//! Sets up the terminal and runs the interactive loop, or hands off to
//! [`super::headless`] when stdout is not a terminal.

use std::io::{self, IsTerminal, Write};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use graft_core::capabilities::Supported;
use graft_term::Transport;
use graft_term::detect::{self, TtyHost};
use graft_term::input::InputMux;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use super::graphics::Graphics;
use super::headless::run_headless;
use super::redetect::Redetector;
use super::row_clear::RowClear;
use super::running::pump_with;
use super::{ChannelWakes, Exit, Wake, input, opening_repository, signals};
use crate::config::Diff;
use crate::crash;
use crate::jobs::{self, JobRunner, Shutdown};
use crate::repos::RepoStore;
use crate::session::{Session, Startup};
use crate::terminal::{SessionOptions, TerminalSession};
use crate::update::{AppState, Size};

/// Start the re-query worker (DESIGN.md §4.5), knowing what startup settled.
///
/// Only a terminal detection was allowed to ask may be asked again:
/// `TERM=dumb` gets no query bytes mid-session either, so it gets `None`.
fn start_redetector(
    environment: &detect::Environment,
    mux: &InputMux,
    settled: &Session,
    wakes: &Sender<Wake>,
) -> io::Result<Option<Redetector>> {
    if !environment.is_probeable() {
        return Ok(None);
    }
    Redetector::spawn(
        TtyHost::new(mux.waiter()),
        settled.capabilities.cell_size,
        settled.capabilities.theme,
        environment.probe_budget(),
        wakes.clone(),
    )
    .map(Some)
}

/// Run Graft until the user quits or a signal says stop.
///
/// The terminal is set up before the loop and restored after it, in that
/// order, whatever happens in between — including a panic, which the hook
/// handles first (`crate::crash`).
pub fn run(startup: Startup) -> io::Result<Exit> {
    // Taken apart here: `run` consumes the startup facts, and the two halves
    // are used at opposite ends of the function.
    let Startup {
        repo_path,
        config,
        harness,
    } = startup;
    // One repository, opened once and shared by every job that follows
    // (`crate::repos`). Opened *before* the terminal is touched: a path that
    // is not a repository is a plain line on a plain terminal, never an
    // alternate screen flashed for the length of the capability probe.
    // `graph.default_branch` is ADR-0001's rule 1, so it goes where the
    // branches are resolved.
    let repos = Arc::new(RepoStore::new(
        repo_path.clone(),
        config.config.graph.default_branch.clone(),
    ));
    if let Err(error) = repos.shared() {
        return Ok(Exit::Fatal(error.to_string()));
    }

    // A stdout that is not a tty — piped, redirected — never gets the
    // alternate screen or a single escape byte (ticket m1-the-graph/19): no
    // probe can tell the other end of a pipe apart from a terminal that
    // simply has not answered yet, so the only safe thing to draw there is
    // plain text (DESIGN.md §4.2 treats an unprobeable terminal as ASCII for
    // the same reason).
    if !io::stdout().is_terminal() {
        return run_headless(&repo_path, &config, repos);
    }

    // Kitty keyboard is deliberately off here and pushed below instead:
    // whether the terminal speaks the protocol is something only the probe
    // knows, and the probe needs the raw mode this session turns on.
    let session = TerminalSession::on_stdout(SessionOptions::default())?;
    crash::install(session.restorer());

    // The mux owns stdin for the process (ticket m1-the-graph/20); this run
    // gets its own `Inputs` from it, and a wrapper that runs again (the
    // gallery) gets the next one (ticket 35c). It starts before the probe,
    // so the probe's replies reach detection through a waiter while any key
    // typed meanwhile is queued, in order, for the loop.
    let (mux, inputs) = InputMux::on_stdin()?;
    let environment = detect::Environment::from_process();
    let capabilities = detect::detect(&environment, &mut TtyHost::new(mux.waiter()));
    if capabilities.kitty_keyboard == Supported::Yes {
        session.enable_kitty_keyboard()?;
    }
    let mut settled = Session::settle(&config, capabilities);

    let (mut wakes, sender) = ChannelWakes::new();
    let signals = signals::watch(sender.clone())?;
    let jobs = JobRunner::with_default_size(sender.clone(), repos)?;
    let redetector = start_redetector(&environment, &mux, &settled, &sender)?;
    let resizes = signals::watch_resizes(sender.clone(), redetector.clone())?;
    let forwarder = input::spawn_reader(inputs, sender, redetector)?;

    let (mut terminal, state) =
        terminal_and_state(&jobs, &repo_path, &settled, harness, config.config.diff)?;
    let mut graphics =
        Graphics::new(mux.waiter()).with_transport(Transport::for_tmux(&settled.capabilities.tmux));
    let outcome = pump_with(
        &mut terminal,
        &mut wakes,
        &jobs,
        &mut settled,
        state,
        &mut graphics,
    );
    let outcome = hand_back_input(&mux, forwarder, wakes, outcome);
    // Dropped before the restore so that its own teardown (it shows the cursor
    // it hid) goes out while the session is still in charge of the terminal.
    drop(terminal);

    // Stop the pool before the terminal goes back, so that a job still writing
    // its result cannot outlive the screen it was writing for.
    let shutdown = jobs.shutdown(jobs::SHUTDOWN_GRACE);

    // Let the signal threads end. The mux's reader is the process's, not
    // this run's (it is blocked waiting on the terminal until the user
    // types), so it is left to die with the process. It holds nothing that
    // matters.
    signals.close();
    resizes.close();

    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    let stalled_report = report_stalled_jobs(shutdown);
    // `finish` already picks which of two possible errors the process exits
    // on; a failure to report stalled jobs is folded in afterwards so it
    // still turns into `ExitCode::FAILURE` (main.rs) instead of vanishing.
    finish(outcome, restored).and_then(|exit| stalled_report.map(|()| exit))
}

/// This run is done reading (ticket m1-the-graph/35c). A key typed from here
/// on is kept for the next `on_stdin` (a wrapper's next run), and so is any
/// key already forwarded past the loop when it stopped: `]q` typed in one go
/// must step once and then quit, not step and lose the `q`. Joining the
/// forwarder first makes the hand-back complete — it ends once `hold` drops
/// its `Inputs` sender, after forwarding all it had.
fn hand_back_input(
    mux: &InputMux,
    forwarder: input::Forwarder,
    wakes: ChannelWakes,
    outcome: io::Result<Exit>,
) -> io::Result<Exit> {
    mux.hold();
    let outcome = outcome.and_then(|exit| forwarder.finish().map(|()| exit));
    mux.give_back(wakes.unhandled_input());
    outcome
}

/// The loop's outcome, unless putting the terminal back failed: a terminal
/// left in raw mode is never reported silently.
fn finish(outcome: io::Result<Exit>, restored: io::Result<()>) -> io::Result<Exit> {
    finish_to(&mut io::stderr(), outcome, restored)
}

/// [`finish`], writing the restore-failure detail to `stderr` instead of the
/// real one — the seam a failing writer is plugged into in tests.
fn finish_to(
    stderr: &mut impl Write,
    outcome: io::Result<Exit>,
    restored: io::Result<()>,
) -> io::Result<Exit> {
    match (outcome, restored) {
        (Ok(exit), Ok(())) => Ok(exit),
        (Ok(_), Err(restore_error)) => Err(restore_error),
        (Err(loop_error), Ok(())) => Err(loop_error),
        (Err(loop_error), Err(restore_error)) => {
            // Both went wrong. The loop's failure is the one worth returning,
            // but a terminal left in raw mode must never be silent.
            match writeln!(
                stderr,
                "graft: could not restore the terminal: {restore_error}"
            )
            .and_then(|()| stderr.flush())
            {
                // Both outcomes are handled the same way, deliberately:
                // `loop_error` is returned below either way, so the run
                // already exits non-zero. This write is only a best-effort
                // extra detail about `restore_error`; losing it when stderr
                // itself is gone loses detail, not the failure.
                Ok(()) | Err(_) => {}
            }
            Err(loop_error)
        }
    }
}

/// Say so when jobs had to be abandoned, now that the terminal is the user's
/// again.
///
/// Rare, and a bug when it happens: it means a job did not check its
/// cancellation token often enough. Printing it is the only way anyone finds
/// out, since by this point there is no screen left to put it on — so a
/// failure to print it is propagated rather than dropped: the caller folds it
/// into the run's outcome, and the exit status is the only signal left that
/// something was lost.
fn report_stalled_jobs(shutdown: Shutdown) -> io::Result<()> {
    report_stalled_jobs_to(&mut io::stderr(), shutdown)
}

/// [`report_stalled_jobs`], writing to `stderr` instead of the real one — the
/// seam a failing writer is plugged into in tests.
fn report_stalled_jobs_to(stderr: &mut impl Write, shutdown: Shutdown) -> io::Result<()> {
    let Shutdown::Stalled { workers } = shutdown else {
        return Ok(());
    };

    writeln!(
        stderr,
        "graft: {workers} background job(s) did not stop in time and were abandoned"
    )
    .and_then(|()| stderr.flush())
}

/// The terminal (resizes cleared row by row: a whole-screen clear frees Kitty
/// image data in Ghostty, and every resize would retransmit the viewport,
/// ADR-0012) and the first frame's `AppState`: `opening_repository`'s shell,
/// sized from the terminal's own report (nothing else knows it before the
/// first resize), plus the harness banner, the tier the probe settled on, and
/// `[diff]`'s startup values into the pane's live options — same fields the
/// `w`/`[`/`]`/`W` keys change afterwards, just seeded once instead of left
/// at `DetailState::default()`'s.
fn terminal_and_state(
    jobs: &impl jobs::Dispatch,
    repo_path: &std::path::Path,
    settled: &Session,
    harness: crate::update::Harness,
    diff: Diff,
) -> io::Result<(Terminal<RowClear<CrosstermBackend<io::Stdout>>>, AppState)> {
    let terminal = Terminal::new(RowClear(CrosstermBackend::new(io::stdout())))?;
    let size = terminal.size()?;
    let size = Size {
        columns: size.width,
        rows: size.height,
    };
    let mut state = opening_repository(jobs, repo_path, size);
    state.settled_tier = Some(settled.rendering.tier);
    state.harness = harness;
    state.detail.options = graft_git::diff::DiffOptions {
        context: diff.context,
        ignore_whitespace: diff.ignore_whitespace,
    };
    state.detail.word_tint = diff.word_tint;
    Ok((terminal, state))
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::{Exit, Shutdown, finish_to, report_stalled_jobs_to};

    /// A writer whose every write fails, standing in for a closed stderr.
    struct FailingWriter;

    impl io::Write for FailingWriter {
        fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
            Err(io::Error::other("stderr is gone"))
        }

        fn flush(&mut self) -> io::Result<()> {
            Err(io::Error::other("stderr is gone"))
        }
    }

    #[test]
    fn report_stalled_jobs_propagates_a_failing_writer() {
        let shutdown = Shutdown::Stalled { workers: 2 };

        let result = report_stalled_jobs_to(&mut FailingWriter, shutdown);

        assert!(result.is_err(), "a failing writer must not be swallowed");
    }

    #[test]
    fn report_stalled_jobs_clean_shutdown_never_writes() {
        // `FailingWriter` would fail the call if `Clean` reached the write at
        // all, so this also proves the early return is taken.
        let result = report_stalled_jobs_to(&mut FailingWriter, Shutdown::Clean);

        assert!(result.is_ok());
    }

    #[test]
    fn finish_with_failing_writer_still_returns_the_loop_error() {
        let loop_error = io::Error::other("loop failed");
        let restore_error = io::Error::other("restore failed");

        // Both the run and the restore failed, and even the stderr note about
        // the restore failure fails to write: the loop's error must still be
        // the one returned, per the comment in `finish_to`.
        let result = finish_to(&mut FailingWriter, Err(loop_error), Err(restore_error));

        let err = result.expect_err("both inputs were errors");
        assert_eq!(err.to_string(), "loop failed");
    }

    #[test]
    fn finish_reports_restore_failure_when_loop_succeeded() {
        let restore_error = io::Error::other("restore failed");

        let result = finish_to(&mut io::sink(), Ok(Exit::Quit), Err(restore_error));

        let err = result.expect_err("restore failed");
        assert_eq!(err.to_string(), "restore failed");
    }
}
