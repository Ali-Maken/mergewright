//! Reading the stash list, without a `git stash list` spawn.
//!
//! `refs/stash`'s own reflog *is* the stash list — each entry's new object id
//! is a stash commit, newest push last written, so reading the reflog newest
//! first through `gix` gives the same order and the same messages `git stash
//! list` prints, with no spawn at all. Only each entry's touched paths need
//! one (`diff-tree --name-only`, plus a second for the untracked-files
//! commit `git stash push -u` makes, when there is one) — a stash commit
//! itself is read through `gix`, never a `git` spawn, to check for that third
//! parent.

use std::path::PathBuf;

use graft_core::commit::Timestamp;
use graft_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

#[cfg(test)]
mod tests;

/// The most stash entries [`list`] reads paths for. `refs/stash`'s reflog can
/// in principle grow without bound; a repository with more stashes than this
/// has bigger problems than a capped list.
const MAX_ENTRIES: usize = 50;

/// One entry of the stash list: what `git stash list` would print as
/// `stash@{index}: message`, plus the paths it touched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashEntry {
    /// Position in the list: `0` is the most recent stash (`stash@{0}`).
    pub index: usize,
    /// The stash commit itself.
    pub commit: Oid,
    /// The message `git stash push` recorded — `git`'s own default (`WIP on
    /// <branch>: <subject>`) or whatever `-m` gave it.
    pub message: String,
    /// When the stash was pushed.
    pub at: Timestamp,
    /// Every path the stash touched: tracked changes and, for a stash pushed
    /// with `-u`/`-a`, the untracked files it also carried.
    pub paths: Vec<PathBuf>,
}

/// Why reading the stash list failed.
#[derive(Debug)]
pub enum StashError {
    /// The caller's [`Stop`] asked, between entries.
    Stopped,
    /// `refs/stash`'s reflog could not be read.
    Reflog(String),
    /// A `git diff-tree` spawn for one entry's paths failed.
    Run(RunError),
    /// A stash commit itself could not be read: a malformed object id, or an
    /// object `gix` cannot find or decode.
    Object(String),
}

impl std::fmt::Display for StashError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StashError::Stopped => write!(f, "reading the stash list was stopped"),
            StashError::Reflog(message) => {
                write!(f, "the stash reflog could not be read: {message}")
            }
            StashError::Run(error) => write!(f, "{error}"),
            StashError::Object(message) => {
                write!(f, "a stash commit could not be read: {message}")
            }
        }
    }
}

impl std::error::Error for StashError {}

impl From<RunError> for StashError {
    fn from(error: RunError) -> Self {
        StashError::Run(error)
    }
}

/// Every stash entry, newest first, capped at [`MAX_ENTRIES`].
///
/// A repository with no stashes (`refs/stash` does not exist) is `Ok(vec![])`,
/// not an error — the same "nothing here yet" reading
/// [`crate::history::previous_tips`] gives a ref with no reflog.
///
/// # Errors
/// See [`StashError`].
pub fn list(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<StashEntry>, StashError> {
    let gix_repo = repo.gix();
    let reference = match gix_repo.find_reference("refs/stash") {
        Ok(reference) => reference,
        Err(gix::reference::find::existing::Error::NotFound { .. }) => return Ok(vec![]),
        Err(source) => return Err(StashError::Reflog(source.to_string())),
    };
    let mut platform = reference.log_iter();
    let Some(entries) = platform
        .rev()
        .map_err(|source| StashError::Reflog(source.to_string()))?
    else {
        return Ok(vec![]);
    };

    let mut result = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_ENTRIES {
            break;
        }
        if stop.should_stop() {
            return Err(StashError::Stopped);
        }
        let line = entry.map_err(|source| StashError::Reflog(source.to_string()))?;
        let commit = Oid::from_bytes(line.new_oid.as_bytes())
            .map_err(|source| StashError::Reflog(source.to_string()))?;
        let message = String::from_utf8_lossy(&line.message).into_owned();
        let at = Timestamp {
            seconds: line.signature.time.seconds,
            offset_seconds: line.signature.time.offset,
        };
        let paths = stash_paths(repo, commit, stop)?;
        result.push(StashEntry {
            index,
            commit,
            message,
            at,
            paths,
        });
    }
    Ok(result)
}

/// Every stash commit with its index, newest first, capped at
/// [`MAX_ENTRIES`] — the reflog and nothing else.
///
/// [`list`]'s cheap half, for the graph's ref badges
/// (m3-working-tree-undo-and-stash/06): it spawns no `diff-tree`, because a
/// badge says `stash@{n}` and never how many files are in it. That matters
/// — the badges are read on every refs refresh, and `list` would spawn up
/// to fifty processes there.
///
/// A repository with no stashes is `Ok(vec![])`, as in [`list`].
///
/// # Errors
/// See [`StashError`]; this half can only fail on the reflog itself.
pub fn commits(repo: &RepoHandle, stop: &dyn Stop) -> Result<Vec<(Oid, usize)>, StashError> {
    let gix_repo = repo.gix();
    let reference = match gix_repo.find_reference("refs/stash") {
        Ok(reference) => reference,
        Err(gix::reference::find::existing::Error::NotFound { .. }) => return Ok(vec![]),
        Err(source) => return Err(StashError::Reflog(source.to_string())),
    };
    let mut platform = reference.log_iter();
    let Some(entries) = platform
        .rev()
        .map_err(|source| StashError::Reflog(source.to_string()))?
    else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_ENTRIES {
            break;
        }
        if stop.should_stop() {
            return Err(StashError::Stopped);
        }
        let line = entry.map_err(|source| StashError::Reflog(source.to_string()))?;
        let commit = Oid::from_bytes(line.new_oid.as_bytes())
            .map_err(|source| StashError::Reflog(source.to_string()))?;
        result.push((commit, index));
    }
    Ok(result)
}

/// Every path a stash commit touched: its ordinary parent's tracked changes,
/// plus — when the stash carries an untracked-files commit as a third
/// parent (`git stash push -u`/`-a`) — that commit's own paths.
fn stash_paths(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Vec<PathBuf>, StashError> {
    let mut paths = diff_tree_name_only(
        repo,
        stop,
        &[
            GitArg::Flag("diff-tree"),
            GitArg::Flag("--no-commit-id"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-r"),
            GitArg::Flag("-z"),
            GitArg::Ref(format!("{commit}^").into_bytes()),
            GitArg::Ref(commit.to_string().into_bytes()),
        ],
    )?;
    if has_untracked_commit(repo, commit)? {
        let untracked = diff_tree_name_only(
            repo,
            stop,
            &[
                GitArg::Flag("diff-tree"),
                GitArg::Flag("--no-commit-id"),
                GitArg::Flag("--root"),
                GitArg::Flag("--name-only"),
                GitArg::Flag("-r"),
                GitArg::Flag("-z"),
                GitArg::Ref(format!("{commit}^3").into_bytes()),
            ],
        )?;
        for path in untracked {
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}

/// Whether `commit` (a stash commit) has a third parent: the untracked-files
/// commit `git stash push -u`/`-a` creates alongside the ordinary one. Read
/// through `gix`, no spawn — a stash commit is small and already local.
///
/// A commit that cannot be read is an error, not a `false`: answering "no
/// untracked files" for a stash whose object is unreadable would drop those
/// paths out of [`StashEntry::paths`] and let the user drop a stash without
/// ever seeing what was in it.
///
/// # Errors
/// [`StashError::Object`] if the id is not a valid hash, or the commit cannot
/// be found or decoded.
fn has_untracked_commit(repo: &RepoHandle, commit: Oid) -> Result<bool, StashError> {
    let oid = gix::hash::oid::try_from_bytes(commit.as_bytes())
        .map_err(|source| StashError::Object(format!("{commit}: {source}")))?;
    let found = repo
        .gix()
        .find_commit(oid)
        .map_err(|source| StashError::Object(format!("{commit}: {source}")))?;
    Ok(found.parent_ids().count() >= 3)
}

/// Runs a `diff-tree --name-only -z` invocation and splits its stdout on NUL.
fn diff_tree_name_only(
    repo: &RepoHandle,
    stop: &dyn Stop,
    args: &[GitArg],
) -> Result<Vec<PathBuf>, StashError> {
    let output = GitInvocation::new(repo).args(args.to_vec()).run(stop)?;
    Ok(split_nul(&output.stdout)
        .into_iter()
        .map(bytes_to_pathbuf)
        .collect())
}

/// Splits `bytes` on NUL, dropping a trailing empty piece from a trailing
/// separator (as `-z` output always ends).
fn split_nul(bytes: &[u8]) -> Vec<&[u8]> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut tokens: Vec<&[u8]> = bytes.split(|&b| b == 0).collect();
    if bytes.ends_with(&[0]) {
        tokens.pop();
    }
    tokens
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
