//! A remote that accepts a connection and never answers.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// A loopback port where every connection is accepted, held open, and never
/// read from or written to — a server that has hung, or a network that
/// silently drops everything after the handshake.
///
/// `git fetch` against [`url`](HungRemote::url) blocks until it is killed:
/// the condition Graft's cancel has to handle. Dropping this closes every
/// held connection and the listening socket.
#[derive(Debug)]
pub struct HungRemote {
    address: SocketAddr,
    thread: Option<JoinHandle<()>>,
    /// The accepted streams, and the `Condvar` the listener raises whenever
    /// it pushes one. [`wait_for_connection`](HungRemote::wait_for_connection)
    /// waits on that; without it a caller has to poll `connections`.
    held: Arc<(Mutex<Vec<TcpStream>>, Condvar)>,
    stopping: Arc<AtomicBool>,
}

impl HungRemote {
    /// Listens on an ephemeral loopback port.
    ///
    /// # Errors
    /// If the port cannot be bound or the thread cannot start.
    pub fn listen() -> io::Result<HungRemote> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let held = Arc::new((Mutex::new(Vec::new()), Condvar::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let thread = {
            let held = Arc::clone(&held);
            let stopping = Arc::clone(&stopping);
            thread::Builder::new()
                .name(format!("hung-remote-{}", address.port()))
                .spawn(move || accept_and_hold(&listener, &held, &stopping))?
        };
        Ok(HungRemote {
            address,
            thread: Some(thread),
            held,
            stopping,
        })
    }

    /// `http://127.0.0.1:<port>/repo.git`.
    pub fn url(&self) -> String {
        format!("http://{}/repo.git", self.address)
    }

    /// The port listened on.
    #[cfg(test)]
    pub(crate) fn port(&self) -> u16 {
        self.address.port()
    }

    /// Connections accepted and held so far.
    pub fn connections(&self) -> usize {
        lock(&self.held.0).len()
    }

    /// Blocks until a client has connected, or `within` elapses; `true` if one
    /// has. Returns straight away when one connected already.
    ///
    /// The event, not a timer: the listener raises a `Condvar` as it accepts,
    /// so a test waiting for "git has reached the remote" waits on that rather
    /// than sleeping and re-checking `connections` (CLAUDE.md rule 2).
    /// `within` is a deadline for the client that never comes, so the test
    /// fails instead of hanging the run.
    pub fn wait_for_connection(&self, within: Duration) -> bool {
        let (streams, arrived) = &*self.held;
        let guard = streams.lock().unwrap_or_else(PoisonError::into_inner);
        let (guard, _timed_out) = arrived
            .wait_timeout_while(guard, within, |streams| streams.is_empty())
            .unwrap_or_else(PoisonError::into_inner);
        !guard.is_empty()
    }
}

/// The listener thread: accept, keep the stream, repeat — until `Drop`
/// raises `stopping` and connects once to wake the `accept`.
///
/// `stopping` is checked after every `accept`. Whatever is accepted once it
/// is set — the wake-up, or a real client that connected in the same moment
/// and was queued ahead of it — is closed, not held. Nothing can count it
/// any more: `connections` needs the fixture, and it is being dropped.
fn accept_and_hold(
    listener: &TcpListener,
    held: &(Mutex<Vec<TcpStream>>, Condvar),
    stopping: &AtomicBool,
) {
    let (streams, arrived) = held;
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            drop(stream);
            // Nothing should be waiting — a waiter holds the fixture and
            // `Drop` has it — but a wait that raced the drop ends here
            // rather than on its deadline.
            arrived.notify_all();
            return;
        }
        match stream {
            Ok(stream) => {
                lock(streams).push(stream);
                arrived.notify_all();
            }
            Err(error) => {
                // A failed `accept` (the client reset before it completed)
                // costs this fixture nothing: the listener is still good and
                // the test counts only connections that were held.
                eprintln!("HungRemote: accept failed: {error}");
            }
        }
    }
}

/// The held streams. A poisoned lock only means a panic elsewhere already
/// failed the test; the vector itself is still sound.
fn lock(held: &Mutex<Vec<TcpStream>>) -> std::sync::MutexGuard<'_, Vec<TcpStream>> {
    held.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Drop for HungRemote {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        let Some(thread) = self.thread.take() else {
            return;
        };
        // Wake the blocked `accept` while the listener is still open; the
        // thread sees `stopping` and returns, dropping the listener (which
        // frees the port). The thread does nothing else, so once woken the
        // join returns.
        if let Err(error) = TcpStream::connect(self.address) {
            // Unwoken, the thread would sit in `accept` for the rest of the
            // test binary. Fail the test rather than leave it silently —
            // unless it is already failing, where a second panic aborts.
            let message = format!(
                "HungRemote on {}: could not wake the listener ({error}); \
                 its thread is still running",
                self.address
            );
            if thread::panicking() {
                eprintln!("{message}");
                return;
            }
            panic!("{message}");
        }
        if thread.join().is_err() {
            eprintln!("HungRemote on {}: listener thread panicked", self.address);
        }
        for stream in lock(&self.held.0).drain(..) {
            if let Err(error) = stream.shutdown(Shutdown::Both) {
                // The client already went away (it was killed); the stream
                // is closed either way when it drops here.
                if error.kind() != io::ErrorKind::NotConnected {
                    eprintln!("HungRemote on {}: shutdown failed: {error}", self.address);
                }
            }
        }
    }
}
