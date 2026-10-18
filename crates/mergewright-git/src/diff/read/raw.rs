//! What `list_files`/`file_diff` need before or after the git spawn itself:
//! working out which two revisions a [`DiffSpec::Commit`]/[`DiffSpec::Range`]
//! names ([`Sides`]/[`resolve_sides`]/[`with_sides_args`]/[`first_parent`]),
//! whether `HEAD` exists at all ([`ensure_head`]), and splitting a combined
//! `--raw --numstat -z` stream back into its two parts. Split out of
//! `read.rs` (m2 review M8) — nothing here runs a git command of its own
//! shape; [`super`] is the layer that decides which command to run.

use mergewright_core::oid::Oid;

use super::{DiffError, DiffSpec, ParseError};
use crate::repo::{HeadState, RepoHandle};
use crate::run::{GitArg, GitInvocation, Stop};

use super::OUTPUT_CAP;

/// The two revisions a [`DiffSpec::Commit`] or [`DiffSpec::Range`] compares.
#[derive(Clone, Copy)]
pub(super) enum Sides {
    /// A root commit: diff against the empty tree (`--root`).
    Root(Oid),
    /// An ordinary old/new pair.
    TwoRevs(Oid, Oid),
}

/// Works out [`Sides`] for a [`DiffSpec::Commit`] or [`DiffSpec::Range`].
/// Never called for [`DiffSpec::WorkingTreeVsHead`], which has its own
/// commands.
pub(super) fn resolve_sides(
    repo: &RepoHandle,
    spec: &DiffSpec,
    stop: &dyn Stop,
) -> Result<Sides, DiffError> {
    match spec {
        DiffSpec::Commit {
            commit,
            against: Some(against),
        } => Ok(Sides::TwoRevs(*against, *commit)),
        DiffSpec::Commit {
            commit,
            against: None,
        } => match first_parent(repo, *commit, stop)? {
            Some(parent) => Ok(Sides::TwoRevs(parent, *commit)),
            None => Ok(Sides::Root(*commit)),
        },
        DiffSpec::Range { base, head } => Ok(Sides::TwoRevs(*base, *head)),
        DiffSpec::WorkingTreeVsHead | DiffSpec::Staged | DiffSpec::Unstaged => {
            unreachable!("callers handle these specs before calling resolve_sides")
        }
    }
}

/// Adds `[--root]`/`<old> <new>` to a `diff-tree` invocation.
pub(super) fn with_sides_args(invocation: GitInvocation, sides: Sides) -> GitInvocation {
    match sides {
        Sides::Root(commit) => invocation
            .arg(GitArg::Flag("--root"))
            .arg(GitArg::Oid(commit)),
        Sides::TwoRevs(old, new) => invocation.args([GitArg::Oid(old), GitArg::Oid(new)]),
    }
}

/// A commit's first parent, or `None` for a root commit.
///
/// `git rev-list --parents -1 <commit>` rather than reading the commit
/// object ourselves: this crate's other object reads go through `gix`
/// ([`crate::history::read_commit`]), whose error type does not fit
/// [`DiffError`]; this stays inside the typed runner this module already
/// depends on.
fn first_parent(repo: &RepoHandle, commit: Oid, stop: &dyn Stop) -> Result<Option<Oid>, DiffError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--parents"),
            GitArg::Flag("-1"),
            GitArg::Oid(commit),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let line = output
        .stdout
        .split(|&b| b == b'\n')
        .next()
        .unwrap_or_default();
    let mut ids = line.split(|&b| b == b' ');
    ids.next(); // the commit itself
    let Some(parent) = ids.next() else {
        return Ok(None);
    };
    let hex = std::str::from_utf8(parent).map_err(|_| {
        DiffError::Parse(ParseError {
            line: 0,
            what: "rev-list printed a parent id that was not UTF-8",
        })
    })?;
    Oid::from_hex(hex).map(Some).map_err(|_| {
        DiffError::Parse(ParseError {
            line: 0,
            what: "rev-list printed a parent id git's own parser rejected",
        })
    })
}

/// `HEAD` names a commit; `Err(NoHead)` on a fresh repository with nothing
/// committed yet.
pub(super) fn ensure_head(repo: &RepoHandle) -> Result<(), DiffError> {
    match repo.head().map_err(|_| DiffError::NoHead)? {
        HeadState::Unborn(_) => Err(DiffError::NoHead),
        HeadState::Branch(_) | HeadState::Detached(_) => Ok(()),
    }
}

/// Splits the bytes of one `--raw --numstat -z` invocation into separate
/// `--raw -z` and `--numstat -z` streams, in the shape [`super::parse_file_list`]
/// expects.
///
/// Git prints every raw record first, then every numstat record (not
/// interleaved file by file), so the split is just "where does the raw
/// block end" — the first token that is not a raw header (does not start
/// with `:`). A single-`<tree-ish>` invocation (the `--root` case) prints a
/// leading commit id first; that token is dropped.
pub(super) fn split_raw_and_numstat(combined: &[u8]) -> (Vec<u8>, Vec<u8>) {
    let tokens = split_nul(combined);
    let mut index = match tokens.first() {
        Some(first) if !first.starts_with(b":") => 1,
        _ => 0,
    };
    let mut raw = Vec::new();
    while let Some(&header) = tokens.get(index) {
        if !header.starts_with(b":") {
            break;
        }
        let rename_or_copy = header
            .rsplit(|&b| b == b' ')
            .next()
            .and_then(|status| status.first())
            .is_some_and(|&letter| letter == b'R' || letter == b'C');
        let path_tokens = if rename_or_copy { 2 } else { 1 };
        let end = index + 1 + path_tokens;
        let Some(slice) = tokens.get(index..end) else {
            break;
        };
        push_record(&mut raw, slice);
        index = end;
    }
    let mut numstat = Vec::new();
    push_record(&mut numstat, &tokens[index..]);
    (raw, numstat)
}

/// Appends `tokens`, each followed by a NUL, to `buf`.
fn push_record(buf: &mut Vec<u8>, tokens: &[&[u8]]) {
    for token in tokens {
        buf.extend_from_slice(token);
        buf.push(0);
    }
}

/// Splits `bytes` on NUL, dropping the empty tail after the final
/// terminator.
pub(super) fn split_nul(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut tokens: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
    if bytes.ends_with(&[0]) {
        tokens.pop();
    }
    tokens
}
