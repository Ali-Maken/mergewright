//! The reads the gates share (ADR-0059 §6.2): the line before the edit,
//! the forks' upstreams, the remote-tracking refs, and which of them hold a
//! commit.

use std::collections::HashMap;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;
use mergewright_git::repo::RepoHandle;
use mergewright_git::run::{GitArg, GitInvocation, RunError, Stop};

use super::super::PreviewError;
use super::{Candidate, ref_name};

/// The line's commits before the edit, `onto..top` on the first parents,
/// oldest first: one `rev-list`.
pub(super) fn line_before(
    repo: &RepoHandle,
    onto: Oid,
    top: Oid,
    stop: &dyn Stop,
) -> Result<Vec<Oid>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("rev-list"),
            GitArg::Flag("--first-parent"),
            GitArg::Flag("--reverse"),
            GitArg::Word(top.to_string()),
            GitArg::Word(format!("^{onto}")),
        ])
        .run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            Oid::from_hex(line.trim()).map_err(|_| {
                PreviewError::Read(RunError::Unparsable(format!("rev-list printed {line:?}")))
            })
        })
        .collect()
}

/// The first remote-tracking ref (by name, symbolic refs aside) that holds
/// `commit`, in full: one `for-each-ref --contains`.
pub(super) fn remote_holding(
    repo: &RepoHandle,
    commit: Oid,
    stop: &dyn Stop,
) -> Result<Option<RefName>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--format=%(refname)%00%(symref)"),
            GitArg::Word(format!("--contains={commit}")),
            GitArg::Flag("refs/remotes"),
        ])
        .run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .find(|(_, symref)| symref.is_empty())
        .map(|(name, _)| ref_name(name))
        .transpose()
}

/// Every remote-tracking ref, full names: one `for-each-ref`.
pub(super) fn remote_refs(
    repo: &RepoHandle,
    stop: &dyn Stop,
) -> Result<Vec<RefName>, PreviewError> {
    let output = GitInvocation::new(repo)
        .args([
            GitArg::Flag("for-each-ref"),
            GitArg::Flag("--format=%(refname)"),
            GitArg::Flag("refs/remotes/"),
        ])
        .run(stop)?;
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(ref_name)
        .collect()
}

/// The remote-tracking refs named like `branch` (`refs/remotes/<remote>/<short>`),
/// ADR-0059 §6.2 (b). Suffix match; a false match errs to the fork staying.
pub(super) fn own_name_remotes<'a>(
    remote_refs: &'a [RefName],
    branch: &RefName,
) -> Vec<&'a RefName> {
    // Candidates are read from `refs/heads/`; a name without the prefix
    // is matched as it stands.
    let short = branch
        .as_bytes()
        .strip_prefix(b"refs/heads/")
        .unwrap_or(branch.as_bytes());
    remote_refs
        .iter()
        .filter(|remote| {
            remote
                .as_bytes()
                .strip_prefix(b"refs/remotes/")
                .and_then(|rest| rest.strip_suffix(short))
                .is_some_and(|remote_name| remote_name.len() > 1 && remote_name.ends_with(b"/"))
        })
        .collect()
}

/// Each candidate's upstream, full names: one `for-each-ref`. Only a
/// remote-tracking upstream is kept: a local one (`--track feature-a`) is no
/// push (ADR-0059 §6.2 a).
pub(super) fn upstreams(
    repo: &RepoHandle,
    candidates: &[Candidate],
    stop: &dyn Stop,
) -> Result<HashMap<String, String>, PreviewError> {
    let mut args = vec![
        GitArg::Flag("for-each-ref"),
        GitArg::Flag("--format=%(refname)%00%(upstream)"),
    ];
    args.extend(
        candidates
            .iter()
            .map(|candidate| GitArg::Ref(candidate.off.branch.as_bytes().to_vec())),
    );
    let output = GitInvocation::new(repo).args(args).run(stop)?;
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\0'))
        .filter(|(_, upstream)| upstream.starts_with("refs/remotes/"))
        .map(|(name, upstream)| (name.to_owned(), upstream.to_owned()))
        .collect())
}

#[cfg(test)]
mod tests {
    use mergewright_core::ref_name::RefName;

    use super::own_name_remotes;

    fn name(full: &str) -> RefName {
        RefName::new(full.as_bytes()).expect("a ref name")
    }

    #[test]
    fn own_name_matches_by_suffix_under_any_remote() {
        let remotes = [
            name("refs/remotes/origin/park"),
            name("refs/remotes/fork/park"),
            name("refs/remotes/origin/xpark"),
            name("refs/remotes/origin/park/2"),
            name("refs/remotes/park"),
            name("refs/remotes/origin/feature-a"),
        ];
        let found: Vec<&[u8]> = own_name_remotes(&remotes, &name("refs/heads/park"))
            .into_iter()
            .map(RefName::as_bytes)
            .collect();
        assert_eq!(
            found,
            [
                b"refs/remotes/origin/park".as_slice(),
                b"refs/remotes/fork/park"
            ]
        );
    }

    #[test]
    fn a_slash_in_the_remote_name_can_match_another_branch_and_errs_to_staying() {
        // Remote `team/a`'s `park` is `refs/remotes/team/a/park`; a branch
        // named `a/park` matches it too — the fork would stay, never move.
        let remotes = [name("refs/remotes/team/a/park")];
        assert_eq!(
            own_name_remotes(&remotes, &name("refs/heads/a/park")).len(),
            1
        );
    }
}
