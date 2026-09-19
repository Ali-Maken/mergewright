//! A remote that accepts a connection and never answers.

use std::io;
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};

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

    /// Connections accepted and held so far.
    pub fn connections(&self) -> usize {
        lock(&self.held).len()
    }
}

/// The listener thread: accept, keep the stream, repeat — until `Drop`
/// raises `stopping` and connects once to wake the `accept`.
fn accept_and_hold(listener: &TcpListener, held: &Mutex<Vec<TcpStream>>, stopping: &AtomicBool) {
    for stream in listener.incoming() {
        if stopping.load(Ordering::SeqCst) {
            return;
        }
        match stream {
            Ok(stream) => lock(held).push(stream),
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
        // Wake the blocked `accept` so the thread sees `stopping` and returns,
        // dropping the listener with it.
        match TcpStream::connect(self.address) {
            Ok(_) => {
                if thread.join().is_err() {
                    eprintln!("HungRemote on {}: listener thread panicked", self.address);
                }
            }
            Err(error) => {
                // Without the wake-up the thread would block forever in
                // `accept`; joining it would hang the test's teardown. It is
                // left behind and the reason reported.
                eprintln!(
                    "HungRemote on {}: could not wake the listener ({error}); left running",
                    self.address
                );
            }
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
