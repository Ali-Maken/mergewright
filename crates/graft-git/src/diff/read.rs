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
    parse_unified, parse_worktree_file_list,
};
use crate::repo::{HeadState, RepoHandle};
use crate::run::{GitArg, GitInvocation, RunError, Stop};

mod raw;

/// The most bytes kept from any one diff command's stdout/stderr. Past it
/// the runner kills the child and hands back a capped, still-successful
/// [`crate::run::GitOutput`]; here that becomes [`DiffError::TooLarge`].
const OUTPUT_CAP: usize = 64 * 1024 * 1024;

/// Git's fixed empty-tree object id (sha1): what [`DiffSpec::Staged`] diffs
/// the index against on an unborn branch, so a file staged before the first
/// commit still shows.
const EMPTY_TREE: &[u8] = b"4b825dc642cb6eb9a060e54bf8d69288fbee4904";

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
    /// The index against `HEAD` (the empty tree on an unborn branch): what
    /// `Commit` stages. No untracked files — the index does not have them.
    Staged,
    /// The worktree against the index: what plain edits leave unstaged.
    /// Untracked files are included, as [`DiffSpec::WorkingTreeVsHead`]
    /// includes them; [`file_diff`] reads each as all added lines.
    Unstaged,
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
    match spec {
        DiffSpec::WorkingTreeVsHead => return list_files_working_tree(repo, *opts, stop),
        DiffSpec::Staged => return list_files_staged(repo, *opts, stop),
        DiffSpec::Unstaged => return list_files_unstaged(repo, *opts, stop),
        DiffSpec::Commit { .. } | DiffSpec::Range { .. } => {}
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

/// The working tree against `HEAD`: tracked changes from `git diff-index
/// HEAD`, plus untracked files from `ls-files` (no line counts for those).
///
/// Plumbing `diff-index`, not porcelain `diff`, on purpose: `git diff`
/// rewrites `.git/index` to refresh the stat cache, taking `index.lock`
/// while it does, and neither `GIT_OPTIONAL_LOCKS=0` nor
/// `--no-optional-locks` stops it (verified on git 2.54; only `git status`
/// honours the switch). A viewer must not make the user's own `git commit`
/// fail, so the listing is read with plumbing and the stat cache's lies are
/// filtered out afterwards — see [`parse_worktree_file_list`].
fn list_files_working_tree(
    repo: &RepoHandle,
    opts: DiffOptions,
    stop: &dyn Stop,
) -> Result<FileList, DiffError> {
    ensure_head(repo)?;

    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff-index"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Flag("-M"),
        GitArg::Flag("-C"),
        GitArg::Flag("-r"),
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
    let mut list = parse_worktree_file_list(&raw, &numstat).map_err(DiffError::Parse)?;
    append_untracked(repo, stop, &mut list)?;
    Ok(list)
}

/// The index against `HEAD` — the empty tree on an unborn branch
/// ([`EMPTY_TREE`]) — via plumbing `diff-index --cached`. Purely
/// content-based (index vs a committed tree, never the worktree), so unlike
/// [`list_files_working_tree`]/[`list_files_unstaged`] there is no stat-cache
/// lie to filter: [`parse_file_list`], not the worktree variant. No
/// untracked files: the index does not have them.
fn list_files_staged(
    repo: &RepoHandle,
    opts: DiffOptions,
    stop: &dyn Stop,
) -> Result<FileList, DiffError> {
    let against = staged_against(repo)?;
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff-index"),
        GitArg::Flag("--cached"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Flag("-M"),
        GitArg::Flag("-C"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag("--raw"),
        GitArg::Flag("--numstat"),
    ]);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = invocation.arg(GitArg::Ref(against));
    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let (raw, numstat) = split_raw_and_numstat(&output.stdout);
    parse_file_list(&raw, &numstat).map_err(DiffError::Parse)
}

/// The worktree against the index, via plumbing `diff-files` — the same
/// stat-cache-lying command M2's review found under `git diff`, run directly
/// instead so nothing refreshes `.git/index` (module doc). Untracked files
/// are appended, as [`list_files_working_tree`] does.
fn list_files_unstaged(
    repo: &RepoHandle,
    opts: DiffOptions,
    stop: &dyn Stop,
) -> Result<FileList, DiffError> {
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff-files"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Flag("-M"),
        GitArg::Flag("-C"),
        GitArg::Flag("-r"),
        GitArg::Flag("-z"),
        GitArg::Flag("--raw"),
        GitArg::Flag("--numstat"),
    ]);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    let output = invocation.output_cap(OUTPUT_CAP).run(stop)?;
    if output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    let (raw, numstat) = split_raw_and_numstat(&output.stdout);
    let mut list = parse_worktree_file_list(&raw, &numstat).map_err(DiffError::Parse)?;
    append_untracked(repo, stop, &mut list)?;
    Ok(list)
}

/// Appends every untracked worktree file (`ls-files -z --others
/// --exclude-standard`) to `list`, hunk-less as M2 defined for
/// [`FileStatus::Untracked`].
fn append_untracked(
    repo: &RepoHandle,
    stop: &dyn Stop,
    list: &mut FileList,
) -> Result<(), DiffError> {
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
    Ok(())
}

/// `HEAD` if it names a commit, else [`EMPTY_TREE`] — the argument
/// [`DiffSpec::Staged`] diffs the index against, so a file staged before the
/// first commit still shows.
fn staged_against(repo: &RepoHandle) -> Result<Vec<u8>, DiffError> {
    match repo.head().map_err(|_| DiffError::NoHead)? {
        HeadState::Unborn(_) => Ok(EMPTY_TREE.to_vec()),
        HeadState::Branch(_) | HeadState::Detached(_) => Ok(b"HEAD".to_vec()),
    }
}

/// An untracked file as all added lines: `git diff --no-index /dev/null
/// <path>`, parsed and capped like any other diff (m6-conflicts/81). The
/// index has no entry for the file, so no plumbing command can diff it;
/// `--no-index` compares two paths and never reads or writes `.git/index`.
/// It exits 1 when the sides differ, so `run_raw` and the code is judged
/// here. An empty file prints nothing: hunk-less, like git's own output.
fn untracked_file_diff(
    repo: &RepoHandle,
    entry: &FileEntry,
    opts: DiffOptions,
    max_lines: usize,
    stop: &dyn Stop,
) -> Result<FileDiff, DiffError> {
    let mut invocation = GitInvocation::new(repo).args([
        GitArg::Flag("diff"),
        GitArg::Flag("--no-index"),
        GitArg::Flag("--no-color"),
        GitArg::Flag("--no-ext-diff"),
        GitArg::Count("-U", u32::from(opts.context)),
    ]);
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = invocation
        .arg(GitArg::Path(b"/dev/null".to_vec()))
        .arg(GitArg::Path(entry.path.clone()))
        .output_cap(OUTPUT_CAP);
    let raw = invocation.run_raw(stop)?;
    if raw.output.truncated {
        return Err(DiffError::TooLarge { bytes: OUTPUT_CAP });
    }
    match raw.code {
        Some(0 | 1) => {}
        Some(status) => {
            return Err(DiffError::Run(RunError::Exited {
                status,
                stderr_first_line: crate::commit_graph::first_line(&raw.output.stderr),
                command: invocation.command_line(),
            }));
        }
        None => {
            return Err(DiffError::Run(RunError::Signalled {
                command: invocation.command_line(),
            }));
        }
    }
    let mut diff = parse_unified(&raw.output.stdout)
        .map_err(DiffError::Parse)?
        .pop()
        .unwrap_or_else(|| empty_diff(entry));
    diff.entry = entry.clone();
    apply_truncation(&mut diff, max_lines);
    Ok(diff)
}

/// One file's full diff.
///
/// An [`FileStatus::Untracked`] entry is read from the working tree as all
/// added lines ([`untracked_file_diff`]).
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
        return untracked_file_diff(repo, entry, *opts, max_lines, stop);
    }

    let mut invocation = file_diff_command(repo, spec)?;
    invocation = invocation
        .arg(GitArg::Flag("-p"))
        .arg(GitArg::Count("-U", u32::from(opts.context)));
    if opts.ignore_whitespace {
        invocation = invocation.arg(GitArg::Flag("-w"));
    }
    invocation = file_diff_ref_args(repo, spec, stop, invocation)?;
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

/// The subcommand and fixed flags for [`file_diff`]'s invocation, before the
/// context/whitespace flags, the tree-ish, and the path arguments.
fn file_diff_command(repo: &RepoHandle, spec: &DiffSpec) -> Result<GitInvocation, DiffError> {
    let invocation = GitInvocation::new(repo);
    Ok(match spec {
        DiffSpec::WorkingTreeVsHead => {
            ensure_head(repo)?;
            invocation.args([
                GitArg::Flag("diff-index"),
                GitArg::Flag("--no-color"),
                GitArg::Flag("--no-ext-diff"),
                GitArg::Flag("-M"),
                GitArg::Flag("-C"),
            ])
        }
        DiffSpec::Staged => invocation.args([
            GitArg::Flag("diff-index"),
            GitArg::Flag("--cached"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Flag("-C"),
        ]),
        DiffSpec::Unstaged => invocation.args([
            GitArg::Flag("diff-files"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Flag("-C"),
        ]),
        DiffSpec::Commit { .. } | DiffSpec::Range { .. } => invocation.args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("--no-color"),
            GitArg::Flag("--no-ext-diff"),
            GitArg::Flag("-M"),
            GitArg::Flag("-C"),
            GitArg::Flag("-r"),
        ]),
    })
}

/// The tree-ish argument(s) for [`file_diff`]'s invocation: `HEAD`, the
/// staged-against ref, nothing (`diff-files` always compares the index to
/// the worktree), or the resolved commit/range sides.
fn file_diff_ref_args(
    repo: &RepoHandle,
    spec: &DiffSpec,
    stop: &dyn Stop,
    invocation: GitInvocation,
) -> Result<GitInvocation, DiffError> {
    Ok(match spec {
        DiffSpec::WorkingTreeVsHead => invocation.arg(GitArg::Ref(b"HEAD".to_vec())),
        DiffSpec::Staged => invocation.arg(GitArg::Ref(staged_against(repo)?)),
        DiffSpec::Unstaged => invocation,
        DiffSpec::Commit { .. } | DiffSpec::Range { .. } => {
            let sides = resolve_sides(repo, spec, stop)?;
            with_sides_args(invocation, sides)
        }
    })
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
