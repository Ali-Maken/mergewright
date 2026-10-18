//! Remotes: which are configured, which one a branch pulls from, how a failed
//! fetch, pull or push is told apart, and how its progress reads.
//!
//! The reads here come from `gix`'s view of the repository's config and ref
//! store — no `git` process is spawned (ADR-0011). What a remote operation
//! *does* runs through [`crate::run`]; this module only helps its caller read
//! the answer: [`classify_failure`] over the whole stderr git wrote, and
//! [`parse_progress`] over each progress segment as it arrives.

use std::cmp::Ordering;

use gix::bstr::ByteSlice;
use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use crate::repo::{RefTarget, RefsError, RepoHandle};

mod failure;
mod progress;

pub use failure::{RemoteFailure, classify_failure};
pub use progress::{RemoteProgress, parse_progress};

/// One configured remote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Remote {
    /// Its name: `origin`.
    pub name: String,
    /// `remote.<name>.url`, as written in the config.
    pub fetch_url: String,
    /// `remote.<name>.pushurl`, when one is set; pushes go to `fetch_url`
    /// otherwise.
    pub push_url: Option<String>,
}

/// Where a local branch pulls from, per its `branch.<name>.*` config.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackedUpstream {
    /// `branch.<name>.remote`: `origin`.
    pub remote: String,
    /// `branch.<name>.merge`, the branch's name on the remote:
    /// `refs/heads/main`.
    pub remote_branch: String,
    /// The local ref that mirrors it, through the remote's fetch refspec:
    /// `refs/remotes/origin/main`.
    pub tracking_ref: String,
    /// Where `tracking_ref` points; `None` before the first fetch.
    pub tracking_tip: Option<Oid>,
}

/// Why the remotes could not be read.
#[derive(Debug, thiserror::Error)]
pub enum RemoteError {
    /// The config says something this module cannot use; the text says what.
    #[error("remote config: {0}")]
    Config(String),
    /// The ref store could not be read.
    #[error(transparent)]
    Refs(#[from] RefsError),
}

/// Every remote with a URL, sorted by name with `origin` first.
///
/// A `[remote "x"]` section with no `url` (only a refspec, say) is left out:
/// git itself cannot fetch from it, so there is nothing to offer. A name or
/// URL that is not UTF-8 is a [`RemoteError::Config`].
///
/// # Errors
/// [`RemoteError::Config`] for a remote name or URL that is not UTF-8.
pub fn list(repo: &RepoHandle) -> Result<Vec<Remote>, RemoteError> {
    let gix = repo.gix();
    let config = gix.config_snapshot();
    let file = config.plumbing();
    let mut remotes = Vec::new();
    for name in gix.remote_names() {
        let text = |key: &str| -> Result<Option<String>, RemoteError> {
            file.string_by("remote", Some(name.as_bstr()), key)
                .map(|value| {
                    String::from_utf8(value.to_vec()).map_err(|_| {
                        RemoteError::Config(format!("remote.{name}.{key} is not UTF-8"))
                    })
                })
                .transpose()
        };
        let Some(fetch_url) = text("url")? else {
            continue;
        };
        let name = name
            .to_str()
            .map_err(|_| RemoteError::Config(format!("remote name {name:?} is not UTF-8")))?
            .to_owned();
        remotes.push(Remote {
            push_url: text("pushurl")?,
            name,
            fetch_url,
        });
    }
    remotes.sort_by(|a, b| origin_first(&a.name, &b.name));
    Ok(remotes)
}

fn origin_first(a: &str, b: &str) -> Ordering {
    (a != "origin", a).cmp(&(b != "origin", b))
}

/// The upstream `branch` (a full name, `refs/heads/main`) pulls from; `None`
/// when it has no `branch.<name>.remote` and `.merge`, or when the remote's
/// fetch refspec does not map it to a local ref.
///
/// The tracking ref comes from [`RepoHandle::upstream`], which applies the
/// remote's fetch refspecs through `gix` rather than guessing at
/// `refs/remotes/<remote>/<branch>`.
///
/// # Errors
/// [`RemoteError::Refs`] when the config or ref store cannot be read;
/// [`RemoteError::Config`] for a value that is not UTF-8.
pub fn upstream_of(
    repo: &RepoHandle,
    branch: &str,
) -> Result<Option<TrackedUpstream>, RemoteError> {
    let Some(short) = branch.strip_prefix("refs/heads/") else {
        return Ok(None);
    };
    let gix = repo.gix();
    let Some(remote) = gix.branch_remote_name(short, gix::remote::Direction::Fetch) else {
        return Ok(None);
    };
    let remote = utf8(remote.as_bstr(), "branch.<name>.remote")?;
    let full = gix::refs::FullName::try_from(branch)
        .map_err(|why| RemoteError::Config(format!("{branch} is not a branch name: {why}")))?;
    let remote_branch = match gix
        .branch_remote_ref_name(full.as_ref(), gix::remote::Direction::Fetch)
    {
        None => return Ok(None),
        Some(Err(why)) => return Err(RemoteError::Config(format!("branch.{short}.merge: {why}"))),
        Some(Ok(name)) => utf8(name.as_bstr(), "branch.<name>.merge")?,
    };
    let branch_name = RefName::new(branch.as_bytes().to_vec())
        .map_err(|why| RemoteError::Config(format!("{branch}: {why}")))?;
    let Some(tracking) = repo.upstream(&branch_name)? else {
        return Ok(None);
    };
    let tracking_tip = match repo.reference(&tracking)? {
        Some(RefTarget::Object(id)) => Some(id),
        // A symbolic remote-tracking ref (rare: `refs/remotes/origin/HEAD` is
        // the usual one, and no branch merges from it) has no tip of its own.
        Some(RefTarget::Symbolic(_)) | None => None,
    };
    Ok(Some(TrackedUpstream {
        remote,
        remote_branch,
        tracking_ref: utf8(tracking.as_bytes().as_bstr(), "the tracking ref")?,
        tracking_tip,
    }))
}

fn utf8(bytes: &gix::bstr::BStr, what: &str) -> Result<String, RemoteError> {
    bytes
        .to_str()
        .map(str::to_owned)
        .map_err(|_| RemoteError::Config(format!("{what} is not UTF-8: {bytes:?}")))
}

#[cfg(test)]
mod tests;
