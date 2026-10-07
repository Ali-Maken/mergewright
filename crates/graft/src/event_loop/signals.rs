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
//!
//! **A second signal** (m8/13b review fix): after the loop ends, the exit
//! may wait for a write that cannot stop half-way (up to
//! `jobs::EXIT_WRITE_WAIT`). The watcher keeps counting signals until it is
//! closed, so the same signal again is heard ([`Watch::again`]) and stops
//! that write at once.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::thread;

use graft_core::event::Event;
use signal_hook::consts::{SIGHUP, SIGINT, SIGTERM, SIGWINCH};
use signal_hook::iterator::{Handle, Signals};

use super::Wake;
use super::redetect::{Query, Redetector};
use crate::update::AppEvent;

/// The signals Graft treats as "stop, cleanly".
///
/// `SIGINT` is `Ctrl+C` when the terminal is not in raw mode (during startup,
/// or if a child process leaves it that way), `SIGTERM` is a `kill` or a
/// shutdown, `SIGHUP` is the terminal window closing underneath us — the one
/// that most often leaves a broken shell behind, because there is no window
/// left to notice.
pub const WATCHED: [i32; 3] = [SIGINT, SIGTERM, SIGHUP];

/// The signal watcher: stop it with [`Watch::close`]; ask it whether
/// another signal came with [`Watch::again`].
pub struct Watch {
    handle: Handle,
    /// Every watched signal that has arrived, the loop's included.
    seen: Arc<AtomicUsize>,
}

impl Watch {
    /// Let the watcher thread end.
    pub fn close(&self) {
        self.handle.close();
    }

    /// A [`graft_git::run::Stop`] that says stop once a watched signal
    /// arrives **after** this call: "the signal again".
    #[must_use]
    pub fn again(&self) -> Again {
        Again {
            seen: Arc::clone(&self.seen),
            at: self.seen.load(Ordering::Acquire),
        }
    }
}

/// See [`Watch::again`].
pub struct Again {
    seen: Arc<AtomicUsize>,
    at: usize,
}

impl graft_git::run::Stop for Again {
    fn should_stop(&self) -> bool {
        self.seen.load(Ordering::Acquire) > self.at
    }
}

/// Start watching, and wake `wakes` when one of [`WATCHED`] arrives.
///
/// The returned [`Watch`] stops the watcher: call [`Watch::close`] once the
/// exit is over so the thread can end.
pub fn watch(wakes: Sender<Wake>) -> io::Result<Watch> {
    // `Signals::new` installs signal-hook's handlers, which only write to a
    // self-pipe. The iterator below is ordinary code on an ordinary thread —
    // nothing in this file runs in a signal handler.
    let mut signals = Signals::new(WATCHED)?;
    let handle = signals.handle();
    let seen = Arc::new(AtomicUsize::new(0));
    let counted = Arc::clone(&seen);

    let spawned = thread::Builder::new()
        .name("graft-signals".to_owned())
        .spawn(move || {
            for signal in &mut signals {
                // Counted first: once the loop has gone, the count is how
                // the exit hears a second signal (`Watch::again`).
                counted.fetch_add(1, Ordering::AcqRel);
                if wakes.send(Wake::Shutdown(signal)).is_err() {
                    // The loop has already finished, so nobody reads the
                    // wake; the count above is what the exit reads. Keep
                    // counting until `close`.
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

    Ok(Watch { handle, seen })
}

/// Start watching for the window changing size, and wake `wakes` with the
/// new size each time it does.
///
/// Stdin belongs to `graft_term::input`'s mux, so nothing else notices
/// `SIGWINCH` for Graft. Call [`Handle::close`] once the loop is over.
pub fn watch_resizes(wakes: Sender<Wake>, redetector: Option<Redetector>) -> io::Result<Handle> {
    let mut signals = Signals::new([SIGWINCH])?;
    let handle = signals.handle();

    let spawned = thread::Builder::new()
        .name("graft-resize".to_owned())
        .spawn(move || {
            for _ in &mut signals {
                let wake = match crossterm::terminal::size() {
                    Ok((columns, rows)) => {
                        Wake::Event(AppEvent::Core(Event::Resize { columns, rows }))
                    }
                    // A terminal whose size cannot be read cannot be drawn
                    // on; the loop stops and says so.
                    Err(error) => Wake::InputFailed(error),
                };
                if wakes.send(wake).is_err() {
                    // The loop has already finished.
                    return;
                }
                // After the resize itself, so the frame re-lays out at once
                // and the cell size follows when the terminal answers.
                if let Some(redetector) = &redetector {
                    redetector.ask(Query::CellSize);
                }
            }
        });

    if let Err(error) = spawned {
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
