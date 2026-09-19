//! In-app credential prompts: git asks, Graft answers, nothing leaks.
//!
//! git (and ssh) ask for a username, password or passphrase by running the
//! program named in `GIT_ASKPASS` (`SSH_ASKPASS`) with the prompt as its one
//! argument and reading the answer from its stdout. Graft names **its own
//! binary** there, and tells that second copy of itself where to call back
//! through `GRAFT_ASKPASS_SOCKET` — an environment variable, not an argument,
//! because git may run the helper through `sh` and an argument cannot ride
//! in `GIT_ASKPASS` reliably. The helper half is [`respond`]; the Graft that
//! started git runs an [`AskPassServer`], which hands each prompt to an
//! [`AskPass`] handler (the app's modal prompt) and sends back its answer.
//!
//! The socket lives in a fresh directory only its owner can enter (`0700`),
//! removed on [`AskPassServer::close`]. A password travels exactly once, over
//! that socket, as a [`Secret`]: never an argument, never an environment
//! variable, never printed by `Debug`, zeroed when dropped.
//!
//! Wire format, one exchange per connection: the helper sends `u32` big-endian
//! prompt length + prompt bytes (at most 4 KiB); the server replies `u8`
//! status (`0` answer, `1` cancelled) + `u32` big-endian length + answer.

use std::fmt;
use std::io::{self, Read, Write};
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The longest prompt the server reads; a longer one closes the connection.
const PROMPT_MAX: u32 = 4 * 1024;

/// The longest answer [`respond`] accepts.
const ANSWER_MAX: u32 = 64 * 1024;

/// How long the server waits for a connected helper to send its prompt.
const PROMPT_WAIT: Duration = Duration::from_secs(5);

/// How often the listener, waiting on a handler, looks for a shutdown.
const SHUTDOWN_POLL: Duration = Duration::from_millis(20);

const ANSWER: u8 = 0;
const CANCELLED: u8 = 1;

/// Answers git's credential prompts. Called on the server's own thread, and
/// may block until the user answers.
pub trait AskPass: Send + Sync {
    /// The answer to `prompt` (`Password for 'https://host':`), or `None`
    /// when the user cancels.
    fn ask(&self, prompt: &str) -> Option<Secret>;
}

/// A credential in memory: never printed, never cloned, zeroed on drop.
///
/// Zeroing is best effort: it covers this value's own allocation (all of
/// its capacity), not copies made before it was built — build it straight
/// from the typed bytes.
pub struct Secret(Vec<u8>);

impl Secret {
    /// Takes ownership of `bytes`.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Secret {
        Secret(bytes)
    }

    /// The bytes, for the one place that must send them.
    #[must_use]
    pub fn expose(&self) -> &[u8] {
        &self.0
    }
}

impl Drop for Secret {
    fn drop(&mut self) {
        wipe(&mut self.0);
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(…)")
    }
}

/// Overwrites the whole allocation with zeros, spare capacity included.
///
/// The crate forbids `unsafe`, so this is not `ptr::write_volatile`; instead
/// `black_box` makes the zeroed buffer observable, which stops the compiler
/// from dropping the writes as dead stores before the free.
fn wipe(bytes: &mut Vec<u8>) {
    let capacity = bytes.capacity();
    bytes.clear();
    // `resize` within capacity never reallocates, so this writes zeros over
    // every byte the allocation holds.
    bytes.resize(capacity, 0);
    std::hint::black_box(&bytes[..]);
}

/// The Graft end of the socket: listens, hands each prompt to the handler,
/// replies. One listener thread; shut down by [`close`](Self::close) or drop.
pub struct AskPassServer {
    dir: PathBuf,
    socket: PathBuf,
    helper: PathBuf,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
    /// Set once the directory and socket are gone, so drop after `close`
    /// does nothing.
    closed: bool,
}

impl fmt::Debug for AskPassServer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AskPassServer")
            .field("socket", &self.socket)
            .field("helper", &self.helper)
            .finish_non_exhaustive()
    }
}

impl AskPassServer {
    /// Makes the private directory and socket and starts listening.
    /// `helper` is the program git will run to ask (Graft's own binary): the
    /// caller names it, the runner never guesses at one.
    ///
    /// # Errors
    /// The directory could not be made or the socket not bound.
    pub fn spawn(helper: PathBuf, handler: Arc<dyn AskPass>) -> io::Result<AskPassServer> {
        let dir = private_dir()?;
        let socket = dir.join("ask.sock");
        let listener = match UnixListener::bind(&socket) {
            Ok(listener) => listener,
            Err(error) => {
                // Report the bind failure; the empty directory we just made
                // is removed first, and if that fails too, that is the error
                // the caller needs more (a directory left behind).
                std::fs::remove_dir(&dir)?;
                return Err(error);
            }
        };
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let stop = Arc::clone(&stop);
            thread::Builder::new()
                .name("graft-askpass".to_owned())
                .spawn(move || listen(&listener, &handler, &stop))?
        };
        Ok(AskPassServer {
            dir,
            socket,
            helper,
            stop,
            thread: Some(thread),
            closed: false,
        })
    }

    /// Where the helper connects: the value of `GRAFT_ASKPASS_SOCKET`.
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    /// The program git runs to ask: the value of `GIT_ASKPASS`.
    #[must_use]
    pub fn helper(&self) -> &Path {
        &self.helper
    }

    /// Stops listening, joins the thread, removes the socket and directory.
    /// A helper still waiting for an answer gets an error. Prefer this to
    /// dropping, which cannot report a failure.
    ///
    /// # Errors
    /// The first step that failed; later steps are still attempted.
    pub fn close(mut self) -> io::Result<()> {
        self.shutdown()
    }

    fn shutdown(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.stop.store(true, Ordering::SeqCst);
        let mut first: Option<io::Error> = None;
        let mut note = |result: io::Result<()>| {
            if let Err(error) = result
                && first.is_none()
            {
                first = Some(error);
            }
        };
        // Wake the listener out of `accept`. If even that fails the thread
        // cannot be woken, so it is left detached rather than joined forever;
        // it exits at its next connection.
        let woke = UnixStream::connect(&self.socket).map(drop);
        let could_wake = woke.is_ok();
        note(woke);
        if let Some(thread) = self.thread.take()
            && could_wake
        {
            note(
                thread
                    .join()
                    .map_err(|_| io::Error::other("the askpass listener panicked")),
            );
        }
        note(std::fs::remove_file(&self.socket));
        note(std::fs::remove_dir(&self.dir));
        first.map_or(Ok(()), Err)
    }
}

impl Drop for AskPassServer {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            // Safe to ignore here, and only here: `close` is the path that
            // reports. What can fail is removing an empty 0700 directory in
            // the temp dir (or its socket), which exposes nothing and costs
            // the user nothing they would see; drop has no one to tell.
            drop(error);
        }
    }
}

/// `temp_dir()/graft-askpass-<pid>-<n>`, created `0700`. `create` (not
/// `create_all`) fails if the name exists, so a planted directory or symlink
/// is never used; a clash just tries the next name.
fn private_dir() -> io::Result<PathBuf> {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = std::env::temp_dir();
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.subsec_nanos());
    let mut last = None;
    for _ in 0..16 {
        let n = seed
            ^ NEXT
                .fetch_add(1, Ordering::Relaxed)
                .wrapping_mul(0x9E37_79B9);
        let dir = base.join(format!("graft-askpass-{}-{n:08x}", std::process::id()));
        match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok(dir),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => last = Some(error),
            Err(error) => return Err(error),
        }
    }
    Err(last.unwrap_or_else(|| io::Error::other("no free askpass directory name")))
}

/// The listener thread: one connection at a time (git asks one question at a
/// time), until shut down.
fn listen(listener: &UnixListener, handler: &Arc<dyn AskPass>, stop: &AtomicBool) {
    for stream in listener.incoming() {
        if stop.load(Ordering::SeqCst) {
            return;
        }
        match stream {
            Ok(stream) => {
                if let Err(error) = serve(stream, handler, stop) {
                    // One helper's broken exchange (it hung up, sent a bad
                    // frame) is that helper's failure: it exits non-zero and
                    // git reports the credential as unreadable. The server
                    // stays up for the next ask.
                    drop(error);
                }
            }
            Err(error) => {
                // `accept` failing (out of descriptors, say) is likewise
                // the waiting helper's failure, which it sees as a refused
                // or dropped connection.
                drop(error);
            }
        }
    }
}

/// One exchange. The handler runs on its own thread so a shutdown is noticed
/// while it blocks: the helper's connection is then dropped (it sees EOF and
/// fails) and the listener returns. The handler thread finishes whenever the
/// handler returns — the app's handler returns `None` once the job is
/// cancelled — and its answer is dropped, and so zeroed, unsent.
fn serve(mut stream: UnixStream, handler: &Arc<dyn AskPass>, stop: &AtomicBool) -> io::Result<()> {
    stream.set_read_timeout(Some(PROMPT_WAIT))?;
    let prompt = read_prompt(&mut stream)?;
    let (answered, answer) = mpsc::channel();
    let handler = Arc::clone(handler);
    // Detached, not scoped: a scope would make shutdown wait for a handler
    // that may be blocked on a user who has gone.
    thread::Builder::new()
        .name("graft-askpass-ask".to_owned())
        .spawn(move || {
            let reply = handler.ask(&prompt);
            if let Err(unwanted) = answered.send(reply) {
                // The listener stopped waiting (shutdown): drop the answer,
                // which zeroes it.
                drop(unwanted);
            }
        })?;
    loop {
        match answer.recv_timeout(SHUTDOWN_POLL) {
            Ok(reply) => return write_reply(&mut stream, reply.as_ref()),
            Err(RecvTimeoutError::Timeout) if stop.load(Ordering::SeqCst) => {
                // The helper sees EOF and exits non-zero; git gives up.
                return stream.shutdown(std::net::Shutdown::Both);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => {
                return Err(io::Error::other("the askpass handler panicked"));
            }
        }
    }
}

fn read_prompt(stream: &mut UnixStream) -> io::Result<String> {
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

fn write_reply(stream: &mut UnixStream, reply: Option<&Secret>) -> io::Result<()> {
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

#[cfg(test)]
mod tests;
