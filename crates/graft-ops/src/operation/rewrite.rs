//! [`Operation::Rewrite`](super::Operation::Rewrite)'s four properties: a
//! rewrite planned by `crate::rewrite::plan`, run by `apply::engine`
//! (ADR-0047).

use graft_git::run::GitArg;
use graft_git::version::GitVersion;

use crate::rewrite::{RewriteParams, squash_groups, validate};

use super::history::{head_is, rebase_invocation, short, short_ref, tip_holds};
use super::{Invocation, Refusal, RepoFacts, Tip};

/// `rewrite feature-a`, or `put staged changes into 1a2b3c4` for staged
/// changes put into an older commit.
pub(super) fn describe(params: &RewriteParams) -> String {
    match amended(params) {
        Some(target) => format!("put staged changes into {}", short(target)),
        None => format!("rewrite {}", short_ref(&params.head.name)),
    }
}

/// The commit Graft's own fixup rows meld into, when the rewrite carries
/// the index (`made`).
fn amended(params: &RewriteParams) -> Option<graft_core::oid::Oid> {
    squash_groups(&params.todo)
        .into_iter()
        .find(|group| {
            group.melded.iter().any(|&row| {
                params
                    .todo
                    .get(row)
                    .is_some_and(|line| params.made.contains(&line.commit))
            })
        })
        .and_then(|group| params.todo.get(group.kept).map(|line| line.commit))
}

/// Every tip the rewrite is pinned to: his branch, then each branch that
/// follows.
pub(super) fn tips(params: &RewriteParams) -> Vec<Tip> {
    let mut tips = vec![params.head.clone()];
    tips.extend(
        params
            .line
            .branches
            .iter()
            .filter(|tip| tip.name != params.head.name)
            .cloned(),
    );
    tips
}

/// `HEAD` still on his branch at the planned tip, every branch that follows
/// still where it was planned, and a todo git can run.
pub(super) fn check(params: &RewriteParams, facts: &RepoFacts) -> Result<(), Refusal> {
    head_is(&params.head, facts)?;
    for tip in tips(params).iter().skip(1) {
        tip_holds(tip, facts)?;
    }
    validate(&params.todo).map_err(Refusal::BadTodo)
}

/// `git rebase -i … <onto> [<top>]`: git is started on the top of the line
/// when branches follow (ADR-0047 §3).
pub(super) fn plan(params: &RewriteParams, git: GitVersion) -> Vec<Invocation> {
    let top = match params.line.branches.as_slice() {
        [_, .., top] => Some(short_ref(&top.name)),
        _ => None,
    };
    vec![rebase_invocation(GitArg::Oid(params.onto), top, git)]
}
