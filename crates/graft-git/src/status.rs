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

#[cfg(test)]
mod tests;

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
    /// `git status` printed more than [`OUTPUT_CAP`] bytes, so what came back
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

/// Parses `git status --porcelain=v2 -z`'s output. Pure: no `git` spawn, a
/// `cargo-fuzz` target (`fuzz/fuzz_targets/parse_porcelain_v2.rs`) — malformed
/// input is an `Err`, never a panic.
///
/// Handles the four record kinds `-z --untracked-files=all --renames`
/// produces: `1` (ordinary), `2` (rename/copy — its NUL-terminated original
/// path is the *next* record in the stream), `u` (unmerged) and `?`
/// (untracked). A `!` (ignored) record is skipped rather than rejected,
/// since a caller could in principle pass `--ignored` even though `read`
/// itself never does. Branch-header `#` records are rejected: `read` never
/// asks for `--branch`, so one appearing means the input is not what this
/// parser was built for.
///
/// # Errors
/// [`StatusError::Malformed`] for anything the format above does not cover.
pub fn parse_porcelain_v2(bytes: &[u8]) -> Result<WorkingTreeStatus, StatusError> {
    let mut entries = Vec::new();
    let mut records = bytes.split(|&b| b == 0);
    while let Some(record) = records.next() {
        if record.is_empty() {
            continue;
        }
        match record[0] {
            b'1' => entries.push(parse_ordinary(record)?),
            b'2' => {
                let mut entry = parse_rename(record)?;
                // An empty next piece reads the same as a missing one: a
                // real path is never empty, so this also catches a rename
                // that is the very last record with no origin following it
                // — the buffer's own trailing NUL would otherwise look like
                // one (a capped or truncated output ends exactly this way).
                let original = records
                    .next()
                    .filter(|piece| !piece.is_empty())
                    .ok_or_else(|| {
                        StatusError::Malformed(
                            "a rename record's original path is missing".to_owned(),
                        )
                    })?;
                entry.original_path = Some(bytes_to_pathbuf(original));
                entries.push(entry);
            }
            b'u' => entries.push(parse_unmerged(record)?),
            b'?' => entries.push(parse_untracked(record)?),
            b'!' => {}
            other => {
                return Err(StatusError::Malformed(format!(
                    "unrecognised porcelain v2 record kind {:?}",
                    other as char
                )));
            }
        }
    }
    Ok(WorkingTreeStatus { entries })
}

/// Splits one record into exactly `count` space-separated fields, the last
/// of which is everything remaining (a path may itself hold spaces).
fn fields(record: &[u8], count: usize) -> Result<Vec<&[u8]>, StatusError> {
    let parts: Vec<&[u8]> = record.splitn(count, |&b| b == b' ').collect();
    if parts.len() != count {
        return Err(StatusError::Malformed(format!(
            "expected {count} fields, found {}",
            parts.len()
        )));
    }
    Ok(parts)
}

/// `XY` is always two ASCII letters.
fn xy(field: &[u8]) -> Result<(u8, u8), StatusError> {
    match field {
        [x, y] => Ok((*x, *y)),
        _ => Err(StatusError::Malformed(
            "an XY field must be two characters".to_owned(),
        )),
    }
}

/// The `sub` field: `N...` (not a submodule) or `S<c><m><u>`, where each
/// letter is its flag or `.`. Anything else is not what git prints.
fn submodule_status(field: &[u8]) -> Result<Option<SubmoduleStatus>, StatusError> {
    let flag = |letter: u8, set: u8| match letter {
        b'.' => Some(false),
        l if l == set => Some(true),
        _ => None,
    };
    let parsed = match field {
        b"N..." => return Ok(None),
        [b'S', c, m, u] => (|| {
            Some(SubmoduleStatus {
                pointer_moved: flag(*c, b'C')?,
                changed_inside: flag(*m, b'M')?,
                untracked_inside: flag(*u, b'U')?,
            })
        })(),
        _ => None,
    };
    parsed.map(Some).ok_or_else(|| {
        StatusError::Malformed(format!(
            "a sub field must be N... or S<C|.><M|.><U|.>, not {:?}",
            String::from_utf8_lossy(field)
        ))
    })
}

/// `1 XY sub mH mI mW hH hI path`
fn parse_ordinary(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 9)?;
    let (x, y) = xy(f[1])?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[8]),
        original_path: None,
        staged: side_change(x)?,
        unstaged: side_change(y)?,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: submodule_status(f[2])?,
    })
}

/// `2 XY sub mH mI mW hH hI X<score> path` (original path is the next
/// record; the caller fills it in).
fn parse_rename(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 10)?;
    let (x, y) = xy(f[1])?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[9]),
        original_path: None,
        staged: side_change(x)?,
        unstaged: side_change(y)?,
        untracked: false,
        unmerged: false,
        unmerged_how: None,
        submodule: submodule_status(f[2])?,
    })
}

/// `u XY sub m1 m2 m3 mW h1 h2 h3 path`
fn parse_unmerged(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 11)?;
    // `XY` here is a conflict pairing (`UU`, `AA`, `DD`, `AU`, `UA`, `UD`,
    // `DU`), not a `Change`: a `U` would not parse as one. It is kept as
    // `UnmergedHow`, which the resolver reads to choose between a marker
    // file and a whole-file choice (m6-conflicts/01).
    let (x, y) = xy(f[1])?;
    let how = UnmergedHow::from_xy(x, y).ok_or_else(|| {
        StatusError::Malformed(format!(
            "an unmerged record's XY must be one of UU, AA, DD, AU, UA, DU, UD, not {:?}",
            String::from_utf8_lossy(f[1])
        ))
    })?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[10]),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: false,
        unmerged: true,
        unmerged_how: Some(how),
        submodule: submodule_status(f[2])?,
    })
}

/// `? path`
fn parse_untracked(record: &[u8]) -> Result<StatusEntry, StatusError> {
    let f = fields(record, 2)?;
    Ok(StatusEntry {
        path: bytes_to_pathbuf(f[1]),
        original_path: None,
        staged: None,
        unstaged: None,
        untracked: true,
        unmerged: false,
        unmerged_how: None,
        submodule: None,
    })
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
/// [`StatusError::TooLarge`] if its output hit [`OUTPUT_CAP`], so only a
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

/// Every submodule that is **not set up**: a gitlink (mode `160000`) in the
/// index whose directory has no `.git` — a clone that never ran `git
/// submodule update --init`, or one `deinit`ed since (m7-power-tools/13).
/// `git status` says nothing about these at all, so [`read`] cannot.
///
/// No `.gitmodules` in the worktree → no spawn and an empty list: the common
/// repository pays nothing. Otherwise one `git ls-files --stage -z`.
///
/// # Errors
/// [`StatusError::Run`] if git could not be run or exited non-zero;
/// [`StatusError::TooLarge`] past [`OUTPUT_CAP`]; [`StatusError::Malformed`]
/// for a line that is not `<mode> <oid> <stage>\t<path>`.
pub fn submodules_not_set_up(
    repo: &RepoHandle,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, StatusError> {
    let Some(workdir) = repo.workdir() else {
        return Ok(Vec::new());
    };
    if !workdir.join(".gitmodules").exists() {
        return Ok(Vec::new());
    }
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("ls-files"),
            GitArg::Flag("--stage"),
            GitArg::Flag("-z"),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    if output.truncated {
        return Err(StatusError::TooLarge { bytes: OUTPUT_CAP });
    }
    let mut paths = Vec::new();
    for record in output.stdout.split(|&b| b == 0) {
        if record.is_empty() {
            continue;
        }
        let tab = record.iter().position(|&b| b == b'\t').ok_or_else(|| {
            StatusError::Malformed("an ls-files --stage record has no tab".to_owned())
        })?;
        let (info, path) = (&record[..tab], &record[tab + 1..]);
        if !info.starts_with(b"160000 ") {
            continue;
        }
        let path = bytes_to_pathbuf(path);
        // A set-up submodule has a `.git` (a gitfile into the parent's
        // `.git/modules`, or an old-style directory); either counts.
        // An unmerged gitlink is listed once per stage, consecutively.
        if paths.last() != Some(&path) && !workdir.join(&path).join(".git").exists() {
            paths.push(path);
        }
    }
    Ok(paths)
}
