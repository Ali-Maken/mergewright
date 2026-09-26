//! The git reads the path table is made of: each a `-z --name-only` list
//! of paths, never decoded.

use std::collections::BTreeSet;
use std::path::PathBuf;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, RunError, Stop};

use crate::facts::FactsError;
use crate::operation::RepoFacts;
use crate::snapshot::SnapshotError;

pub(super) fn head() -> GitArg {
    GitArg::Ref(b"HEAD".to_vec())
}

pub(super) fn unborn(facts: &RepoFacts) -> bool {
    matches!(facts.head, HeadRef::Unborn(_))
}

/// `diff-tree -r --name-only --no-renames -z [flags] <from> <to>`.
pub(super) fn diff_tree(
    repo: &RepoHandle,
    flags: &[&'static str],
    [from, to]: [GitArg; 2],
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let mut args = vec![
        GitArg::Flag("diff-tree"),
        GitArg::Flag("-r"),
        GitArg::Flag("--name-only"),
        GitArg::Flag("--no-renames"),
        GitArg::Flag("-z"),
    ];
    args.extend(flags.iter().map(|flag| GitArg::Flag(flag)));
    args.extend([from, to]);
    names(repo, args, stop)
}

/// Every path a tree holds: `ls-tree -r --name-only -z <tree>`.
pub(super) fn listed(
    repo: &RepoHandle,
    tree: GitArg,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    names(
        repo,
        vec![
            GitArg::Flag("ls-tree"),
            GitArg::Flag("-r"),
            GitArg::Flag("--name-only"),
            GitArg::Flag("-z"),
            tree,
        ],
        stop,
    )
}

/// A commit's parents, in order: `rev-parse <commit>^@`.
pub(super) fn parents_of(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Vec<Oid>, FactsError> {
    let output = GitInvocation::new(repo)
        .args(vec![
            GitArg::Flag("rev-parse"),
            GitArg::Ref(format!("{commit}^@").into_bytes()),
        ])
        .run(stop)
        .map_err(run_error)?;
    oids(&output.stdout, "rev-parse")
}

/// Object ids split by whitespace, as `rev-parse` and `rev-list` print
/// them (one a line; `--parents` puts a commit's parents after it).
pub(super) fn oids(stdout: &[u8], what: &'static str) -> Result<Vec<Oid>, FactsError> {
    String::from_utf8_lossy(stdout)
        .split_whitespace()
        .map(|hex| {
            Oid::from_hex(hex).map_err(|_| {
                FactsError::Snapshot(SnapshotError::NotAnOid {
                    what,
                    printed: hex.to_owned(),
                })
            })
        })
        .collect()
}

/// Run a `-z --name-only` read and collect its paths.
pub(super) fn names(
    repo: &RepoHandle,
    args: Vec<GitArg>,
    stop: &dyn Stop,
) -> Result<BTreeSet<PathBuf>, FactsError> {
    let output = GitInvocation::new(repo)
        .args(args)
        .run(stop)
        .map_err(run_error)?;
    Ok(paths_in(&output.stdout))
}

/// NUL-separated raw path bytes as `PathBuf`s, never decoded.
pub(super) fn paths_in(stdout: &[u8]) -> BTreeSet<PathBuf> {
    use std::os::unix::ffi::OsStrExt;
    stdout
        .split(|byte| *byte == 0)
        .filter(|name| !name.is_empty())
        .map(|bytes| PathBuf::from(std::ffi::OsStr::from_bytes(bytes)))
        .collect()
}

pub(super) fn run_error(why: RunError) -> FactsError {
    match why {
        RunError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(SnapshotError::Run(other)),
    }
}

pub(super) fn snapshot_error(why: SnapshotError) -> FactsError {
    match why {
        SnapshotError::Stopped => FactsError::Stopped,
        other => FactsError::Snapshot(other),
    }
}
