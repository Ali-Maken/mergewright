//! Ending a merge **begun in the shell**: a `git merge` or `git pull` that
//! stopped on a conflict, then finished (m6-conflicts/20) or aborted
//! (m6-conflicts/57) from Graft. A merge Graft began is ended by
//! `apply::resume` on its own entry; this one has none, so each end is an
//! operation of its own, with its own entry and backup.

use std::path::PathBuf;

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
    /// The files the resolver calls solved, still unmerged: staged by this
    /// operation's first line, inside its own entry, just before the commit
    /// (ease-pass/05b, hard choice 3). Empty when every path is staged.
    pub solved: Vec<PathBuf>,
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
///
/// The solved files go first, `git --literal-pathspecs add -- <solved>`, so
/// the stage and the commit are one entry and `u` puts both back.
pub(super) fn plan(params: &FinishMergeParams) -> Vec<Invocation> {
    let mut plan: Vec<Invocation> = stage_solved_args(&params.solved)
        .map(Invocation::new)
        .into_iter()
        .collect();
    plan.push(match &params.message {
        None => Invocation::new(vec![GitArg::Flag("merge"), GitArg::Flag("--continue")]),
        Some(message) => Invocation::with_stdin(
            vec![
                GitArg::Flag("commit"),
                GitArg::Flag("--cleanup=whitespace"),
                GitArg::Flag("-F"),
                GitArg::Flag("-"),
            ],
            message.clone(),
        ),
    });
    plan
}

/// `git --literal-pathspecs add -- <solved>`: a merge's solved files staged
/// as its finish begins — `None` when there are none. One line for both
/// merge origins: `FinishMerge`'s plan and `resume`'s live entry.
pub(crate) fn stage_solved_args(solved: &[PathBuf]) -> Option<Vec<GitArg>> {
    (!solved.is_empty()).then(|| super::stage::with_paths(vec![GitArg::Flag("add")], solved))
}

/// `finish merge of <short id>`: the fallback `describe` can give without a
/// read. `apply` names the merge by `FinishSummary::from`'s rule instead
/// (`finish merge of feature`), which needs `MERGE_MSG`.
pub(super) fn describe(params: &FinishMergeParams) -> String {
    format!("finish merge of {}", super::short(params.merge_head))
}

/// What aborting a shell-begun merge needs (m6-conflicts/57).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AbortMergeParams {
    /// The commit `MERGE_HEAD` named when the user asked: `apply` refuses
    /// [`Refusal::NotInProgress`] when it names another by the time the
    /// operation runs, so the merge aborted is the merge the user saw.
    pub merge_head: Oid,
}

/// Which end of a shell-begun merge a refusal is about, so its words name
/// the command that does it in the shell (m6-conflicts/57).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MergeEnd {
    /// `c`: finishing it, `git commit`.
    Finish,
    /// `a`: aborting it, `git merge --abort`.
    Abort,
}

impl MergeEnd {
    /// The tail of a refusal: where this end is done instead.
    #[must_use]
    pub fn in_the_shell(self) -> &'static str {
        match self {
            MergeEnd::Finish => "finish it in the shell: git commit",
            MergeEnd::Abort => "abort it in the shell: git merge --abort",
        }
    }
}

/// `git merge --abort`: git puts the index and the working tree back to
/// `HEAD` and removes its merge files. The backup taken before it keeps
/// everything the abort throws away, which is what `u` puts back.
pub(super) fn plan_abort() -> Vec<Invocation> {
    vec![Invocation::new(vec![
        GitArg::Flag("merge"),
        GitArg::Flag("--abort"),
    ])]
}

/// `abort merge of <short id>`: the fallback, as for [`describe`]; `apply`
/// names it `abort merge of feature`.
pub(super) fn describe_abort(params: &AbortMergeParams) -> String {
    format!("abort merge of {}", super::short(params.merge_head))
}
