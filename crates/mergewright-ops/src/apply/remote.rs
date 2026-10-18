//! Running an invocation that talks to a remote: stderr streamed and kept
//! whole for [`classify_failure`], progress forwarded at most every 100 ms,
//! and credential prompts answered through an [`AskPassServer`] that lives
//! exactly as long as the child.
//!
//! Local invocations never come here; `run_plan` in the parent runs them as
//! it always has.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use mergewright_core::journal::{JournalEntry, OperationKind};
use mergewright_git::remote::{RemoteFailure, RemoteProgress, classify_failure, parse_progress};
use mergewright_git::run::askpass::{AskPass, AskPassServer, Secret};
use mergewright_git::run::{GitInvocation, RunError, Stop};

use super::{AfterState, Context};
use crate::keys::{KeyNames, OpKey, clause, either, then};
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
    /// The askpass helper git runs: Mergewright's own binary, which calls back
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

/// What the runner reads, and nothing else: the hooks and the stop. Narrowed
/// from a whole [`Context`] so a clone, which has no repository and no
/// journal yet, runs through the same askpass and progress path as a fetch
/// (m8-polish-and-ship/18a, rulings R1).
#[derive(Clone, Copy)]
pub(crate) struct RemoteRun<'a> {
    /// `None` is headless: no prompt (git fails auth instead), no progress.
    pub(crate) hooks: Option<&'a RemoteHooks>,
    /// Cancellation.
    pub(crate) stop: &'a dyn Stop,
}

impl Context<'_> {
    /// This operation's hooks and stop, for [`run`].
    pub(crate) fn remote_run(&self) -> RemoteRun<'_> {
        RemoteRun {
            hooks: self.remote.as_ref(),
            stop: self.stop,
        }
    }
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
    at: RemoteRun<'_>,
    mut git: GitInvocation,
    invocation: &Invocation,
    cleanup: &mut Vec<AfterState>,
) -> Result<(), Failed> {
    let hooks = at
        .hooks
        .filter(|hooks| remote_invocation(invocation, Some(hooks)));
    let segments: Arc<Mutex<Vec<Vec<u8>>>> = Arc::default();
    let (sender, receiver) = mpsc::channel();
    git = git.on_stderr_line(sink(Arc::clone(&segments), hooks.map(|_| sender.clone())));
    drop(sender);

    let asks = hooks.map(|hooks| Arc::new(Refusals::new(Arc::clone(&hooks.askpass))));
    let server = match hooks.zip(asks.as_ref()) {
        Some((hooks, asks)) => {
            let handler: Arc<dyn AskPass> = asks.clone();
            match AskPassServer::spawn(hooks.helper.clone(), handler) {
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
        let result = git.run(at.stop);
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

    let refused = asks.is_some_and(|asks| asks.refused());
    result.map(|_| ()).map_err(|error| Failed {
        error: cancelled_at_the_prompt(error, refused, || at.stop.should_stop()),
        // A poisoned lock means a sink call panicked on the drain thread; the
        // segments it had already pushed are still whole, and they are all
        // the failure has to go on.
        segments: std::mem::take(&mut *segments.lock().unwrap_or_else(PoisonError::into_inner)),
    })
}

/// The app's [`AskPass`], remembering whether it ever gave git no answer.
///
/// `None` from an ask is the user cancelling the prompt (the trait's
/// contract). git, refused, gives up on its own and exits — often before the
/// runner polls the stop that the same cancel asked for, so the runner
/// truthfully reports an exit, not a stop (m7-power-tools/27b). This is the
/// structural record that the exit was the user's cancel, read by
/// [`cancelled_at_the_prompt`].
struct Refusals {
    inner: Arc<dyn AskPass>,
    refused: AtomicBool,
}

impl Refusals {
    fn new(inner: Arc<dyn AskPass>) -> Self {
        Refusals {
            inner,
            refused: AtomicBool::new(false),
        }
    }

    fn refused(&self) -> bool {
        self.refused.load(Ordering::SeqCst)
    }
}

impl AskPass for Refusals {
    fn ask(&self, prompt: &str) -> Option<Secret> {
        let answer = self.inner.ask(prompt);
        if answer.is_none() {
            self.refused.store(true, Ordering::SeqCst);
        }
        answer
    }
}

/// git's failed exit, or the user's cancel it stands for.
///
/// A non-zero exit after an ask went unanswered, with the stop asked, is the
/// cancel at the prompt: git failed *because* the user pressed `Esc`, and
/// its words ("authentication failed") would blame a password nobody typed.
/// It reads as the stop it is. Anything else is git's own: a refusal with no
/// stop asked stays git's failure, and a stop with no refusal is judged by
/// the runner alone (a git that exited on its own before the stop is its
/// exit). `stop_asked` is only polled when the other two hold, so a counting
/// test stop sees no extra poll.
fn cancelled_at_the_prompt(
    error: RunError,
    refused: bool,
    stop_asked: impl FnOnce() -> bool,
) -> RunError {
    match error {
        RunError::Exited { .. } if refused && stop_asked() => RunError::Stopped,
        other => other,
    }
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

/// [`RemoteFailure::NoSuchRemote`] in his words, the one formatter for the
/// journal's reason and the status line (20b R3). git said `'<x>' does not
/// appear to be a git repository`: `<x>` is a location, and git's two causes
/// are nothing there or no access. (`Refusal::NoSuchRemote`, before git
/// runs, keeps `no remote named`: there the name truly is not configured.)
fn nothing_there(name: &str) -> String {
    format!("no repository at '{name}', or no access to it")
}

/// The journal's `Aborted` reason: short, no advice.
pub(crate) fn short(failure: &RemoteFailure) -> String {
    match failure {
        RemoteFailure::AuthFailed { target } => with_target("authentication failed", target),
        RemoteFailure::NonFastForward => "rejected: the remote has commits you don't".to_owned(),
        RemoteFailure::StaleInfo => "rejected: stale lease".to_owned(),
        RemoteFailure::RemoteRejected { reason } => format!("rejected by the remote: {reason}"),
        RemoteFailure::NoSuchRemote { name } => nothing_there(name),
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
/// words ("Failure line"), naming the operation's own branch and remote —
/// then what its fetch moved before it stopped, or that that could not be
/// read (ADR-0062).
pub(crate) fn explain(
    entry: &JournalEntry,
    failure: &RemoteFailure,
    stderr_tail: &[String],
    names: &RemoteNames,
    moves_unread: Option<&RunError>,
    keys: &dyn KeyNames,
) -> String {
    let line = failure_line(entry, failure, stderr_tail, names, keys);
    match super::fetched::stopped_words(entry, moves_unread) {
        Some(moved) => format!("{line} \u{b7} {moved}"),
        None => line,
    }
}

/// [`explain`]'s failure itself.
fn failure_line(
    entry: &JournalEntry,
    failure: &RemoteFailure,
    stderr_tail: &[String],
    names: &RemoteNames,
    keys: &dyn KeyNames,
) -> String {
    let retry = match entry.kind {
        OperationKind::Fetch => OpKey::Fetch,
        OperationKind::Pull => OpKey::Pull,
        _ => OpKey::Push,
    };
    match failure {
        // Names the whole way back (m7-power-tools/15, :774): pull, then
        // merge or rebase what it brings, or force. No ahead/behind count:
        // `explain` has none at hand. The app splits at the first ` · `, so
        // the keys part stays whole when the reason is cut at 80 columns.
        // Each key clause goes when its key is unbound (W3); `m or r` keeps
        // the bound one, and `then …` goes with both.
        RemoteFailure::NonFastForward => {
            let then_land = either(
                &[keys.label(OpKey::Merge), keys.label(OpKey::Rebase)],
                " or ",
            );
            let pull = keys.label(OpKey::Pull).map(|pull| match then_land {
                Some(land) => format!("{pull} pull, then {land}"),
                None => format!("{pull} pull"),
            });
            then(
                format!("push rejected: {} has commits you don't", names.remote_side),
                [pull, clause(keys, OpKey::Push, "force with lease")],
            )
        }
        RemoteFailure::AuthFailed { target } => then(
            with_target("authentication failed", target),
            [clause(keys, retry, "to try again")],
        ),
        // No plan shorthand and no shell command: the app's own line names
        // `m`/`r` wherever it knows the tip to aim them at
        // (m6-conflicts/85, the earlier walk's m11).
        RemoteFailure::NotFastForward => format!(
            "pull stopped: {} and {} diverged — merge or rebase",
            names.local, names.remote_side
        ),
        // His edits, named as `EditsAt::words` names them, and no keys: the
        // app adds the ones that work where he is (m6-conflicts/85 D).
        RemoteFailure::WouldOverwrite { paths } => format!(
            "pull refused: your edits to {} would be overwritten",
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
        RemoteFailure::StaleInfo => then(
            format!(
                "push refused: {} moved since your last fetch (stale lease)",
                names.remote_side
            ),
            [clause(keys, OpKey::Fetch, "to fetch, then look")],
        ),
        RemoteFailure::RemoteRejected { reason } => {
            format!("{} rejected by the remote: {reason}", entry.description)
        }
        RemoteFailure::NoSuchRemote { name } => {
            format!("{} failed: {}", entry.description, nothing_there(name))
        }
        RemoteFailure::Cancelled => format!("{} cancelled", entry.description),
        RemoteFailure::Other { first_line } => {
            format!("{} failed: {first_line}", entry.description)
        }
    }
}

#[cfg(test)]
mod tests;
