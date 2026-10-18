//! The plumbing every snapshot is built from: `write-tree`, `mktree`,
//! `commit-tree`, `update-ref`, and the byte helpers their output needs (the
//! index copy is `temp_index`). Nothing here writes the real `.git/index`.

use std::path::{Path, PathBuf};

use mergewright_core::journal::IndexState;
use mergewright_core::oid::Oid;
use mergewright_core::on_disk::Generation;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

use super::SnapshotError;

/// `GIT_INDEX_FILE=<temp> git write-tree`, with an unmerged index an ordinary
/// failure. Used after `add -A`, which resolves every conflict in the copy, so
/// an unmerged index cannot reach it.
pub(super) fn write_tree(
    repo: &RepoHandle,
    index: &Path,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let output = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "write-tree")
}

/// The same, with `Ok(None)` exactly when git refused because the index holds
/// unmerged paths — the one failure the caller has another answer for.
pub(super) fn try_write_tree(
    repo: &RepoHandle,
    index: &Path,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let ran = GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("write-tree"))
        .run(stop);
    match ran {
        Ok(output) => oid_from(&output.stdout, "write-tree").map(Some),
        // `<path>: unmerged (<oid>)` is git's own first line here, and the
        // only way `write-tree` fails that the caller can work with.
        Err(RunError::Exited {
            stderr_first_line, ..
        }) if stderr_first_line.contains("unmerged") => Ok(None),
        Err(why) => Err(stopped(why)),
    }
}

pub(crate) fn path_of(name: Vec<u8>) -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(std::ffi::OsString::from_vec(name))
}

/// `path` as one line `hash-object --stdin-paths` reads back byte for byte:
/// in double quotes, `"` and `\` escaped, every byte outside printable ASCII
/// as `\ooo` — git's own C quoting, which it unquotes on the way in.
pub(crate) fn c_quoted(path: &[u8]) -> Vec<u8> {
    let mut line = vec![b'"'];
    for &byte in path {
        match byte {
            b'"' | b'\\' => line.extend([b'\\', byte]),
            0x20..=0x7e => line.push(byte),
            other => line.extend(format!("\\{other:03o}").bytes()),
        }
    }
    line.push(b'"');
    line
}

/// NUL-terminated records, the trailing empty one dropped.
pub(super) fn split_nul(bytes: &[u8]) -> Vec<Vec<u8>> {
    bytes
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
        .map(<[u8]>::to_vec)
        .collect()
}

/// One `git` with no index redirection, for the reads that go against an
/// object id rather than an index.
pub(super) fn run_git(
    repo: &RepoHandle,
    args: Vec<GitArg>,
    stdin: Option<Vec<u8>>,
    stop: &dyn Stop,
) -> Result<mergewright_git::run::GitOutput, SnapshotError> {
    let mut invocation = GitInvocation::new(repo).args(args);
    if let Some(bytes) = stdin {
        invocation = invocation.stdin(bytes);
    }
    invocation.run(stop).map_err(stopped)
}

/// [`run_git`], with its command line pushed to `log` first when there is
/// one: the reads a restore makes belong in its entry's Command Log
/// (m6-conflicts/80), and a read made for no entry passes `None`.
pub(super) fn run_git_logged(
    repo: &RepoHandle,
    args: Vec<GitArg>,
    stop: &dyn Stop,
    log: Option<&mut Vec<String>>,
) -> Result<mergewright_git::run::GitOutput, SnapshotError> {
    let invocation = GitInvocation::new(repo).args(args);
    if let Some(commands) = log {
        commands.push(invocation.command_line());
    }
    invocation.run(stop).map_err(stopped)
}

/// A ref's commit, or `None` when it is not there. `for-each-ref` exits 0
/// either way, so an absent ref is an empty answer rather than a failure to
/// tell apart from a broken one.
pub(super) fn read_ref(
    repo: &RepoHandle,
    name: &str,
    stop: &dyn Stop,
) -> Result<Option<Oid>, SnapshotError> {
    let output = run_git(
        repo,
        vec![
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--format=%(objectname)"),
            GitArg::Ref(name.as_bytes().to_vec()),
        ],
        None,
        stop,
    )?;
    let text = String::from_utf8_lossy(&output.stdout);
    let first = text.lines().next().unwrap_or("").trim();
    if first.is_empty() {
        return Ok(None);
    }
    Oid::from_hex(first)
        .map(Some)
        .map_err(|_| SnapshotError::NotAnOid {
            what: "for-each-ref",
            printed: first.to_owned(),
        })
}

/// `GIT_INDEX_FILE=<temp> git --literal-pathspecs add -A --`: the working
/// tree into the copy. The stat cache the copy carries means only files whose
/// stat data changed are re-hashed.
pub(super) fn add_all(
    repo: &RepoHandle,
    index: &Path,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    GitInvocation::new(repo)
        .env("GIT_INDEX_FILE", index)
        .arg(GitArg::Flag("--literal-pathspecs"))
        .arg(GitArg::Flag("add"))
        .arg(GitArg::Flag("-A"))
        .run(stop)
        .map_err(stopped)?;
    Ok(())
}

/// `git mktree` over the listing: the index side, `raw/` when the snapshot
/// keeps one (ADR-0052), `worktree/`.
///
/// `mktree` wants its entries sorted: `index` < `raw` < `unmerged` <
/// `worktree`, so `raw` goes between the two spellings of the index side.
pub(super) fn mktree(
    repo: &RepoHandle,
    index: Option<IndexState>,
    worktree_tree: Option<Oid>,
    raw: Option<Oid>,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut listing = String::new();
    if let Some(IndexState::Merged(tree)) = index {
        listing.push_str(&format!("040000 tree {tree}\tindex\n"));
    }
    if let Some(raw) = raw {
        listing.push_str(&format!("040000 tree {raw}\traw\n"));
    }
    if let Some(IndexState::Unmerged { unmerged }) = index {
        listing.push_str(&format!("040000 tree {unmerged}\tunmerged\n"));
    }
    if let Some(worktree_tree) = worktree_tree {
        listing.push_str(&format!("040000 tree {worktree_tree}\tworktree\n"));
    }
    let output = GitInvocation::new(repo)
        .arg(GitArg::Flag("mktree"))
        .stdin(listing.into_bytes())
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "mktree")
}

/// `git commit-tree <tree> [-p <parent>]…` with a fixed identity.
///
/// The identity is set explicitly rather than left to the user's config: a
/// backup commit is Mergewright's, not theirs, and a repository configured with
/// `user.useConfigOnly` would otherwise refuse to make one.
/// The author and committer every backup commit carries: the app, not the
/// user, so a backup never passes for the user's own work. Nothing reads it
/// back; it is only what `git log` shows on a backup ref.
/// The current generation's dir (ADR-0057, 2026-10-11 §1).
pub const BACKUP_AUTHOR_NAME: &str = Generation::CURRENT.dir();
/// See [`BACKUP_AUTHOR_NAME`]: `<dir>@backup`.
pub const BACKUP_AUTHOR_EMAIL: &str = Generation::CURRENT.backup_email();

pub(super) fn commit_tree(
    repo: &RepoHandle,
    tree: Oid,
    parents: &[Oid],
    message: &str,
    stop: &dyn Stop,
) -> Result<Oid, SnapshotError> {
    let mut invocation = GitInvocation::new(repo)
        .env("GIT_AUTHOR_NAME", BACKUP_AUTHOR_NAME)
        .env("GIT_AUTHOR_EMAIL", BACKUP_AUTHOR_EMAIL)
        .env("GIT_COMMITTER_NAME", BACKUP_AUTHOR_NAME)
        .env("GIT_COMMITTER_EMAIL", BACKUP_AUTHOR_EMAIL)
        .arg(GitArg::Flag("commit-tree"))
        .arg(GitArg::Oid(tree));
    for parent in parents {
        invocation = invocation.arg(GitArg::Flag("-p")).arg(GitArg::Oid(*parent));
    }
    let output = invocation
        .arg(GitArg::Flag("-F"))
        .arg(GitArg::Word("-".to_owned()))
        .stdin(message.as_bytes().to_vec())
        .run(stop)
        .map_err(stopped)?;
    oid_from(&output.stdout, "commit-tree")
}

/// `git update-ref <name> <commit>`.
pub(super) fn update_ref(
    repo: &RepoHandle,
    name: &str,
    to: Oid,
    stop: &dyn Stop,
) -> Result<(), SnapshotError> {
    GitInvocation::new(repo)
        .arg(GitArg::Flag("update-ref"))
        .arg(GitArg::Ref(name.as_bytes().to_vec()))
        .arg(GitArg::Oid(to))
        .run(stop)
        .map_err(stopped)?;
    Ok(())
}

/// `RunError::Stopped` is the caller's own cancellation, not a git failure;
/// every other error stays as it is.
pub(super) fn stopped(why: RunError) -> SnapshotError {
    match why {
        RunError::Stopped => SnapshotError::Stopped,
        other => SnapshotError::Run(other),
    }
}

pub(super) fn oid_from(stdout: &[u8], what: &'static str) -> Result<Oid, SnapshotError> {
    let text = String::from_utf8_lossy(stdout);
    let first = text.lines().next().unwrap_or("").trim();
    Oid::from_hex(first).map_err(|_| SnapshotError::NotAnOid {
        what,
        printed: first.to_owned(),
    })
}
