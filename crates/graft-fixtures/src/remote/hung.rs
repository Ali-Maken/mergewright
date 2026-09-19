//! A remote that accepts a connection and never answers.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
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
    held: Arc<Mutex<Vec<TcpStream>>>,
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
        listener.set_nonblocking(true)?;
        let held = Arc::new(Mutex::new(Vec::new()));
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
        lock(&self.held).len()
    }
}

/// How long the idle listener parks before looking for a connection again.
/// `Drop` does not wait on it: it unparks the thread.
const ACCEPT_POLL: Duration = Duration::from_millis(10);

/// The listener thread: accept, keep the stream, repeat — until `Drop`
/// raises `stopping` and unparks it.
///
/// `listener` is non-blocking, so the thread never sits in an `accept` only a
/// new connection could end. (It used to, and `Drop` woke it by connecting;
/// when that connect failed the thread was stranded for the life of the test
/// binary.) Every connection accepted before `stopping` is seen is held, so
/// none is dropped by a teardown racing it.
fn accept_and_hold(listener: &TcpListener, held: &Mutex<Vec<TcpStream>>, stopping: &AtomicBool) {
    while !stopping.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => lock(held).push(stream),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                thread::park_timeout(ACCEPT_POLL);
            }
            Err(error) => {
                // A failed `accept` (the client reset before it completed)
                // costs this fixture nothing: the listener is still good and
                // the test counts only connections that were held. Parked as
                // when idle, so a lasting failure does not spin.
                eprintln!("HungRemote: accept failed: {error}");
                thread::park_timeout(ACCEPT_POLL);
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
        // The thread never blocks without a bound, so this join always
        // returns; unparking makes it see `stopping` now. It drops the
        // listener as it returns, which frees the port.
        thread.thread().unpark();
        if thread.join().is_err() {
            eprintln!("HungRemote on {}: listener thread panicked", self.address);
        }
        for stream in lock(&self.held).drain(..) {
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
