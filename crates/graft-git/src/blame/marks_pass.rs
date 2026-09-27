//! Blame's second pass (m7-power-tools/18e): whether it runs, the
//! human-format spawn, and merging its marks onto the porcelain's lines.
//!
//! git before [`GitVersion::BLAME_PORCELAIN_MARKS`] writes no marks in the
//! porcelain; [`super::marks`] reads them from the human format instead.

use std::path::Path;

use super::ignore_revs::FoundIgnoreRevs;
use super::invocation::{Format, blame_invocation};
use super::{BlameError, BlameLine, BlameOptions, LineMark, Rev, parse_marks};
use crate::repo::RepoHandle;
use crate::run::{DEFAULT_CAP, GitArg, GitInvocation, RunError, Stop};
use crate::version::{GitVersion, system_git};

#[cfg(test)]
thread_local! {
    /// How many human-format passes [`read_marks`] ran on this thread: what
    /// a test observes to say "one blame spawn" (the runner is synchronous,
    /// so the spawn happens on the caller's thread).
    pub(crate) static MARKS_PASSES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Whether blame must read its marks from a second, human-format pass.
///
/// All four must hold:
/// 1. `ignore_revs`: with `false`, blame passes `--no-ignore-revs-file` and
///    git marks nothing;
/// 2. `!porcelain_has_marks`: a git that wrote them has answered;
/// 3. `list_in_effect`: the root's list was passed, or `blame.ignoreRevsFile`
///    is set (no list, nothing to mark);
/// 4. `git` is older than [`GitVersion::BLAME_PORCELAIN_MARKS`], or its
///    version is unknown (`None`): the pass is then slower, never wrong.
pub(crate) fn needs_marks_pass(
    ignore_revs: bool,
    porcelain_has_marks: bool,
    list_in_effect: bool,
    git: Option<&GitVersion>,
) -> bool {
    ignore_revs
        && !porcelain_has_marks
        && list_in_effect
        && git.is_none_or(|version| *version < GitVersion::BLAME_PORCELAIN_MARKS)
}

/// [`needs_marks_pass`], each input read only once the cheaper ones leave
/// the answer open: conditions 1 and 2 cost nothing, the configured list is
/// one `git config` spawn, the version one `git --version` spawn.
pub(super) fn marks_pass_wanted(
    repo: &RepoHandle,
    opts: BlameOptions,
    passed_root_list: bool,
    porcelain_has_marks: bool,
    stop: &dyn Stop,
) -> Result<bool, BlameError> {
    if !opts.ignore_revs || porcelain_has_marks {
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
pub(crate) fn configured_list(repo: &RepoHandle, stop: &dyn Stop) -> Result<bool, BlameError> {
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

/// The second pass: `git blame -l -s` with the porcelain's options, its
/// marks read. Output cut at `cap` is refused, as the porcelain's is.
pub(super) fn read_marks(
    repo: &RepoHandle,
    rev: &Rev,
    path: &Path,
    opts: BlameOptions,
    found: Option<&FoundIgnoreRevs>,
    cap: usize,
    stop: &dyn Stop,
) -> Result<Vec<LineMark>, BlameError> {
    #[cfg(test)]
    MARKS_PASSES.with(|passes| passes.set(passes.get() + 1));
    let output = blame_invocation(repo, rev, path, opts, found, Format::Human)
        .output_cap(cap)
        .run(stop)?;
    if output.truncated {
        return Err(BlameError::TooLarge { cap });
    }
    parse_marks(&output.stdout).map_err(BlameError::Parse)
}

/// The human-format marks onto the porcelain's lines, by position. A
/// different count is [`BlameError::Changed`]: possible only for
/// [`Rev::WorkingTree`], whose file both passes read from disk.
pub(crate) fn merge_marks(lines: &mut [BlameLine], marks: Vec<LineMark>) -> Result<(), BlameError> {
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

#[cfg(test)]
mod tests;
