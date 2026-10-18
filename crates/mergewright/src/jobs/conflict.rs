//! The resolver's jobs: opening a conflicted path (m6-conflicts/04), its
//! base, the operation's conflict set, and the two writes in [`write`] — a
//! take, and a pick put back (ease-pass 05).
//!
//! All are thin: `mergewright_ops::conflict` decides everything, this file only
//! opens the repository and turns `ops`' errors into the words the status
//! line shows. The app never calls `mergewright_git::conflict`'s writing half
//! itself — that is the seam `tests/it/seam_rule.rs` locks.

use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use mergewright_git::conflict::ConflictFile;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::Stop;
use mergewright_git::status::UnmergedHow;
use mergewright_ops::operation::RestartKind;

use crate::repos::RepoStore;

mod write;

pub use write::{TakeFailure, UntakeFailure, take, untake};

/// What [`open`] found: `mergewright_ops::conflict::Opened`, as the app holds it.
///
/// A copy rather than the `ops` type itself because a [`Product`] is
/// compared with `assert_eq!` — every one of them is `PartialEq` — and
/// `Opened` is not. The three fields are, so the mirror is (and `ops` owning
/// its own shape is the point of the seam, not an accident of it).
///
/// [`Product`]: crate::update::Product
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedFile {
    /// The parsed file — `None` for a whole-file shape (no markers to
    /// parse) and for a binary `UU`/`AA`.
    pub file: Option<ConflictFile>,
    /// The shape git's status read (`both modified`, `deleted by them`, …).
    pub how: UnmergedHow,
    /// git calls the working-tree file binary (a NUL in the first 8 KiB).
    pub binary: bool,
    /// What a whole-file shape's card (`file: None`) shows of the file on
    /// disk under it — `None` when the shape has no file there
    /// (`BothDeleted`, or a side that removed it) and always `None` when
    /// `file` is `Some` (m6-conflicts/05).
    pub on_disk: Option<OnDisk>,
    /// The file is there but could not be read: `can't read <path>:
    /// <reason>`, for the card's notice. `None` when it read, or when it
    /// is simply not there (`KNOWN_GAPS` :572).
    pub unreadable: Option<String>,
    /// Whether `d` can start a whole-file shape (`file: None`) over — ops'
    /// own answer, asked when it opened (ticket 30: no app mirror of ops'
    /// rule). `None` for a document, whose `d` is not gated here.
    pub start_over: Option<StartOver>,
}

/// The most of a whole-file card's file that is read: its first
/// [`CARD_LINES`] lines, and never more than this many bytes of them.
///
/// A card is a glimpse, not a pager — `e` opens the whole file. Reading it
/// whole froze the UI on an ordinary 50–100 MB generated file or binary
/// asset: every byte was held in the state, which `update` clones on every
/// event, and sanitised on every frame (m6-05 review, Major 4).
pub const CARD_BYTES: u64 = 64 * 1024;

/// The most lines a whole-file card holds; see [`CARD_BYTES`].
pub const CARD_LINES: usize = 500;

/// A whole-file card's glimpse of the file on disk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OnDisk {
    /// The file's whole size, in bytes.
    pub size: u64,
    /// Its first lines, at most [`CARD_LINES`] of them and [`CARD_BYTES`]
    /// in all — none for a binary file, which has no lines worth drawing.
    /// Raw bytes: sanitised at the view, never here. `Arc` because the
    /// resolver's state holds it and every event clones that state.
    pub head: Arc<[Vec<u8>]>,
    /// Lines past [`Self::head`] were left unread.
    pub cut: bool,
}

/// Why a path could not be opened, with the one distinction the resolver
/// acts on kept as a field rather than left in the words.
///
/// `mergewright_ops::conflict::OpenError` carries it in its type and this job is
/// the last place that type exists, so the flag is read off it here. The
/// alternative — matching the rendered sentence back at the other end — is
/// a control-flow decision resting on a string nobody promised not to
/// reword.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenFailure {
    /// `mergewright_ops`' own words, ready to show.
    pub words: String,
    /// git does not call the path unmerged. After a handover that means the
    /// user resolved it in `$EDITOR` or a mergetool; on a first open it
    /// means the row was never a conflict.
    pub not_unmerged: bool,
    /// `Some` when the read failed on the done route ([`Opening::Done`]'s):
    /// the path is not unmerged, and the resolver keeps it open as a card
    /// saying why rather than closing (m6-conflicts/25). It carries the
    /// start-over answer, asked before the read, so the card offers `d`
    /// exactly where ops would take it (m6-conflicts/28).
    pub done: Option<StartOver>,
}

/// Whether `d` can start a done file over: ops' own answer
/// (`mergewright_ops::apply::restart_gate`), asked when the file opens
/// (m6-conflicts/28). The act's refusal after the confirm stays the backstop
/// for a repository that changed since.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StartOver {
    /// `d` opens the box, and ops would restart it.
    Can {
        /// What the restart puts back — markers, or a keep-or-delete
        /// choice. Sent back to ops in `RestartParams::kind`, and the box's
        /// words and command line come from it (m6-conflicts/58).
        kind: RestartKind,
    },
    /// `d` answers in words: ops' refusal, or `could not check: …` when the
    /// question could not be asked. Raw; sanitised where drawn.
    Cannot {
        /// Why, ready to follow `can't start over · `.
        why: String,
    },
}

/// What [`open`] found, by the route git's status picked at open time
/// (m6-conflicts/25).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Opening {
    /// git calls the path unmerged: a document or a whole-file card.
    Unmerged(Arc<OpenedFile>),
    /// git no longer calls it unmerged: the file on disk, read-only, and
    /// whether `d` can start it over.
    Done(Arc<mergewright_ops::conflict::DoneFile>, StartOver),
}

/// Reads one conflicted path for the resolver.
///
/// The status is read **here**, not taken from `AppState`: this job is what
/// runs after `$EDITOR` or a mergetool has been at the file, and the one
/// question it has to be able to answer is whether the path is still
/// unmerged at all. A status read before the handover could never say no.
///
/// # Errors
/// [`OpenFailure`], carrying `mergewright_ops::conflict::OpenError`'s own
/// `Display` — which is `Refusal::explain()` for a path git does not call
/// unmerged.
pub fn open(store: &RepoStore, path: &Path, stop: &dyn Stop) -> Result<Opening, OpenFailure> {
    let repo = store.shared().map_err(|error| failed(&error))?;
    let handle = repo.handle();
    let status = mergewright_git::status::read(&handle, stop).map_err(|error| failed(&error))?;
    // The route is git's state at open time, read just above — not a retry
    // after a refusal. A path staged between this read and the open below
    // still comes back `NotUnmerged` (m6-conflicts/25).
    let unmerged = status.entry(path).is_some_and(|entry| entry.unmerged);
    if !unmerged {
        // Asked before the read, so a read that fails still carries it.
        let start_over = start_over(&handle, path, stop);
        return match mergewright_ops::conflict::open_done(&handle, &status, path) {
            Ok(done) => Ok(Opening::Done(Arc::new(done), start_over)),
            Err(error) => Err(OpenFailure {
                words: error.to_string(),
                not_unmerged: false,
                done: Some(start_over),
            }),
        };
    }
    let opened = mergewright_ops::conflict::open_conflict(&handle, &status, path).map_err(|error| {
        OpenFailure {
            words: error.to_string(),
            not_unmerged: matches!(
                error,
                mergewright_ops::conflict::OpenError::Refused(
                    mergewright_ops::operation::Refusal::NotUnmerged { .. }
                )
            ),
            done: None,
        }
    })?;
    let (on_disk, unreadable, start_over) = if opened.file.is_none() {
        let (on_disk, unreadable) = match worktree_glimpse(&handle, path, opened.binary) {
            Ok(on_disk) => (on_disk, None),
            Err(words) => (None, Some(words)),
        };
        (on_disk, unreadable, Some(start_over(&handle, path, stop)))
    } else {
        (None, None, None)
    };
    Ok(Opening::Unmerged(Arc::new(OpenedFile {
        file: opened.file,
        how: opened.how,
        binary: opened.binary,
        on_disk,
        unreadable,
        start_over,
    })))
}

/// A whole-file shape's card shows the file as it stands on disk, when
/// there is one there (`DeletedByThem` keeps it, `BothDeleted` never has
/// one — read directly rather than through `mergewright_git::conflict::read`,
/// which parses markers this shape does not have. A path that is not there
/// draws nothing (`Ok(None)`): nothing is the right answer for a file that
/// is gone. Any other failure is `Err` with the card notice's words,
/// `can't read <path>: <reason>` (`KNOWN_GAPS` :572) — never an empty file.
/// Neither fails the whole open: the card's `how`/`keys` line still stands.
fn worktree_glimpse(
    handle: &RepoHandle,
    path: &Path,
    binary: bool,
) -> Result<Option<OnDisk>, String> {
    let full = match handle.workdir() {
        Some(workdir) => workdir.join(path),
        None => path.to_path_buf(),
    };
    let words = |why: &std::io::Error| format!("can't read {}: {why}", path.display());
    let file = match std::fs::File::open(&full) {
        Ok(file) => file,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(why) => return Err(words(&why)),
    };
    let size = file.metadata().map_err(|why| words(&why))?.len();
    glimpse(file, size, binary)
        .map(Some)
        .map_err(|why| words(&why))
}

/// The first lines of `file` (whose whole size is `size`), as a card holds
/// them: at most [`CARD_LINES`] lines and [`CARD_BYTES`] bytes read, and
/// nothing read at all for a binary file.
///
/// # Errors
/// The read's own.
pub fn glimpse(file: impl Read, size: u64, binary: bool) -> std::io::Result<OnDisk> {
    if binary {
        return Ok(OnDisk {
            size,
            head: Arc::from(Vec::new()),
            cut: false,
        });
    }
    let mut bytes = Vec::new();
    file.take(CARD_BYTES).read_to_end(&mut bytes)?;
    let short_read = u64::try_from(bytes.len()).map_or(true, |read| read < size);
    let mut lines: Vec<Vec<u8>> = bytes
        .split(|&byte| byte == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line).to_vec())
        .collect();
    // A trailing `\n` splits into one empty line past the last real one;
    // drop it so `wc -l`'s count and this one agree. Cut short by the byte
    // cap, the last line is a fragment instead, and is dropped too.
    if short_read || lines.last().is_some_and(Vec::is_empty) {
        lines.pop();
    }
    let cut = short_read || lines.len() > CARD_LINES;
    lines.truncate(CARD_LINES);
    Ok(OnDisk {
        size,
        head: Arc::from(lines),
        cut,
    })
}

/// A repository or status read that failed: real, and never "not unmerged".
fn failed(error: &impl std::fmt::Display) -> OpenFailure {
    OpenFailure {
        words: error.to_string(),
        not_unmerged: false,
        done: None,
    }
}

/// Ask ops whether `d` can start `path` over, with facts of its own read the
/// way [`conflict_set`] reads them. A question that could not be asked is
/// [`StartOver::Cannot`] saying so — never `Can` on an error.
fn start_over(handle: &RepoHandle, path: &Path, stop: &dyn Stop) -> StartOver {
    let could_not = |why: String| StartOver::Cannot {
        why: format!("could not check: {why}"),
    };
    let git = match mergewright_git::version::system_git() {
        Ok(found) => found.version(),
        Err(error) => return could_not(error.to_string()),
    };
    // No remotes, no named refs: the gate reads the status and the
    // operation in progress only.
    let scope = mergewright_ops::operation::FactsScope::default();
    let facts = match mergewright_ops::facts::read_facts(handle, git, stop, &scope) {
        Ok(facts) => facts,
        Err(error) => return could_not(error.explain()),
    };
    match mergewright_ops::apply::restart_gate(handle, &facts, path) {
        Ok(Ok(plan)) => StartOver::Can { kind: plan.kind },
        // Keyless (m8/10e): the gate refuses only `NotMarkerShape`,
        // `NoStagesKept` and `NotUnmerged` (`apply::restart_gate`), whose
        // words name no key, so this job holds no keymap.
        Ok(Err(refusal)) => StartOver::Cannot {
            why: refusal.explain(&mergewright_ops::keys::NoKeys),
        },
        Err(error) => could_not(error.explain()),
    }
}

/// Attaches the merge base to every block of `file` (`B`, m6-conflicts/05).
///
/// `file` is the resolver's own copy, takes and all — `mergewright_ops::conflict::base_for`
/// mutates it in place rather than reading the path again, so a block
/// already taken keeps its base exactly as an open one does
/// (`ConflictFile::take` moves the block whole, `mergewright_git::conflict` tests).
///
/// # Errors
/// `mergewright_ops::conflict::BaseError`'s own words: `this conflict has no
/// common ancestor to show`, `base unavailable after edits`, or git's own
/// `diff3` failure — paired with whether it is a problem (only a failure is).
pub fn load_base(
    store: &RepoStore,
    path: &Path,
    mut file: ConflictFile,
) -> Result<(ConflictFile, usize), (String, bool)> {
    // The repository not opening is a failure, not a fact of the file.
    let repo = store.shared().map_err(|error| (error.to_string(), true))?;
    mergewright_ops::conflict::base_for(&repo.handle(), path, &mut file)
        .map(|attached| (file, attached))
        .map_err(|error| base_refusal(&error))
}

/// The files the operation in progress conflicted on, for the resolver's
/// file list (m6-conflicts/13): `mergewright_ops::conflict::operation_conflicts`
/// over facts read now, so a relaunch or a shell-begun operation lists the
/// same files as one begun here.
///
/// # Errors
/// Why the repository, git, the facts or the set could not be read, in
/// their own words — the list then falls back to the unmerged paths and
/// says so.
pub fn conflict_set(
    store: &RepoStore,
    stop: &dyn Stop,
) -> Result<mergewright_ops::conflict::ConflictSet, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let git = mergewright_git::version::system_git()
        .map_err(|error| error.to_string())?
        .version();
    // No remotes, no named refs: the set needs the status and the
    // operation in progress only.
    let scope = mergewright_ops::operation::FactsScope::default();
    let facts = mergewright_ops::facts::read_facts(&handle, git, stop, &scope)
        .map_err(|error| error.to_string())?;
    mergewright_ops::conflict::operation_conflicts(&handle, &facts).map_err(|error| error.to_string())
}

/// `B`'s refusal as the resolver's notice: the words, and whether they are
/// a problem. No ancestor (`NoBase`) and a file edited past what git wrote
/// (`Mismatch`) are facts of the file — nothing went wrong, so they must not
/// read as errors. Only git's own `diff3` failing is one (planner ruling,
/// 2026-09-23).
fn base_refusal(error: &mergewright_ops::conflict::BaseError) -> (String, bool) {
    use mergewright_ops::conflict::BaseError;
    let problem = matches!(error, BaseError::Diff3(_));
    (error.to_string(), problem)
}

#[cfg(test)]
mod tests;
