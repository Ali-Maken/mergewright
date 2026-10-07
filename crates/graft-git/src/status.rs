//! The working tree's changes: [`read`] gives the file-by-file staging view
//! (M3), and [`WorkingTreeStatus::counts`] the uncommitted-changes row's
//! numbers (PROMPT.md §1.3) from that same read.
//!
//! It is a read, it touches disk, it is a job's work, never `update`'s. It
//! parses `git status --porcelain=v2 -z` — the one git command that honours
//! `GIT_OPTIONAL_LOCKS=0` (M2 review round 2, R2 Major 1). A read must never
//! refresh and rewrite `.git/index`, and porcelain `git diff` does exactly
//! that, which is why every other read in this crate goes through plumbing
//! instead. `git status` is the exception, not the rule.
//!
//! There was a second way in: a `counts` that walked `gix`'s status iterator
//! for the row alone. It went in m5-history-and-ghost-preview/22, once the
//! app had stopped calling it — it could not report an unmerged path
//! honestly (m5/21), and a second walk that disagreed with the first is a
//! worse answer than one walk.

mod parse;
mod submodule;
#[cfg(test)]
mod tests;
mod tracked;

pub use parse::parse_porcelain_v2;
pub use submodule::submodules_not_set_up;
pub use tracked::{TRACKED_CAP, TrackedPaths, tracked_paths};

use submodule::submodule_status;

use std::path::{Path, PathBuf};

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// How many files are staged, changed but unstaged, and untracked.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorkingTreeCounts {
    /// Staged: differences between `HEAD`'s tree and the index. On an
    /// unborn branch this compares against the empty tree (gix's own
    /// `HEAD^{tree}` fallback), so a staged file on a first commit still
    /// counts.
    pub staged: usize,
    /// Unstaged: tracked files that differ between the index and the
    /// worktree.
    pub unstaged: usize,
    /// Untracked: worktree entries the index does not know about.
    /// Ignored files are not counted — `gix::Repository::status`'s default
    /// matches `git status --ignored=no`.
    pub untracked: usize,
}

impl WorkingTreeCounts {
    /// No staged, unstaged or untracked changes: the uncommitted row does
    /// not draw at all.
    #[must_use]
    pub fn is_clean(self) -> bool {
        self.staged == 0 && self.unstaged == 0 && self.untracked == 0
    }
}

/// Why counting or reading the working tree's changes failed.
#[derive(Debug, thiserror::Error)]
pub enum StatusError {
    /// The status platform could not be built.
    #[error(transparent)]
    Start(#[from] Box<gix::status::Error>),
    /// The status walk could not begin.
    #[error(transparent)]
    Begin(#[from] Box<gix::status::into_iter::Error>),
    /// A status entry could not be read partway through the walk.
    #[error(transparent)]
    Item(#[from] Box<gix::status::iter::Error>),
    /// `git status` itself could not be run.
    #[error(transparent)]
    Run(#[from] RunError),
    /// `git status --porcelain=v2 -z` printed something
    /// [`parse_porcelain_v2`] cannot read: an unrecognised record kind, an
    /// `XY` letter that is not one of `.MADRCT`, a rename record with no
    /// following original-path record, or too few fields in a record.
    #[error("git status printed something Graft cannot parse: {0}")]
    Malformed(String),
    /// `git status` printed more than `OUTPUT_CAP` bytes, so what came back
    /// is a prefix of the real status. Parsing it would report a shorter,
    /// plausible-looking list of changes; saying so is the only honest
    /// answer, as `diff::read` already does for the same condition.
    #[error("git status printed more than {bytes} bytes; the working tree is too large to read")]
    TooLarge {
        /// The cap that was hit, in bytes.
        bytes: usize,
    },
}

/// The most bytes kept from `git status`'s stdout. The same cap
/// `diff::read` uses; past it the status is reported as
/// [`StatusError::TooLarge`], never parsed as if it were complete.
const OUTPUT_CAP: usize = 64 * 1024 * 1024;

/// One side's state in an `XY` pair, or a submodule's `sub` field: `Some` for
/// a real change, `None` for `.` (unchanged on that side).
fn side_change(letter: u8) -> Result<Option<Change>, StatusError> {
    Ok(match letter {
        b'.' => None,
        b'M' => Some(Change::Modified),
        b'A' => Some(Change::Added),
        b'D' => Some(Change::Deleted),
        b'R' => Some(Change::Renamed),
        b'C' => Some(Change::Copied),
        b'T' => Some(Change::TypeChanged),
        other => {
            return Err(StatusError::Malformed(format!(
                "unrecognised status letter {:?}",
                other as char
            )));
        }
    })
}

/// One kind of per-file change a porcelain v2 record's `X` or `Y` column can
/// hold. Coarser than [`crate::diff::FileStatus`]: no similarity score, no
/// `Untracked`/`Unmerged` (those are [`StatusEntry`]'s own flags, since a
/// record's two columns can never both be untracked or unmerged at once).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// `A`: new to the index or the worktree.
    Added,
    /// `M`: content differs.
    Modified,
    /// `D`: gone from the index or the worktree.
    Deleted,
    /// `R`: moved, same or different content (a rename record, `2`).
    Renamed,
    /// `C`: copied from another tracked file (a rename record, `2`).
    Copied,
    /// `T`: the entry type changed — file to symlink, and the like.
    TypeChanged,
}

/// One path `git status --porcelain=v2` reported: at most one staged and one
/// unstaged [`Change`], or untracked, or a merge conflict — never more than
/// one of those three at once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StatusEntry {
    /// The path, as git printed it — the new path for a rename.
    pub path: PathBuf,
    /// The path this one was renamed or copied from (rename records only).
    pub original_path: Option<PathBuf>,
    /// The change between `HEAD` and the index, if any.
    pub staged: Option<Change>,
    /// The change between the index and the worktree, if any.
    pub unstaged: Option<Change>,
    /// A worktree entry the index does not know about at all.
    pub untracked: bool,
    /// An unresolved merge conflict (a porcelain v2 `u` record).
    pub unmerged: bool,
    /// Which conflict it is, for an unmerged record: the `u` record's `XY`
    /// pair (m6-conflicts/01). `Some` exactly when `unmerged` is true.
    pub unmerged_how: Option<UnmergedHow>,
    /// What git says of a submodule at this path (the record's `sub`
    /// field, `S<c><m><u>`); `None` for anything that is not a submodule
    /// (`N...`, and every untracked record).
    pub submodule: Option<SubmoduleStatus>,
}

/// Porcelain v2's `S<c><m><u>`: the three things git can say about a
/// submodule's working tree (m7-power-tools/13). Each is measured against
/// the **index**: a pointer move that is already staged reads `S...` with a
/// staged change on the entry, not `pointer_moved` (measured, git 2.54).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct SubmoduleStatus {
    /// `C`: the submodule's checked-out commit is not the one the index
    /// records.
    pub pointer_moved: bool,
    /// `M`: tracked changes inside the submodule.
    pub changed_inside: bool,
    /// `U`: untracked files inside the submodule.
    pub untracked_inside: bool,
}

/// The seven shapes an unmerged path takes — porcelain v2's `XY` on a `u`
/// record — in git's own words (m6-conflicts/01).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum UnmergedHow {
    /// `UU`: changed on both sides; the working-tree file has markers.
    BothModified,
    /// `AA`: new on both sides with different content; markers.
    BothAdded,
    /// `DD`: gone on both sides (each renamed it elsewhere); no file.
    BothDeleted,
    /// `AU`: only our side has it (our name for a file both renamed).
    AddedByUs,
    /// `UA`: only their side has it.
    AddedByThem,
    /// `DU`: we deleted what they changed.
    DeletedByUs,
    /// `UD`: they deleted what we changed.
    DeletedByThem,
}

impl UnmergedHow {
    /// The `XY` pair as git prints it, or `None` for a pair that is not an
    /// unmerged one.
    fn from_xy(x: u8, y: u8) -> Option<Self> {
        Some(match (x, y) {
            (b'U', b'U') => UnmergedHow::BothModified,
            (b'A', b'A') => UnmergedHow::BothAdded,
            (b'D', b'D') => UnmergedHow::BothDeleted,
            (b'A', b'U') => UnmergedHow::AddedByUs,
            (b'U', b'A') => UnmergedHow::AddedByThem,
            (b'D', b'U') => UnmergedHow::DeletedByUs,
            (b'U', b'D') => UnmergedHow::DeletedByThem,
            _ => return None,
        })
    }

    /// git's own phrase for it, as `git status` prints under "Unmerged
    /// paths".
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            UnmergedHow::BothModified => "both modified",
            UnmergedHow::BothAdded => "both added",
            UnmergedHow::BothDeleted => "both deleted",
            UnmergedHow::AddedByUs => "added by us",
            UnmergedHow::AddedByThem => "added by them",
            UnmergedHow::DeletedByUs => "deleted by us",
            UnmergedHow::DeletedByThem => "deleted by them",
        }
    }

    /// Whether the working-tree file carries conflict markers to resolve
    /// block by block (`UU`, `AA` — unless binary). Every other shape is
    /// resolved whole: keep one side's file, or none.
    #[must_use]
    pub fn has_markers(self) -> bool {
        matches!(self, UnmergedHow::BothModified | UnmergedHow::BothAdded)
    }
}

impl StatusEntry {
    /// Has a staged change: differs between `HEAD` and the index.
    #[must_use]
    pub fn is_staged(&self) -> bool {
        self.staged.is_some()
    }

    /// Has an unstaged change: differs between the index and the worktree.
    #[must_use]
    pub fn is_unstaged(&self) -> bool {
        self.unstaged.is_some()
    }
}

/// Every changed, untracked and unmerged path in the working tree, as
/// [`read`] or [`parse_porcelain_v2`] found them.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct WorkingTreeStatus {
    /// One entry per path git printed, in the order git printed them.
    pub entries: Vec<StatusEntry>,
}

impl WorkingTreeStatus {
    /// Entries with a staged change.
    pub fn staged(&self) -> impl Iterator<Item = &StatusEntry> {
        self.entries.iter().filter(|entry| entry.is_staged())
    }

    /// Entries with an unstaged change.
    pub fn unstaged(&self) -> impl Iterator<Item = &StatusEntry> {
        self.entries.iter().filter(|entry| entry.is_unstaged())
    }

    /// Untracked entries.
    pub fn untracked(&self) -> impl Iterator<Item = &StatusEntry> {
        self.entries.iter().filter(|entry| entry.untracked)
    }

    /// No entries at all: nothing staged, unstaged, untracked or unmerged.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }

    /// How many entries are staged, unstaged and untracked, counted from
    /// the entries [`read`] already returned. Unmerged paths are their own
    /// state and are counted by neither (m5-history-and-ghost-preview/21).
    #[must_use]
    pub fn counts(&self) -> WorkingTreeCounts {
        WorkingTreeCounts {
            staged: self.staged().count(),
            unstaged: self.unstaged().count(),
            untracked: self.untracked().count(),
        }
    }

    /// The entry at `path`, if git reported one.
    #[must_use]
    pub fn entry(&self, path: &Path) -> Option<&StatusEntry> {
        self.entries.iter().find(|entry| entry.path == path)
    }
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    // Windows paths are UTF-16; git for Windows reads/writes paths as UTF-8.
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

/// Reads the working tree's full status: every staged, unstaged, untracked
/// and unmerged path.
///
/// One spawn, `git -c core.quotepath=off status --porcelain=v2 -z
/// --untracked-files=all --renames` — `GIT_OPTIONAL_LOCKS=0` is already set
/// by [`GitInvocation`] itself. A bare repository has no worktree to read and
/// reports empty — not an error, since there is truthfully nothing
/// uncommitted to show.
///
/// # Errors
/// [`StatusError::Run`] if `git` could not be run or exited non-zero;
/// [`StatusError::TooLarge`] if its output hit `OUTPUT_CAP`, so only a
/// prefix of the status came back; [`StatusError::Malformed`] if it printed
/// something [`parse_porcelain_v2`] cannot read.
pub fn read(repo: &RepoHandle, stop: &dyn Stop) -> Result<WorkingTreeStatus, StatusError> {
    if repo.is_bare() {
        return Ok(WorkingTreeStatus::default());
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("-c"),
            GitArg::Flag("core.quotepath=off"),
            GitArg::Flag("status"),
            GitArg::Flag("--porcelain=v2"),
            GitArg::Flag("-z"),
            GitArg::Flag("--untracked-files=all"),
            GitArg::Flag("--renames"),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    status_from(&output)
}

/// What [`read`] makes of one `git status` run.
///
/// Split out so the capped case is testable without a 64 MiB working tree:
/// a truncated output is [`StatusError::TooLarge`], never parsed. A cut at a
/// record boundary would otherwise parse into a shorter, perfectly plausible
/// list of changes, and a cut mid-record into a [`StatusError::Malformed`]
/// that names the wrong cause.
///
/// # Errors
/// See [`read`].
fn status_from(output: &crate::run::GitOutput) -> Result<WorkingTreeStatus, StatusError> {
    if output.truncated {
        return Err(StatusError::TooLarge { bytes: OUTPUT_CAP });
    }
    parse_porcelain_v2(&output.stdout)
}
