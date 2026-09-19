//! Running an invocation that talks to a remote: stderr streamed and kept
//! whole for [`classify_failure`], progress forwarded at most every 100 ms,
//! and credential prompts answered through an [`AskPassServer`] that lives
//! exactly as long as the child.
//!
//! Local invocations never come here; `run_plan` in the parent runs them as
//! it always has.

use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use graft_core::journal::{JournalEntry, OperationKind};
use graft_git::remote::{RemoteFailure, RemoteProgress, classify_failure, parse_progress};
use graft_git::run::askpass::{AskPass, AskPassServer};
use graft_git::run::{GitInvocation, RunError};

use super::{AfterState, Context};
use crate::operation::{Invocation, RemoteNames};

/// What a remote operation needs from the app: somewhere to show progress
/// and someone to answer a credential prompt. `Context.remote = None` is
/// headless — no prompt (git fails auth instead), no progress — which is
/// what tests and scripted runs want.
pub struct RemoteHooks {
    /// Called with parsed progress, at most every 100 ms, from a thread the
    /// executor owns (not the caller's).
    pub progress: Box<dyn Fn(RemoteProgress) + Send + Sync>,
    /// Answers git's `Username for …`/`Password for …` prompts.
    pub askpass: Arc<dyn AskPass>,
    /// The askpass helper git runs: Graft's own binary, which calls back
    /// over the server's socket.
    pub helper: PathBuf,
}

impl std::fmt::Debug for RemoteHooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RemoteHooks")
            .field("helper", &self.helper)
            .finish_non_exhaustive()
    }
}

/// What [`GitInvocation::on_stderr_line`] takes.
type Sink = Box<dyn FnMut(&[u8]) + Send>;

/// The least time between two progress calls.
const PROGRESS_EVERY: Duration = Duration::from_millis(100);

/// How many trailing stderr segments an error carries.
const TAIL: usize = 8;

/// Whether this invocation gets the hooks: it talks to a remote **and** the
/// context has hooks. A local invocation never spawns an askpass server,
/// hooks or not. Takes the hooks rather than the whole `Context` so the rule
/// is testable without a repository.
fn remote_invocation(invocation: &Invocation, hooks: Option<&RemoteHooks>) -> bool {
    invocation.remote && hooks.is_some()
}

/// One invocation of a remote operation that failed, with everything git
/// wrote to stderr (as far as the stream got before a stop).
pub(crate) struct Failed {
    pub(crate) error: RunError,
    pub(crate) segments: Vec<Vec<u8>>,
}

/// Run one invocation of a remote operation, local or not: its stderr is
/// always collected (a pull's `merge --ff-only` fails in words the
/// classifier reads too); progress and askpass only when
/// [`remote_invocation`] says so. A failure to clean up the askpass server
/// is pushed onto `cleanup`, never dropped and never fatal.
pub(crate) fn run(
    ctx: &Context<'_>,
    mut git: GitInvocation,
    invocation: &Invocation,
    cleanup: &mut Vec<AfterState>,
) -> Result<(), Failed> {
    let hooks = ctx
        .remote
        .as_ref()
        .filter(|hooks| remote_invocation(invocation, Some(hooks)));
    let segments: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let (sender, receiver) = mpsc::channel();
    git = git.on_stderr_line(sink(Arc::clone(&segments), hooks.map(|_| sender.clone())));
    drop(sender);

    let server = match hooks {
        Some(hooks) => {
            match AskPassServer::spawn(hooks.helper.clone(), Arc::clone(&hooks.askpass)) {
                Ok(server) => {
                    git = git.askpass(&server);
                    Some(server)
                }
                Err(error) => {
                    // git never started because its credential prompt could
                    // not: say which, or the line blames git (review Minor 1).
                    let error = std::io::Error::new(
                        error.kind(),
                        format!("askpass: the credential prompt's socket could not open: {error}"),
                    );
                    return Err(Failed {
                        error: RunError::Spawn(error),
                        segments: Vec::new(),
                    });
                }
            }
        }
        None => None,
    };

    let result = std::thread::scope(|scope| {
        if let Some(hooks) = hooks {
            scope.spawn(move || forward(receiver, &*hooks.progress));
        }
        let result = git.run(ctx.stop);
        // The sink, and with it the last progress sender, lives in `git`:
        // dropping it here is what ends `forward` (review Minor 3).
        drop(git);
        result
    });
    // The child has exited: nothing can ask for a password any more.
    if let Some(server) = server
        && let Err(error) = server.close()
    {
        cleanup.push(AfterState::AskPass(error));
    }

    result.map(|_| ()).map_err(|error| Failed {
        error,
        // A poisoned lock means a sink call panicked on the drain thread; the
        // segments it had already pushed are still whole, and they are all
        // the failure has to go on.
        segments: std::mem::take(&mut *segments.lock().unwrap_or_else(PoisonError::into_inner)),
    })
}

/// The stderr sink: keep every segment, and when there is somewhere to send
/// progress, send a parsed one no more often than [`PROGRESS_EVERY`].
fn sink(segments: Arc<Mutex<Vec<Vec<u8>>>>, progress: Option<Sender<RemoteProgress>>) -> Sink {
    let mut last: Option<Instant> = None;
    Box::new(move |segment: &[u8]| {
        // See `run`: a poisoned lock still holds whole segments.
        segments
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(segment.to_vec());
        let Some(sender) = &progress else { return };
        if last.is_some_and(|at| at.elapsed() < PROGRESS_EVERY) {
            return;
        }
        if let Some(parsed) = parse_progress(segment) {
            last = Some(Instant::now());
            // Safe to ignore: the receiver is gone only once the child has
            // exited and the forwarder stopped, when progress means nothing.
            let _ = sender.send(parsed);
        }
    })
}

/// Hand progress to the app's callback until the sink is dropped (the
/// invocation is over) and the queue is empty. A blocking loop: it ends on
/// the channel closing, never on a timer.
fn forward(
    receiver: mpsc::Receiver<RemoteProgress>,
    progress: &(dyn Fn(RemoteProgress) + Send + Sync),
) {
    for parsed in receiver {
        progress(parsed);
    }
}

/// What went wrong, in `RemoteFailure`'s terms: a stop is `Cancelled`, a
/// non-zero exit is whatever the whole stderr says, and a git that never ran
/// (not found, spawn failed, killed by a signal) is `Other` with the reason.
pub(crate) fn failure_of(error: &RunError, segments: &[Vec<u8>]) -> RemoteFailure {
    match error {
        RunError::Stopped => RemoteFailure::Cancelled,
        RunError::Exited { .. } => classify_failure(&segments.join(&b'\n')),
        other => RemoteFailure::Other {
            first_line: other.to_string(),
        },
    }
}

/// The last few segments, lossily decoded: what git said just before it
/// gave up, for a detail view.
pub(crate) fn tail(segments: &[Vec<u8>]) -> Vec<String> {
    let from = segments.len().saturating_sub(TAIL);
    segments[from..]
        .iter()
        .map(|segment| String::from_utf8_lossy(segment).into_owned())
        .collect()
}

/// The journal's `Aborted` reason: short, no advice.
pub(crate) fn short(failure: &RemoteFailure) -> String {
    match failure {
        RemoteFailure::AuthFailed { target } => with_target("authentication failed", target),
        RemoteFailure::NonFastForward => "rejected: the remote has commits you don't".to_owned(),
        RemoteFailure::StaleInfo => "rejected: stale lease".to_owned(),
        RemoteFailure::RemoteRejected { reason } => format!("rejected by the remote: {reason}"),
        RemoteFailure::NoSuchRemote { name } => format!("no remote named '{name}'"),
        RemoteFailure::Unreachable { target } => with_target("unreachable", target),
        RemoteFailure::NotFastForward => "diverged: not possible to fast-forward".to_owned(),
        RemoteFailure::WouldOverwrite { paths } => format!("would overwrite {}", paths.join(", ")),
        RemoteFailure::Cancelled => "cancelled".to_owned(),
        RemoteFailure::Other { first_line } => first_line.clone(),
    }
}

fn with_target(what: &str, target: &str) -> String {
    if target.is_empty() {
        what.to_owned()
    } else {
        format!("{what} for {target}")
    }
}

/// The status line for a failed remote operation, in the design note's
/// words ("Failure line"), naming the operation's own branch and remote.
pub(crate) fn explain(
    entry: &JournalEntry,
    failure: &RemoteFailure,
    stderr_tail: &[String],
    names: &RemoteNames,
) -> String {
    let retry = match entry.kind {
        OperationKind::Fetch => "f",
        OperationKind::Pull => "p",
        _ => "P",
    };
    match failure {
        RemoteFailure::NonFastForward => format!(
            "push rejected: {} has commits you don't · p to pull first, or P to force with lease",
            names.remote_side
        ),
        RemoteFailure::AuthFailed { target } => format!(
            "{} · {retry} to try again",
            with_target("authentication failed", target)
        ),
        RemoteFailure::NotFastForward => format!(
            "pull stopped: {} and {} diverged — merge or rebase (M5); shell: git pull --rebase",
            names.local, names.remote_side
        ),
        RemoteFailure::WouldOverwrite { paths } => format!(
            "pull refused: would overwrite {} (uncommitted) · commit or stash first",
            paths.join(", ")
        ),
        RemoteFailure::Unreachable { target } => {
            let said = stderr_tail
                .iter()
                .rev()
                .find_map(|line| line.strip_prefix("fatal: "))
                .map_or_else(|| target.clone(), str::to_owned);
            format!("{} unreachable: {said}", names.remote)
        }
        RemoteFailure::StaleInfo => format!(
            "push refused: {} moved since your last fetch (stale lease) · f to fetch, then look",
            names.remote_side
        ),
        RemoteFailure::RemoteRejected { reason } => {
            format!("{} rejected by the remote: {reason}", entry.description)
        }
        RemoteFailure::NoSuchRemote { name } => {
            format!("{} failed: no remote named '{name}'", entry.description)
        }
        RemoteFailure::Cancelled => format!("{} cancelled", entry.description),
        RemoteFailure::Other { first_line } => {
            format!("{} failed: {first_line}", entry.description)
        }
    }
}

#[cfg(test)]
mod tests;
