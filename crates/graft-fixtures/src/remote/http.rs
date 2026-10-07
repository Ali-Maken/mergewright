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

use std::fmt;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::{hermetic_command, run_git};

/// How long [`Drop`] waits for the listener thread and every connection
/// handler, after ending any `http-backend` and closing every client, before
/// declaring the teardown failed.
///
/// A test's teardown must never hang, and must never quietly leave a thread
/// behind either: past this, drop panics (the test fails, saying why) rather
/// than return with a thread still running.
const STOP_BOUND: Duration = Duration::from_secs(2);

/// The longest one read from a client may block. A backstop only: `Drop`
/// shuts every client's stream, which ends a blocked read at once. It bounds
/// a handler whose client went silent (half-open) even without a `Drop`.
const READ_TIMEOUT: Duration = Duration::from_secs(1);

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

/// `SIGKILL` to every process in `group`. Through `kill(1)`: this crate keeps
/// its one `unsafe` site for `getrusage` (ADR-0015).
fn kill_group(group: u32) -> Result<(), String> {
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

/// A poisoned lock only means a panic elsewhere already failed the test;
/// what it guards is still sound.
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
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

/// Everything a handler needs to answer a request.
struct Backend {
    /// The directory holding the bare repository: `GIT_PROJECT_ROOT`.
    project_root: PathBuf,
    /// The bare repository's directory name, which `/repo.git` maps to.
    repo_name: String,
    /// The whole `Authorization` header value that is accepted.
    expected_authorization: String,
    user: String,
    home: PathBuf,
    shared: Arc<Shared>,
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

/// The listener thread: accept, hand the client to its own handler thread,
/// repeat — until `Drop` sets `closing` and connects once to wake `accept`.
///
/// `closing` is checked, under the lock `Drop` sets it with, after every
/// `accept`. Whatever is accepted once it is set — the wake-up, or a real
/// client that connected in the same moment — is closed, not answered.
fn accept_loop(listener: &TcpListener, backend: &Arc<Backend>, finished: &Sender<()>) {
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
    fn drop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        let port = self.port();
        // 1. Closed: from here on the listener registers no connection.
        lock(&self.shared.connections).closing = true;
        // 2. A handler may be waiting on an `http-backend` that will never
        //    finish (a hung hook, say). End it; no backend started from here
        //    on survives either.
        let killed: Vec<Result<(), String>> = {
            let mut running = lock(&self.shared.running);
            running.stopping = true;
            running.groups.drain(..).map(kill_group).collect()
        };
        // 3. Every handler blocked on its client — idle, half-open, or not
        //    reading its answer — wakes with its stream shut.
        for connection in &lock(&self.shared.connections).open {
            shut(&connection.stream, port);
        }
        // 4. Wake the blocked `accept` while the listener is still open; the
        //    listener sees `closing`, returns, and drops the listener.
        //
        //    A refused wake is not by itself a failure: a client queued
        //    before step 1 (a test's "is the port open?" probe, say) can be
        //    accepted after it, and then the listener has already seen
        //    `closing`, returned and closed the socket — nothing is left to
        //    wake. A listener that really is still in `accept` is caught by
        //    step 5's bound, whose panic carries this error.
        let wake = TcpStream::connect(self.address).map(drop);
        // 5. Wait for the listener thread and every handler, within the bound.
        let stopped = self
            .stopped
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        match stopped.recv_timeout(STOP_BOUND) {
            Err(RecvTimeoutError::Disconnected) => {}
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
                    return;
                }
                panic!("{message}");
            }
        }
        // Every thread has ended (all senders are gone): these joins return
        // at once. The listener thread dropped the listening socket before
        // its sender, so the port is closed now.
        if thread.join().is_err() {
            eprintln!("HttpRemote on port {port}: listener thread panicked");
        }
        let open = std::mem::take(&mut lock(&self.shared.connections).open);
        for connection in open {
            join_handler(connection.handler);
        }
    }
}

/// One HTTP request, body already de-chunked.
struct Request {
    method: String,
    target: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Request {
    /// The first header called `name`, compared case-insensitively.
    fn header(&self, name: &str) -> Option<&str> {
        find_header(&self.headers, name)
    }
}

fn find_header<'a>(headers: &'a [(String, String)], name: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(field, _)| field.eq_ignore_ascii_case(name))
        .map(|(_, value)| value.as_str())
}

/// An answer, before framing.
struct Reply {
    status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Reply {
    fn text(status: u16, body: impl Into<String>) -> Reply {
        Reply {
            status,
            headers: vec![("Content-Type".to_owned(), "text/plain".to_owned())],
            body: body.into().into_bytes(),
        }
    }
}

/// An error that means the client sent something that is not HTTP/1.1 this
/// server understands: answered `400`, unlike an I/O error.
fn malformed(what: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, what.into())
}

/// The peer closed in the middle of a request.
fn cut_short() -> io::Error {
    io::Error::new(
        io::ErrorKind::UnexpectedEof,
        "the client closed mid-request",
    )
}

/// Reads one `\r\n`- or `\n`-terminated line, without its terminator.
/// `Ok(None)`: the peer closed before sending a byte of it.
fn read_line(reader: &mut impl BufRead) -> io::Result<Option<String>> {
    let mut line = Vec::new();
    (&mut *reader).take(MAX_LINE).read_until(b'\n', &mut line)?;
    if line.is_empty() {
        return Ok(None);
    }
    if line.pop() != Some(b'\n') {
        return Err(
            if line.len() + 1 >= usize::try_from(MAX_LINE).unwrap_or(usize::MAX) {
                malformed("line too long")
            } else {
                cut_short()
            },
        );
    }
    if line.last() == Some(&b'\r') {
        line.pop();
    }
    String::from_utf8(line)
        .map(Some)
        .map_err(|_| malformed("line is not UTF-8"))
}

/// Reads one request; answers `100 Continue` itself when asked. `Ok(None)`:
/// the peer closed before a request line. A request this server cannot
/// parse is an `InvalidData` error.
fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
    let mut reader = BufReader::new(&*stream);
    let Some(line) = read_line(&mut reader)? else {
        return Ok(None);
    };
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(malformed(format!("bad request line {line:?}")));
    };
    if method.is_empty() || !target.starts_with('/') || !version.starts_with("HTTP/1.") {
        return Err(malformed(format!("bad request line {line:?}")));
    }
    let (method, target) = (method.to_owned(), target.to_owned());

    let mut headers = Vec::new();
    loop {
        let line = read_line(&mut reader)?.ok_or_else(cut_short)?;
        if line.is_empty() {
            break;
        }
        if headers.len() == MAX_HEADERS {
            return Err(malformed("too many headers"));
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(malformed(format!("bad header {line:?}")));
        };
        if name.is_empty() || name.contains(|c: char| c.is_ascii_whitespace()) {
            return Err(malformed(format!("bad header {line:?}")));
        }
        headers.push((name.to_owned(), value.trim().to_owned()));
    }

    let chunked = match find_header(&headers, "Transfer-Encoding") {
        None => false,
        Some(coding) if coding.eq_ignore_ascii_case("chunked") => true,
        Some(coding) => return Err(malformed(format!("transfer coding {coding:?}"))),
    };
    let length = match find_header(&headers, "Content-Length") {
        Some(length) if !chunked => Some(
            length
                .parse::<u64>()
                .map_err(|_| malformed(format!("Content-Length {length:?}")))?,
        ),
        _ => None,
    };
    if find_header(&headers, "Expect").is_some_and(|v| v.eq_ignore_ascii_case("100-continue")) {
        (&*stream).write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
    }
    let body = if chunked {
        read_chunked(&mut reader)?
    } else if let Some(length) = length {
        read_exactly(&mut reader, length)?
    } else {
        Vec::new()
    };
    Ok(Some(Request {
        method,
        target,
        headers,
        body,
    }))
}

/// Appends exactly `length` bytes to a new buffer, or fails.
fn read_exactly(reader: &mut impl BufRead, length: u64) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    append_exactly(reader, length, &mut body)?;
    Ok(body)
}

fn append_exactly(reader: &mut impl BufRead, length: u64, body: &mut Vec<u8>) -> io::Result<()> {
    let read = (&mut *reader).take(length).read_to_end(body)?;
    if u64::try_from(read).ok() == Some(length) {
        Ok(())
    } else {
        Err(cut_short())
    }
}

/// A `Transfer-Encoding: chunked` body: hex size lines (extensions ignored),
/// each chunk followed by a line end, a `0` chunk, then trailers (ignored)
/// up to a blank line.
fn read_chunked(reader: &mut impl BufRead) -> io::Result<Vec<u8>> {
    let mut body = Vec::new();
    loop {
        let line = read_line(reader)?.ok_or_else(cut_short)?;
        let size = line.split_once(';').map_or(line.as_str(), |(size, _)| size);
        let size = u64::from_str_radix(size.trim(), 16)
            .map_err(|_| malformed(format!("bad chunk size {line:?}")))?;
        if size == 0 {
            break;
        }
        append_exactly(reader, size, &mut body)?;
        if !read_line(reader)?.ok_or_else(cut_short)?.is_empty() {
            return Err(malformed("chunk longer than its size"));
        }
    }
    while !read_line(reader)?.ok_or_else(cut_short)?.is_empty() {}
    Ok(body)
}

/// The reason phrase for the statuses this server sends. Clients ignore it.
fn reason(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        500 => "Internal Server Error",
        _ => "Status",
    }
}

/// Status line + headers + body, always `Connection: close`. The framing is
/// this function's: any `Content-Length`, `Transfer-Encoding` or
/// `Connection` in `headers` is replaced by its own.
fn write_response(
    stream: &mut TcpStream,
    status: u16,
    headers: &[(String, String)],
    body: &[u8],
) -> io::Result<()> {
    let mut head = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (name, value) in headers {
        let framing = ["Content-Length", "Transfer-Encoding", "Connection"]
            .iter()
            .any(|field| name.eq_ignore_ascii_case(field));
        if !framing {
            head.push_str(&format!("{name}: {value}\r\n"));
        }
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let mut out = head.into_bytes();
    out.extend_from_slice(body);
    stream.write_all(&out)?;
    stream.flush()
}

/// Ends a connection whose answer is written: says so (`FIN`), then reads
/// whatever the client still sends until it closes. Closing with unread
/// bytes would send a reset instead, which can destroy the answer before
/// the client reads it (a `400` sent before the body was read, say).
fn close(stream: &TcpStream) {
    if let Err(error) = stream.shutdown(Shutdown::Write) {
        if error.kind() != io::ErrorKind::NotConnected {
            eprintln!("HttpRemote: could not end the answer: {error}");
        }
        return;
    }
    let mut sink = [0u8; 4096];
    loop {
        match (&*stream).read(&mut sink) {
            Ok(n) if n > 0 => {}
            // The client closed — or an error: a timeout, a reset or `Drop`'s
            // shutdown. Either way the answer is out and the client is gone
            // or silent; nothing is left to do with it.
            Ok(_) | Err(_) => return,
        }
    }
}

impl Backend {
    /// Answers the one request on `stream`, then closes it.
    fn handle(&self, mut stream: TcpStream) {
        let reply = match read_request(&mut stream) {
            Ok(Some(request)) => self.answer(request),
            // Closed before a request — the client gave up, or `Drop` shut
            // the stream. Nothing to answer.
            Ok(None) => return,
            Err(error) if error.kind() == io::ErrorKind::InvalidData => {
                Reply::text(400, format!("bad request: {error}\n"))
            }
            Err(error) => {
                // A timeout or a client that hung up mid-request: there is
                // no one to answer. The client sees its own error; this line
                // says what the server saw.
                eprintln!("HttpRemote: reading a request: {error}");
                return;
            }
        };
        if let Err(error) = write_response(&mut stream, reply.status, &reply.headers, &reply.body) {
            // The client hung up before the answer went out — a killed
            // `git` does that. Nothing waits on this answer any more; the
            // test that killed the client asserts on the client instead.
            eprintln!("HttpRemote: could not send the response: {error}");
            return;
        }
        close(&stream);
    }

    /// Answers one request: a challenge, or whatever `http-backend` says.
    fn answer(&self, request: Request) -> Reply {
        let counters = &self.shared.counters;
        let authorised = request.header("Authorization") == Some(&self.expected_authorization);
        if !authorised {
            counters.rejected.fetch_add(1, Ordering::SeqCst);
            let mut reply = Reply::text(401, "authentication required\n");
            reply.headers.push((
                "WWW-Authenticate".to_owned(),
                "Basic realm=\"graft\"".to_owned(),
            ));
            return reply;
        }
        counters.authorised.fetch_add(1, Ordering::SeqCst);
        let line = format!("{} {}", request.method, request.target);
        self.cgi(request).unwrap_or_else(|error| {
            // The client sees a 500 and fails; this line says why.
            eprintln!("HttpRemote: {line}: {error}");
            Reply::text(500, error.to_string())
        })
    }

    /// Runs `git http-backend` for one authorised request.
    fn cgi(&self, request: Request) -> io::Result<Reply> {
        let (path, query) = request
            .target
            .split_once('?')
            .unwrap_or((&request.target, ""));
        let Some(rest) = path.strip_prefix(REPO_PATH) else {
            return Ok(Reply::text(404, "no such repository\n"));
        };
        let counters = &self.shared.counters;
        if request
            .header("Transfer-Encoding")
            .is_some_and(|v| v.eq_ignore_ascii_case("chunked"))
        {
            counters.chunked.fetch_add(1, Ordering::SeqCst);
        }
        let content_encoding = request.header("Content-Encoding");
        if content_encoding.is_some() {
            counters.gzipped.fetch_add(1, Ordering::SeqCst);
        }

        // The body is already de-chunked, so `http-backend` gets a plain
        // `CONTENT_LENGTH`.
        let mut command = hermetic_command(&self.home, &self.project_root, &["http-backend"]);
        command
            .env("GIT_PROJECT_ROOT", &self.project_root)
            .env("GIT_HTTP_EXPORT_ALL", "1")
            .env("PATH_INFO", format!("/{}{rest}", self.repo_name))
            .env("QUERY_STRING", query)
            .env("REQUEST_METHOD", &request.method)
            .env("CONTENT_LENGTH", request.body.len().to_string())
            .env("REMOTE_USER", &self.user)
            .env("REMOTE_ADDR", "127.0.0.1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(value) = request.header("Content-Type") {
            command.env("CONTENT_TYPE", value);
        }
        if let Some(value) = content_encoding {
            // `http-backend` inflates a gzipped body itself when told.
            command.env("HTTP_CONTENT_ENCODING", value);
        }
        if let Some(value) = request.header("Git-Protocol") {
            command.env("HTTP_GIT_PROTOCOL", value);
        }

        let output = self.run_backend(&mut command, request.body)?;
        parse_cgi(&output)
    }

    /// Runs `http-backend` with `body` on its stdin and returns its stdout.
    /// The backend gets its own process group, recorded in `running` while
    /// it runs, so `Drop` can kill it and everything it started.
    fn run_backend(&self, command: &mut Command, body: Vec<u8>) -> io::Result<Vec<u8>> {
        let child = command.process_group(0).spawn()?;
        let group = child.id();
        {
            let mut running = lock(&self.shared.running);
            if running.stopping {
                // `Drop` came between the spawn and here: it found nothing
                // to kill, so this does. The wait below then returns. If the
                // kill failed, `Drop`'s bounded wait is what notices: this
                // line only says why.
                if let Err(error) = kill_group(group) {
                    eprintln!("HttpRemote: {error}");
                }
            } else {
                running.groups.push(group);
            }
        }
        let output = feed_and_wait(child, body);
        // Between the reap above and this line a `Drop` could still kill
        // the group id; for it to name another process, the pid would have
        // to be reused as a group leader in that window. Accepted in a test
        // fixture.
        lock(&self.shared.running).groups.retain(|&g| g != group);
        output
    }
}

/// Feeds `body` to the child's stdin and waits for it to exit.
fn feed_and_wait(mut child: std::process::Child, body: Vec<u8>) -> io::Result<Vec<u8>> {
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("http-backend's stdin was not piped"))?;
    // Fed from its own thread: `http-backend` may start writing before it
    // has read everything, and both pipes are bounded.
    let feeder = thread::spawn(move || stdin.write_all(&body));
    let output = child.wait_with_output()?;
    let fed = feeder
        .join()
        .map_err(|_| io::Error::other("the thread feeding http-backend panicked"))?;
    if !output.status.success() {
        return Err(io::Error::other(format!(
            "http-backend exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    fed?;
    Ok(output.stdout)
}

/// Turns a CGI script's output — headers, a blank line, the body — into a
/// reply. `Status:` becomes the status code (200 if absent).
fn parse_cgi(output: &[u8]) -> io::Result<Reply> {
    let (head, body) = split_head(output)
        .ok_or_else(|| io::Error::other("http-backend's output had no end of headers"))?;
    let head = std::str::from_utf8(head).map_err(io::Error::other)?;
    let mut status = 200;
    let mut headers = Vec::new();
    for line in head.lines() {
        let Some((name, value)) = line.split_once(':') else {
            return Err(io::Error::other(format!("malformed CGI header {line:?}")));
        };
        let value = value.trim();
        if name.eq_ignore_ascii_case("Status") {
            status = value
                .split_whitespace()
                .next()
                .and_then(|code| code.parse().ok())
                .ok_or_else(|| io::Error::other(format!("malformed CGI status {value:?}")))?;
        } else {
            headers.push((name.to_owned(), value.to_owned()));
        }
    }
    Ok(Reply {
        status,
        headers,
        body: body.to_vec(),
    })
}

/// Splits CGI output at the blank line ending the headers, `\r\n` or `\n`.
fn split_head(output: &[u8]) -> Option<(&[u8], &[u8])> {
    let crlf = output
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|i| (i, 4));
    let lf = output.windows(2).position(|w| w == b"\n\n").map(|i| (i, 2));
    let (at, len) = match (crlf, lf) {
        (Some(a), Some(b)) => a.min(b),
        (a, b) => a.or(b)?,
    };
    Some((&output[..at], &output[at + len..]))
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

    #[test]
    fn cgi_status_line_sets_the_code() {
        let response = parse_cgi(b"Status: 404 Not Found\r\nContent-Type: text/plain\r\n\r\nnope")
            .expect("parses");
        assert_eq!(response.status, 404);
    }
}
