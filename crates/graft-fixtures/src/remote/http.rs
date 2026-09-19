//! A bare repository behind git's smart HTTP protocol, with Basic auth.
//!
//! [`tiny_http`] listens on `127.0.0.1:0`; every authorised request is handed
//! to `git http-backend` as a CGI script, which is exactly what a real Git
//! server does behind Apache or nginx. Plain HTTP only — no TLS (see the
//! module above).

use std::fmt;
use std::io::{self, Write};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use tiny_http::{Header, Request, Response, Server};

use super::{hermetic_command, run_git};

/// How long [`Drop`] waits for the listener thread, after ending any
/// `http-backend` it is running, before declaring the teardown failed.
///
/// A test's teardown must never hang, and must never quietly leave a thread
/// behind either: past this, drop panics (the test fails, saying why) rather
/// than return with the thread still running.
const STOP_BOUND: Duration = Duration::from_secs(2);

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
    server: Arc<Server>,
    thread: Option<JoinHandle<()>>,
    /// Disconnects when the listener thread ends, however it ends: the one
    /// sender lives in the thread. In a `Mutex` only so the fixture stays
    /// `Sync`.
    stopped: Mutex<Receiver<()>>,
    running: Arc<Mutex<Running>>,
    port: u16,
    user: String,
    counters: Arc<Counters>,
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

/// The `http-backend` the listener is waiting on, so [`Drop`] can end it.
///
/// The backend runs in its own process group, and the whole group is killed:
/// `http-backend` hands a push to `receive-pack`, which runs the repository's
/// hooks, and each of them holds the output pipe the listener reads to EOF.
#[derive(Debug, Default)]
struct Running {
    /// Set by `Drop`: a backend started after this is killed at once.
    stopping: bool,
    /// The running backend's process group id (its pid).
    group: Option<u32>,
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
/// the two fields are still sound.
fn lock(running: &Mutex<Running>) -> std::sync::MutexGuard<'_, Running> {
    running.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Everything the listener thread needs to answer a request.
struct Backend {
    /// The directory holding the bare repository: `GIT_PROJECT_ROOT`.
    project_root: PathBuf,
    /// The bare repository's directory name, which `/repo.git` maps to.
    repo_name: String,
    /// The whole `Authorization` header value that is accepted.
    expected_authorization: String,
    user: String,
    home: PathBuf,
    counters: Arc<Counters>,
    running: Arc<Mutex<Running>>,
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

        let server = Server::http("127.0.0.1:0").map_err(io::Error::other)?;
        let port = server
            .server_addr()
            .to_ip()
            .ok_or_else(|| io::Error::other("tiny_http bound a non-IP address"))?
            .port();
        let server = Arc::new(server);
        let counters = Arc::new(Counters::default());
        let running = Arc::new(Mutex::new(Running::default()));
        let backend = Backend {
            project_root: project_root.to_path_buf(),
            repo_name: repo_name.to_string_lossy().into_owned(),
            expected_authorization: format!(
                "Basic {}",
                base64(format!("{user}:{password}").as_bytes())
            ),
            user: user.to_owned(),
            home: home.path().to_path_buf(),
            counters: Arc::clone(&counters),
            running: Arc::clone(&running),
        };
        let listening = Arc::clone(&server);
        let (finished, stopped) = mpsc::channel::<()>();
        let thread = thread::Builder::new()
            .name(format!("http-remote-{port}"))
            .spawn(move || {
                // Owned here so it drops — disconnecting `stopped` — when the
                // thread ends, by returning or by unwinding.
                let _finished = finished;
                // Ends when `Drop` calls `unblock`: `recv` then returns an
                // error and the iterator stops.
                for request in listening.incoming_requests() {
                    backend.answer(request);
                }
            })?;

        Ok(HttpRemote {
            server,
            thread: Some(thread),
            stopped: Mutex::new(stopped),
            running,
            port,
            user: user.to_owned(),
            counters,
            _home: home,
        })
    }

    /// `http://<user>@127.0.0.1:<port>/repo.git` — git asks only for the
    /// password.
    pub fn url(&self) -> String {
        format!("http://{}@127.0.0.1:{}{REPO_PATH}", self.user, self.port)
    }

    /// `http://127.0.0.1:<port>/repo.git` — git asks for user and password.
    pub fn url_anonymous(&self) -> String {
        format!("http://127.0.0.1:{}{REPO_PATH}", self.port)
    }

    /// The port the server listens on.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Requests seen so far: (authorised, rejected with `401`).
    ///
    /// Git's first request never carries credentials, so even a clone with
    /// the right password counts one rejection.
    pub fn requests(&self) -> (usize, usize) {
        (
            self.counters.authorised.load(Ordering::SeqCst),
            self.counters.rejected.load(Ordering::SeqCst),
        )
    }

    /// Authorised requests whose body came chunked.
    #[cfg(test)]
    pub(crate) fn chunked_requests(&self) -> usize {
        self.counters.chunked.load(Ordering::SeqCst)
    }

    /// Authorised requests whose body came gzipped.
    #[cfg(test)]
    pub(crate) fn gzipped_requests(&self) -> usize {
        self.counters.gzipped.load(Ordering::SeqCst)
    }
}

impl Drop for HttpRemote {
    fn drop(&mut self) {
        self.server.unblock();
        let Some(thread) = self.thread.take() else {
            return;
        };
        // The listener may be inside a request, waiting on an `http-backend`
        // that will never finish (a hung hook, say). End it; no backend
        // started from here on survives either.
        let killed = {
            let mut running = lock(&self.running);
            running.stopping = true;
            running.group.take().map(kill_group)
        };
        let stopped = self
            .stopped
            .get_mut()
            .unwrap_or_else(PoisonError::into_inner);
        match stopped.recv_timeout(STOP_BOUND) {
            Err(RecvTimeoutError::Disconnected) => {}
            Ok(()) => unreachable!("the listener thread never sends, it only disconnects"),
            Err(RecvTimeoutError::Timeout) => {
                // Stuck even with its backend killed: something outside the
                // backend's process group holds the request open. Returning
                // would leave the thread (and its port) running for the rest
                // of the test binary, silently. Fail the test instead — unless
                // it is already failing, where a second panic would abort.
                let message = format!(
                    "HttpRemote on port {}: listener thread still running {STOP_BOUND:?} \
                     after its backend was killed ({killed:?}); teardown failed",
                    self.port
                );
                if thread::panicking() {
                    eprintln!("{message}");
                    return;
                }
                panic!("{message}");
            }
        }
        if thread.join().is_err() {
            eprintln!("HttpRemote on port {}: listener thread panicked", self.port);
        }
        // `self.server` drops after this; `Server::drop` closes the listening
        // socket, so the port is free once `drop` returns.
    }
}

impl Backend {
    /// Answers one request: a challenge, or whatever `http-backend` says.
    fn answer(&self, mut request: Request) {
        let authorised = request
            .headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .is_some_and(|h| h.value.as_str() == self.expected_authorization);
        let response = if authorised {
            self.counters.authorised.fetch_add(1, Ordering::SeqCst);
            match self.cgi(&mut request) {
                Ok(response) => response,
                Err(error) => {
                    // The client sees a 500 and fails; this line says why.
                    eprintln!(
                        "HttpRemote: {} {}: {error}",
                        request.method(),
                        request.url()
                    );
                    Response::from_string(error.to_string()).with_status_code(500)
                }
            }
        } else {
            self.counters.rejected.fetch_add(1, Ordering::SeqCst);
            Response::from_string("authentication required\n")
                .with_status_code(401)
                .with_header(header("WWW-Authenticate", "Basic realm=\"graft\""))
        };
        if let Err(error) = request.respond(response) {
            // The client hung up before the answer went out — a killed
            // `git` does that. Nothing waits on this answer any more; the
            // test that killed the client asserts on the client instead.
            eprintln!("HttpRemote: could not send the response: {error}");
        }
    }

    /// Runs `git http-backend` for one authorised request.
    fn cgi(&self, request: &mut Request) -> io::Result<Response<io::Cursor<Vec<u8>>>> {
        let url = request.url().to_owned();
        let (path, query) = url.split_once('?').unwrap_or((&url, ""));
        let Some(rest) = path.strip_prefix(REPO_PATH) else {
            return Ok(Response::from_string("no such repository\n").with_status_code(404));
        };
        let method = request.method().as_str().to_owned();
        let find = |name: &'static str| {
            request
                .headers()
                .iter()
                .find(|h| h.field.equiv(name))
                .map(|h| h.value.as_str().to_owned())
        };
        let content_type = find("Content-Type");
        let content_encoding = find("Content-Encoding");
        let git_protocol = find("Git-Protocol");
        if find("Transfer-Encoding").is_some_and(|v| v.eq_ignore_ascii_case("chunked")) {
            self.counters.chunked.fetch_add(1, Ordering::SeqCst);
        }
        if content_encoding.is_some() {
            self.counters.gzipped.fetch_add(1, Ordering::SeqCst);
        }

        // tiny_http's reader has already undone any chunked transfer
        // encoding, so reading it whole gives the exact body and its length;
        // `http-backend` then gets a plain `CONTENT_LENGTH`.
        let mut body = Vec::new();
        request.as_reader().read_to_end(&mut body)?;

        let mut command = hermetic_command(&self.home, &self.project_root, &["http-backend"]);
        command
            .env("GIT_PROJECT_ROOT", &self.project_root)
            .env("GIT_HTTP_EXPORT_ALL", "1")
            .env("PATH_INFO", format!("/{}{rest}", self.repo_name))
            .env("QUERY_STRING", query)
            .env("REQUEST_METHOD", &method)
            .env("CONTENT_LENGTH", body.len().to_string())
            .env("REMOTE_USER", &self.user)
            .env("REMOTE_ADDR", "127.0.0.1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(value) = &content_type {
            command.env("CONTENT_TYPE", value);
        }
        if let Some(value) = &content_encoding {
            // `http-backend` inflates a gzipped body itself when told.
            command.env("HTTP_CONTENT_ENCODING", value);
        }
        if let Some(value) = &git_protocol {
            command.env("HTTP_GIT_PROTOCOL", value);
        }

        parse_cgi(&self.run_backend(&mut command, body)?)
    }

    /// Runs `http-backend` with `body` on its stdin and returns its stdout.
    /// The backend gets its own process group, recorded in `running` while
    /// it runs, so `Drop` can kill it and everything it started.
    fn run_backend(&self, command: &mut Command, body: Vec<u8>) -> io::Result<Vec<u8>> {
        let child = command.process_group(0).spawn()?;
        {
            let mut running = lock(&self.running);
            if running.stopping {
                // `Drop` came between the spawn and here: it found nothing
                // to kill, so this does. The wait below then returns. If the
                // kill failed, `Drop`'s bounded wait is what notices: this
                // line only says why.
                if let Err(error) = kill_group(child.id()) {
                    eprintln!("HttpRemote: {error}");
                }
            } else {
                running.group = Some(child.id());
            }
        }
        let output = feed_and_wait(child, body);
        // Between the reap above and this line a `Drop` could still kill
        // the group id; for it to name another process, the pid would have
        // to be reused as a group leader in that window. Accepted in a test
        // fixture.
        lock(&self.running).group = None;
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
/// response. `Status:` becomes the status code (200 if absent).
fn parse_cgi(output: &[u8]) -> io::Result<Response<io::Cursor<Vec<u8>>>> {
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
            headers.push(header(name, value));
        }
    }
    let mut response = Response::from_data(body.to_vec()).with_status_code(status);
    for h in headers {
        response.add_header(h);
    }
    Ok(response)
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

/// A header built from strings this module controls or `http-backend`
/// wrote. Both are ASCII, so the conversion cannot fail in practice; if it
/// ever does, the header is `X-Graft-Bad-Header` and the test sees it.
fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name.as_bytes().to_vec(), value.as_bytes().to_vec()).unwrap_or_else(|()| {
        Header::from_bytes(b"X-Graft-Bad-Header".to_vec(), name.as_bytes().to_vec())
            .unwrap_or_else(|()| unreachable!("a fixed ASCII header name is valid"))
    })
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
        assert_eq!(response.status_code().0, 404);
    }
}
