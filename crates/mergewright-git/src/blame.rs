//! Who last changed each line of a file: `git blame --porcelain`, run through
//! the typed runner so it is async-friendly and cancellable
//! (m7-power-tools/18). CLI git rather than a library blame: libgit2's is
//! reported far slower (`docs/research/blame-line-history.md` §3), and gix's
//! is off in this crate's feature set.
//!
//! The command, always: `git -c blame.markIgnoredLines=true -c
//! blame.markUnblamableLines=true blame --porcelain -M [-w]
//! [--ignore-revs-file …] [<rev>] -- <path>`. `-C` is off: it costs too much
//! on a large repository for what it adds.
//!
//! **Marks on older git** (18e): git before
//! [`crate::version::GitVersion::BLAME_PORCELAIN_MARKS`] (2.50) writes no `ignored` or
//! `unblamable` lines in the porcelain. When a list is in effect and the
//! porcelain carried no mark (`marks_pass::needs_marks_pass`), a second `git
//! blame -l -s` with the same options reads them from the human format's
//! `?`/`*` ([`parse_marks`]), by position.
//!
//! The working tree is blamed by passing **no** rev: git then reads the file
//! from disk and gives every line it cannot find in `HEAD` the all-zero id,
//! which [`parse_porcelain`] turns into `commit: None`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use mergewright_core::oid::Oid;

use crate::diff::ParseError;
use crate::repo::RepoHandle;
use crate::run::{GitOutput, RunError, Stop};
use crate::shallow::ShallowList;

mod ignore_revs;
mod invocation;
mod knobs;
mod marks;
mod marks_pass;
mod porcelain;
mod refusal;

#[cfg(test)]
use ignore_revs::count_revisions;
use ignore_revs::find_ignore_revs;
use invocation::{Format, blame_invocation};
pub(crate) use knobs::{GitFrom, Knobs, MarksFrom, PorcelainMarks};
pub use marks::{MarkedLine, parse_marks};
use marks_pass::{marks_pass_wanted, merge_marks, read_marks};
pub use porcelain::parse_porcelain;
use refusal::refuse_early;

/// The file git reads ignore-revs from by convention, at the repository root.
pub const IGNORE_REVS_FILE: &str = ".git-blame-ignore-revs";

/// Which version of the file to blame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rev {
    /// The file as of this commit.
    Commit(Oid),
    /// The file on disk; lines not in `HEAD` are not committed yet.
    WorkingTree,
}

/// How to blame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlameOptions {
    /// `-w`: a whitespace-only change passes the line to the commit before.
    pub ignore_whitespace: bool,
    /// Pass `.git-blame-ignore-revs` when the repository has one; `false`
    /// clears any configured list too (`--no-ignore-revs-file`).
    pub ignore_revs: bool,
}

impl Default for BlameOptions {
    fn default() -> Self {
        Self {
            ignore_whitespace: true,
            ignore_revs: true,
        }
    }
}

/// One commit a blame names, from its porcelain headers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameCommit {
    /// The commit's id.
    pub id: Oid,
    /// `author`, as git printed it (mailmap applied by git).
    pub author: String,
    /// `author-time`, Unix seconds.
    pub author_time: i64,
    /// `summary`: the subject line.
    pub summary: String,
    /// `filename`: the file's path in this commit. Differs from the blamed
    /// path for lines older than a rename git followed.
    pub path: PathBuf,
    /// `previous`: the parent the line came through and the path it had
    /// there. `None` at a root or boundary commit.
    pub previous: Option<(Oid, PathBuf)>,
    /// `boundary`: git's walk stopped here (a root commit, usually).
    pub boundary: bool,
    /// The id is in `<common_dir>/shallow`: git stopped here because the
    /// clone has nothing older, so who wrote the lines is unknown. Set by
    /// [`blame`], never by [`parse_porcelain`] (m7-power-tools/20d).
    pub shallow: bool,
}

/// What git said about how it attributed a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineMark {
    /// An ordinary attribution.
    Plain,
    /// git's `?` (`ignored`): an ignored revision changed the line and git
    /// passed it to another commit.
    Skipped,
    /// git's `*` (`unblamable`): an ignored revision changed the line and
    /// git could pass it to no one.
    Unblamable,
}

/// One line of the blamed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlameLine {
    /// The commit that last changed it; `None` for a line not committed yet.
    pub commit: Option<Oid>,
    /// The line's number in that commit's version of the file.
    pub orig_line: u32,
    /// The line's number in the blamed version.
    pub final_line: u32,
    /// The line's bytes, without the newline.
    pub text: Vec<u8>,
    /// How git attributed it.
    pub mark: LineMark,
}

/// The ignore-revs file a blame found, whether or not it passed it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IgnoreRevs {
    /// The file, relative to the repository root.
    pub file: PathBuf,
    /// Its non-comment, non-blank lines.
    pub revisions: usize,
    /// Whether this blame passed it to git.
    pub applied: bool,
}

/// A whole file's blame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blame {
    /// The path blamed, relative to the repository root.
    pub path: PathBuf,
    /// The version blamed.
    pub rev: Rev,
    /// Every line, in file order.
    pub lines: Vec<BlameLine>,
    /// Every commit a line names.
    pub commits: BTreeMap<Oid, BlameCommit>,
    /// `Some` when the repository root has an ignore-revs file listing at
    /// least one revision (the status rule: nothing to say otherwise).
    pub ignore_revs: Option<IgnoreRevs>,
}

/// Why a blame gave no answer.
#[derive(Debug)]
pub enum BlameError {
    /// git failed: missing, refused, or killed from outside.
    Run(RunError),
    /// git's porcelain did not parse.
    Parse(ParseError),
    /// The file is binary, as the diff machinery decides it: refused before
    /// blame runs.
    Binary,
    /// The path is not in the version asked for (for the working tree: not
    /// in `HEAD`).
    Untracked,
    /// The caller's stop fired; git was killed.
    Stopped,
    /// `<common_dir>/shallow` exists but could not be read (20d).
    Shallow {
        /// The shallow file.
        path: PathBuf,
        /// Why reading it failed.
        source: std::io::Error,
    },
    /// git's output passed the runner's cap; the blame is refused rather
    /// than shown short.
    TooLarge {
        /// The cap, in bytes.
        cap: usize,
    },
    /// The porcelain and the human-format pass (18e) disagree on the lines:
    /// a different count, or a line whose commit differs (18f). The
    /// working-tree file changed between the two reads.
    Changed {
        /// Lines in the porcelain. The passes' line counts are equal when
        /// a line's commit differed.
        first: usize,
        /// Lines in the human format.
        second: usize,
    },
}

impl fmt::Display for BlameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BlameError::Run(error) => write!(f, "{error}"),
            BlameError::Parse(error) => write!(
                f,
                "git blame's output made no sense at line {}: {}",
                error.line, error.what
            ),
            BlameError::Binary => write!(f, "the file is binary"),
            BlameError::Untracked => write!(f, "the file isn't committed yet"),
            BlameError::Stopped => write!(f, "blame was stopped"),
            BlameError::Shallow { path, source } => {
                write!(f, "couldn't read {}: {source}", path.display())
            }
            BlameError::TooLarge { cap } => write!(
                f,
                "this file's blame is over {} MiB, too large to show",
                cap / (1024 * 1024)
            ),
            BlameError::Changed { .. } => {
                write!(f, "the file changed while blame read it; open blame again")
            }
        }
    }
}

impl std::error::Error for BlameError {}

impl From<RunError> for BlameError {
    fn from(error: RunError) -> Self {
        match error {
            RunError::Stopped => BlameError::Stopped,
            other => BlameError::Run(other),
        }
    }
}

/// Blames `path` (relative to the repository root) as of `rev`.
///
/// Refuses a binary file and a path the version does not have before the
/// blame itself spawns; the check is one `diff-tree`/`diff-index --numstat`
/// against the empty tree, so "binary" means what it means to the diff
/// (attributes first, then content).
///
/// # Errors
/// See [`BlameError`].
pub fn blame(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    opts: &BlameOptions,
    stop: &dyn Stop,
) -> Result<Blame, BlameError> {
    blame_with(repo, rev, path, opts, stop, &Knobs::real())
}

/// [`blame`] with its inputs set by the caller ([`Knobs`]): a test lowers a
/// cap to reach the too-large refusal through the real spawn, forces the
/// human-format pass, or makes the running git look older than it is.
#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "the same parameters as `blame`, whose public signature takes `&BlameOptions`"
)]
pub(crate) fn blame_with(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    opts: &BlameOptions,
    stop: &dyn Stop,
    knobs: &Knobs,
) -> Result<Blame, BlameError> {
    refuse_early(repo, rev, path, stop)?;
    let found = find_ignore_revs(repo);

    let output = blame_invocation(repo, rev, path, *opts, found.as_ref(), Format::Porcelain)
        .output_cap(knobs.cap)
        .run(stop)?;
    let (mut lines, mut commits) = read_porcelain(&output, knobs.cap)?;
    if knobs.porcelain_marks == PorcelainMarks::Dropped {
        for line in &mut lines {
            line.mark = LineMark::Plain;
        }
    }
    let porcelain_has_marks = lines.iter().any(|line| line.mark != LineMark::Plain);
    let wanted = match knobs.marks {
        MarksFrom::HumanFormat => true,
        MarksFrom::Auto => marks_pass_wanted(
            repo,
            *opts,
            found.is_some(),
            porcelain_has_marks,
            &knobs.git,
            stop,
        )?,
    };
    if wanted {
        let marks = read_marks(
            repo,
            rev,
            path,
            *opts,
            found.as_ref(),
            knobs.marks_cap,
            stop,
        )?;
        merge_marks(&mut lines, marks)?;
    }
    let shallow = ShallowList::read(repo).map_err(|error| BlameError::Shallow {
        path: error.path,
        source: error.source,
    })?;
    for commit in commits.values_mut() {
        commit.shallow = shallow.contains(&commit.id);
    }
    Ok(Blame {
        path: path.to_path_buf(),
        rev: *rev,
        lines,
        commits,
        ignore_revs: found.and_then(|found| found.summary(opts.ignore_revs)),
    })
}

/// git's porcelain, parsed. Output cut at the runner's `cap` is refused: a
/// cut inside a text line parses cleanly as a shorter file, and the screen
/// would show it as whole.
fn read_porcelain(
    output: &GitOutput,
    cap: usize,
) -> Result<(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>), BlameError> {
    if output.truncated {
        return Err(BlameError::TooLarge { cap });
    }
    parse_porcelain(&output.stdout).map_err(BlameError::Parse)
}

#[cfg(unix)]
pub(crate) fn path_bytes(path: &Path) -> Vec<u8> {
    use std::os::unix::ffi::OsStrExt;
    path.as_os_str().as_bytes().to_vec()
}

#[cfg(not(unix))]
pub(crate) fn path_bytes(path: &Path) -> Vec<u8> {
    path.to_string_lossy().into_owned().into_bytes()
}

#[cfg(unix)]
pub(crate) fn bytes_path(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
pub(crate) fn bytes_path(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests;
