//! Ending a run: the terminal given back first, then the pool stopped and
//! the exit picked (m8/13b, 13g; the hang-up exemption m8/21b-4).

use std::io::{self, Write};

use ratatui::Terminal as Frames;
use ratatui::backend::CrosstermBackend;
use signal_hook::consts::SIGHUP;

use super::super::exit_wait::{Boxed, Ending, Pool};
use super::super::row_clear::RowClear;
use super::super::{Exit, input};
use crate::jobs::Shutdown;
use crate::terminal::TerminalSession;
use crate::trace::TerminalOut;

/// Show the cursor ratatui hid, then drop its `Terminal`.
///
/// ratatui's own `Drop` shows the cursor too, and `eprintln!`s when that
/// fails. After a hang-up the terminal and stderr are the same dead pty:
/// the show fails with `EIO`, `eprintln!` panics on its own failed write,
/// and the process exits 101 here — before the pool is stopped, so git was
/// left running (m8/13g, observed: `failed printing to stderr: Input/output
/// error (os error 5)` from ratatui-core 0.1.2 `terminal.rs:483`, dropped at
/// this call). Shown here instead, where the failure can be handled.
pub(super) fn put_away(mut terminal: Frames<RowClear<CrosstermBackend<TerminalOut>>>) {
    if terminal.show_cursor().is_err() {
        // Not reported here, and that is safe: the session's restore, which
        // `wind_down` runs next, shows the cursor again and its result is the
        // one the exit acts on. After a hang-up there is nobody to tell, and
        // the next launch shows where git was left.
        terminal.backend_mut().cursor_given_up();
    }
}

/// The run is over: give the terminal back, **then** stop the pool and close
/// the watchers (m8/13b review fix: nothing that can wait longer than
/// `jobs::SHUTDOWN_GRACE` runs before the restore), and pick the error the
/// process exits on.
pub(super) fn wind_down(
    session: &TerminalSession,
    pool: Option<Pool>,
    outcome: io::Result<Exit>,
) -> io::Result<Exit> {
    // Restore explicitly rather than leaving it to `Drop`, so a restore
    // failure can be returned instead of merely printed.
    let restored = session.restore();
    let ending = if matches!(outcome, Ok(Exit::Signal(_))) {
        Ending::Signalled
    } else {
        Ending::Exited
    };
    let boxed = match &outcome {
        Ok(Exit::StoppedHalfWay { job, what, after }) => Some(Boxed {
            job: *job,
            what,
            after: *after,
        }),
        _ => None,
    };
    let stopped = pool.map_or(Ok(Shutdown::CLEAN), |pool| {
        pool.stop(ending, boxed.as_ref())
    });
    let stalled_report = stopped.and_then(|shutdown| report_stalled_jobs(&shutdown));
    // `finish` already picks which of two possible errors the process exits
    // on; a failure to report stalled jobs is folded in afterwards so it
    // still turns into `ExitCode::FAILURE` (main.rs) instead of vanishing —
    // except after a hang-up, where stderr is the dead terminal: the process
    // dies of the `SIGHUP` either way, and the words have nowhere to go.
    finish(outcome, restored).and_then(|exit| match stalled_report {
        Err(nowhere_to_go) if excused_by_hangup(&exit, &nowhere_to_go) => Ok(exit),
        report => report.map(|()| exit),
    })
}

/// A hang-up excuses a failed restore, or a line it can't print, only when
/// the failure is the dead terminal's (m8/21b-4, rule 3): any other error
/// is still the exit's.
pub(super) fn excused_by_hangup(exit: &Exit, error: &io::Error) -> bool {
    matches!(exit, Exit::Signal(SIGHUP)) && input::hung_up(error)
}

/// The loop's outcome, unless putting the terminal back failed: a terminal
/// left in raw mode is never reported silently.
fn finish(outcome: io::Result<Exit>, restored: io::Result<()>) -> io::Result<Exit> {
    finish_to(&mut io::stderr(), outcome, restored)
}

/// [`finish`], writing the restore-failure detail to `stderr` instead of the
/// real one — the seam a failing writer is plugged into in tests.
pub(super) fn finish_to(
    stderr: &mut impl Write,
    outcome: io::Result<Exit>,
    restored: io::Result<()>,
) -> io::Result<Exit> {
    match (outcome, restored) {
        (Ok(exit), Ok(())) => Ok(exit),
        // The terminal hung up (m8/13g): there is nothing left to restore,
        // and nobody to tell that the restore was refused. The process dies
        // of the `SIGHUP` as a `kill -HUP` makes it die, never exit 1.
        (Ok(exit), Err(nothing_left_to_restore))
            if excused_by_hangup(&exit, &nothing_left_to_restore) =>
        {
            Ok(exit)
        }
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
fn report_stalled_jobs(shutdown: &Shutdown) -> io::Result<()> {
    report_stalled_jobs_to(&mut io::stderr(), shutdown)
}

/// [`report_stalled_jobs`], writing to `stderr` instead of the real one — the
/// seam a failing writer is plugged into in tests.
pub(super) fn report_stalled_jobs_to(
    stderr: &mut impl Write,
    shutdown: &Shutdown,
) -> io::Result<()> {
    let workers = shutdown.stalled;
    if workers == 0 {
        return Ok(());
    }

    writeln!(
        stderr,
        "graft: {workers} background job(s) did not stop in time and were abandoned"
    )
    .and_then(|()| stderr.flush())
}
