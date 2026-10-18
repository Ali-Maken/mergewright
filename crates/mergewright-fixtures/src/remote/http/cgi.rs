//! The `git http-backend` CGI bridge.

use std::io::{self, Write};
use std::net::TcpStream;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::thread;

use super::super::hermetic_command;
use super::listener::kill_group;
use super::request::{Reply, Request, close, read_request, write_response};
use super::{REPO_PATH, Shared, lock};

/// Everything a handler needs to answer a request.
pub(super) struct Backend {
    /// The directory holding the bare repository: `GIT_PROJECT_ROOT`.
    pub(super) project_root: PathBuf,
    /// The bare repository's directory name, which `/repo.git` maps to.
    pub(super) repo_name: String,
    /// The whole `Authorization` header value that is accepted.
    pub(super) expected_authorization: String,
    pub(super) user: String,
    pub(super) home: PathBuf,
    pub(super) shared: Arc<Shared>,
}

impl Backend {
    /// Answers the one request on `stream`, then closes it.
    pub(super) fn handle(&self, mut stream: TcpStream) {
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

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn cgi_status_line_sets_the_code() {
        let response = parse_cgi(b"Status: 404 Not Found\r\nContent-Type: text/plain\r\n\r\nnope")
            .expect("parses");
        assert_eq!(response.status, 404);
    }
}
