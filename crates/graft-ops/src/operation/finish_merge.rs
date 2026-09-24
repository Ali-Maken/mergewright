//! Finishing a merge **begun in the shell** (m6-conflicts/20): a `git merge`
//! or `git pull` that stopped on a conflict, resolved, and committed from
//! Graft. A merge Graft began is finished by `apply::resume` on its own
//! entry; this one has none, so it is an operation of its own.

use graft_core::oid::Oid;
use graft_git::run::GitArg;
use graft_git::state::InProgress;

use super::{Invocation, Refusal, RepoFacts};

/// What finishing a shell-begun merge needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FinishMergeParams {
    /// The commit `MERGE_HEAD` named when the user asked: `apply` refuses
    /// [`Refusal::NotInProgress`] when it names another by the time the
    /// operation runs, so the merge finished is the merge the user saw.
    pub merge_head: Oid,
    /// The message the user edited, or `None` for git's own `MERGE_MSG`.
    pub message: Option<Vec<u8>>,
}

/// The kind's own rule: a merge is what git has in progress. Unmerged paths
/// were refused before this (`Operation::check_conflict_state`), and
/// `MERGE_HEAD`'s commit is compared in `apply`, which can read it — `check`
/// is pure over [`RepoFacts`], which does not carry it.
pub(super) fn check(facts: &RepoFacts) -> Result<(), Refusal> {
    if facts.in_progress == Some(InProgress::Merge) {
        Ok(())
    } else {
        Err(Refusal::NotInProgress)
    }
}

/// `git merge --continue` for git's own message — byte for byte what the
/// banner's `c` has always sent a merge Graft began — or `git commit
/// --cleanup=whitespace -F -` with the edited message on stdin: the mode the
/// live-entry `ContinueWith` runs (m6-conflicts/22), since `finish_summary`
/// already took git's comment lines out and a `#` line left is the user's.
/// `GIT_EDITOR=true` comes from the plan runner, as for every plan step.
pub(super) fn plan(params: &FinishMergeParams) -> Vec<Invocation> {
    match &params.message {
        None => vec![Invocation::new(vec![
            GitArg::Flag("merge"),
            GitArg::Flag("--continue"),
        ])],
        Some(message) => vec![Invocation::with_stdin(
            vec![
                GitArg::Flag("commit"),
                GitArg::Flag("--cleanup=whitespace"),
                GitArg::Flag("-F"),
                GitArg::Flag("-"),
            ],
            message.clone(),
        )],
    }
}

/// `finish merge of <short id>`: the fallback `describe` can give without a
/// read. `apply` names the merge by `FinishSummary::from`'s rule instead
/// (`finish merge of feature`), which needs `MERGE_MSG`.
pub(super) fn describe(params: &FinishMergeParams) -> String {
    format!("finish merge of {}", super::short(params.merge_head))
}
