//! The listener thread and teardown: accept, one handler per connection,
//! and `Drop`, which ends them all.

use std::io;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::process::{Command, Stdio};
use std::sync::mpsc::{RecvTimeoutError, Sender};
use std::sync::{Arc, PoisonError};
use std::thread::{self, JoinHandle};

use super::cgi::Backend;
use super::{Connection, HttpRemote, READ_TIMEOUT, STOP_BOUND, lock};

/// `SIGKILL` to every process in `group`. Through `kill(1)`: this crate keeps
/// its one `unsafe` site for `getrusage` (ADR-0015).
pub(super) fn kill_group(group: u32) -> Result<(), String> {
    let output = Command::new("kill")
        .args(["-KILL", "--", &format!("-{group}")])
        .stdin(Stdio::null())
        .output()
        .map_err(|error| format!("could not run kill: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "kill -KILL -{group}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// `shutdown(Both)`: wakes whatever thread is blocked reading or writing it.
fn shut(stream: &TcpStream, port: u16) {
    if let Err(error) = stream.shutdown(Shutdown::Both) {
        // The client already went away; the stream is closed either way.
        if error.kind() != io::ErrorKind::NotConnected {
            eprintln!("HttpRemote on port {port}: shutdown failed: {error}");
        }
    }
}

/// The listener thread: accept, hand the client to its own handler thread,
/// repeat — until `Drop` sets `closing` and connects once to wake `accept`.
///
/// `closing` is checked, under the lock `Drop` sets it with, after every
/// `accept`. Whatever is accepted once it is set — the wake-up, or a real
/// client that connected in the same moment — is closed, not answered.
pub(super) fn accept_loop(listener: &TcpListener, backend: &Arc<Backend>, finished: &Sender<()>) {
    for stream in listener.incoming() {
        let mut connections = lock(&backend.shared.connections);
        if connections.closing {
            return;
        }
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                // A failed `accept` (the client reset before it completed)
                // costs nothing: the listener is still good, and that
                // client sees its own error.
                eprintln!("HttpRemote: accept failed: {error}");
                continue;
            }
        };
        reap_finished(&mut connections.open);
        match spawn_handler(stream, backend, finished) {
            Ok(connection) => connections.open.push(connection),
            // The stream dropped with the error: that client sees a closed
            // connection and fails. This line says why.
            Err(error) => eprintln!("HttpRemote: could not start a handler: {error}"),
        }
    }
}

/// Joins the handlers that have ended, so `open` holds only live ones.
fn reap_finished(open: &mut Vec<Connection>) {
    let (done, live): (Vec<_>, Vec<_>) = open
        .drain(..)
        .partition(|connection| connection.handler.is_finished());
    *open = live;
    for connection in done {
        join_handler(connection.handler);
    }
}

/// Joins one handler. A handler never panics by design; if one did, its
/// client already failed, and this line says why.
fn join_handler(handler: JoinHandle<()>) {
    if handler.join().is_err() {
        eprintln!("HttpRemote: a connection handler panicked");
    }
}

/// Starts the thread that answers `stream`'s one request.
fn spawn_handler(
    stream: TcpStream,
    backend: &Arc<Backend>,
    finished: &Sender<()>,
) -> io::Result<Connection> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    let kept = stream.try_clone()?;
    let backend = Arc::clone(backend);
    let finished = finished.clone();
    let handler = thread::Builder::new()
        .name("http-remote-connection".to_owned())
        .spawn(move || {
            // Owned here so it drops when the handler ends, by returning or
            // by unwinding.
            let _finished = finished;
            backend.handle(stream);
        })?;
    Ok(Connection {
        stream: kept,
        handler,
    })
}

impl Drop for HttpRemote {
    /// The teardown, in this order — each step relies on the ones before it.
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        let port = self.port();
        self.close_to_new_connections();
        let killed = self.kill_backends();
        self.shut_clients(port);
        let wake = self.wake_listener();
        if self.wait_for_threads(port, &killed, &wake) {
            self.join_threads(thread, port);
        }
    }
}

/// `Drop`'s steps, numbered in the order it runs them.
impl HttpRemote {
    /// 1. Closed: from here on the listener registers no connection.
    fn close_to_new_connections(&self) {
        lock(&self.shared.connections).closing = true;
    }

    /// 2. A handler may be waiting on an `http-backend` that will never
    ///    finish (a hung hook, say). End it; no backend started from here
    ///    on survives either.
    fn kill_backends(&self) -> Vec<Result<(), String>> {
        let mut running = lock(&self.shared.running);
        running.stopping = true;
        running.groups.drain(..).map(kill_group).collect()
    }

    /// 3. Every handler blocked on its client — idle, half-open, or not
    ///    reading its answer — wakes with its stream shut.
    fn shut_clients(&self, port: u16) {
        for connection in &lock(&self.shared.connections).open {
            shut(&connection.stream, port);
        }
    }

    /// 4. Wake the blocked `accept` while the listener is still open; the
    ///    listener sees `closing`, returns, and drops the listener.
    ///
    ///    A refused wake is not by itself a failure: a client queued
    ///    before step 1 (a test's "is the port open?" probe, say) can be
    ///    accepted after it, and then the listener has already seen
    ///    `closing`, returned and closed the socket — nothing is left to
    ///    wake. A listener that really is still in `accept` is caught by
    ///    step 5's bound, whose panic carries this error.
    fn wake_listener(&self) -> io::Result<()> {
        TcpStream::connect(self.address).map(drop)
    }

    /// 5. Wait for the listener thread and every handler, within the bound.
    ///    `true` once all have ended; `false` only when the bound passed
    ///    while the thread was already panicking (said on stderr instead).
    fn wait_for_threads(
        &mut self,
        port: u16,
        killed: &[Result<(), String>],
        wake: &io::Result<()>,
    ) -> bool {
        let stopped = self
            .stopped
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        match stopped.recv_timeout(STOP_BOUND) {
            Err(RecvTimeoutError::Disconnected) => true,
            Ok(()) => unreachable!("no thread sends; each only disconnects"),
            Err(RecvTimeoutError::Timeout) => {
                // Stuck even with backends killed and clients shut: something
                // outside a backend's process group holds a request open.
                // Returning would leave a thread (maybe the port) running for
                // the rest of the test binary, silently. Fail the test
                // instead — unless it is already failing.
                let message = format!(
                    "HttpRemote on port {port}: a thread still running {STOP_BOUND:?} \
                     after its backends were killed ({killed:?}) and the listener \
                     was woken ({wake:?}); teardown failed"
                );
                if thread::panicking() {
                    eprintln!("{message}");
                    return false;
                }
                panic!("{message}");
            }
        }
    }

    /// 6. Every thread has ended (all senders are gone): these joins return
    ///    at once. The listener thread dropped the listening socket before
    ///    its sender, so the port is closed now.
    fn join_threads(&self, thread: JoinHandle<()>, port: u16) {
        if thread.join().is_err() {
            eprintln!("HttpRemote on port {port}: listener thread panicked");
        }
        let open = std::mem::take(&mut lock(&self.shared.connections).open);
        for connection in open {
            join_handler(connection.handler);
        }
    }
}
