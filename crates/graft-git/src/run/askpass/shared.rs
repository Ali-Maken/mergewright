//! What the listener thread shares with the server, and how an exchange's
//! wait on its handler ends.

use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Mutex, MutexGuard, PoisonError};

use super::peer::Refusal;
use super::{Refused, Secret};

/// What the listener thread shares with the server that owns it.
#[derive(Default)]
pub(super) struct Shared {
    /// Set once by shutdown; never cleared.
    stop: AtomicBool,
    /// The exchange waiting on its handler, if one is: shutdown sends it
    /// [`Wake::Stop`], so it returns at once rather than whenever the handler
    /// does. `serve` checks `stop` and registers here under one lock, so a
    /// shutdown cannot fall between the check and the registration.
    waiting: Mutex<Option<Sender<Wake>>>,
    /// The first refused peer's reason, and how many were refused.
    refused: Mutex<Option<(String, usize)>>,
}

impl Shared {
    pub(super) fn stopped(&self) -> bool {
        self.stop.load(Ordering::SeqCst)
    }

    /// Set `stop`, then wake the exchange waiting on its handler, if any.
    pub(super) fn stop(&self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(waiting) = self.waiting().take()
            && let Err(unheard) = waiting.send(Wake::Stop)
        {
            // Safe to ignore: the exchange already ended (its answer came
            // first), so there is no one left to wake.
            drop(unheard);
        }
    }

    /// Keeps the first refusal's text and counts the rest: bounded however
    /// many connect. A poisoned lock still holds a whole pair; used as is.
    pub(super) fn refuse(&self, refusal: &Refusal) {
        let mut refused = self.refused.lock().unwrap_or_else(PoisonError::into_inner);
        refused.get_or_insert_with(|| (refusal.to_string(), 0)).1 += 1;
    }

    /// Any refusal, as the error `close` reports.
    pub(super) fn refusals(&self) -> io::Result<()> {
        let refused = self.refused.lock().unwrap_or_else(PoisonError::into_inner);
        match &*refused {
            None => Ok(()),
            Some((first, count)) => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                Refused {
                    first: first.clone(),
                    count: *count,
                },
            )),
        }
    }

    /// The slot. A poisoned lock still holds a whole `Option<Sender>` — the
    /// only writes are `take` and a plain assignment — so it is used as is.
    pub(super) fn waiting(&self) -> MutexGuard<'_, Option<Sender<Wake>>> {
        self.waiting.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// What ends an exchange's wait on its handler.
pub(super) enum Wake {
    /// The handler answered (`None`: the user cancelled).
    Answer(Option<Secret>),
    /// The server is shutting down.
    Stop,
    /// The handler panicked before answering.
    Panicked,
}

/// The handler thread's end of an exchange: sends the answer once or, if the
/// thread unwinds first, [`Wake::Panicked`] from `Drop` — the listener is
/// never left waiting on a handler that is gone.
pub(super) struct Reply(pub(super) Option<Sender<Wake>>);

impl Reply {
    pub(super) fn send(mut self, answer: Option<Secret>) {
        if let Some(wake) = self.0.take()
            && let Err(unwanted) = wake.send(Wake::Answer(answer))
        {
            // The listener stopped waiting (shutdown): drop the answer,
            // which zeroes it.
            drop(unwanted);
        }
    }
}

impl Drop for Reply {
    fn drop(&mut self) {
        if let Some(wake) = self.0.take()
            && let Err(unheard) = wake.send(Wake::Panicked)
        {
            // Safe to ignore: the listener stopped waiting (shutdown).
            drop(unheard);
        }
    }
}
