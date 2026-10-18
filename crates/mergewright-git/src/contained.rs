//! "Contained in": which branches, remote branches and tags reach a commit.
//!
//! One `git for-each-ref --contains` per question (design note m2, choice 3):
//! `gix` has no cheap reachability query, and building an index over the
//! whole history is a ticket of its own (ADR-0011 names this gap).

use mergewright_core::oid::Oid;

use crate::repo::RepoHandle;
use crate::run::{GitArg, GitInvocation, RunError, Stop};

/// The most refs kept per class; the rest are only counted.
const PER_CLASS: usize = 200;

/// The refs a commit is reachable from, short names, in git's ref order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Contained {
    /// Local branches, without `refs/heads/`: `main`.
    pub branches: Vec<Vec<u8>>,
    /// Remote-tracking branches, without `refs/remotes/`: `origin/main`.
    pub remotes: Vec<Vec<u8>>,
    /// Tags, without `refs/tags/`: `v1.0`.
    pub tags: Vec<Vec<u8>>,
    /// Refs past the per-class cap of 200, across all three classes.
    pub more: usize,
}

/// Lists the branches, remote branches and tags that contain `commit`.
///
/// Symbolic refs (`refs/remotes/origin/HEAD`) are left out: they are another
/// name for a ref already listed. Refs outside the three classes (notes, the
/// stash) are not asked for.
///
/// # Errors
/// [`RunError`] from the runner: git missing, stopped, or refusing — an id
/// that names no commit in this repository is git's `Exited` error.
pub fn contained_in(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Contained, RunError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--contains"),
            GitArg::Oid(commit),
            GitArg::Flag("--format=%(refname)%00%(symref)%00"),
            GitArg::Ref(b"refs/heads".to_vec()),
            GitArg::Ref(b"refs/remotes".to_vec()),
            GitArg::Ref(b"refs/tags".to_vec()),
        ])
        .run(stop)?;
    Ok(classify(&output.stdout))
}

/// Sorts `for-each-ref` records (`name NUL symref NUL LF`) into classes.
fn classify(stdout: &[u8]) -> Contained {
    let mut contained = Contained::default();
    for record in stdout.split(|&b| b == b'\n') {
        let mut fields = record.split(|&b| b == 0);
        let (Some(name), Some(symref)) = (fields.next(), fields.next()) else {
            continue; // the empty tail after the last newline
        };
        if !symref.is_empty() {
            continue;
        }
        let (class, short) = if let Some(short) = name.strip_prefix(b"refs/heads/") {
            (&mut contained.branches, short)
        } else if let Some(short) = name.strip_prefix(b"refs/remotes/") {
            (&mut contained.remotes, short)
        } else if let Some(short) = name.strip_prefix(b"refs/tags/") {
            (&mut contained.tags, short)
        } else {
            continue; // only the three prefixes were asked for
        };
        if class.len() < PER_CLASS {
            class.push(short.to_vec());
        } else {
            contained.more += 1;
        }
    }
    contained
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_split_by_class_symrefs_dropped_and_capped() {
        let mut stdout = b"refs/heads/main\0\0\nrefs/remotes/origin/HEAD\0refs/remotes/origin/main\0\nrefs/remotes/origin/main\0\0\nrefs/tags/v1\0\0\n".to_vec();
        for n in 0..(PER_CLASS + 3) {
            stdout.extend_from_slice(format!("refs/tags/t{n}\0\0\n").as_bytes());
        }
        let contained = classify(&stdout);
        assert_eq!(contained.branches, vec![b"main".to_vec()]);
        assert_eq!(contained.remotes, vec![b"origin/main".to_vec()]);
        assert_eq!(contained.tags.len(), PER_CLASS);
        assert_eq!(contained.tags[0], b"v1".to_vec());
        assert_eq!(contained.more, 4);
    }
}
