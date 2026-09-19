//! Fetch, pull and push: the three operations that talk to a remote.
//!
//! Their plans need no facts — every argument comes from the params — which
//! is what lets [`super::Operation::preview`] show the push box's command
//! lines before anything is read. Every invocation that reaches the network
//! is `remote: true` and carries `--progress` (git only prints progress to a
//! terminal otherwise, and the runner's stderr is a pipe).
//!
//! Verified against git 2.54 in a throwaway repository before writing:
//! `merge --ff-only` over a dirty file the fast-forward changes refuses and
//! leaves the bytes; on diverged branches it fails `Not possible to
//! fast-forward` with `HEAD` unchanged; `fetch --all --prune` drops a
//! tracking ref whose branch was deleted; `push -u origin
//! refs/heads/x:refs/heads/x` writes `branch.x.remote`/`.merge`; a lease push
//! after a local amend succeeds and after a remote-side move is `stale info`;
//! `fetch` in a bare repository works.

use graft_core::journal::HeadRef;
use graft_core::oid::Oid;
use graft_git::remote::TrackedUpstream;
use graft_git::run::GitArg;

use super::{Invocation, Refusal, RepoFacts};

/// What a fetch needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FetchParams {
    /// One remote by name; `None` is `--all`.
    pub remote: Option<String>,
    /// `--prune`: drop tracking refs whose remote branch is gone.
    pub prune: bool,
}

/// What a fast-forward pull needs: the branch and the upstream the app saw,
/// so a pull against a stale picture is refused rather than guessed at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullParams {
    /// The branch being pulled into, full name (`refs/heads/main`).
    pub branch: String,
    /// Its upstream as the app read it.
    pub upstream: TrackedUpstream,
}

/// What a push needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PushParams {
    /// The local branch, full name (`refs/heads/main`).
    pub branch: String,
    /// The remote's name.
    pub remote: String,
    /// The branch on the remote, full name (`refs/heads/main`).
    pub remote_branch: String,
    /// `-u`: record the pushed branch as the local branch's upstream.
    pub set_upstream: bool,
    /// A force push, guarded by a lease. Never a force without one.
    pub lease: Option<Lease>,
}

/// `--force-with-lease=<remote_branch>:<expected> --force-if-includes`: the
/// remote branch is overwritten only if it is still at `expected`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    /// Where the remote branch must still be.
    pub expected: Oid,
}

/// `main` from `refs/heads/main`; anything else unchanged.
pub(super) fn short_branch(full: &str) -> &str {
    full.strip_prefix("refs/heads/").unwrap_or(full)
}

/// `origin/main` from `refs/remotes/origin/main`; anything else unchanged.
fn short_tracking(full: &str) -> &str {
    full.strip_prefix("refs/remotes/").unwrap_or(full)
}

pub(super) fn describe_fetch(params: &FetchParams) -> String {
    match &params.remote {
        None => "fetch all remotes".to_owned(),
        Some(name) => format!("fetch {name}"),
    }
}

pub(super) fn describe_pull(params: &PullParams) -> String {
    format!("pull {}", short_tracking(&params.upstream.tracking_ref))
}

pub(super) fn describe_push(params: &PushParams) -> String {
    let target = format!(
        "{} → {}/{}",
        short_branch(&params.branch),
        params.remote,
        short_branch(&params.remote_branch)
    );
    match (params.lease.is_some(), params.set_upstream) {
        (false, false) => format!("push {target}"),
        (false, true) => format!("push {target} (set upstream)"),
        (true, false) => format!("force-push {target} (with lease)"),
        (true, true) => format!("force-push {target} (with lease, set upstream)"),
    }
}

/// The remote refusals' sentences, in the design note's words.
/// `Refusal::explain` hands only these here; any other refusal goes back to
/// it, so the answer is the same whichever of the two is called.
pub(super) fn explain(refusal: &Refusal) -> String {
    match refusal {
        Refusal::DetachedHead => "HEAD is detached — check out a branch first".to_owned(),
        Refusal::NoRemote => {
            "no remote — add one in the shell: git remote add origin <url>".to_owned()
        }
        Refusal::NoSuchRemote { name } => format!("no remote named '{name}'"),
        Refusal::NoUpstream { branch } => {
            format!("{branch} has no upstream — P pushes and sets one")
        }
        Refusal::UpstreamChanged => {
            "the branch's upstream changed since it was read — press p again".to_owned()
        }
        Refusal::BranchChanged => {
            "HEAD moved to another branch since it was read — look again".to_owned()
        }
        other => other.explain(),
    }
}

fn check_remote_named(name: &str, facts: &RepoFacts) -> Result<(), Refusal> {
    if facts.remotes.is_empty() {
        return Err(Refusal::NoRemote);
    }
    if facts.remotes.iter().any(|remote| remote.name == name) {
        Ok(())
    } else {
        Err(Refusal::NoSuchRemote {
            name: name.to_owned(),
        })
    }
}

/// The branch `HEAD` is on, or `DetachedHead` (an unborn branch has nothing
/// to pull into or push, and is refused the same way).
fn head_branch(facts: &RepoFacts) -> Result<&str, Refusal> {
    match &facts.head {
        HeadRef::Branch(name) => Ok(name),
        HeadRef::Detached(_) | HeadRef::Unborn(_) => Err(Refusal::DetachedHead),
    }
}

pub(super) fn check_fetch(params: &FetchParams, facts: &RepoFacts) -> Result<(), Refusal> {
    match &params.remote {
        Some(name) => check_remote_named(name, facts),
        None if facts.remotes.is_empty() => Err(Refusal::NoRemote),
        None => Ok(()),
    }
}

pub(super) fn check_pull(params: &PullParams, facts: &RepoFacts) -> Result<(), Refusal> {
    let branch = head_branch(facts)?;
    let Some(upstream) = &facts.upstream else {
        return Err(Refusal::NoUpstream {
            branch: short_branch(branch).to_owned(),
        });
    };
    if params.branch != branch {
        return Err(Refusal::BranchChanged);
    }
    if params.upstream != *upstream {
        return Err(Refusal::UpstreamChanged);
    }
    Ok(())
}

pub(super) fn check_push(params: &PushParams, facts: &RepoFacts) -> Result<(), Refusal> {
    let branch = head_branch(facts)?;
    check_remote_named(&params.remote, facts)?;
    if params.branch != branch {
        return Err(Refusal::BranchChanged);
    }
    Ok(())
}

fn remote_invocation(args: Vec<GitArg>) -> Invocation {
    Invocation {
        args,
        stdin: None,
        remote: true,
    }
}

pub(super) fn plan_fetch(params: &FetchParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("fetch"), GitArg::Flag("--progress")];
    if params.prune {
        args.push(GitArg::Flag("--prune"));
    }
    match &params.remote {
        None => args.push(GitArg::Flag("--all")),
        Some(name) => args.push(GitArg::Word(name.clone())),
    }
    vec![remote_invocation(args)]
}

/// Fetch the upstream's remote, then fast-forward onto the tracking ref. The
/// merge is local (`remote: false`): it is the step that can refuse over a
/// dirty file or diverged branches, and it runs only if the fetch succeeded.
pub(super) fn plan_pull(params: &PullParams) -> Vec<Invocation> {
    let fetch = remote_invocation(vec![
        GitArg::Flag("fetch"),
        GitArg::Flag("--progress"),
        GitArg::Word(params.upstream.remote.clone()),
    ]);
    let merge = Invocation::new(vec![
        GitArg::Flag("merge"),
        GitArg::Flag("--ff-only"),
        GitArg::Flag("--no-edit"),
        GitArg::Ref(params.upstream.tracking_ref.clone().into_bytes()),
    ]);
    vec![fetch, merge]
}

pub(super) fn plan_push(params: &PushParams) -> Vec<Invocation> {
    let mut args = vec![GitArg::Flag("push"), GitArg::Flag("--progress")];
    if params.set_upstream {
        args.push(GitArg::Flag("-u"));
    }
    if let Some(lease) = params.lease {
        args.push(GitArg::Word(format!(
            "--force-with-lease={}:{}",
            params.remote_branch, lease.expected
        )));
        args.push(GitArg::Flag("--force-if-includes"));
    }
    args.push(GitArg::Word(params.remote.clone()));
    args.push(GitArg::Ref(
        format!("{}:{}", params.branch, params.remote_branch).into_bytes(),
    ));
    vec![remote_invocation(args)]
}

/// The tracking ref a push moves, when the facts know it: the head branch's
/// upstream, if the push goes there. Recorded for the journal browser's
/// `origin/main a1b2c3d → 9f8e7d6`, never for undo.
pub(super) fn push_tracking_ref(params: &PushParams, facts: &RepoFacts) -> Vec<String> {
    facts
        .upstream
        .as_ref()
        .filter(|up| up.remote == params.remote && up.remote_branch == params.remote_branch)
        .map(|up| up.tracking_ref.clone())
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests;
