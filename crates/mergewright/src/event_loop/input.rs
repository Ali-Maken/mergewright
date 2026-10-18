//! Terminal input, handed to the loop.
//!
//! Decoding lives in `mergewright_term::input`, whose mux owns stdin for the whole
//! run so that a key typed during the capability probe is queued rather than
//! lost. This module only carries what the mux decoded into the loop's wake
//! channel, in order, on a thread of its own.

use std::io;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use mergewright_core::event::Event;
use mergewright_term::input::Inputs;

use super::Wake;
use super::redetect::{Query, Redetector};
use crate::update::AppEvent;

/// Forward every decoded input to the loop on a thread of its own.
///
/// The thread blocks on the mux's queue, so an idle Mergewright costs nothing. It
/// ends when the loop has gone (the wake receiver is dropped) or when the
/// mux has stopped, whose last word is the failure the loop reports.
///
/// `stamps` is the frame trace's (`crate::trace`, `Some` only while tracing):
/// each key and mouse event's forwarding time goes down it just before the
/// event, so the loop finds the stamp waiting when it takes the event and
/// can say how old the input was when its frame was written.
pub fn spawn_reader(
    inputs: Inputs,
    wakes: Sender<Wake>,
    redetector: Option<Redetector>,
    stamps: Option<Sender<Instant>>,
) -> io::Result<Forwarder> {
    let thread = thread::Builder::new()
        .name("graft-input-forward".to_owned())
        .spawn(move || {
            let mut stamps = stamps;
            for input in inputs {
                if let (Ok(Event::FocusGained), Some(redetector)) = (&input, &redetector) {
                    // Refresh-on-focus: a theme switched while Mergewright was in
                    // the background is re-read (DESIGN.md §4.5).
                    redetector.ask(Query::Theme);
                }
                if let (Ok(Event::Key(_) | Event::Mouse(_)), Some(sender)) = (&input, &stamps)
                    && sender.send(Instant::now()).is_err()
                {
                    // The tracer is gone (the loop has ended). Stamping
                    // stops; forwarding does not — keys typed after the
                    // loop still go back to the mux (`hand_back_input`).
                    stamps = None;
                }
                let wake = match input {
                    Ok(event) => Wake::Event(AppEvent::Core(event)),
                    // Not survivable — there is no input any more — so it
                    // goes to the loop, which stops and reports it.
                    Err(error) => Wake::InputFailed(error),
                };
                if wakes.send(wake).is_err() {
                    // The loop has finished; nobody is left to tell.
                    return;
                }
            }
        })?;
    Ok(Forwarder { thread })
}

/// `EIO`: what a read of a terminal that has hung up fails with. The same
/// number on macOS and Linux; `graft` has no `libc` to name it.
const EIO: i32 = 5;

/// Whether a failed terminal read is the terminal hanging up (m8/13g): the
/// mux's end of input (a read of 0, `UnexpectedEof`) or `EIO`. Either one
/// ends the loop as `SIGHUP` does — "Quit from outside" — whichever of the
/// read and the signal arrives first; never a failure that waits silently.
pub(super) fn hung_up(error: &io::Error) -> bool {
    error.kind() == io::ErrorKind::UnexpectedEof || error.raw_os_error() == Some(EIO)
}

/// The running input forwarder, for the run that started it.
#[derive(Debug)]
pub struct Forwarder {
    thread: JoinHandle<()>,
}

impl Forwarder {
    /// Wait for the forwarder to end, so that everything it had is in the
    /// wake channel. Call after `InputMux::hold` has dropped its `Inputs`
    /// sender: from then on the thread only drains what was already sent
    /// and stops, so this is a bounded wait on the loop's own input, never
    /// on a job (the rule `event_loop::tests` checks).
    pub fn finish(self) -> io::Result<()> {
        self.thread
            .join()
            .map_err(|_panic| io::Error::other("the input forwarder panicked"))
    }
}
