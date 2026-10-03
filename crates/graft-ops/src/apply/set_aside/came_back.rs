//! Files a put-back brought back from the index (m8-polish-and-ship/14).
//!
//! A new file he staged, then deleted from disk (`AD`), is kept in the
//! set-aside `S` by its index: `git stash apply --index` reinstates the
//! entry and checks the file out again. Nothing is lost, but a file he
//! deleted is on disk again, so the landing line names it
//! (`notes.md came back from the index`) instead of a silent
//! `your edits are back`. Git's result is kept (his ruling).

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError};

use super::AfterState;
use super::put_back::name_status;

/// The paths staged as new in `stash` (`S^1..S^2` added) that were not in
/// his working tree before the operation (`worktree_before`, the entry's
/// snapshot): what the put-back brought back from the index. Sorted.
///
/// One `diff-tree` spawn, and a second only when something was staged as
/// new.
///
/// # Errors
/// [`RunError`] when either `diff-tree` fails or prints a status with no
/// path.
pub fn came_back_from_index(
    repo: &RepoHandle,
    stash: Oid,
    worktree_before: Oid,
) -> Result<Vec<PathBuf>, RunError> {
    let added: BTreeSet<PathBuf> = name_status(&diff_tree(
        repo,
        &format!("{stash}^1"),
        &format!("{stash}^2"),
    )?)?
    .into_iter()
    .filter_map(|(status, path)| (status == b'A').then_some(path))
    .collect();
    if added.is_empty() {
        return Ok(Vec::new());
    }
    // In the index, not in his working tree: `D` from `S^2` to it.
    let gone = name_status(&diff_tree(
        repo,
        &format!("{stash}^2"),
        &worktree_before.to_string(),
    )?)?;
    Ok(gone
        .into_iter()
        .filter(|(status, path)| *status == b'D' && added.contains(path))
        .map(|(_, path)| path)
        .collect())
}

/// [`came_back_from_index`] for [`super::close`], once the edits are back:
/// the paths, or none and the read's failure for what was not read. Nothing
/// is read without a before-snapshot to compare with; `S`'s objects outlive
/// its drop, so the read can follow it.
pub(super) fn read(
    repo: &RepoHandle,
    stash: Oid,
    worktree_before: Option<Oid>,
) -> (Vec<PathBuf>, Option<AfterState>) {
    let Some(before) = worktree_before else {
        return (Vec::new(), None);
    };
    match came_back_from_index(repo, stash, before) {
        Ok(paths) => (paths, None),
        Err(error) => (Vec::new(), Some(AfterState::CameBackUnread(error))),
    }
}

/// `notes.md came back from the index`, `notes.md and 2 more came back from
/// the index`; `None` when nothing did.
#[must_use]
pub fn came_back_words(paths: &[PathBuf]) -> Option<String> {
    let first = paths.first()?.display();
    Some(match paths.len() {
        1 => format!("{first} came back from the index"),
        n => format!("{first} and {} more came back from the index", n - 1),
    })
}

fn diff_tree(repo: &RepoHandle, from: &str, to: &str) -> Result<Vec<u8>, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("diff-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--no-renames"),
            GitArg::Flag("--name-status"),
            GitArg::Flag("-z"),
            GitArg::Word(from.to_owned()),
            GitArg::Word(to.to_owned()),
        ])
        .run(&())?;
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_name_one_path_and_count_the_rest() {
        assert_eq!(came_back_words(&[]), None);
        assert_eq!(
            came_back_words(&[PathBuf::from("notes.md")]).as_deref(),
            Some("notes.md came back from the index")
        );
        assert_eq!(
            came_back_words(&[PathBuf::from("a.md"), PathBuf::from("b.md")]).as_deref(),
            Some("a.md and 1 more came back from the index")
        );
    }
}
