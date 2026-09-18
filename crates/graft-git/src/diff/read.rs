//! Reading diffs from system git: a file list and one file's patch, with
//! truncation and cancellation.
//!
//! Every command here composes the typed runner ([`crate::run`]) with the
//! pure parsers in [`super`] (design note, hard-to-reverse choice 1): one
//! small git spawn per question, hunks parsed only for the file the user is
//! looking at.

use std::fmt;

use graft_core::oid::Oid;

use raw::{ensure_head, resolve_sides, split_nul, split_raw_and_numstat, with_sides_args};

use super::{
    FileDiff, FileEntry, FileList, FileStatus, ParseError, Truncated, parse_file_list,
    parse_unified,
};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

mod raw;

/// The most bytes kept from any one diff command's stdout/stderr. Past it
/// the runner kills the child and hands back a capped, still-successful
/// [`crate::run::GitOutput`]; here that becomes [`DiffError::TooLarge`].
const OUTPUT_CAP: usize = 64 * 1024 * 1024;

/// Which two sides of history to diff.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DiffSpec {
    /// One commit against a parent: `against: None` means its first parent
    /// (the root commit, with none, diffs against the empty tree).
    Commit {
        /// The commit being looked at.
        commit: Oid,
        /// The side to diff against, if not the first parent.
        against: Option<Oid>,
    },
    /// Two arbitrary commits, base first.
    Range {
        /// The old side.
        base: Oid,
        /// The new side.
        head: Oid,
    },
    /// The working tree (index and unstaged edits) against `HEAD`.
    WorkingTreeVsHead,
}

/// How to run a diff: how much context, and whether to ignore whitespace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DiffOptions {
    /// Unchanged lines kept around a change, each side.
    pub context: u8,
    /// `git diff`'s `-w`: whitespace-only changes produce no hunk.
    pub ignore_whitespace: bool,
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self {
            context: 3,
            ignore_whitespace: false,
        }
    }
}

/// Why a diff read failed.
#[derive(Debug)]
pub enum DiffError {
    /// The git command itself failed: missing, stopped, or refused.
    Run(RunError),
    /// git printed something these parsers do not recognise.
    Parse(ParseError),
    /// The command's output hit the cap before finishing.
    TooLarge {
        /// The cap that was hit.
        bytes: usize,
    },
    /// [`DiffSpec::WorkingTreeVsHead`] was asked for, but `HEAD` has no
    /// commit to diff against (a fresh repository with nothing committed).
    NoHead,
}

impl fmt::Display for DiffError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiffError::Run(error) => write!(f, "{error}"),
            DiffError::Parse(error) => {
                write!(f, "git's diff output made no sense: {}", error.what)
            }
            DiffError::TooLarge { bytes } => {
                write!(f, "diff too large to read (over {bytes} bytes)")
            }
            DiffError::NoHead => write!(f, "nothing committed yet"),
        }
    }
}

impl std::error::Error for DiffError {}

impl From<RunError> for DiffError {
    fn from(error: RunError) -> Self {
        DiffError::Run(error)
    }
}

/// Lists the files a diff touches, and their line counts.
///
/// # Errors
/// See [`DiffError`].
pub fn list_files(
    repo: &RepoHandle,
    spec: &DiffSpec,
    opts: &DiffOptions,
    stop: &dyn Stop,
) -> Result<FileList, DiffError> {
    if matches!(spec, DiffSpec::WorkingTreeVsHead) {
        return list_files_working_tree(repo, *opts, stop);
    }

    let sides = resolve_sides(repo, spec, stop)?;
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff-tree"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Flag("-M"),
        GitArg::Flag("-C"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag("--raw"),
        GitArg::Flag("--numstat"),
    ]);
    invocation = with_sides_args(invocation, sides);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let (raw, numstat) = split_raw_and_numstat(&output.stdout);
    parse_file_list(&raw, &numstat).map_err(DiffError::Parse)
}

/// The working tree against `HEAD`: tracked changes from `git diff HEAD`,
/// plus untracked files from `ls-files` (no line counts for those).
///
/// Porcelain `diff`, not plumbing `diff-index`, on purpose. `diff-index`
/// trusts the index's stat cache: a tracked file whose mtime moved without
/// its content changing (a branch switch, an editor save, a build) is
/// reported as modified with a `0 0` numstat, so the pane listed files the
/// user never touched. `diff` refreshes the stat cache in memory first and
/// compares content, and `GIT_OPTIONAL_LOCKS=0` (set on every invocation)
/// keeps that refresh from writing the index. The output shape is the same.
fn list_files_working_tree(
    repo: &RepoHandle,
    opts: DiffOptions,
    stop: &dyn Stop,
) -> Result<FileList, DiffError> {
    ensure_head(repo)?;

    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Flag("-M"),
        GitArg::Flag("-C"),
        GitArg::Flag("-z"),
        GitArg::Flag("--raw"),
        GitArg::Flag("--numstat"),
    ]);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = invocation.arg(GitArg::Ref(b"HEAD".to_vec()));
    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let (raw, numstat) = split_raw_and_numstat(&output.stdout);
    let mut list = parse_file_list(&raw, &numstat).map_err(DiffError::Parse)?;

    let untracked = GitInvocation::new(repo)
        .args([
            GitArg::Flag("ls-files"),
            GitArg::Flag("-z"),
            GitArg::Flag("--others"),
            GitArg::Flag("--exclude-standard"),
        ])
        .output_cap(OUTPUT_CAP)
        .run(stop)?;
    if untracked.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    for path in split_nul(&untracked.stdout) {
        list.files.push(FileEntry {
            path: path.to_vec(),
            old_path: None,
            status: FileStatus::Untracked,
            added: None,
            removed: None,
        });
    }
    Ok(list)
}

/// One file's full diff.
///
/// An [`FileStatus::Untracked`] entry has no diff yet (M3 stages it first):
/// this returns hunk-less, `truncated: None`, with no git spawned.
///
/// # Errors
/// See [`DiffError`].
pub fn file_diff(
    repo: &RepoHandle,
    spec: &DiffSpec,
    entry: &FileEntry,
    opts: &DiffOptions,
    max_lines: usize,
    stop: &dyn Stop,
) -> Result<FileDiff, DiffError> {
    if entry.status == FileStatus::Untracked {
        return Ok(empty_diff(entry));
    }

    let mut invocation = GitInvocation::new(repo);
    invocation = if matches!(spec, DiffSpec::WorkingTreeVsHead) {
        ensure_head(repo)?;
        invocation.args([
            GitArg::Flag("diff-index"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Flag("-C"),
        ])
    } else {
        invocation.args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Flag("-C"),
            GitArg::Flag("-r"),
        ])
    };
    invocation = invocation
        .arg(GitArg::Flag("-p"))
        .arg(GitArg::Count("-U", u32::from(opts.context)));
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = if matches!(spec, DiffSpec::WorkingTreeVsHead) {
        invocation.arg(GitArg::Ref(b"HEAD".to_vec()))
    } else {
        let sides = resolve_sides(repo, spec, stop)?;
        with_sides_args(invocation, sides)
    };
    invocation = invocation.arg(GitArg::Path(entry.path.clone()));
    if let Some(old_path) = &entry.old_path {
        invocation = invocation.arg(GitArg::Path(old_path.clone()));
    }

    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let mut diff = parse_unified(&output.stdout)
        .map_err(DiffError::Parse)?
        .pop()
        .unwrap_or_else(|| empty_diff(entry));
    diff.entry = entry.clone();
    apply_truncation(&mut diff, max_lines);
    Ok(diff)
}

/// A [`FileDiff`] with no hunks, for an entry no diff was read for.
fn empty_diff(entry: &FileEntry) -> FileDiff {
    FileDiff {
        entry: entry.clone(),
        hunks: Vec::new(),
        binary: None,
        truncated: None,
        old_mode: None,
        new_mode: None,
    }
}

/// Keeps whole hunks up to `max_lines`, then records how much was cut.
/// `usize::MAX` (the `L` key) keeps everything.
fn apply_truncation(diff: &mut FileDiff, max_lines: usize) {
    if max_lines == usize::MAX {
        return;
    }
    let total_lines: usize = diff.hunks.iter().map(|hunk| hunk.lines.len()).sum();
    if total_lines <= max_lines {
        return;
    }
    let mut shown_lines = 0;
    let mut kept = Vec::new();
    for hunk in diff.hunks.drain(..) {
        if shown_lines >= max_lines {
            break;
        }
        shown_lines += hunk.lines.len();
        kept.push(hunk);
    }
    diff.hunks = kept;
    diff.truncated = Some(Truncated {
        shown_lines,
        total_lines,
    });
}

#[cfg(test)]
mod tests;
