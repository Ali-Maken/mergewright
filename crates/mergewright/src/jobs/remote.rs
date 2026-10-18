//! The remote facts read behind `f`, `p` and `P` (m4-remotes/05): which
//! remotes are configured, which branch `HEAD` is on, and what that branch
//! pulls from. Config and refs only — no spawn, no network — so it rides
//! along with every refresh the way the refs read does.

use std::sync::Arc;

use mergewright_core::oid::Oid;
use mergewright_git::history_reads::{commits_between, count_between};
use mergewright_git::remote::{self, Remote, TrackedUpstream};
use mergewright_git::repo::HeadState;

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::remote::{LEAVES_SHOWN, Leaves};

/// What the remote keys decide from.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RemoteFacts {
    /// Every configured remote with a URL, `origin` first.
    pub remotes: Vec<Remote>,
    /// `HEAD`'s branch, full (`refs/heads/main`); `None` when detached.
    /// An unborn branch is named: it has nothing to push, and `ops` says so.
    pub head_branch: Option<String>,
    /// What `head_branch` pulls from, when it is set up to.
    pub head_upstream: Option<TrackedUpstream>,
}

/// Reads [`RemoteFacts`] off the loop's thread.
///
/// # Errors
/// The failure text, ready to show: the repository could not be opened, or
/// its config, `HEAD` or the upstream's refs could not be read. A branch
/// name that is not UTF-8 is an error too — `ops` names branches as text,
/// and a mangled one would push somewhere the user did not mean.
pub fn read_remotes(repos: &RepoStore, cancel: &Cancel) -> Result<Arc<RemoteFacts>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let remotes = remote::list(&handle).map_err(|error| error.to_string())?;
    if cancel.is_cancelled() {
        return Err("cancelled".to_owned());
    }
    let head_branch = match handle.head().map_err(|error| error.to_string())? {
        HeadState::Branch(name) | HeadState::Unborn(name) => Some(
            String::from_utf8(name.as_bytes().to_vec())
                .map_err(|_| "HEAD's branch name is not UTF-8".to_owned())?,
        ),
        HeadState::Detached(_) => None,
    };
    let head_upstream = match &head_branch {
        Some(branch) => remote::upstream_of(&handle, branch).map_err(|error| error.to_string())?,
        None => None,
    };
    Ok(Arc::new(RemoteFacts {
        remotes,
        head_branch,
        head_upstream,
    }))
}

/// What a force-with-lease push of `sent` over `expected` takes off the
/// remote: `sent..expected`, newest first, at most [`LEAVES_SHOWN`], and the
/// count (m7-power-tools/16b).
///
/// # Errors
/// The repository not opening, or `git` failing (stopped included), as one
/// line.
pub fn read_leaves(
    repos: &RepoStore,
    sent: Oid,
    expected: Oid,
    cancel: &Cancel,
) -> Result<Leaves, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let range = commits_between(&handle, sent, expected, LEAVES_SHOWN, cancel)
        .map_err(|error| error.to_string())?;
    let count =
        count_between(&handle, sent, expected, cancel).map_err(|error| error.to_string())?;
    let mut newest = range.commits;
    newest.reverse();
    Ok(Leaves { newest, count })
}
