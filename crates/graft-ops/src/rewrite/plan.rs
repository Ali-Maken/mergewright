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

use crate::absorb;
use crate::facts;
use crate::operation::{Refusal, RepoFacts, RewriteKey, Tip};

use super::dependents::line_of;
use super::reads::{
    RangeCommit, ancestor, base_of, commit_index, parents_of, read_range, root_of, unparsable,
};
use super::todo::autosquash;
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
/// `default_branch` is the user's `graph.default_branch` when set: it wins
/// over git's guess (ADR-0001 rule 1), for the base of the range and for
/// which branch is never a dependent.
///
/// # Errors
/// [`RewriteError::Refused`], checked in this order: not on a branch;
/// nothing staged (`AmendInto`); the commit not on his branch; the first
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
    let head = Tip {
        name: name.clone(),
        oid,
    };
    match req {
        RewriteRequest::Open { from } => open(repo, git, head, *from, default_branch, stop),
        RewriteRequest::AmendInto { target } => {
            amend_into(repo, git, head, *target, facts, default_branch, stop)
        }
        RewriteRequest::Absorb => absorb(repo, git, head, default_branch, stop),
    }
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
    })
}

/// Absorb: each staged hunk the map places ([`crate::absorb`]) goes into a
/// commit object with no ref (parent `HEAD`, one per target), planned as a
/// fixup right after its target. The rows start at the oldest target.
/// Hunks that stay are left where they are: the set-aside puts them back.
fn absorb(
    repo: &RepoHandle,
    git: &GitVersion,
    head: Tip,
    default_branch: Option<&RefName>,
    stop: &dyn Stop,
) -> Result<RewriteParams, RewriteError> {
    let base = match base_of(repo, &head, default_branch, stop)? {
        Some(base) => base,
        // No upstream and no default branch to measure from: the whole
        // history above the first commit is his branch.
        None => root_of(repo, head.oid, stop)?,
    };
    let range = read_range(repo, base, head.oid, false, stop)?;
    let mapped = absorb::mapped(repo, head.oid, base, stop)?;
    let Some(first) = range.iter().position(|commit| {
        mapped
            .placed
            .iter()
            .any(|(_, target)| *target == commit.oid)
    }) else {
        return Err(RewriteError::Refused(Refusal::NothingToAbsorb));
    };
    let rows = range.get(first..).unwrap_or_default();
    refuse_merges(rows)?;
    let Some(&onto) = rows.first().and_then(|commit| commit.parents.first()) else {
        return Err(unparsable("absorb's first target has no parent".to_owned()).into());
    };
    let line = line_of(repo, git, &head, default_branch, stop)?;
    let mut todo = Vec::with_capacity(rows.len() + mapped.placed.len());
    let mut made = Vec::new();
    for commit in rows {
        todo.push(commit.pick());
        let pieces: Vec<&absorb::Piece> = mapped
            .placed
            .iter()
            .filter(|(_, target)| *target == commit.oid)
            .map(|(piece, _)| piece)
            .collect();
        if pieces.is_empty() {
            continue;
        }
        let subject = format!("fixup! {}", commit.subject);
        let oid = absorb::commit_pieces(repo, head.oid, &pieces, &subject, stop)?;
        todo.push(TodoLine {
            action: TodoAction::Fixup,
            commit: oid,
            subject,
        });
        made.push(oid);
    }
    Ok(RewriteParams {
        head,
        onto,
        todo,
        line,
        messages: BTreeMap::new(),
        made,
    })
}

/// The first merge in the range refuses the rewrite: `--rebase-merges` is
/// not in M7.
fn refuse_merges(range: &[RangeCommit]) -> Result<(), RewriteError> {
    match range.iter().find(|commit| commit.parents.len() > 1) {
        Some(merge) => Err(RewriteError::Refused(Refusal::MergeInRange {
            merge: merge.oid,
        })),
        None => Ok(()),
    }
}
