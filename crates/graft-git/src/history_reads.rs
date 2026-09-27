//! Reads M5's history operations and Ghost Preview are planned on: where two
//! tips diverged, the commits between two points, whether a merge or a
//! replayed commit would conflict, which paths a commit touched, and the
//! detail of a rebase in progress (`state.rs`'s `in_progress_detail`).
//!
//! Every spawn here goes through [`crate::run::GitInvocation`], never a
//! bare shell string (PROMPT.md Technology). Every reader here except
//! [`forecast_merge`] never writes: `git merge-tree --write-tree` writes
//! the merged/conflicted tree and its blobs into the repository's object
//! database (that is how it can print a tree id) — see [`forecast_merge`]'s
//! own doc for what that costs.

use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use graft_core::commit::{Identity, Timestamp};
use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};
use crate::version::GitVersion;

mod compare;
mod unreached;

pub use compare::{CompareSpan, StashOnly, compare_span, count_between, only_in_stash};
pub use unreached::{LostRun, Unreached, unreached};

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
                RunError::Unparsable(format!("merge-base printed {hex:?}, not an id: {why}"))
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
    rev_list(
        repo,
        cap,
        vec![GitArg::Oid(include), GitArg::Word(format!("^{exclude}"))],
        stop,
    )
}

/// The commits `git rebase <onto>` would replay from `branch`, oldest
/// first, capped at `cap` entries.
///
/// This is **not** `commits_between(onto, branch)`. `git rebase` builds its
/// todo list from a narrower walk than `branch ^onto`, and the Ghost
/// Preview draws one ghost per entry in that list, so the two must agree or
/// the box promises commits git will not write (m5/43):
///
/// - `--no-merges`: without `--rebase-merges`, git drops merge commits from
///   the range instead of recreating them. graft never passes
///   `--rebase-merges` (`operation::history::plan_rebase`).
/// - `--cherry-pick --right-only`: git skips a commit whose change is
///   already on `onto` under a different id — the "skipped previously
///   applied commit" hint. That is git's default; only
///   `--reapply-cherry-picks` turns it off, and graft never passes it.
/// - `--topo-order`: git walks the todo list in graph order, not in commit
///   date order, so a clock skew cannot reorder the ghosts.
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or either id naming
/// no commit in this repository.
pub fn replayed_commits(
    repo: &RepoHandle,
    onto: Oid,
    branch: Oid,
    cap: usize,
    stop: &dyn Stop,
) -> Result<Range, RunError> {
    rev_list(
        repo,
        cap,
        vec![
            GitArg::Flag("--topo-order"),
            GitArg::Flag("--no-merges"),
            GitArg::Flag("--cherry-pick"),
            GitArg::Flag("--right-only"),
            GitArg::Word(format!("{onto}...{branch}")),
        ],
        stop,
    )
}

/// One `rev-list` in the shape [`parse_rev_list`] reads, `revs` naming what
/// to walk and how. Capped by fetching one more than `cap` so the caller
/// learns whether more exist; `--reverse` is applied after `--max-count`,
/// so the entries kept are the ones nearest the tip.
pub(crate) fn rev_list(
    repo: &RepoHandle,
    cap: usize,
    revs: Vec<GitArg>,
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
        ])
        .args(revs)
        .run(stop)?;

    let mut commits = parse_rev_list(&output.stdout).map_err(|why| {
        // git ran and exited 0; the output just did not match the
        // `--format` shape this reader parses. `RunError::Unparsable`,
        // not `Spawn` — git answered fine, graft-git could not read it.
        RunError::Unparsable(format!("rev-list's output did not parse: {why}"))
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
        /// The conflicting paths, `merge-tree --name-only -z`'s raw bytes
        /// (never git's C-quoted form: see [`forecast_merge`]).
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
        // Without `-z`, `--name-only` prints conflicting paths through
        // git's C-quoting: any non-ASCII byte, `"`, `\` or control
        // character comes back as a quoted, octal-escaped literal instead
        // of the path's raw bytes (verified on git 2.54: a conflict in
        // `café.txt` prints `"caf\303\251.txt"`, quotes included). `-z`
        // NUL-terminates every line instead, tree id included, so the
        // same split handles both.
        GitArg::Flag("-z"),
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
            let mut fields = raw.output.stdout.split(|&b| b == 0);
            match fields.next() {
                // A tree id in the first field: the rest, if any, are the
                // conflicting paths `--name-only -z` printed, raw bytes.
                Some(tree_id) if !tree_id.is_empty() => Ok(Forecast::Conflicts {
                    // The trailing NUL after the last path leaves one
                    // empty field at the end; drop it (and any other
                    // stray empties) rather than treat it as a path.
                    paths: fields
                        .filter(|field| !field.is_empty())
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

/// One step of a chained replay: what `merge-tree` made of it.
///
/// [`Forecast`] without the tree is what put-back and stash need; a chained
/// rebase forecast (m7/05) needs the clean result's tree too, to make the
/// next step's `ours` from it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MergeStep {
    /// No conflicting paths; `tree` is the merged result.
    Clean {
        /// The tree `merge-tree --write-tree` wrote.
        tree: Oid,
    },
    /// These paths would conflict (raw bytes, as in [`Forecast::Conflicts`]).
    Conflicts {
        /// The conflicting paths.
        paths: Vec<PathBuf>,
    },
    /// Not checkable: an old git, or an answer this reader does not
    /// recognise. Never a clean bill.
    NotCheckable(String),
}

/// Replays `theirs` onto `ours` from `base` (the replayed commit's own
/// parent), the way one rebase step does, and returns the result tree.
///
/// `git merge-tree --write-tree --name-only --no-messages -z
/// --merge-base=<base> <ours> <theirs>` — the read [`forecast_merge`] runs,
/// with the tree id kept on a clean exit. `ours` may be a commit with no
/// ref (a `commit-tree` of the previous step's tree): git answers as for
/// the real commit (`forecast_merge_accepts_a_commit_tree_result_as_ours`).
/// git 2.40+ (`--merge-base`); older → [`MergeStep::NotCheckable`].
///
/// # Errors
/// [`RunError`] from the runner: git missing or stopped.
pub fn merge_tree_step(
    repo: &RepoHandle,
    git: &GitVersion,
    base: Oid,
    ours: Oid,
    theirs: Oid,
    stop: &dyn Stop,
) -> Result<MergeStep, RunError> {
    if *git < MERGE_TREE_BASE_MIN {
        return Ok(MergeStep::NotCheckable(
            "git 2.40+ needed for a rebase forecast".to_owned(),
        ));
    }
    let raw = GitInvocation::new(repo)
        .args([
            GitArg::Flag("merge-tree"),
            GitArg::Flag("--write-tree"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("--no-messages"),
            GitArg::Flag("-z"),
            GitArg::Word(format!("--merge-base={base}")),
            GitArg::Oid(ours),
            GitArg::Oid(theirs),
        ])
        .run_raw(stop)?;
    let first_stderr_line = crate::commit_graph::first_line(&raw.output.stderr);
    let mut fields = raw.output.stdout.split(|&b| b == 0);
    let tree = fields.next().filter(|field| !field.is_empty());
    match (raw.code, tree) {
        (Some(0), Some(tree)) => match Oid::from_hex(String::from_utf8_lossy(tree).trim()) {
            Ok(tree) => Ok(MergeStep::Clean { tree }),
            Err(_) => Ok(MergeStep::NotCheckable(format!(
                "merge-tree printed {:?}, not a tree id",
                String::from_utf8_lossy(tree)
            ))),
        },
        (Some(1), Some(_)) => Ok(MergeStep::Conflicts {
            paths: fields
                .filter(|field| !field.is_empty())
                .map(bytes_to_pathbuf)
                .collect(),
        }),
        _ => Ok(MergeStep::NotCheckable(first_stderr_line)),
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

/// The 1-indexed line number of the first line that opens a conflict block in
/// the file at `path`, if any: an unresolved conflict marker left in a staged
/// file. See [`opens_a_conflict`] for what counts. Plain fs read, no `git`
/// spawn.
///
/// Reads bytes, never decodes: a conflicted file with a non-UTF-8 byte
/// anywhere above the marker (a Latin-1 comment, binary-ish content) must
/// still find the marker, not fail to read it (M5/02 fix round —
/// `BufRead::lines()` returns `Err(InvalidData)` on the first invalid byte,
/// which made this look like an io failure, or a "no marker" `None` to a
/// caller that folds the error away, for a file that plainly has one).
///
/// # Errors
/// [`io::Error`] if the file could not be opened or read.
pub fn first_marker_line(path: &Path) -> io::Result<Option<usize>> {
    let file = std::fs::File::open(path)?;
    let mut reader = io::BufReader::new(file);
    let mut line = Vec::new();
    let mut index = 0usize;
    loop {
        line.clear();
        // `read_until` includes the `\n` (absent only on the file's last,
        // newline-less line) but that never affects a `starts_with` check.
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(None);
        }
        index += 1;
        if opens_a_conflict(&line) {
            return Ok(Some(index));
        }
    }
}

/// Whether a raw line (terminator included or not) opens a conflict block:
/// seven or more `<` at column 0, then a space and a label, or nothing.
///
/// Seven **or more**, unlike [`crate::conflict::parse`], which reads only
/// git's default size: this is the "are there markers here" check that guards
/// a stage and a commit, so it errs towards seeing one. Seven with a label is
/// the everyday form; bare `<<<<<<<` is what `rerere` writes; longer is a
/// path with `conflict-marker-size` set, which `parse` refuses by name
/// (`ParseErrorKind::MarkerSize`) rather than calling plain.
fn opens_a_conflict(line: &[u8]) -> bool {
    let run = line.iter().take_while(|&&byte| byte == b'<').count();
    if run < 7 {
        return false;
    }
    matches!(&line[run..], [] | [b'\n'] | [b'\r', b'\n'] | [b' ', ..])
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
