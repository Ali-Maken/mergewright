//! The resolver's two jobs: opening a conflicted path, and taking one side
//! of one block (m6-conflicts/04).
//!
//! Both are thin: `graft_ops::conflict` decides everything, this file only
//! opens the repository and turns `ops`' errors into the words the status
//! line shows. The app never calls `graft_git::conflict`'s writing half
//! itself — that is the seam `tests/it/seam_rule.rs` locks.

use std::io::Read;
use std::path::Path;
use std::sync::Arc;

use graft_git::conflict::{ConflictFile, Take};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::status::UnmergedHow;

use crate::repos::RepoStore;

/// What [`open`] found: `graft_ops::conflict::Opened`, as the app holds it.
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
/// `graft_ops::conflict::OpenError` carries it in its type and this job is
/// the last place that type exists, so the flag is read off it here. The
/// alternative — matching the rendered sentence back at the other end — is
/// a control-flow decision resting on a string nobody promised not to
/// reword.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenFailure {
    /// `graft_ops`' own words, ready to show.
    pub words: String,
    /// git does not call the path unmerged. After a handover that means the
    /// user resolved it in `$EDITOR` or a mergetool; on a first open it
    /// means the row was never a conflict.
    pub not_unmerged: bool,
}

/// Why a take did not land, with the same distinction kept the same way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TakeFailure {
    /// `graft_ops`' own words, ready to show.
    pub words: String,
    /// The bytes on disk are not the ones the take was chosen against, so
    /// nothing was written and the document must be read again.
    pub reread: bool,
}

/// Reads one conflicted path for the resolver.
///
/// The status is read **here**, not taken from `AppState`: this job is what
/// runs after `$EDITOR` or a mergetool has been at the file, and the one
/// question it has to be able to answer is whether the path is still
/// unmerged at all. A status read before the handover could never say no.
///
/// # Errors
/// [`OpenFailure`], carrying `graft_ops::conflict::OpenError`'s own
/// `Display` — which is `Refusal::explain()` for a path git does not call
/// unmerged.
pub fn open(store: &RepoStore, path: &Path, stop: &dyn Stop) -> Result<OpenedFile, OpenFailure> {
    let repo = store.shared().map_err(|error| failed(&error))?;
    let handle = repo.handle();
    let status = graft_git::status::read(&handle, stop).map_err(|error| failed(&error))?;
    let opened = graft_ops::conflict::open_conflict(&handle, &status, path).map_err(|error| {
        OpenFailure {
            words: error.to_string(),
            not_unmerged: matches!(
                error,
                graft_ops::conflict::OpenError::Refused(
                    graft_ops::operation::Refusal::NotUnmerged { .. }
                )
            ),
        }
    })?;
    let on_disk = if opened.file.is_none() {
        worktree_glimpse(&handle, path, opened.binary)
    } else {
        None
    };
    Ok(OpenedFile {
        file: opened.file,
        how: opened.how,
        binary: opened.binary,
        on_disk,
    })
}

/// A whole-file shape's card shows the file as it stands on disk, when
/// there is one there (`DeletedByThem` keeps it, `BothDeleted` never has
/// one, …) — read directly rather than through `graft_git::conflict::read`,
/// which parses markers this shape does not have. A path that is not there,
/// or not readable, draws nothing rather than fail the whole open: the
/// card's `how`/`keys` line is what matters, and nothing is a fine answer
/// for a file that is gone.
fn worktree_glimpse(handle: &RepoHandle, path: &Path, binary: bool) -> Option<OnDisk> {
    let full = match handle.workdir() {
        Some(workdir) => workdir.join(path),
        None => path.to_path_buf(),
    };
    // Every failure here is the "not there, or not readable" of the doc
    // comment above: the card draws no glimpse and still offers `o`/`t`.
    let file = std::fs::File::open(&full).ok()?;
    let size = file.metadata().ok()?.len();
    glimpse(file, size, binary).ok()
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
    }
}

/// Takes one side of one block and writes the file back, returning the
/// document as it now stands.
///
/// # Errors
/// [`TakeFailure`], carrying `graft_ops::conflict::TakeError`'s own
/// `Display`. `Changed` is the one the resolver acts on: it re-reads rather
/// than writing over an edit made outside Graft.
pub fn take(
    store: &RepoStore,
    path: &Path,
    file: &ConflictFile,
    block: usize,
    side: Take,
) -> Result<ConflictFile, TakeFailure> {
    let repo = store.shared().map_err(|error| TakeFailure {
        words: error.to_string(),
        reread: false,
    })?;
    graft_ops::conflict::apply_take(&repo.handle(), path, file, block, side).map_err(|error| {
        TakeFailure {
            words: error.to_string(),
            reread: matches!(error, graft_ops::conflict::TakeError::Changed { .. }),
        }
    })
}

/// Attaches the merge base to every block of `file` (`B`, m6-conflicts/05).
///
/// `file` is the resolver's own copy, takes and all — `graft_ops::conflict::base_for`
/// mutates it in place rather than reading the path again, so a block
/// already taken keeps its base exactly as an open one does
/// (`ConflictFile::take` moves the block whole, `graft_git::conflict` tests).
///
/// # Errors
/// `graft_ops::conflict::BaseError`'s own words: `this conflict has no
/// common ancestor to show`, `base unavailable after edits`, or git's own
/// `diff3` failure — paired with whether it is a problem (only a failure is).
pub fn load_base(
    store: &RepoStore,
    path: &Path,
    mut file: ConflictFile,
) -> Result<(ConflictFile, usize), (String, bool)> {
    // The repository not opening is a failure, not a fact of the file.
    let repo = store.shared().map_err(|error| (error.to_string(), true))?;
    graft_ops::conflict::base_for(&repo.handle(), path, &mut file)
        .map(|attached| (file, attached))
        .map_err(|error| base_refusal(&error))
}

/// The files the operation in progress conflicted on, for the resolver's
/// file list (m6-conflicts/13): `graft_ops::conflict::operation_conflicts`
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
) -> Result<graft_ops::conflict::ConflictSet, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let git = graft_git::version::system_git()
        .map_err(|error| error.to_string())?
        .version();
    // No remotes, no named refs: the set needs the status and the
    // operation in progress only.
    let scope = graft_ops::operation::FactsScope::default();
    let facts = graft_ops::facts::read_facts(&handle, git, stop, &scope)
        .map_err(|error| error.to_string())?;
    graft_ops::conflict::operation_conflicts(&handle, &facts).map_err(|error| error.to_string())
}

/// `B`'s refusal as the resolver's notice: the words, and whether they are
/// a problem. No ancestor (`NoBase`) and a file edited past what git wrote
/// (`Mismatch`) are facts of the file — nothing went wrong, so they must not
/// read as errors. Only git's own `diff3` failing is one (planner ruling,
/// 2026-09-23).
fn base_refusal(error: &graft_ops::conflict::BaseError) -> (String, bool) {
    use graft_ops::conflict::BaseError;
    let problem = matches!(error, BaseError::Diff3(_));
    (error.to_string(), problem)
}

#[cfg(test)]
mod tests {
    use graft_git::conflict::{BaseMismatch, Diff3Error};
    use graft_ops::conflict::BaseError;

    use super::{CARD_BYTES, CARD_LINES, base_refusal, glimpse};

    /// A file of `lines` numbered lines, and its size.
    fn numbered(lines: usize) -> (Vec<u8>, u64) {
        let bytes: Vec<u8> = (0..lines)
            .flat_map(|n| format!("line {n}\n").into_bytes())
            .collect();
        let size = u64::try_from(bytes.len()).expect("fits");
        (bytes, size)
    }

    #[test]
    fn a_short_file_is_read_whole() {
        let (bytes, size) = numbered(3);
        let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
        assert_eq!(
            &*seen.head,
            [b"line 0".to_vec(), b"line 1".to_vec(), b"line 2".to_vec()]
        );
        assert!(!seen.cut);
        assert_eq!(seen.size, size);
    }

    /// Many short lines: the line cap is what stops it.
    #[test]
    fn many_lines_stop_at_the_line_cap() {
        let (bytes, size) = numbered(CARD_LINES * 3);
        let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
        assert_eq!(seen.head.len(), CARD_LINES);
        assert!(seen.cut);
    }

    /// A few long lines: the byte cap is what stops it, and the fragment it
    /// cuts through is not shown as though it were a line.
    #[test]
    fn long_lines_stop_at_the_byte_cap() {
        let line = vec![b'x'; 10_000];
        let mut bytes = Vec::new();
        for _ in 0..20 {
            bytes.extend_from_slice(&line);
            bytes.push(b'\n');
        }
        let size = u64::try_from(bytes.len()).expect("fits");
        let seen = glimpse(bytes.as_slice(), size, false).expect("reads");
        let held: usize = seen.head.iter().map(Vec::len).sum();
        assert!(u64::try_from(held).expect("fits") <= CARD_BYTES);
        assert!(
            seen.head.iter().all(|kept| kept.len() == 10_000),
            "no fragment"
        );
        assert!(seen.cut);
    }

    /// A binary file is not read at all: its size is the whole card.
    #[test]
    fn a_binary_file_reads_nothing() {
        let (bytes, size) = numbered(10);
        let seen = glimpse(bytes.as_slice(), size, true).expect("reads");
        assert!(seen.head.is_empty());
        assert_eq!(seen.size, size);
    }

    #[test]
    fn no_base_and_mismatch_are_facts_not_problems() {
        assert!(!base_refusal(&BaseError::NoBase).1);
        assert!(!base_refusal(&BaseError::Mismatch(BaseMismatch { block: 0 })).1);
    }

    #[test]
    fn a_diff3_failure_is_a_problem() {
        let error = BaseError::Diff3(Diff3Error::Io(std::io::Error::other("disk full")));
        let (words, problem) = base_refusal(&error);
        assert!(problem, "git itself failed");
        assert!(words.contains("disk full"), "{words}");
    }
}
