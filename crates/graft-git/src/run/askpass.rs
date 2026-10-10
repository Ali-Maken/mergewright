//! In-app credential prompts: git asks, Graft answers, nothing leaks.
//!
//! git (and ssh) ask for a username, password or passphrase by running the
//! program named in `GIT_ASKPASS` (`SSH_ASKPASS`) with the prompt as its one
//! argument and reading the answer from its stdout. Graft names **its own
//! binary** there, and tells that second copy of itself where to call back
//! through `TRIB_ASKPASS_SOCKET` — an environment variable, not an argument,
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

mod peer;
mod secret;
mod shared;
mod wire;

pub use secret::Secret;
pub use wire::{RespondError, respond};

#[cfg(test)]
use secret::wipe;
use shared::{Reply, Shared, Wake};
use wire::{read_prompt, write_reply};

use std::fmt;
use std::io;
use std::os::unix::fs::DirBuilderExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long the server waits for a connected helper to send its prompt.
const PROMPT_WAIT: Duration = Duration::from_secs(5);

/// How long the server waits to hand its answer to a helper that has stopped
/// reading. Bounds the one write in the exchange, so `close` never waits on a
/// stuck helper for longer than this.
#[cfg(not(test))]
const REPLY_WAIT: Duration = Duration::from_secs(5);
/// Short in tests, so the stuck-helper test does not wait five seconds.
#[cfg(test)]
const REPLY_WAIT: Duration = Duration::from_millis(300);

/// Answers git's credential prompts. Called on the server's own thread, and
/// may block until the user answers.
pub trait AskPass: Send + Sync {
    /// The answer to `prompt` (`Password for 'https://host':`), or `None`
    /// when the user cancels.
    fn ask(&self, prompt: &str) -> Option<Secret>;
}

/// The Graft end of the socket: listens, hands each prompt to the handler,
/// replies. One listener thread; shut down by [`close`](Self::close) or drop.
pub struct AskPassServer {
    dir: PathBuf,
    socket: PathBuf,
    helper: PathBuf,
    shared: Arc<Shared>,
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
        Self::spawn_owned_by(helper, handler, nix::unistd::geteuid().as_raw())
    }

    /// [`spawn`](Self::spawn), serving only peers whose uid is `owner`. Real
    /// use passes our own effective uid; a test passes another, to be
    /// "another user" to the server without needing root.
    fn spawn_owned_by(
        helper: PathBuf,
        handler: Arc<dyn AskPass>,
        owner: u32,
    ) -> io::Result<AskPassServer> {
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
        let shared = Arc::new(Shared::default());
        let thread = {
            let shared = Arc::clone(&shared);
            thread::Builder::new()
                .name("graft-askpass".to_owned())
                .spawn(move || listen(&listener, &handler, &shared, owner))?
        };
        Ok(AskPassServer {
            dir,
            socket,
            helper,
            shared,
            thread: Some(thread),
            closed: false,
        })
    }

    /// Where the helper connects: the value of `TRIB_ASKPASS_SOCKET`.
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
    /// The listener could not be woken: it is still running, and the socket
    /// and directory are left in place (removing the socket would make it
    /// unwakeable for good). Otherwise the first step that failed; later
    /// steps are still attempted. Last, a peer of another uid was refused
    /// (`PermissionDenied`: the first refusal and the count) — reported here,
    /// never written to the terminal the TUI owns.
    /// The refusal's `io::Error` carries a [`Refused`] payload: read it with
    /// [`refused`], never by kind (a cleanup failure can be
    /// `PermissionDenied` too).
    pub fn close(mut self) -> io::Result<()> {
        self.shutdown()
    }

    /// Order matters: stop flag, wake, join, and only then remove the socket
    /// file — removing it first would leave no way to wake the listener.
    fn shutdown(&mut self) -> io::Result<()> {
        if self.closed {
            return Ok(());
        }
        self.closed = true;
        self.shared.stop();
        let Some(thread) = self.thread.take() else {
            return Ok(());
        };
        // Wake the listener out of its blocking `accept`; it sees `stop` and
        // returns. Everything it can be doing instead is bounded (prompt read
        // and reply write have timeouts; a wait on a blocked handler was just
        // ended by `Shared::stop`), so once woken the join returns.
        let cleanup = match UnixStream::connect(&self.socket) {
            Err(error) => Err(io::Error::new(
                error.kind(),
                format!(
                    "could not wake the askpass listener at {}: {error}; \
                     its thread is still running and the socket is left in place",
                    self.socket.display()
                ),
            )),
            Ok(woke) => {
                // Its only job was to wake `accept`; closed before the join,
                // as before.
                drop(woke);
                Self::clean_up(thread, &self.socket, &self.dir)
            }
        };
        // A refusal wins: it is the event he must hear about, and a cleanup
        // failure rides inside it rather than hiding it (20b R3 amendment 2).
        match (self.shared.refusals(), cleanup) {
            (Some(refused), cleanup) => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                match cleanup {
                    Err(error) => refused.with_cleanup(error),
                    Ok(()) => refused,
                },
            )),
            (None, cleanup) => cleanup,
        }
    }

    /// Join the listener, then remove the socket and the directory: every
    /// step attempted, the first failure returned.
    fn clean_up(thread: JoinHandle<()>, socket: &Path, dir: &Path) -> io::Result<()> {
        let mut first: Option<io::Error> = None;
        let mut note = |result: io::Result<()>| {
            if let Err(error) = result
                && first.is_none()
            {
                first = Some(error);
            }
        };
        note(
            thread
                .join()
                .map_err(|_| io::Error::other("the askpass listener panicked")),
        );
        note(std::fs::remove_file(socket));
        note(std::fs::remove_dir(dir));
        first.map_or(Ok(()), Err)
    }
}

impl Drop for AskPassServer {
    fn drop(&mut self) {
        if let Err(error) = self.shutdown() {
            // Drop has no one to tell — `close` is the path that reports, and
            // callers that can act on a failure use it. What is lost here:
            // either an empty 0700 directory (or its socket) left in the temp
            // dir, which exposes nothing; or, if the wake-up connect failed
            // (descriptors exhausted), one idle listener thread blocked in
            // `accept` on that private socket for the rest of the process.
            // It holds no credential and answers nothing: `stop` is set, so
            // any later connection is closed unanswered.
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
///
/// `stop` is checked after every `accept`. Whatever was accepted once it is
/// set — the shutdown's wake-up, or a real helper that connected in the same
/// moment and was queued ahead of it — is dropped unanswered: the helper sees
/// EOF and fails, it is never given an answer by a server that is closing.
///
/// A connection from any uid but `owner` is closed unread and unanswered,
/// recorded for [`close`](AskPassServer::close) to report: the `0700` directory already keeps other users
/// out, so only a peer that got past it (root, or a bug in that guard) is
/// ever refused here.
fn listen(listener: &UnixListener, handler: &Arc<dyn AskPass>, shared: &Shared, owner: u32) {
    for stream in listener.incoming() {
        if shared.stopped() {
            drop(stream);
            return;
        }
        match stream {
            Ok(stream) => {
                if let Err(refusal) = peer::admit(peer::peer_uid(&stream), owner) {
                    // Never written to stderr: the TUI owns the terminal.
                    shared.refuse(&refusal);
                    drop(stream);
                    continue;
                }
                if let Err(error) = serve(stream, handler, shared) {
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
/// while it blocks: the wait is one blocking `recv`, ended by the answer or
/// by shutdown's [`Wake::Stop`], whichever comes first. On a stop the
/// helper's connection is dropped (it sees EOF and fails) and the listener
/// returns. The handler thread finishes whenever the handler returns — the
/// app's handler returns `None` once the job is cancelled — and its answer
/// is dropped, and so zeroed, unsent.
fn serve(mut stream: UnixStream, handler: &Arc<dyn AskPass>, shared: &Shared) -> io::Result<()> {
    stream.set_read_timeout(Some(PROMPT_WAIT))?;
    stream.set_write_timeout(Some(REPLY_WAIT))?;
    let prompt = read_prompt(&mut stream)?;
    let (wake, woken) = mpsc::channel();
    {
        let mut waiting = shared.waiting();
        if shared.stopped() {
            // Shut down while the prompt was arriving: do not ask the user.
            return stream.shutdown(std::net::Shutdown::Both);
        }
        *waiting = Some(wake.clone());
    }
    let handler = Arc::clone(handler);
    let reply = Reply(Some(wake));
    // Detached, not scoped: a scope would make shutdown wait for a handler
    // that may be blocked on a user who has gone.
    let spawned = thread::Builder::new()
        .name("graft-askpass-ask".to_owned())
        .spawn(move || {
            let answer = handler.ask(&prompt);
            reply.send(answer);
        });
    let outcome = spawned.map(|_| woken.recv());
    // The exchange is over either way: shutdown has nothing left to wake.
    shared.waiting().take();
    match outcome? {
        Ok(Wake::Answer(answer)) => write_reply(&mut stream, answer.as_ref()),
        // The helper sees EOF and exits non-zero; git gives up.
        Ok(Wake::Stop) => stream.shutdown(std::net::Shutdown::Both),
        Ok(Wake::Panicked) => Err(io::Error::other("the askpass handler panicked")),
        // The slot held a sender until the line above, so this cannot
        // happen; reported rather than assumed.
        Err(mpsc::RecvError) => Err(io::Error::other("the askpass wait lost every sender")),
    }
}

/// Peers of another uid were turned away: the first refusal's words and how
/// many in all. The payload of [`AskPassServer::close`]'s refusal error.
#[derive(Debug)]
pub struct Refused {
    first: String,
    count: usize,
    cleanup: Option<io::Error>,
}

impl Refused {
    /// A refusal record, for callers' tests: real ones come only from
    /// [`AskPassServer::close`], since another uid is needed to make one.
    #[must_use]
    pub fn new(first: impl Into<String>, count: usize) -> Self {
        Self {
            first: first.into(),
            count,
            cleanup: None,
        }
    }

    /// The same refusal, with the cleanup failure that happened alongside it.
    #[must_use]
    pub fn with_cleanup(self, error: io::Error) -> Self {
        Self {
            cleanup: Some(error),
            ..self
        }
    }

    /// The socket's cleanup failure, when it failed as well as the refusal.
    #[must_use]
    pub fn cleanup(&self) -> Option<&io::Error> {
        self.cleanup.as_ref()
    }

    /// How many connections were refused.
    #[must_use]
    pub fn count(&self) -> usize {
        self.count
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}; {} refused in all", self.first, self.count)
    }
}

impl std::error::Error for Refused {}

/// The [`Refused`] payload `error` carries, if it is a refusal.
#[must_use]
pub fn refused(error: &io::Error) -> Option<&Refused> {
    error.get_ref()?.downcast_ref()
}

#[cfg(test)]
mod tests;
