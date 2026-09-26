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
//! The working tree is blamed by passing **no** rev: git then reads the file
//! from disk and gives every line it cannot find in `HEAD` the all-zero id,
//! which [`parse_porcelain`] turns into `commit: None`.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use graft_core::oid::Oid;

use crate::diff::ParseError;
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

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
    /// clears any configured list too (`--ignore-revs-file ""`).
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
    /// `previous`: the parent the line came through and the path it had
    /// there. `None` at a root or boundary commit.
    pub previous: Option<(Oid, PathBuf)>,
    /// `boundary`: git's walk stopped here (a root commit, usually).
    pub boundary: bool,
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
    refuse_early(repo, rev, path, stop)?;
    let found = find_ignore_revs(repo);

    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markIgnoredLines=true"),
        GitArg::Flag("-c"),
        GitArg::Flag("blame.markUnblamableLines=true"),
        GitArg::Flag("blame"),
        GitArg::Flag("--porcelain"),
        GitArg::Flag("-M"),
    ]);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = match (&found, opts.ignore_revs) {
        // Passed even when it could not be read here: git then fails with
        // its own words, rather than this reader quietly blaming without it.
        (Some(found), true) => invocation.arg(GitArg::Glued(
            "--ignore-revs-file=",
            path_bytes(&found.path),
        )),
        // Asked to pass it, and there is none at the root: a configured
        // `blame.ignoreRevsFile` is git's to honour.
        (None, true) => invocation,
        // An empty name clears every list, the configured one included.
        (_, false) => invocation.arg(GitArg::Flag("--ignore-revs-file=")),
    };
    if let Rev::Commit(commit) = rev {
        invocation = invocation.arg(GitArg::Oid(*commit));
    }
    let output = invocation
        .arg(GitArg::Separator)
        .arg(GitArg::Path(path_bytes(path)))
        .run(stop)?;
    let (lines, commits) = parse_porcelain(&output.stdout).map_err(BlameError::Parse)?;
    Ok(Blame {
        path: path.to_path_buf(),
        rev: *rev,
        lines,
        commits,
        ignore_revs: found.and_then(|found| found.summary(opts.ignore_revs)),
    })
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

/// The root's ignore-revs file, as found on disk.
struct FoundIgnoreRevs {
    /// Its absolute path, for `--ignore-revs-file`.
    path: PathBuf,
    /// How many revisions it lists; `None` when it exists and could not be
    /// read (git, handed it, reports why).
    revisions: Option<usize>,
}

impl FoundIgnoreRevs {
    /// What [`Blame::ignore_revs`] says: only a readable file that lists
    /// something is worth a line on screen.
    fn summary(self, applied: bool) -> Option<IgnoreRevs> {
        let revisions = self.revisions.filter(|&count| count > 0)?;
        Some(IgnoreRevs {
            file: PathBuf::from(IGNORE_REVS_FILE),
            revisions,
            applied,
        })
    }
}

/// The root's `.git-blame-ignore-revs`, if there is one. A bare repository
/// has no root to look in.
fn find_ignore_revs(repo: &RepoHandle) -> Option<FoundIgnoreRevs> {
    let path = repo.workdir()?.join(IGNORE_REVS_FILE);
    match std::fs::read(&path) {
        Ok(bytes) => Some(FoundIgnoreRevs {
            path,
            revisions: Some(count_revisions(&bytes)),
        }),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_unreadable) => Some(FoundIgnoreRevs {
            path,
            revisions: None,
        }),
    }
}

/// Non-comment, non-blank lines: what git reads as revisions.
fn count_revisions(bytes: &[u8]) -> usize {
    bytes
        .split(|&b| b == b'\n')
        .map(<[u8]>::trim_ascii)
        .filter(|line| !line.is_empty() && !line.starts_with(b"#"))
        .count()
}

/// The commit headers one porcelain group may carry, gathered until the
/// line's text.
#[derive(Default)]
struct Headers {
    author: Option<String>,
    author_time: Option<i64>,
    summary: Option<String>,
    previous: Option<(Oid, PathBuf)>,
    boundary: bool,
}

/// Parses `git blame --porcelain` (or `--line-porcelain`) output. Pure, no
/// spawn; a `cargo-fuzz` target.
///
/// Each line is `<id> <orig> <final>[ <count>]`, then header lines (the
/// commit's `author`…`summary`, `previous`, `boundary`, `filename` the first
/// time the commit appears; `ignored`/`unblamable` per line), then a tab and
/// the line's text. The all-zero id is a line not committed yet: its line has
/// `commit: None` and no entry in the map.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line where the shape broke.
pub fn parse_porcelain(
    bytes: &[u8],
) -> Result<(Vec<BlameLine>, BTreeMap<Oid, BlameCommit>), ParseError> {
    let mut lines = Vec::new();
    let mut commits: BTreeMap<Oid, BlameCommit> = BTreeMap::new();
    let mut input = bytes.split(|&b| b == b'\n').enumerate().peekable();
    while let Some((index, line)) = input.next() {
        if line.is_empty() && input.peek().is_none() {
            break; // the tail after the final newline
        }
        let at = index + 1;
        let (id, orig_line, final_line) = parse_group_line(line, at)?;
        let mut headers = Headers::default();
        let mut mark = LineMark::Plain;
        let text = loop {
            let Some((index, header)) = input.next() else {
                return Err(ParseError {
                    line: at,
                    what: "a blame entry ended before its text line",
                });
            };
            if let Some(text) = header.strip_prefix(b"\t") {
                break text.to_vec();
            }
            read_header(header, index + 1, &mut headers, &mut mark)?;
        };
        let commit = (!is_zero(&id)).then_some(id);
        if let Some(id) = commit {
            record_commit(&mut commits, id, headers, at)?;
        }
        lines.push(BlameLine {
            commit,
            orig_line,
            final_line,
            text,
            mark,
        });
    }
    Ok((lines, commits))
}

/// `<id> <orig> <final>[ <count>]`.
fn parse_group_line(line: &[u8], at: usize) -> Result<(Oid, u32, u32), ParseError> {
    let mut fields = line.split(|&b| b == b' ');
    let id = fields.next().and_then(oid).ok_or(ParseError {
        line: at,
        what: "expected `<id> <orig> <final>` to start a blame entry",
    })?;
    let mut number = || {
        fields
            .next()
            .and_then(|field| std::str::from_utf8(field).ok())
            .and_then(|field| field.parse::<u32>().ok())
            .ok_or(ParseError {
                line: at,
                what: "a blame entry's line numbers are missing or not numbers",
            })
    };
    let orig = number()?;
    let fin = number()?;
    Ok((id, orig, fin))
}

/// One header line into `headers`/`mark`; unknown keys are skipped (git adds
/// headers over time — `author-mail`, `committer-*`, `filename` are not
/// kept).
fn read_header(
    line: &[u8],
    at: usize,
    headers: &mut Headers,
    mark: &mut LineMark,
) -> Result<(), ParseError> {
    let (key, value) = match line.iter().position(|&b| b == b' ') {
        Some(space) => (&line[..space], &line[space + 1..]),
        None => (line, &[][..]),
    };
    match key {
        b"author" => headers.author = Some(lossy(value)),
        b"author-time" => {
            headers.author_time = Some(
                std::str::from_utf8(value)
                    .ok()
                    .and_then(|value| value.parse().ok())
                    .ok_or(ParseError {
                        line: at,
                        what: "`author-time` is not a number",
                    })?,
            );
        }
        b"summary" => headers.summary = Some(lossy(value)),
        b"boundary" => headers.boundary = true,
        b"ignored" => *mark = LineMark::Skipped,
        b"unblamable" => *mark = LineMark::Unblamable,
        b"previous" => {
            let space = value.iter().position(|&b| b == b' ').ok_or(ParseError {
                line: at,
                what: "`previous` has no path",
            })?;
            let id = oid(&value[..space]).ok_or(ParseError {
                line: at,
                what: "`previous` names no valid id",
            })?;
            headers.previous = Some((id, bytes_path(&unquote(&value[space + 1..]))));
        }
        _ => {}
    }
    Ok(())
}

/// Adds a commit the first time its headers appear; later entries for it
/// carry none and change nothing.
fn record_commit(
    commits: &mut BTreeMap<Oid, BlameCommit>,
    id: Oid,
    headers: Headers,
    at: usize,
) -> Result<(), ParseError> {
    if commits.contains_key(&id) {
        return Ok(());
    }
    let (Some(author), Some(author_time), Some(summary)) =
        (headers.author, headers.author_time, headers.summary)
    else {
        return Err(ParseError {
            line: at,
            what: "a commit's first entry lacks `author`, `author-time` or `summary`",
        });
    };
    commits.insert(
        id,
        BlameCommit {
            id,
            author,
            author_time,
            summary,
            previous: headers.previous,
            boundary: headers.boundary,
        },
    );
    Ok(())
}

fn oid(hex: &[u8]) -> Option<Oid> {
    Oid::from_hex(std::str::from_utf8(hex).ok()?).ok()
}

fn is_zero(id: &Oid) -> bool {
    id.as_bytes().iter().all(|&b| b == 0)
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A path git C-quoted (`"src/\303\251.rs"`) back to its bytes; an unquoted
/// one as it is.
pub(crate) fn unquote(bytes: &[u8]) -> Vec<u8> {
    let Some(inner) = bytes
        .strip_prefix(b"\"")
        .and_then(|rest| rest.strip_suffix(b"\""))
    else {
        return bytes.to_vec();
    };
    let mut out = Vec::with_capacity(inner.len());
    let mut i = 0;
    while i < inner.len() {
        if inner[i] != b'\\' || i + 1 >= inner.len() {
            out.push(inner[i]);
            i += 1;
            continue;
        }
        let next = inner[i + 1];
        let octal = inner
            .get(i + 1..i + 4)
            .filter(|digits| digits.iter().all(|d| (b'0'..=b'7').contains(d)) && digits[0] <= b'3');
        if let Some(digits) = octal {
            out.push((digits[0] - b'0') * 64 + (digits[1] - b'0') * 8 + (digits[2] - b'0'));
            i += 4;
            continue;
        }
        out.push(match next {
            b'n' => b'\n',
            b't' => b'\t',
            b'r' => b'\r',
            b'a' => 0x07,
            b'b' => 0x08,
            b'f' => 0x0c,
            b'v' => 0x0b,
            other => other,
        });
        i += 2;
    }
    out
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
