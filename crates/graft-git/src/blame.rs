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
//! porcelain carried no mark ([`marks::needs_marks_pass`]), a second `git
//! blame -l -s` with the same options reads them from the human format's
//! `?`/`*` ([`parse_marks`]), by position.
//!
//! The working tree is blamed by passing **no** rev: git then reads the file
//! from disk and gives every line it cannot find in `HEAD` the all-zero id,
//! which [`parse_porcelain`] turns into `commit: None`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use graft_core::oid::Oid;

use crate::diff::ParseError;
use crate::repo::RepoHandle;
use crate::run::{DEFAULT_CAP, GitArg, GitInvocation, GitOutput, RunError, Stop};
use crate::trace::shallow::read_shallow;
use crate::version::system_git;

mod ignore_revs;
mod marks;
mod porcelain;

#[cfg(test)]
use ignore_revs::count_revisions;
use ignore_revs::{FoundIgnoreRevs, find_ignore_revs};
pub(crate) use marks::needs_marks_pass;
pub use marks::parse_marks;
pub use porcelain::parse_porcelain;

/// Where [`blame_with`] takes each line's mark from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MarksFrom {
    /// The porcelain, plus the human-format pass when
    /// [`needs_marks_pass`] says so.
    Auto,
    /// Always the human-format pass: the oracle test on a git whose
    /// porcelain has marks of its own.
    #[cfg_attr(not(test), allow(dead_code))]
    HumanFormat,
}

/// What [`blame`] fixes and a test may change.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Knobs {
    /// The runner's output cap for each blame spawn.
    pub cap: usize,
    /// Where the marks come from.
    pub marks: MarksFrom,
}

#[cfg(test)]
thread_local! {
    /// How many human-format passes [`blame_with`] ran on this thread: what
    /// a test observes to say "one blame spawn" (the runner is synchronous,
    /// so the spawn happens on the caller's thread).
    pub(crate) static MARKS_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The file git reads ignore-revs from by convention, at the repository root.
pub const IGNORE_REVS_FILE: &str = ".git-blame-ignore-revs";

/// Git's id for the empty tree, which every repository knows without storing
/// it: the old side of the "is this path here, and is it text" check.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

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
    /// The porcelain and the human-format pass (18e) saw different line
    /// counts: the working-tree file changed between the two reads.
    Changed {
        /// Lines in the porcelain.
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
    blame_with(
        repo,
        rev,
        path,
        opts,
        stop,
        &Knobs {
            cap: DEFAULT_CAP,
            marks: MarksFrom::Auto,
        },
    )
}

/// [`blame`] with the runner's cap and the marks' source set by the caller:
/// a test lowers the cap to reach the too-large refusal through the real
/// spawn, or forces the human-format pass.
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
    let porcelain_has_marks = lines.iter().any(|line| line.mark != LineMark::Plain);
    let wanted = match knobs.marks {
        MarksFrom::HumanFormat => true,
        MarksFrom::Auto => {
            marks_pass_wanted(repo, *opts, found.is_some(), porcelain_has_marks, stop)?
        }
    };
    if wanted {
        #[cfg(test)]
        MARKS_PASSES.with(|passes| passes.set(passes.get() + 1));
        let output = blame_invocation(repo, rev, path, *opts, found.as_ref(), Format::Human)
            .output_cap(knobs.cap)
            .run(stop)?;
        if output.truncated {
            return Err(BlameError::TooLarge { cap: knobs.cap });
        }
        merge_marks(
            &mut lines,
            parse_marks(&output.stdout).map_err(BlameError::Parse)?,
        )?;
    }
    let shallow =
        read_shallow(repo).map_err(|(path, source)| BlameError::Shallow { path, source })?;
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

/// The human-format marks onto the porcelain's lines, by position. A
/// different count is [`BlameError::Changed`]: possible only for
/// [`Rev::WorkingTree`], whose file both passes read from disk.
fn merge_marks(lines: &mut [BlameLine], marks: Vec<LineMark>) -> Result<(), BlameError> {
    if marks.len() != lines.len() {
        return Err(BlameError::Changed {
            first: lines.len(),
            second: marks.len(),
        });
    }
    for (line, mark) in lines.iter_mut().zip(marks) {
        line.mark = mark;
    }
    Ok(())
}

/// [`needs_marks_pass`], each input read only once the cheaper ones leave
/// the answer open: conditions 1 and 2 cost nothing, the configured list is
/// one `git config` spawn, the version one `git --version` spawn. A
/// repository with no list pays neither on any git.
fn marks_pass_wanted(
    repo: &RepoHandle,
    opts: BlameOptions,
    passed_root_list: bool,
    porcelain_has_marks: bool,
    stop: &dyn Stop,
) -> Result<bool, BlameError> {
    if !needs_marks_pass(opts.ignore_revs, porcelain_has_marks, true, None) {
        return Ok(false);
    }
    let list_in_effect = passed_root_list || configured_list(repo, stop)?;
    if !list_in_effect {
        return Ok(false);
    }
    let git = match system_git() {
        Ok(found) => Some(found.version()),
        // An unreadable version takes the pass: slower, still right. The
        // blame itself just ran, so git is there; what failed is only the
        // version read, and guessing "new" would drop marks.
        Err(_unreadable) => None,
    };
    Ok(needs_marks_pass(
        opts.ignore_revs,
        porcelain_has_marks,
        list_in_effect,
        git.as_ref(),
    ))
}

/// Whether `blame.ignoreRevsFile` is set. A spawn, not the in-process
/// config snapshot: `RepoHandle` offers no config read, and a snapshot taken
/// when the handle opened could miss a later `git config`, which would drop
/// the marks.
fn configured_list(repo: &RepoHandle, stop: &dyn Stop) -> Result<bool, BlameError> {
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("config"),
            GitArg::Flag("--get"),
            GitArg::Flag("blame.ignoreRevsFile"),
        ])
        .run_raw(stop)?;
    match raw.code {
        Some(0) => Ok(true),
        // `git config --get` exits 1 for a key that is not set.
        Some(1) => Ok(false),
        Some(status) => Err(BlameError::Run(RunError::Exited {
            status,
            stderr_first_line: crate::commit_graph::first_line(&raw.output.stderr),
            command: "git config --get blame.ignoreRevsFile".to_owned(),
        })),
        // Only the runner kills without a code: its output cap, which a
        // config value cannot reach in practice; still refused, not guessed.
        None => Err(BlameError::TooLarge { cap: DEFAULT_CAP }),
    }
}

/// Which of blame's two output formats to ask for.
#[derive(Clone, Copy)]
enum Format {
    /// `--porcelain`: lines, commits, and (git ≥ 2.50) marks.
    Porcelain,
    /// `-l -s`: one line per final line, marks before the full id (18e).
    Human,
}

/// `git blame` with the options both passes share, so they blame the same
/// thing. The human format's configs are pinned (2.38.5 `builtin/blame.c`):
/// `blame.blankBoundary=false` keeps the id where `-b` would blank it
/// (`emit_other`, line 472); `blame.coloring=none` and `color.ui=never` keep
/// colour codes off the front of the line (lines 449-469, 735-740, 1181).
fn blame_invocation(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    opts: BlameOptions,
    found: Option<&FoundIgnoreRevs>,
    format: Format,
) -> GitInvocation {
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markIgnoredLines=true"),
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markUnblamableLines=true"),
    ]);
    invocation = match format {
        Format::Porcelain => invocation.args([GitArg::Flag("blame"), GitArg::Flag("--porcelain")]),
        Format::Human => invocation.args([
            GitArg::Flag("-c"),
            GitArg::Flag("blame.blankBoundary=false"),
            GitArg::Flag("-c"),
            GitArg::Flag("blame.coloring=none"),
            GitArg::Flag("-c"),
            GitArg::Flag("color.ui=never"),
            GitArg::Flag("blame"),
            GitArg::Flag("-l"),
            GitArg::Flag("-s"),
        ]),
    };
    invocation = invocation.arg(GitArg::Flag("-M"));
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = match (found, opts.ignore_revs) {
        // Passed even when it could not be read here: git then fails with
        // its own words, rather than this reader quietly blaming without it.
        (Some(found), true) => invocation.arg(GitArg::Glued(
            "--ignore-revs-file=",
            path_bytes(&found.path),
        )),
        // Asked to pass it, and there is none at the root: a configured
        // `blame.ignoreRevsFile` is git's to honour.
        (None, true) => invocation,
        // `--no-ignore-revs-file` drops the configured `blame.ignoreRevsFile`
        // before git opens it. An empty `--ignore-revs-file=` does not: git
        // still opens a configured file first, and a missing one is fatal
        // ("could not open object name list"), probed on git 2.54 (18d).
        (_, false) => invocation.arg(GitArg::Flag("--no-ignore-revs-file")),
    };
    if let Rev::Commit(commit) = rev {
        invocation = invocation.arg(GitArg::Oid(*commit));
    }
    invocation
        .arg(GitArg::Separator)
        .arg(GitArg::Path(path_bytes(path)))
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

/// [`BlameError::Untracked`] or [`BlameError::Binary`] before blame runs.
fn refuse_early(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    stop: &dyn Stop,
) -> Result<(), BlameError> {
    let tree_side = match rev {
        Rev::Commit(commit) => GitArg::Oid(*commit),
        Rev::WorkingTree => GitArg::Ref(b"HEAD".to_vec()),
    };
    let in_tree = numstat(
        repo,
        [
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("--numstat"),
            GitArg::Flag(EMPTY_TREE),
            tree_side,
        ],
        path,
        stop,
    )?;
    let Some(binary) = in_tree else {
        return Err(BlameError::Untracked);
    };
    // For the working tree, what counts is the file on disk: `diff-index`
    // without `--cached` reads the working-tree content.
    let binary = match rev {
        Rev::Commit(_) => binary,
        Rev::WorkingTree => numstat(
            repo,
            [
                GitArg::Flag("diff-index"),
                GitArg::Flag("--no-renames"),
                GitArg::Flag("--numstat"),
                GitArg::Flag(EMPTY_TREE),
            ],
            path,
            stop,
        )?
        .unwrap_or(binary),
    };
    if binary {
        return Err(BlameError::Binary);
    }
    Ok(())
}

/// Runs a `--numstat` diff against the empty tree for `path` alone:
/// `None` when the path is not on the new side, else whether it is binary
/// (`-\t-\t`).
fn numstat(
    repo: &RepoHandle,
    args: impl IntoIterator<Item = GitArg>,
    path: &Path,
    stop: &dyn Stop,
) -> Result<Option<bool>, BlameError> {
    let output = GitInvocation::new(repo)
        .args(args)
        .arg(GitArg::Separator)
        .arg(GitArg::Path(path_bytes(path)))
        .run(stop)?;
    let first = output
        .stdout
        .split(|&b| b == b'\n')
        .find(|line| !line.is_empty());
    Ok(first.map(|line| line.starts_with(b"-\t-\t")))
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
