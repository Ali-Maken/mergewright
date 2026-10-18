//! Which paths an index state holds, and which differ between two: the
//! scope a `RestoreIndex` step restores over (m6-conflicts/08).

use std::collections::BTreeSet;
use std::path::PathBuf;

use mergewright_core::journal::IndexState;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, Stop};

use super::SnapshotError;
use super::plumbing::{path_of, run_git_logged, split_nul};
use super::stage_tree::{strip_stage, tree_stage_entries};

/// Every path a recorded index state holds. The read's command line goes to
/// `commands`, as every read below does: a restore's reads are part of its
/// Command Log (m6-conflicts/80).
pub(crate) fn index_paths(
    repo: &RepoHandle,
    index: IndexState,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    Ok(tree_stage_entries(repo, index, None, stop, Some(commands))?
        .into_keys()
        .collect())
}

/// Every path the real index tracks right now, unmerged ones included.
pub(crate) fn tracked_paths_now(
    repo: &RepoHandle,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    let output = run_git_logged(
        repo,
        vec![GitArg::Flag("ls-files"), GitArg::Flag("-z")],
        stop,
        Some(commands),
    )?;
    Ok(split_nul(&output.stdout).into_iter().map(path_of).collect())
}

/// The paths whose index entries differ between two recorded states.
///
/// Two states of the same shape diff as trees, with the stage prefix stripped
/// and the result de-duplicated (three stage lines are one path). Two of
/// **different** shapes cannot be diffed at all — one holds `2/a.rs` where the
/// other holds `a.rs` — so each is listed and the entry sets compared.
///
/// Each `git` it runs is pushed to `commands`: one `diff-tree` for two of a
/// shape, two `ls-tree -r` for two of different shapes (m6-conflicts/80).
///
/// # Errors
/// [`SnapshotError`] when `git` will not run or prints an unreadable record.
pub fn index_paths_changed(
    repo: &RepoHandle,
    from: IndexState,
    to: IndexState,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<BTreeSet<PathBuf>, SnapshotError> {
    if from == to {
        return Ok(BTreeSet::new());
    }
    if from.is_unmerged() == to.is_unmerged() {
        let output = run_git_logged(
            repo,
            vec![
                GitArg::Flag("diff-tree"),
                GitArg::Flag("-r"),
                GitArg::Flag("--name-only"),
                GitArg::Flag("-z"),
                GitArg::Oid(from.tree()),
                GitArg::Oid(to.tree()),
            ],
            stop,
            Some(commands),
        )?;
        let strip = from.is_unmerged();
        return Ok(split_nul(&output.stdout)
            .into_iter()
            .map(|name| if strip { strip_stage(&name) } else { name })
            .map(path_of)
            .collect());
    }
    let before = tree_stage_entries(repo, from, None, stop, Some(commands))?;
    let after = tree_stage_entries(repo, to, None, stop, Some(commands))?;
    let mut changed = BTreeSet::new();
    for (path, entries) in &before {
        if after.get(path) != Some(entries) {
            changed.insert(path.clone());
        }
    }
    for path in after.keys() {
        if !before.contains_key(path) {
            changed.insert(path.clone());
        }
    }
    Ok(changed)
}
