//! The refusals before blame spawns: a binary file, or a path the version
//! does not have (m7-power-tools/18). One `diff-tree`/`diff-index
//! --numstat` against the empty tree, so "binary" means what it means to
//! the diff (attributes first, then content).

use std::path::Path;

use super::{BlameError, Rev, path_bytes};
use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, Stop};

/// Git's id for the empty tree, which every repository knows without storing
/// it: the old side of the "is this path here, and is it text" check.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

/// [`BlameError::Untracked`] or [`BlameError::Binary`] before blame runs.
pub(super) fn refuse_early(
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
