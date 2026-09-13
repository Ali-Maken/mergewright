//! Terminal input, handed to the loop.
//!
//! Decoding lives in `graft_term::input`, whose mux owns stdin for the whole
//! run so that a key typed during the capability probe is queued rather than
//! lost. This module only carries what the mux decoded into the loop's wake
//! channel, in order, on a thread of its own.

use std::io;
use std::sync::mpsc::Sender;
use std::thread::{self, JoinHandle};

use graft_core::event::Event;
use graft_term::input::Inputs;

use super::Wake;
use super::redetect::{Query, Redetector};
use crate::update::AppEvent;

/// Forward every decoded input to the loop on a thread of its own.
///
/// The thread blocks on the mux's queue, so an idle Graft costs nothing. It
/// ends when the loop has gone (the wake receiver is dropped) or when the
/// mux has stopped, whose last word is the failure the loop reports.
pub fn spawn_reader(
    inputs: Inputs,
    wakes: Sender<Wake>,
    redetector: Option<Redetector>,
) -> io::Result<Forwarder> {
    let thread = thread::Builder::new()
        .name("graft-input-forward".to_owned())
        .spawn(move || {
            for input in inputs {
                if let (Ok(Event::FocusGained), Some(redetector)) = (&input, &redetector) {
                    // Refresh-on-focus: a theme switched while Graft was in
                    // the background is re-read (DESIGN.md §4.5).
                    redetector.ask(Query::Theme);
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
