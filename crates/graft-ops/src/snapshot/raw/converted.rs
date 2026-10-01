//! Question 2 of `raw_record`: which files git would convert.

use graft_git::repo::RepoHandle;
use graft_git::run::{GitArg, GitInvocation, Stop};

use super::super::SnapshotError;
use super::super::plumbing::stopped;
use super::{File, run_logged, unreadable};

/// The conversion attributes `check-attr` is asked about: any of these set
/// means git may rewrite the file's bytes on the way in or out.
const CONVERTING: [&str; 6] = [
    "text",
    "eol",
    "crlf",
    "filter",
    "ident",
    "working-tree-encoding",
];

/// One regular file of the worktree tree.
/// Question 2: the files git would convert, by `check-attr` on
/// [`CONVERTING`] and `core.autocrlf`.
///
/// A file converts when any of the attributes has a value other than
/// `unspecified` or `unset`, or `core.autocrlf` is on and `text` is not
/// `unset` (`-text`: git's "binary").
pub(super) fn converted(
    repo: &RepoHandle,
    files: Vec<File>,
    autocrlf: bool,
    stop: &dyn Stop,
    commands: &mut Vec<String>,
) -> Result<Vec<File>, SnapshotError> {
    let mut stdin = Vec::new();
    for file in &files {
        stdin.extend_from_slice(&file.path);
        stdin.push(0);
    }
    let mut invocation = GitInvocation::new(repo)
        .arg(GitArg::Flag("check-attr"))
        .arg(GitArg::Flag("-z"))
        .arg(GitArg::Flag("--stdin"));
    for attribute in CONVERTING {
        invocation = invocation.arg(GitArg::Flag(attribute));
    }
    let output = run_logged(&invocation.stdin(stdin), stop, commands).map_err(stopped)?;
    let converts = converting_paths(&output.stdout, autocrlf)?;
    Ok(files
        .into_iter()
        .filter(|file| converts.contains(&file.path))
        .collect())
}

/// `check-attr -z` triples, `<path>\0<attribute>\0<value>\0`, folded into the
/// paths that convert. Pure, so the rule is testable without a repository.
pub(super) fn converting_paths(
    stdout: &[u8],
    autocrlf: bool,
) -> Result<std::collections::BTreeSet<Vec<u8>>, SnapshotError> {
    use std::collections::BTreeMap;

    // Per path: (text is unset, some attribute is set).
    let mut seen: BTreeMap<Vec<u8>, (bool, bool)> = BTreeMap::new();
    for triple in split_nul_keeping_empty(stdout).chunks(3) {
        let [path, attribute, value] = triple else {
            return Err(unreadable("check-attr", &triple.concat()));
        };
        let at = seen.entry(path.clone()).or_insert((false, false));
        let quiet = value.as_slice() == b"unspecified" || value.as_slice() == b"unset";
        if !quiet {
            at.1 = true;
        }
        if attribute.as_slice() == b"text" && value.as_slice() == b"unset" {
            at.0 = true;
        }
    }
    Ok(seen
        .into_iter()
        .filter(|(_, (text_unset, set))| *set || (autocrlf && !text_unset))
        .map(|(path, _)| path)
        .collect())
}

/// NUL-terminated fields, an empty one kept (a value can be empty); only the
/// terminator's own trailing split is dropped.
fn split_nul_keeping_empty(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut fields: Vec<Vec<u8>> = bytes.split(|byte| *byte == 0).map(<[u8]>::to_vec).collect();
    if fields.last().is_some_and(Vec::is_empty) {
        fields.pop();
    }
    fields
}
