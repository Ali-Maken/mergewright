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
use crate::operation::{Refusal, RepoFacts, RewriteKey, Tip};

use super::dependents::line_of;
use super::reads::{RangeCommit, ancestor, base_of, commit_index, parents_of, read_range};
use super::todo::autosquash;
use super::unpushed::{RangeStop, YourCommits, your_commits};
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
/// On a branch with no base of its own — the base comes out as `HEAD`, or
/// there is none (the default or only branch, no upstream) — absorb and
/// `AmendInto` both reach his unpushed commits only, at most the last
/// [`super::ABSORB_FALLBACK_DEPTH`], above any merge and the first commit
/// (06c ruling).
///
/// `default_branch` is the user's `graph.default_branch` when set: it wins
/// over git's guess (ADR-0001 rule 1), for the base of the range and for
/// which branch is never a dependent.
///
/// # Errors
/// [`RewriteError::Refused`], checked in this order: not on a branch;
/// nothing staged (`AmendInto`); the commit not on his branch, or past his
/// unpushed commits on a branch with no base of its own; every commit
/// pushed there (`Absorb`); the first
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
    }
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

/// On a branch with no base of its own, `a` reaches exactly the commits
/// absorb could put a hunk into ([`your_commits`]); a target past them is
/// refused by what ended the walk (06c ruling). With a base of its own,
/// nothing here refuses.
fn refuse_out_of_reach(
    repo: &RepoHandle,
    head: &Tip,
    target: Oid,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<(), RewriteError> {
    let YourCommits::Unpushed(range) = your_commits(repo, head, default_branch, stop)? else {
        return Ok(());
    };
    if range.commits.contains(&target) {
        return Ok(());
    }
    let refusal = match range.stop {
        RangeStop::Pushed => Refusal::AlreadyPushed {
            commit: target,
            by: RewriteKey::AmendInto,
        },
        RangeStop::Cap => Refusal::BeyondReach { commit: target },
        RangeStop::Merge(merge) => Refusal::MergeInRange { merge },
        RangeStop::Root => Refusal::RootCommit { commit: target },
    };
    Err(RewriteError::Refused(refusal))
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
