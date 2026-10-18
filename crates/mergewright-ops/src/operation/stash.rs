//! Stash push, apply, pop and drop.

use std::path::PathBuf;

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::run::GitArg;
use mergewright_git::stash::StashEntry;
use mergewright_git::version::GitVersion;

use super::{Invocation, Refusal, RepoFacts, StashParams, StashScope};

/// The stash ref; its reflog is the stash list.
pub(super) const STASH_REF: &str = "refs/stash";

/// `git stash push --staged` arrived in 2.35.
const STAGED_ONLY_SINCE: GitVersion = GitVersion {
    major: 2,
    minor: 35,
    patch: 0,
};

pub(super) fn describe_push(params: &StashParams) -> String {
    match &params.message {
        Some(message) => format!("stash \"{message}\""),
        None => "stash the working tree".to_owned(),
    }
}

/// A push needs a commit to base the stash on (`You do not have the initial
/// commit yet`, exit 1 — verified), something in its scope (`--staged` with
/// nothing staged is `No staged changes`, exit 1 — verified), and a git new
/// enough for `--staged`.
pub(super) fn check_push(params: &StashParams, facts: &RepoFacts) -> Result<(), Refusal> {
    if matches!(facts.head, HeadRef::Unborn(_)) {
        return Err(Refusal::UnbornHead);
    }
    let status = &facts.status;
    let something = match params.scope {
        StashScope::StagedOnly => status.staged().next().is_some(),
        StashScope::All { include_untracked } => {
            status.staged().next().is_some()
                || status.unstaged().next().is_some()
                || (include_untracked && status.untracked().next().is_some())
        }
    };
    if !something {
        return Err(Refusal::NothingToStash);
    }
    if params.scope == StashScope::StagedOnly && facts.git < STAGED_ONLY_SINCE {
        return Err(Refusal::NeedsGit {
            feature: "stash --staged",
            minimum: STAGED_ONLY_SINCE,
        });
    }
    Ok(())
}

/// The stash at `index`, only if it is still the `commit` the user chose.
pub(super) fn find(index: usize, commit: Oid, facts: &RepoFacts) -> Result<&StashEntry, Refusal> {
    let entry = facts
        .stashes
        .iter()
        .find(|stash| stash.index == index)
        .ok_or(Refusal::NoSuchStash)?;
    if entry.commit == commit {
        Ok(entry)
    } else {
        Err(Refusal::StashListChanged)
    }
}

/// Where the stash `commit` sits in the list now, found by its id: the
/// position is looked up, never assumed (ADR-0045 §2–3). `NoSuchStash` when
/// it is gone.
pub(super) fn position_of(commit: Oid, facts: &RepoFacts) -> Result<usize, Refusal> {
    facts
        .stashes
        .iter()
        .find(|stash| stash.commit == commit)
        .map(|stash| stash.index)
        .ok_or(Refusal::NoSuchStash)
}

/// `stash drop stash@{n}` for the stash whose commit is `commit`, `n` from
/// the facts `check` has just passed. Were it somehow not listed, the plan
/// names the commit itself, which `git stash drop` refuses as "not a stash
/// reference": a loud failure with nothing dropped, never a guess at a
/// position.
pub(super) fn plan_drop_by_commit(commit: Oid, facts: &RepoFacts) -> Invocation {
    match position_of(commit, facts) {
        Ok(index) => plan_at("drop", index, false),
        Err(_) => Invocation::new(vec![
            GitArg::Flag("stash"),
            GitArg::Flag("drop"),
            GitArg::Ref(commit.to_string().into_bytes()),
        ]),
    }
}

/// Apply and pop: the stash must exist as chosen, and none of the paths it
/// changes may have a change of its own now. That rule keeps M3 out of
/// conflict territory before M6 resolves conflicts.
pub(super) fn check_apply(index: usize, commit: Oid, facts: &RepoFacts) -> Result<(), Refusal> {
    let entry = find(index, commit, facts)?;
    let dirty: Vec<PathBuf> = entry
        .paths
        .iter()
        .filter(|path| {
            facts
                .status
                .entry(path)
                .is_some_and(|now| now.is_staged() || now.is_unstaged() || now.untracked)
        })
        .cloned()
        .collect();
    if dirty.is_empty() {
        Ok(())
    } else {
        Err(Refusal::TouchesChangedFiles { paths: dirty })
    }
}

pub(super) fn plan_push(params: &StashParams) -> Invocation {
    let mut args = vec![GitArg::Flag("stash"), GitArg::Flag("push")];
    match params.scope {
        StashScope::All {
            include_untracked: true,
        } => args.push(GitArg::Flag("-u")),
        StashScope::All {
            include_untracked: false,
        } => {}
        StashScope::StagedOnly => args.push(GitArg::Flag("--staged")),
    }
    if let Some(message) = &params.message {
        args.push(GitArg::Flag("-m"));
        args.push(GitArg::Word(message.clone()));
    }
    Invocation::new(args)
}

/// `stash <verb> [--index] stash@{<index>}`. `--index` puts the index back as
/// it was stashed, not just the working tree.
pub(super) fn plan_at(verb: &'static str, index: usize, restore_index: bool) -> Invocation {
    let mut args = vec![GitArg::Flag("stash"), GitArg::Flag(verb)];
    if restore_index {
        args.push(GitArg::Flag("--index"));
    }
    args.push(GitArg::Ref(format!("stash@{{{index}}}").into_bytes()));
    Invocation::new(args)
}
