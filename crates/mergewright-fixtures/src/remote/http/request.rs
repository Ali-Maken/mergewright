//! HTTP/1.1 framing: reading one request, writing one response.

use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};

use super::{MAX_HEADERS, MAX_LINE};

/// One HTTP request, body already de-chunked.
pub(super) struct Request {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Request {
    /// The first header called `name`, compared case-insensitively.
    pub(super) fn header(&self, name: &str) -> Option<&str> {
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
pub(super) struct Reply {
    pub(super) status: u16,
    pub(super) headers: Vec<(String, String)>,
    pub(super) body: Vec<u8>,
}

impl Reply {
    pub(super) fn text(status: u16, body: impl Into<String>) -> Reply {
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
pub(super) fn read_request(stream: &mut TcpStream) -> io::Result<Option<Request>> {
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
pub(super) fn write_response(
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
pub(super) fn close(stream: &TcpStream) {
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
