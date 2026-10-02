//! What a rewrite request plans to — the range of his branch and the
//! refusals before anything runs. The line of branches built on his is
//! `dependents`'s; the history reads are `reads`'s.

use std::collections::BTreeMap;

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::GitVersion;

use crate::facts;
use crate::operation::{RebaseParams, Refusal, RepoFacts, RewriteKey, Tip};

use super::dependents::line_of;
use super::reads::{RangeCommit, ancestor, base_of, commit_index, parents_of, read_range};
use super::todo::autosquash;
use super::unpushed::{RangeStop, your_commits};
use super::{
    RewriteError, RewriteParams, RewriteRequest, TodoAction, TodoLine, default_message,
    squash_groups,
};

/// Plan a rewrite of the branch `HEAD` is on.
///
/// `Open { from }`: his rows run from `HEAD` down to where the branch left
/// its base — the merge-base with its upstream if one is set and fetched,
/// else with the default branch — and further down to `from` when `from`
/// is that deep or deeper. `fixup!`/`squash!` rows come back pre-marked
/// and moved above their target, never applied unseen; a squash group gets
/// its default message ([`default_message`]). The first commit of the
/// repository is never a row: it has no parent to replay onto, so a range
/// reaching it stops above it and it is the base.
///
/// `AmendInto { target }`: the index is written as a commit object with no
/// ref (parent `HEAD`) and planned as a fixup right after `target`; `made`
/// records it. On `HEAD` with nothing built on it this is no rewrite — the
/// app keeps its plain amend — and is refused.
///
/// `Absorb`: each staged hunk goes to the commit that last changed its lines
/// ([`crate::absorb`]), one commit object per target, each a fixup right
/// after its target; none placed is [`Refusal::NothingToAbsorb`].
///
/// `FromRebase(params)`: the rebase a preview drew, as its `y` would run it
/// — the rows it replays onto `onto_tip`, its drops marked `drop`, its line
/// as given. `HEAD` not on `params.branch` at its tip (moved or switched
/// since the preview), or a drop with no row, is [`Refusal::Moved`].
///
/// Absorb and `AmendInto` reach one range, on every base (06e ruling): his
/// first-parent commits down to the first a remote-tracking ref holds, the
/// base, a merge or the first commit — and, on the default or only branch
/// with no base below `HEAD`, at most the last
/// [`super::ABSORB_FALLBACK_DEPTH`]. `AmendInto` on `HEAD` skips it.
///
/// `default_branch` is the user's `graph.default_branch` when set: it wins
/// over git's guess (ADR-0001 rule 1), for the base of the range and for
/// which branch is never a dependent.
///
/// # Errors
/// [`RewriteError::Refused`], checked in this order: not on a branch;
/// nothing staged (`AmendInto`); the commit not on his branch, or past his
/// commits (already pushed, past the cap, below a merge); every commit
/// pushed, or none of its own (`Absorb`); the first
/// commit (`AmendInto`); a merge in the range; the branches built on his
/// forking, or one checked out in another worktree; git older than 2.38 with
/// branches to follow; `AmendInto` on `HEAD` with none. [`RewriteError::Facts`]
/// when a read fails; [`RewriteError::Stopped`].
pub fn plan(
    repo: &RepoHandle,
    git: &GitVersion,
    req: &RewriteRequest,
    facts: &RepoFacts,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    let head = head_of(repo, facts, stop)?;
    match req {
        RewriteRequest::Open { from } => open(repo, git, head, *from, default_branch, stop),
        RewriteRequest::AmendInto { target } => {
            amend_into(repo, git, head, *target, facts, default_branch, stop)
        }
        RewriteRequest::Absorb => super::absorb::absorb(repo, git, head, default_branch, stop),
        RewriteRequest::FromRebase(params) => from_rebase(repo, head, params, stop),
        RewriteRequest::RewordHead { message } => {
            reword_head(repo, git, head, message, default_branch, stop)
        }
    }
}

/// The rebase `params` as an editor todo: exactly the rows `r`'s run
/// replays ([`super::rebase_rows`], as `apply::engine` reads them), every
/// one a pick but the suggested drops. No autosquash re-marking and no
/// range walk: unedited, the editor runs the rebase `y` would. The line is
/// the preview's, as-is — `plan_as_run` checked it, and `y`'s pin check
/// catches a move since.
fn from_rebase(
    repo: &RepoHandle,
    head: Tip,
    params: &RebaseParams,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if head != params.branch {
        // HEAD moved or switched since the preview was drawn.
        return Err(RewriteError::Refused(Refusal::Moved {
            name: params.branch.name.clone(),
        }));
    }
    let mut todo = super::rebase_rows(repo, params.onto_tip, head.oid, stop)?;
    for dropped in &params.drops {
        let Some(row) = todo.iter_mut().find(|row| row.commit == *dropped) else {
            // A drop with no row: the branch is not what the preview
            // measured — as `y` refuses it (`apply::engine::prepare`).
            return Err(RewriteError::Refused(Refusal::Moved {
                name: params.branch.name.clone(),
            }));
        };
        row.action = TodoAction::Drop;
    }
    Ok(RewriteParams {
        head,
        onto: params.onto_tip,
        todo,
        line: params.line.clone(),
        messages: BTreeMap::new(),
        made: Vec::new(),
        index_base: None,
    })
}

/// The branch `HEAD` is on and its tip.
pub(super) fn head_of(
    repo: &RepoHandle,
    facts: &RepoFacts,
    stop: &dyn Stop,
) -> Result<Tip, RewriteError> {
    if stop.should_stop() {
        return Err(RewriteError::Stopped);
    }
    let HeadRef::Branch(name) = &facts.head else {
        return Err(RewriteError::Refused(Refusal::NotOnABranch));
    };
    let Some(oid) = facts::read_refs(repo, std::slice::from_ref(name))?
        .get(name)
        .copied()
        .flatten()
    else {
        // A branch with no commit yet: there is nothing to rewrite, and
        // nothing a rewrite could stand on.
        return Err(RewriteError::Refused(Refusal::NotOnABranch));
    };
    Ok(Tip {
        name: name.clone(),
        oid,
    })
}

fn open(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    from: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if !ancestor(repo, from, head.oid, stop)? {
        return Err(RewriteError::Refused(Refusal::NotOnYourBranch {
            commit: from,
            by: RewriteKey::Open,
        }));
    }
    let onto = match base_of(repo, &head, default_branch, stop)? {
        Some(base) if !ancestor(repo, from, base, stop)? => base,
        // `from` is at the base or deeper, or there is no base: the rows run
        // down to `from` itself.
        _ => match parents_of(repo, from, stop)?.first() {
            Some(parent) => *parent,
            // The first commit: nothing to replay it onto. It stays as the
            // base, and the rows stop above it.
            None => from,
        },
    };
    let range = read_range(repo, onto, head.oid, false, stop)?;
    refuse_merges(&range)?;
    let line = line_of(repo, git, &head, default_branch, stop)?;
    let todo = autosquash(range.iter().map(RangeCommit::pick).collect());
    let by_oid: BTreeMap<Oid, &str> = range
        .iter()
        .map(|commit| (commit.oid, commit.message.as_str()))
        .collect();
    let message_of = |row: usize| {
        todo.get(row)
            .and_then(|line| by_oid.get(&line.commit))
            .map_or_else(String::new, |message| (*message).to_owned())
    };
    let messages = squash_groups(&todo)
        .into_iter()
        .filter(|group| {
            group
                .melded
                .iter()
                .any(|&row| todo[row].action == TodoAction::Squash)
        })
        .map(|group| (group.kept, default_message(&group, &message_of, &todo)))
        .collect();
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages,
        made: Vec::new(),
        index_base: None,
    })
}

fn amend_into(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    target: Oid,
    facts: &RepoFacts,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    if !facts
        .status
        .entries
        .iter()
        .any(graft_git::status::StatusEntry::is_staged)
    {
        return Err(RewriteError::Refused(Refusal::NothingStagedFor { target }));
    }
    if !ancestor(repo, target, head.oid, stop)? {
        return Err(RewriteError::Refused(Refusal::NotOnYourBranch {
            commit: target,
            by: RewriteKey::AmendInto,
        }));
    }
    refuse_out_of_reach(repo, &head, target, default_branch, stop)?;
    let Some(&onto) = parents_of(repo, target, stop)?.first() else {
        return Err(RewriteError::Refused(Refusal::RootCommit {
            commit: target,
        }));
    };
    let range = read_range(repo, onto, head.oid, false, stop)?;
    refuse_merges(&range)?;
    let line = line_of(repo, git, &head, default_branch, stop)?;
    if target == head.oid && line.branches.len() == 1 {
        return Err(RewriteError::Refused(Refusal::PlainAmend));
    }
    let subject = range
        .iter()
        .find(|commit| commit.oid == target)
        .map_or_else(String::new, |commit| commit.subject.clone());
    let fixup_subject = format!("fixup! {subject}");
    let made = commit_index(repo, head.oid, &fixup_subject, stop)?;
    let mut todo: Vec<TodoLine> = range.iter().map(RangeCommit::pick).collect();
    let at = todo
        .iter()
        .position(|line| line.commit == target)
        .map_or(0, |at| at + 1);
    todo.insert(
        at,
        TodoLine {
            action: TodoAction::Fixup,
            commit: made,
            subject: fixup_subject,
        },
    );
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages: BTreeMap::new(),
        made: vec![made],
        index_base: None,
    })
}

/// `a` on `HEAD` with nothing staged and branches built on his: `HEAD`'s
/// message becomes `message`, and the line follows as an amend's does
/// (m8-polish-and-ship/14, his ruling: "let him edit the message"). One
/// `Reword` row, its message saved in `messages` so git's editor never
/// opens; `onto` = `HEAD^`. [`RewriteRequest::RewordHead`]'s plan.
///
/// Refused, in this order (after [`plan`]'s not-on-a-branch): `HEAD` is
/// the first commit (`RootCommit`) or a merge (`MergeInRange`); the line
/// forks, or a branch of it is checked out elsewhere; git older than 2.38;
/// nothing built on `HEAD` (`PlainAmend`: the app's amend rewords it).
fn reword_head(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    message: &str,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    let Some(&onto) = parents_of(repo, head.oid, stop)?.first() else {
        return Err(RewriteError::Refused(Refusal::RootCommit {
            commit: head.oid,
        }));
    };
    let range = read_range(repo, onto, head.oid, false, stop)?;
    refuse_merges(&range)?;
    let line = line_of(repo, git, &head, default_branch, stop)?;
    if line.branches.len() == 1 {
        return Err(RewriteError::Refused(Refusal::PlainAmend));
    }
    let mut todo: Vec<TodoLine> = range.iter().map(RangeCommit::pick).collect();
    for row in &mut todo {
        row.action = TodoAction::Reword;
    }
    let messages = (0..todo.len())
        .map(|row| (row, message.to_owned()))
        .collect();
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages,
        made: Vec::new(),
        index_base: None,
    })
}

/// `a` reaches exactly the commits absorb could put a hunk into
/// ([`your_commits`]); a target past them — an ancestor of `HEAD`, checked
/// before — is refused by what ended the walk (06e ruling table). `HEAD`
/// itself is amend, whatever a remote holds or what is built on it: no
/// range check (06e ruling 5).
fn refuse_out_of_reach(
    repo: &RepoHandle,
    head: &Tip,
    target: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<(), RewriteError> {
    if target == head.oid {
        return Ok(());
    }
    let range = your_commits(repo, head, default_branch, stop)?;
    if range.commits.contains(&target) {
        return Ok(());
    }
    let refusal = match range.stop {
        // Every commit not taken is an ancestor of the one the walk stopped
        // at, so a pushed stop means the target is pushed.
        RangeStop::Pushed => Refusal::AlreadyPushed { commit: target },
        RangeStop::Base | RangeStop::NotItsOwn => Refusal::NotOnYourBranch {
            commit: target,
            by: RewriteKey::AmendInto,
        },
        RangeStop::Merge(merge) => Refusal::MergeInRange { merge },
        RangeStop::Root => Refusal::RootCommit { commit: target },
        RangeStop::Cap => Refusal::BeyondReach {
            commit: target,
            branch: short_name(head),
        },
    };
    Err(RewriteError::Refused(refusal))
}

/// The branch's short name (`main`), for the words.
pub(super) fn short_name(head: &Tip) -> String {
    head.name
        .strip_prefix("refs/heads/")
        .unwrap_or(&head.name)
        .to_owned()
}

/// The first merge in the range refuses the rewrite: `--rebase-merges` is
/// not in M7.
pub(super) fn refuse_merges(range: &[RangeCommit]) -> Result<(), RewriteError> {
    match range.iter().find(|commit| commit.parents.len() > 1) {
        Some(merge) => Err(RewriteError::Refused(Refusal::MergeInRange {
            merge: merge.oid,
        })),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests;
