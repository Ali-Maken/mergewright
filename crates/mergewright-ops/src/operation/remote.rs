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

use mergewright_core::journal::HeadRef;
use mergewright_core::oid::Oid;
use mergewright_git::remote::TrackedUpstream;
use mergewright_git::run::GitArg;

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
    /// Push this commit instead of the branch's tip: the guided re-push
    /// after an undo (ADR-0037 §3, ADR-0038 §4). The refspec becomes
    /// `<oid>:<remote_branch>`, so the remote goes back to where it was
    /// while the local branch keeps every commit it has. It must be an
    /// ancestor of `branch`'s tip — pushing an unrelated commit is not a
    /// re-push — which needs a `merge-base --is-ancestor` spawn and so is
    /// checked in [`crate::preview::plan`], not in `check`
    /// ([`Refusal::NotOnBranch`]). `None` pushes the tip, as every push
    /// before M5 did.
    pub source_commit: Option<Oid>,
}

/// `--force-with-lease=<remote_branch>:<expected>`: the remote branch is
/// overwritten only if it is still at `expected`.
///
/// Two checks guard it. Before git runs, `check` refuses
/// [`Refusal::LeaseStale`] unless `expected` is the local tracking ref's tip
/// — the number the force-push box showed — so work fetched since the box
/// was read, but never looked at, is not overwritten. Then git's own
/// compare-and-swap refuses if the remote moved since the last fetch.
/// (`--force-if-includes` is not used: git makes it a no-op alongside an
/// explicit `<refname>:<expect>`, m4/04 review M1.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Lease {
    /// Where the remote branch must still be.
    pub expected: Oid,
}

/// The names a remote operation's failure line uses, taken from its params
/// — never read back out of its description, which a refname containing
/// `→` or ` (` would garble (m4/04 review, M4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RemoteNames {
    /// The local branch, short: `main`. Empty for a fetch.
    pub local: String,
    /// The branch on the remote as the user knows it: `origin/main`. For a
    /// fetch, the remote's name or `all remotes`.
    pub remote_side: String,
    /// The remote: `origin`, or `the remote` for a fetch of all of them.
    pub remote: String,
}

pub(super) fn names_of_fetch(params: &FetchParams) -> RemoteNames {
    RemoteNames {
        local: String::new(),
        remote_side: params
            .remote
            .clone()
            .unwrap_or_else(|| "all remotes".to_owned()),
        remote: params
            .remote
            .clone()
            .unwrap_or_else(|| "the remote".to_owned()),
    }
}

pub(super) fn names_of_pull(params: &PullParams) -> RemoteNames {
    RemoteNames {
        local: short_branch(&params.branch).to_owned(),
        remote_side: short_tracking(&params.upstream.tracking_ref).to_owned(),
        remote: params.upstream.remote.clone(),
    }
}

pub(super) fn names_of_push(params: &PushParams) -> RemoteNames {
    RemoteNames {
        local: short_branch(&params.branch).to_owned(),
        remote_side: format!("{}/{}", params.remote, short_branch(&params.remote_branch)),
        remote: params.remote.clone(),
    }
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
    // A re-push names the commit, not the branch: what moves is the remote,
    // and the local branch is not where it is going (ADR-0038 §4).
    let source = match params.source_commit {
        Some(oid) => super::history::short(oid),
        None => short_branch(&params.branch).to_owned(),
    };
    let target = format!(
        "{source} → {}/{}",
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

/// Whether a pull sets edits aside (m7-power-tools/03b): its fetch brought
/// a fast-forward, and a path it changes is a changed tracked file here —
/// staged or not, a rename's original path included. `false` before the
/// fetch ([`RepoFacts::incoming`] unread) and over untracked files alone.
pub(super) fn pull_overlaps(facts: &RepoFacts) -> bool {
    let Some(incoming) = facts
        .incoming
        .as_ref()
        .filter(|incoming| incoming.fast_forward)
    else {
        return false;
    };
    let brings = |path: &std::path::PathBuf| incoming.paths.contains(path);
    facts
        .status
        .entries
        .iter()
        .filter(|entry| entry.is_staged() || entry.is_unstaged())
        .any(|entry| brings(&entry.path) || entry.original_path.as_ref().is_some_and(brings))
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
    if let Some(lease) = params.lease {
        check_lease(lease, params, facts)?;
    } else if params.source_commit.is_some() {
        // A re-push moves the remote branch backwards, so git would refuse
        // it without a force anyway; this refusal is the one that says why,
        // and keeps "lease or nothing" in one place (ADR-0038 §4).
        return Err(Refusal::RepushNeedsLease);
    }
    Ok(())
}

/// The lease's `expected` must be the local tracking ref's tip for the
/// branch being overwritten. The tip is known only through `HEAD`'s
/// upstream; a push elsewhere, or one never fetched, has nothing to compare
/// against, and is refused the same way rather than trusted.
fn check_lease(lease: Lease, params: &PushParams, facts: &RepoFacts) -> Result<(), Refusal> {
    let fetched = facts
        .upstream
        .as_ref()
        .filter(|up| up.remote == params.remote && up.remote_branch == params.remote_branch)
        .and_then(|up| up.tracking_tip);
    if fetched == Some(lease.expected) {
        Ok(())
    } else {
        Err(Refusal::LeaseStale {
            remote_side: format!("{}/{}", params.remote, short_branch(&params.remote_branch)),
        })
    }
}

fn remote_step(args: Vec<GitArg>) -> Invocation {
    Invocation {
        args,
        stdin: None,
        remote: true,
        index: super::IndexFile::Real,
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
    vec![remote_step(args)]
}

/// Fetch the upstream's remote, then fast-forward onto the tracking ref. The
/// merge is local (`remote: false`): it is the step that can refuse over a
/// dirty file or diverged branches, and it runs only if the fetch succeeded.
pub(super) fn plan_pull(params: &PullParams) -> Vec<Invocation> {
    let fetch = remote_step(vec![
        GitArg::Flag("fetch"),
        GitArg::Flag("--progress"),
        GitArg::Word(params.upstream.remote.clone()),
    ]);
    let merge = Invocation::new(vec![
        GitArg::Flag("merge"),
        GitArg::Flag("--ff-only"),
        // ADR-0049 §4: the fetch runs between Mergewright's read of the files in
        // the way and this write, so git's own guard stands at write time.
        GitArg::Flag("--no-overwrite-ignore"),
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
    }
    args.push(GitArg::Word(params.remote.clone()));
    // The source side of the refspec is the commit for a re-push, the
    // branch otherwise; the destination is always the remote branch.
    let source = match params.source_commit {
        Some(oid) => oid.to_string(),
        None => params.branch.clone(),
    };
    args.push(GitArg::Ref(
        format!("{source}:{}", params.remote_branch).into_bytes(),
    ));
    vec![remote_step(args)]
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
