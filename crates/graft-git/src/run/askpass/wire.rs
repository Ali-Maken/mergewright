//! The wire format both halves speak, and the helper half ([`respond`]).

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;

use super::Secret;

/// The longest prompt the server reads; a longer one closes the connection.
pub(super) const PROMPT_MAX: u32 = 4 * 1024;

/// The longest answer [`respond`] accepts.
pub(super) const ANSWER_MAX: u32 = 64 * 1024;

pub(super) const ANSWER: u8 = 0;
pub(super) const CANCELLED: u8 = 1;

pub(super) fn read_prompt(stream: &mut UnixStream) -> io::Result<String> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len)?;
    let len = u32::from_be_bytes(len);
    if len > PROMPT_MAX {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "askpass prompt over 4 KiB",
        ));
    }
    let mut prompt = vec![0; len as usize];
    stream.read_exact(&mut prompt)?;
    Ok(String::from_utf8_lossy(&prompt).into_owned())
}

pub(super) fn write_reply(stream: &mut UnixStream, reply: Option<&Secret>) -> io::Result<()> {
    let Some(secret) = reply else {
        stream.write_all(&[CANCELLED, 0, 0, 0, 0])?;
        return stream.flush();
    };
    let len = u32::try_from(secret.expose().len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "answer too long"))?;
    let mut header = [ANSWER, 0, 0, 0, 0];
    header[1..].copy_from_slice(&len.to_be_bytes());
    stream.write_all(&header)?;
    // Written straight from the secret: no copy to wipe.
    stream.write_all(secret.expose())?;
    stream.flush()
}

/// Why the helper half could not answer git.
#[derive(Debug)]
pub enum RespondError {
    /// Talking to the socket failed, or Graft went away mid-question.
    Io(io::Error),
    /// The user cancelled; the helper should exit non-zero.
    Cancelled,
    /// The server sent something this helper cannot read.
    Protocol(&'static str),
}

impl fmt::Display for RespondError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RespondError::Io(error) => write!(f, "askpass: {error}"),
            RespondError::Cancelled => f.write_str("askpass: cancelled"),
            RespondError::Protocol(what) => write!(f, "askpass: {what}"),
        }
    }
}

impl std::error::Error for RespondError {}

impl From<io::Error> for RespondError {
    fn from(error: io::Error) -> Self {
        RespondError::Io(error)
    }
}

/// The helper half, run by Graft's binary in askpass mode: sends `prompt` to
/// the server at `socket`, writes the answer and a newline to `out` (git
/// reads the helper's stdout). Nothing else is written anywhere.
///
/// # Errors
/// [`RespondError::Cancelled`] when the user cancelled; `Io` or `Protocol`
/// when the exchange failed. Either way the helper should exit non-zero.
pub fn respond(socket: &Path, prompt: &str, out: &mut dyn Write) -> Result<(), RespondError> {
    let prompt = prompt.as_bytes();
    let len = u32::try_from(prompt.len())
        .ok()
        .filter(|len| *len <= PROMPT_MAX)
        .ok_or(RespondError::Protocol("prompt over 4 KiB"))?;
    let mut stream = UnixStream::connect(socket)?;
    stream.write_all(&len.to_be_bytes())?;
    stream.write_all(prompt)?;
    stream.flush()?;
    let mut header = [0u8; 5];
    stream.read_exact(&mut header)?;
    let [status, len @ ..] = header;
    match status {
        CANCELLED => return Err(RespondError::Cancelled),
        ANSWER => {}
        _ => return Err(RespondError::Protocol("unknown askpass status")),
    }
    let len = u32::from_be_bytes(len);
    if len > ANSWER_MAX {
        return Err(RespondError::Protocol("askpass answer over 64 KiB"));
    }
    let mut answer = Secret::new(vec![0; len as usize]);
    stream.read_exact(&mut answer.0)?;
    out.write_all(answer.expose())?;
    out.write_all(b"\n")?;
    out.flush()?;
    Ok(())
}
