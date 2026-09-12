//! Leaving cleanly when the operating system says stop.
//!
//! A signal that kills Graft outright is a bug the user has to live with: the
//! process dies without unwinding, so nothing restores the terminal and the
//! shell is left in raw mode with no echo — measured, not assumed (ticket 03).
//!
//! The fix is to make a signal look exactly like the user pressing `q`. The
//! handler the kernel runs does one thing: write a byte into signal-hook's
//! self-pipe. A thread here reads that pipe and sends the loop a
//! [`Wake::Shutdown`]. The loop returns normally, the stack unwinds, and
//! `TerminalSession`'s `Drop` restores the terminal the way it always does.
//!
//! Two rules follow, and they are not style preferences — breaking either one
//! is how this turns back into a crash:
//!
//! - **Never restore the terminal from inside a handler.** A handler runs on
//!   whatever stack was interrupted and may only call async-signal-safe
//!   functions; taking the session's mutex there can deadlock against the
//!   thread that already holds it.
//! - **Never exit from inside a handler.** Exiting skips the unwind, which is
//!   the entire mechanism that puts the terminal back.
//!
//! The one thing the process must still do is *die like it was signalled*:
//! after the terminal is restored, `main` re-raises the signal with the
//! default disposition, so `$?` is the 128 + N a shell script expects and a
//! `SIGINT` still reads as an interrupt rather than as a clean exit.

use std::io;
use std::sync::mpsc::Sender;
use std::thread;

use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM};
use signal_hook::iterator::{Handle, Signals};

use super::Wake;

/// The signals Graft treats as "stop, cleanly".
///
/// `SIGINT` is `Ctrl+C` when the terminal is not in raw mode (during startup,
/// or if a child process leaves it that way), `SIGTERM` is a `kill` or a
/// shutdown, `SIGHUP` is the terminal window closing underneath us — the one
/// that most often leaves a broken shell behind, because there is no window
/// left to notice.
pub const WATCHED: [i32; 3] = [SIGINT, SIGTERM, SIGHUP];

/// Start watching, and wake `wakes` when one of [`WATCHED`] arrives.
///
/// The returned [`Handle`] stops the watcher: call [`Handle::close`] once the
/// loop is over so the thread can end.
pub fn watch(wakes: Sender<Wake>) -> io::Result<Handle> {
    // `Signals::new` installs signal-hook's handlers, which only write to a
    // self-pipe. The iterator below is ordinary code on an ordinary thread —
    // nothing in this file runs in a signal handler.
    let mut signals = Signals::new(WATCHED)?;
    let handle = signals.handle();

    let spawned = thread::Builder::new()
        .name("graft-signals".to_owned())
        .spawn(move || {
            for signal in &mut signals {
                if wakes.send(Wake::Shutdown(signal)).is_err() {
                    // The loop has already finished. Whatever the signal was
                    // for, the process is on its way out anyway.
                    return;
                }
            }
        });

    if let Err(error) = spawned {
        // Without the thread the handlers would be installed but unread, which
        // is worse than not installing them: signal-hook's handler would
        // swallow the signal and Graft would ignore a `kill` entirely.
        handle.close();
        return Err(error);
    }

    Ok(handle)
}

/// Die the way the signal meant Graft to die.
///
/// Call this only after the terminal has been restored: it puts the default
/// disposition back and re-raises, so the process really is killed by the
/// signal and reports the exit status a shell expects.
pub fn reraise(signal: i32) -> io::Result<()> {
    signal_hook::low_level::emulate_default_handler(signal)
}
