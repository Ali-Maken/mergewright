//! Blame's second pass (m7-power-tools/18e): whether it runs, the
//! human-format spawn, and merging its marks onto the porcelain's lines.
//!
//! git before [`GitVersion::BLAME_PORCELAIN_MARKS`] writes no marks in the
//! porcelain; [`super::marks`] reads them from the human format instead.

use std::path::Path;
use std::sync::OnceLock;

use graft_core::oid::Oid;

use super::ignore_revs::FoundIgnoreRevs;
use super::invocation::{Format, blame_invocation};
use super::{BlameError, BlameLine, BlameOptions, GitFrom, MarkedLine, Rev, parse_marks};
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
/// the answer open: conditions 1 and 2 cost nothing. With no root file, one
/// `git config --get blame.ignoreRevsFile` spawn per blame learns whether a
/// list is configured (when the porcelain carried no mark). The version is
/// one `git --version` per process, cached.
pub(super) fn marks_pass_wanted(
    repo: &RepoHandle,
    opts: BlameOptions,
    passed_root_list: bool,
    porcelain_has_marks: bool,
    git: &GitFrom,
    stop: &dyn Stop,
) -> Result<bool, BlameError> {
    if !opts.ignore_revs || porcelain_has_marks {
        return Ok(false);
    }
    let list_in_effect = passed_root_list || configured_list(repo, stop)?;
    if !list_in_effect {
        return Ok(false);
    }
    let git = match git {
        GitFrom::System => system_version(),
        GitFrom::Given(version) => Some(*version),
    };
    Ok(needs_marks_pass(
        opts.ignore_revs,
        porcelain_has_marks,
        list_in_effect,
        git.as_ref(),
    ))
}

/// The system git's version, kept from the first successful read in this
/// process (review 18e-20f Minor 2). The first read can't be stopped by the
/// caller; `system_git`'s own deadline bounds it.
static SYSTEM_VERSION: OnceLock<GitVersion> = OnceLock::new();

#[cfg(test)]
thread_local! {
    /// How many times [`system_version`] spawned `git --version` on this
    /// thread.
    pub(crate) static VERSION_READS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// [`SYSTEM_VERSION`], read on first use. An unreadable version is `None`
/// and **not** kept: the pass is taken (slower, still right; guessing "new"
/// would drop marks), and the next blame reads again.
fn system_version() -> Option<GitVersion> {
    if let Some(version) = SYSTEM_VERSION.get() {
        return Some(*version);
    }
    #[cfg(test)]
    VERSION_READS.with(|reads| reads.set(reads.get() + 1));
    match system_git() {
        Ok(found) => {
            let version = found.version();
            // `Err` here means another blame stored first; it read the same
            // `git --version` in the same process, so the answer kept is
            // this one. Nothing is lost by dropping ours.
            let _lost_race = SYSTEM_VERSION.set(version);
            Some(version)
        }
        // The blame itself just ran, so git is there; what failed is only
        // the version read.
        Err(_unreadable) => None,
    }
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
) -> Result<Vec<MarkedLine>, BlameError> {
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

/// The human-format marks onto the porcelain's lines, by position, once
/// every line is shown to be the same line in both passes: same count, and
/// each porcelain line's commit starts with the id git printed after the
/// marks (all zeros: not committed yet). Anything else is
/// [`BlameError::Changed`], and nothing is merged: possible only for
/// [`Rev::WorkingTree`], whose file both passes read from disk.
pub(crate) fn merge_marks(
    lines: &mut [BlameLine],
    marks: Vec<MarkedLine>,
) -> Result<(), BlameError> {
    let changed = BlameError::Changed {
        first: lines.len(),
        second: marks.len(),
    };
    if marks.len() != lines.len() {
        return Err(changed);
    }
    if !lines
        .iter()
        .zip(&marks)
        .all(|(line, marked)| same_commit(line.commit.as_ref(), &marked.id_prefix))
    {
        return Err(changed);
    }
    for (line, marked) in lines.iter_mut().zip(marks) {
        line.mark = marked.mark;
    }
    Ok(())
}

/// Whether the human format's `id_prefix` names `commit`.
fn same_commit(commit: Option<&Oid>, id_prefix: &str) -> bool {
    match commit {
        Some(commit) => commit.to_string().starts_with(id_prefix),
        None => id_prefix.bytes().all(|b| b == b'0'),
    }
}

#[cfg(test)]
mod tests;
