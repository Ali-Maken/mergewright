//! A bare repository behind git's smart HTTP protocol, with Basic auth.
//!
//! The fixture owns a `TcpListener` on `127.0.0.1:0` and speaks just enough
//! HTTP/1.1 itself: one request per connection, a body by `Content-Length`
//! or chunked, `Expect: 100-continue`, and `Connection: close` on every
//! answer. Every authorised request is handed to `git http-backend` as a CGI
//! script, which is exactly what a real Git server does behind Apache or
//! nginx. Plain HTTP only — no TLS (see the module above).
//!
//! Why not a server crate: `tiny_http`, used until m8 13j, moved its listener
//! into an accept thread it never joined, so the port could still accept a
//! connection after `drop` returned (13i). Here the listener lives in the
//! thread `Drop` joins, so the port is closed once `drop` returns — by
//! construction, as for [`HungRemote`](super::HungRemote).

mod cgi;
mod listener;
mod request;

use std::fmt;
use std::io;
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use self::cgi::Backend;
use self::listener::accept_loop;
use super::run_git;

/// How long [`Drop`] waits for the listener thread and every connection
/// handler, after ending any `http-backend` and closing every client, before
/// declaring the teardown failed.
///
/// A test's teardown must never hang, and must never quietly leave a thread
/// behind either: past this, drop panics (the test fails, saying why) rather
/// than return with a thread still running.
const STOP_BOUND: Duration = Duration::from_secs(2);

/// The longest one read from a client may block. It bounds only a handler
/// whose client went silent (half-open) with no `Drop` coming; teardown is
/// `Drop`'s, which shuts every client's stream and so ends a blocked read at
/// once. Measured, not assumed (m8/21b-3): `http_remote_drop_ends_an_idle_client`
/// drops within 500 ms, 20/20, while this was still 1 s — so the shutdown
/// woke the read. Long, so a slow client under load is never cut off.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// The longest request line, header line or chunk-size line accepted.
const MAX_LINE: u64 = 8 * 1024;

/// The most header lines one request may carry.
const MAX_HEADERS: usize = 100;

/// The one repository path the server answers for.
const REPO_PATH: &str = "/repo.git";

/// A smart-HTTP remote that demands a user and password.
///
/// Serves one bare repository at `http://127.0.0.1:<port>/repo.git`. A
/// request without the right `Authorization: Basic` header gets `401` and a
/// `WWW-Authenticate` challenge, as GitHub or GitLab would send; the right
/// one is served by `git http-backend`, fetch and push both.
///
/// Dropping it stops the listener and frees the port.
pub struct HttpRemote {
    address: SocketAddr,
    /// The listener thread: it owns the `TcpListener`, so the port closes
    /// when it ends.
    thread: Option<JoinHandle<()>>,
    /// Disconnects once the listener thread and every connection handler
    /// have ended, however they end: each holds a clone of the one sender.
    /// In a `Mutex` only so the fixture stays `Sync`.
    stopped: Mutex<Receiver<()>>,
    shared: Arc<Shared>,
    user: String,
    /// Keeps the empty `HOME` the backend runs with.
    _home: tempfile::TempDir,
}

impl fmt::Debug for HttpRemote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HttpRemote")
            .field("url", &self.url())
            .field("requests", &self.requests())
            .finish_non_exhaustive()
    }
}

/// What the fixture, its listener and its handlers share.
#[derive(Default)]
struct Shared {
    connections: Mutex<Connections>,
    running: Mutex<Running>,
    counters: Counters,
}

/// Every accepted connection, so [`Drop`] can close and join it.
#[derive(Default)]
struct Connections {
    /// Set by `Drop`. The listener checks it under this lock before it
    /// registers a connection, so once `Drop` has set it, nothing it does
    /// not see in `open` can be registered.
    closing: bool,
    open: Vec<Connection>,
}

/// One accepted client and the thread answering it.
struct Connection {
    /// A clone of the handler's stream: shutting it ends the handler's
    /// blocked read or write.
    stream: TcpStream,
    handler: JoinHandle<()>,
}

/// What the server has seen, for tests to assert on.
#[derive(Debug, Default)]
struct Counters {
    authorised: AtomicUsize,
    rejected: AtomicUsize,
    /// Authorised requests whose body arrived with
    /// `Transfer-Encoding: chunked` — the pushes past `http.postBuffer`.
    chunked: AtomicUsize,
    /// Authorised requests whose body was gzipped (`Content-Encoding`).
    gzipped: AtomicUsize,
}

/// The `http-backend`s the handlers are waiting on, so [`Drop`] can end them.
///
/// Each backend runs in its own process group, and the whole group is
/// killed: `http-backend` hands a push to `receive-pack`, which runs the
/// repository's hooks, and each of them holds the output pipe the handler
/// reads to EOF.
#[derive(Debug, Default)]
struct Running {
    /// Set by `Drop`: a backend started after this is killed at once.
    stopping: bool,
    /// The running backends' process group ids (each its leader's pid).
    groups: Vec<u32>,
}

/// A poisoned lock only means a panic elsewhere already failed the test;
/// what it guards is still sound.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

impl HttpRemote {
    /// Serves `bare` on an ephemeral loopback port, accepting only `user` with
    /// `password`.
    ///
    /// Sets `http.receivepack=true` on `bare`, so a push is accepted whatever
    /// `http-backend`'s default for an authenticated user.
    ///
    /// # Errors
    /// If `bare` has no parent or file name, the config cannot be written,
    /// or the port cannot be bound.
    pub fn serve(bare: &Path, user: &str, password: &str) -> io::Result<HttpRemote> {
        Self::start(bare, user, password, Duration::ZERO)
    }

    /// [`serve`](Self::serve), with the listener thread holding the listening
    /// socket for `exit_delay` after its loop ends. The test seam that makes
    /// "`drop` returns only once the port is closed" fail deterministically
    /// if `drop` stops waiting for that thread.
    #[cfg(test)]
    pub(crate) fn serve_with_exit_delay(
        bare: &Path,
        user: &str,
        password: &str,
        exit_delay: Duration,
    ) -> io::Result<HttpRemote> {
        Self::start(bare, user, password, exit_delay)
    }

    fn start(
        bare: &Path,
        user: &str,
        password: &str,
        exit_delay: Duration,
    ) -> io::Result<HttpRemote> {
        let (Some(project_root), Some(repo_name)) = (bare.parent(), bare.file_name()) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{} has no parent directory or name", bare.display()),
            ));
        };
        let home = tempfile::Builder::new()
            .prefix("graft-http-home-")
            .tempdir()?;
        run_git(
            home.path(),
            bare,
            &["config", "http.receivepack", "true"],
            &[],
        )
        .map_err(io::Error::other)?;

        let listener = TcpListener::bind("127.0.0.1:0")?;
        let address = listener.local_addr()?;
        let shared = Arc::new(Shared::default());
        let backend = Arc::new(Backend {
            project_root: project_root.to_path_buf(),
            repo_name: repo_name.to_string_lossy().into_owned(),
            expected_authorization: format!(
                "Basic {}",
                base64(format!("{user}:{password}").as_bytes())
            ),
            user: user.to_owned(),
            home: home.path().to_path_buf(),
            shared: Arc::clone(&shared),
        });
        let (finished, stopped) = mpsc::channel::<()>();
        let thread = thread::Builder::new()
            .name(format!("http-remote-{}", address.port()))
            .spawn(move || {
                accept_loop(&listener, &backend, &finished);
                if !exit_delay.is_zero() {
                    // Test seam only (`serve_with_exit_delay`): stands in for
                    // a thread descheduled between its loop and its exit.
                    thread::sleep(exit_delay);
                }
                // The port closes here; only then may `Drop` see this thread
                // as finished, so the sender goes after it.
                drop(listener);
                drop(finished);
            })?;

        Ok(HttpRemote {
            address,
            thread: Some(thread),
            stopped: Mutex::new(stopped),
            shared,
            user: user.to_owned(),
            _home: home,
        })
    }

    /// `http://<user>@127.0.0.1:<port>/repo.git` — git asks only for the
    /// password.
    pub fn url(&self) -> String {
        format!("http://{}@127.0.0.1:{}{REPO_PATH}", self.user, self.port())
    }

    /// `http://127.0.0.1:<port>/repo.git` — git asks for user and password.
    pub fn url_anonymous(&self) -> String {
        format!("http://127.0.0.1:{}{REPO_PATH}", self.port())
    }

    /// The port the server listens on.
    pub fn port(&self) -> u16 {
        self.address.port()
    }

    /// Requests seen so far: (authorised, rejected with `401`).
    ///
    /// Git's first request never carries credentials, so even a clone with
    /// the right password counts one rejection.
    pub fn requests(&self) -> (usize, usize) {
        let counters = &self.shared.counters;
        (
            counters.authorised.load(Ordering::SeqCst),
            counters.rejected.load(Ordering::SeqCst),
        )
    }

    /// Authorised requests whose body came chunked.
    #[cfg(test)]
    pub(crate) fn chunked_requests(&self) -> usize {
        self.shared.counters.chunked.load(Ordering::SeqCst)
    }

    /// Authorised requests whose body came gzipped.
    #[cfg(test)]
    pub(crate) fn gzipped_requests(&self) -> usize {
        self.shared.counters.gzipped.load(Ordering::SeqCst)
    }

    /// Accepted connections whose handler is still running.
    #[cfg(test)]
    pub(crate) fn open_connections(&self) -> usize {
        lock(&self.shared.connections)
            .open
            .iter()
            .filter(|connection| !connection.handler.is_finished())
            .count()
    }
}

/// Standard base64 with padding — all Basic auth needs, not worth a crate.
pub(super) fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(char::from(ALPHABET[(n >> (18 - 6 * i)) as usize & 63]));
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn base64_matches_rfc_4648_vectors() {
        let cases = [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ];
        for (input, expected) in cases {
            assert_eq!(base64(input.as_bytes()), expected, "{input:?}");
        }
    }
}
