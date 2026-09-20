//! Reads M5's history operations and Ghost Preview are planned on: where two
//! tips diverged, the commits between two points, whether a merge or a
//! replayed commit would conflict, which paths a commit touched, and the
//! detail of a rebase in progress (`state.rs`'s `in_progress_detail`).
//!
//! Every spawn here goes through [`crate::run::GitInvocation`], never a
//! bare shell string (PROMPT.md Technology). Nothing writes.

use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use graft_core::commit::{Identity, Timestamp};
use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};
use crate::version::GitVersion;

/// The oldest git that can run `merge-tree --write-tree` at all.
const MERGE_TREE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 38,
    patch: 0,
};

/// The oldest git whose `merge-tree` understands `--merge-base`.
const MERGE_TREE_BASE_MIN: GitVersion = GitVersion {
    major: 2,
    minor: 40,
    patch: 0,
};

/// Where two commits' histories diverged, or `None` if they share no
/// ancestor at all (two unrelated root commits).
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped, or either id
/// naming no commit in this repository.
pub fn merge_base(repo: &RepoHandle, a: Oid, b: Oid) -> Result<Option<Oid>, RunError> {
    let invocation =
        GitInvocation::new(repo).args([GitArg::Flag("merge-base"), GitArg::Oid(a), GitArg::Oid(b)]);
    match invocation.run(&()) {
        Ok(output) => {
            let hex = String::from_utf8_lossy(&output.stdout);
            let hex = hex.trim();
            Oid::from_hex(hex).map(Some).map_err(|why| {
                // git ran, exited 0 and printed something that is not a
                // valid id: not a shape `merge-base` should ever produce.
                RunError::Spawn(io::Error::other(format!(
                    "merge-base printed {hex:?}, not an id: {why}"
                )))
            })
        }
        // No common ancestor: `merge-base` exits 1 with empty stdout, not
        // an error condition for this reader.
        Err(RunError::Exited { status: 1, .. }) => Ok(None),
        Err(other) => Err(other),
    }
}

/// One commit's summary, as `commits_between` reads it: identity, not the
/// full message body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommitSummary {
    /// The commit's own id.
    pub id: Oid,
    /// The parents, first parent first (a merge has more than one, a root
    /// commit has none).
    pub parents: Vec<Oid>,
    /// The subject line only.
    pub subject: String,
    /// Who authored it.
    pub author: Identity,
}

/// A bounded slice of history: [`commits_between`]'s answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range {
    /// The commits, oldest first.
    pub commits: Vec<CommitSummary>,
    /// `true` when more commits exist past `cap`, nearest history dropped
    /// first: the entries kept are always the ones closest to `include`.
    pub capped: bool,
}

/// The commits reachable from `include` but not from `exclude`
/// (`exclude..include`), oldest first, capped at `cap` entries.
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or either id naming
/// no commit in this repository.
pub fn commits_between(
    repo: &RepoHandle,
    exclude: Oid,
    include: Oid,
    cap: usize,
    stop: &dyn Stop,
) -> Result<Range, RunError> {
    let max_count = cap.saturating_add(1);
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--reverse"),
            GitArg::Word("--date=raw".to_owned()),
            GitArg::Word(format!("--max-count={max_count}")),
            GitArg::Word("--format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%s".to_owned()),
            GitArg::Oid(include),
            GitArg::Word(format!("^{exclude}")),
        ])
        .run(stop)?;

    let mut commits = parse_rev_list(&output.stdout).map_err(|why| {
        // git's own well-known `--format` output not parsing is not a
        // clean exit-code failure; the runner's "talking to git failed"
        // bucket is the closest fit (`RunError` has no dedicated "git
        // answered but graft-git could not read it" case, unlike
        // `version::VersionError::Unrecognised` — adding one just for this
        // single caller was not worth the extra surface).
        RunError::Spawn(io::Error::other(format!(
            "rev-list's output did not parse: {why}"
        )))
    })?;
    let capped = commits.len() > cap;
    if capped {
        // Keep the entries nearest `include`: drop from the oldest end.
        commits.drain(0..commits.len() - cap);
    }
    Ok(Range { commits, capped })
}

/// Parses `git rev-list --format=%H%x00%P%x00%an%x00%ae%x00%ad%x00%s
/// --date=raw` output: each commit is two lines, `commit <id>` then the
/// formatted line, fields NUL-separated. Pure, no `git` spawn — shaped for a
/// `cargo-fuzz` target alongside `diff::parse_unified`.
///
/// # Errors
/// [`ParseError`] naming the 1-indexed line where the shape broke.
pub fn parse_rev_list(bytes: &[u8]) -> Result<Vec<CommitSummary>, ParseError> {
    let mut commits = Vec::new();
    let mut lines = bytes.split(|&b| b == b'\n').enumerate();
    while let Some((index, line)) = lines.next() {
        if line.is_empty() {
            continue; // the blank tail after the final newline
        }
        if !line.starts_with(b"commit ") {
            return Err(ParseError {
                line: index + 1,
                what: "expected a `commit` header line",
            });
        }
        let (data_index, data) = lines.next().ok_or(ParseError {
            line: index + 2,
            what: "missing the formatted line after `commit`",
        })?;
        commits.push(parse_summary_line(data, data_index + 1)?);
    }
    Ok(commits)
}

/// One `%H\0%P\0%an\0%ae\0%ad\0%s` line, `%ad` read with `--date=raw`
/// (`<seconds> <+/-offset>`, git's own on-disk shape).
fn parse_summary_line(data: &[u8], line: usize) -> Result<CommitSummary, ParseError> {
    let mut fields = data.split(|&b| b == 0);
    let mut next = |what: &'static str| fields.next().ok_or(ParseError { line, what });

    let id_hex = next("missing the id field")?;
    let parents_field = next("missing the parents field")?;
    let name = next("missing the author name field")?;
    let email = next("missing the author email field")?;
    let when = next("missing the author date field")?;
    let subject = next("missing the subject field")?;

    let id = Oid::from_hex(&String::from_utf8_lossy(id_hex)).map_err(|_| ParseError {
        line,
        what: "the id is not valid hex",
    })?;
    let parents = if parents_field.is_empty() {
        Vec::new()
    } else {
        parents_field
            .split(|&b| b == b' ')
            .map(|hex| {
                Oid::from_hex(&String::from_utf8_lossy(hex)).map_err(|_| ParseError {
                    line,
                    what: "a parent id is not valid hex",
                })
            })
            .collect::<Result<_, _>>()?
    };
    let when = parse_raw_date(when).ok_or(ParseError {
        line,
        what: "the author date is not `--date=raw`'s `<seconds> <offset>`",
    })?;

    Ok(CommitSummary {
        id,
        parents,
        subject: String::from_utf8_lossy(subject).into_owned(),
        author: Identity {
            name: name.to_vec(),
            email: email.to_vec(),
            when,
        },
    })
}

/// `<seconds> <+/-HHMM>`, exactly what `--date=raw` prints and what git
/// itself stores — the same two numbers [`Timestamp`] holds.
fn parse_raw_date(field: &[u8]) -> Option<Timestamp> {
    let text = std::str::from_utf8(field).ok()?;
    let (seconds, offset) = text.split_once(' ')?;
    let seconds: i64 = seconds.parse().ok()?;
    if offset.len() != 5 {
        return None;
    }
    let sign = match offset.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hours: i32 = offset[1..3].parse().ok()?;
    let minutes: i32 = offset[3..5].parse().ok()?;
    Some(Timestamp {
        seconds,
        offset_seconds: sign * (hours * 3600 + minutes * 60),
    })
}

/// Why [`parse_rev_list`] could not read `rev-list`'s output.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("rev-list output at line {line}: {what}")]
pub struct ParseError {
    /// The 1-indexed line the shape broke at.
    pub line: usize,
    /// What was expected there.
    pub what: &'static str,
}

/// What a hypothetical merge of `theirs` into `ours` would do, per
/// `git merge-tree`.
///
/// **Per commit, not chained** (design note, Hard choice 5): forecasting a
/// whole rebase or cherry-pick means calling this once per replayed commit,
/// each time against its own predecessor — not by building the rebased
/// history through temporary commits, which needs object writes this reader
/// never does. A commit that would conflict against its immediate
/// predecessor but not against the final rebased state (or vice versa) is
/// not distinguished: the wording callers show is "may conflict", never
/// "will".
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forecast {
    /// No conflicting paths.
    Clean,
    /// These paths would conflict.
    Conflicts {
        /// The conflicting paths, as `merge-tree --name-only` named them.
        paths: Vec<PathBuf>,
    },
    /// The forecast could not be made — an old git, or an answer this
    /// reader does not recognise. Never treated as an error: PROMPT.md
    /// §1.4's rule that absence of a check is reported as itself, not as a
    /// pass.
    NotCheckable(String),
}

/// Forecasts merging `theirs` into `ours`, optionally from a given merge
/// base (a rebase or cherry-pick's forecast: `--merge-base=<the replayed
/// commit's own parent>`).
///
/// `git merge-tree --write-tree --name-only --no-messages [--merge-base=<base>]
/// <ours> <theirs>`: exit 0 is a clean merge, exit 1 is a tree id line
/// followed by the conflicting paths, anything else is an error this
/// reader could not make sense of.
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped. **Never** for a
/// forecast that could not be made — that is [`Forecast::NotCheckable`].
pub fn forecast_merge(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Option<Oid>,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<Forecast, RunError> {
    if base.is_some() {
        if *git < MERGE_TREE_BASE_MIN {
            return Ok(Forecast::NotCheckable(
                "git 2.40+ needed for a rebase forecast".to_owned(),
            ));
        }
    } else if *git < MERGE_TREE_MIN {
        return Ok(Forecast::NotCheckable(
            "git 2.38+ needed to forecast a merge".to_owned(),
        ));
    }

    let mut args = vec![
        GitArg::Flag("merge-tree"),
        GitArg::Flag("--write-tree"),
        GitArg::Flag("--name-only"),
        GitArg::Flag("--no-messages"),
    ];
    if let Some(base) = base {
        args.push(GitArg::Word(format!("--merge-base={base}")));
    }
    args.push(GitArg::Oid(ours));
    args.push(GitArg::Oid(theirs));

    let raw = GitInvocation::new(repo).args(args).run_raw(stop)?;
    let first_stderr_line = crate::commit_graph::first_line(&raw.output.stderr);
    match raw.code {
        Some(0) => Ok(Forecast::Clean),
        Some(1) => {
            let mut lines = raw.output.stdout.split(|&b| b == b'\n');
            match lines.next() {
                // A tree id on the first line: the rest, if any, are the
                // conflicting paths `--name-only` printed.
                Some(tree_id) if !tree_id.is_empty() => Ok(Forecast::Conflicts {
                    paths: lines
                        .filter(|line| !line.is_empty())
                        .map(bytes_to_pathbuf)
                        .collect(),
                }),
                // Exit 1 with nothing on stdout is `merge-tree` refusing
                // outright (a bad ref, say) rather than reporting a
                // conflict: the shape this reader knows how to trust is
                // gone, so this is not checkable, not an error.
                _ => Ok(Forecast::NotCheckable(first_stderr_line)),
            }
        }
        _ => Ok(Forecast::NotCheckable(first_stderr_line)),
    }
}

/// The paths `from` and `to` differ on, capped at `cap`.
///
/// `git diff-tree -r --name-only -z <from> <to>`.
///
/// # Errors
/// [`RunError`] from the runner: git missing, or either id naming no commit
/// in this repository. Never stopped: this is a single small spawn.
pub fn changed_paths(
    repo: &RepoHandle,
    from: Oid,
    to: Oid,
    cap: usize,
) -> Result<(Vec<PathBuf>, bool), RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            GitArg::Oid(from),
            GitArg::Oid(to),
        ])
        .run(&())?;

    let mut paths: Vec<&[u8]> = output
        .stdout
        .split(|&b| b == 0)
        .filter(|path| !path.is_empty())
        .collect();
    let capped = paths.len() > cap;
    paths.truncate(cap);
    Ok((paths.into_iter().map(bytes_to_pathbuf).collect(), capped))
}

/// The 1-indexed line number of the first line starting `<<<<<<< ` in the
/// file at `path`, if any: an unresolved conflict marker left in a staged
/// file. Plain fs read, no `git` spawn.
///
/// # Errors
/// [`io::Error`] if the file could not be opened or read.
pub fn first_marker_line(path: &Path) -> io::Result<Option<usize>> {
    let file = std::fs::File::open(path)?;
    for (index, line) in io::BufReader::new(file).lines().enumerate() {
        if line?.starts_with("<<<<<<< ") {
            return Ok(Some(index + 1));
        }
    }
    Ok(None)
}

#[cfg(unix)]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn bytes_to_pathbuf(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
mod tests;
