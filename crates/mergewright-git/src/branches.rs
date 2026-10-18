//! Which branch owns lane 0, and which branches sit beside it.
//!
//! The graph reserves lane 0 for the **default branch**, so nothing can be
//! drawn until that branch is known, and `main` is the wrong guess often
//! enough to matter. ADR-0001 settles it with an ordered precedence, tried
//! top to bottom and stopping at the first rule that resolves:
//!
//! 1. an override for this repository, from config;
//! 2. `origin/HEAD`, which is what the remote itself calls its default;
//! 3. `init.defaultBranch` from git config, if that branch exists here;
//! 4. the first of `main`, then `master`, that exists;
//! 5. the branch the most first-parent merges land on.
//!
//! Rule 5 is a heuristic over history and is the slow one: it reads each
//! branch's first-parent chain. That is why the whole of this is computed
//! once per repository, off the UI thread, and cached — and why the cheaper
//! rules come first, so most repositories never reach it.
//!
//! **Integration branches** — `develop` and `release/*` — are named
//! separately and never compete for lane 0. Without that, a busy release
//! branch can out-merge the default branch under rule 5 and the graph's
//! spine moves between one open and the next.

use std::path::PathBuf;

use mergewright_core::oid::Oid;
use mergewright_core::ref_name::RefName;

use crate::history::{WalkError, first_parent_merges};
use crate::repo::{RefEntry, RefTarget, RefsError, RepoHandle};

/// Where a branch name lives.
const HEADS: &[u8] = b"refs/heads/";

/// The remote whose `HEAD` speaks for the project.
const ORIGIN_HEAD: &[u8] = b"refs/remotes/origin/HEAD";

/// Branches that get a lane next to lane 0, and never lane 0 itself.
const INTEGRATION: &[u8] = b"develop";

/// The prefix every release branch shares.
const RELEASE_PREFIX: &[u8] = b"release/";

/// Which of ADR-0001's rules produced the answer.
///
/// Kept because it is the difference between "this is what the remote says"
/// and "this is a guess from counting merges", and the UI is allowed to say
/// which.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Chosen {
    /// The user named it for this repository.
    Configured,
    /// `origin/HEAD` resolved to it.
    OriginHead,
    /// `init.defaultBranch` names it and it exists.
    InitDefaultBranch,
    /// It is `main`, or `master` where there is no `main`.
    ConventionalName,
    /// More first-parent merges land here than on any other branch.
    MostMergedInto,
}

/// The branches the graph's lanes are anchored on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Branches {
    default: Option<RefName>,
    chosen: Option<Chosen>,
    integration: Vec<RefName>,
}

impl Branches {
    /// Resolves the default and integration branches for one repository.
    ///
    /// `override_name` is the per-repository setting ADR-0001 requires, as a
    /// full ref name. It wins over every other rule when the branch exists,
    /// and is ignored when it does not — a stale name in config must not
    /// leave the graph with no spine.
    ///
    /// Call this once per repository and keep the answer: rule 5 reads
    /// history.
    ///
    /// # Errors
    /// [`BranchError`] if the refs cannot be read, or a commit on a
    /// first-parent chain cannot be.
    pub fn resolve(
        handle: &RepoHandle,
        override_name: Option<&RefName>,
    ) -> Result<Self, BranchError> {
        Branches::resolve_from(handle, &handle.references()?, override_name)
    }

    /// [`resolve`](Self::resolve) over refs the caller has already read —
    /// every ref, as [`RepoHandle::references`] returns them — so a caller
    /// that needs the refs anyway does not read the ref store twice
    /// (m1-the-graph/55). `handle` is still read: rule 3's config, rule 5's
    /// history.
    ///
    /// # Errors
    /// [`BranchError`] if a commit on a first-parent chain cannot be read.
    pub fn resolve_from(
        handle: &RepoHandle,
        refs: &[RefEntry],
        override_name: Option<&RefName>,
    ) -> Result<Self, BranchError> {
        let locals: Vec<&RefEntry> = refs
            .iter()
            .filter(|entry| entry.name.as_bytes().starts_with(HEADS))
            .collect();

        let (default, chosen) = match pick(handle, refs, &locals, override_name)? {
            Some((name, chosen)) => (Some(name), Some(chosen)),
            None => (None, None),
        };

        let integration = locals
            .iter()
            .map(|entry| entry.name.clone())
            .filter(|name| Some(name) != default.as_ref() && is_integration(name))
            .collect();

        Ok(Branches {
            default,
            chosen,
            integration,
        })
    }

    /// The branch lane 0 belongs to, or [`None`] in a repository with no
    /// branch at all — a fresh `git init`, or one with only a detached
    /// `HEAD`.
    #[must_use]
    pub fn default_branch(&self) -> Option<&RefName> {
        self.default.as_ref()
    }

    /// Which rule produced [`default_branch`](Self::default_branch), or
    /// [`None`] when there is no default branch.
    #[must_use]
    pub fn chosen_by(&self) -> Option<Chosen> {
        self.chosen
    }

    /// The integration branches, in ref-name order. Never includes the
    /// default branch, even when it is called `develop`.
    #[must_use]
    pub fn integration(&self) -> &[RefName] {
        &self.integration
    }
}

/// ADR-0001's precedence, in order, stopping at the first rule that answers.
fn pick(
    handle: &RepoHandle,
    refs: &[RefEntry],
    locals: &[&RefEntry],
    override_name: Option<&RefName>,
) -> Result<Option<(RefName, Chosen)>, BranchError> {
    if let Some(name) = override_name
        && let Some(found) = named(locals, name.as_bytes())
    {
        return Ok(Some((found, Chosen::Configured)));
    }
    if let Some(found) = from_origin_head(refs, locals) {
        return Ok(Some((found, Chosen::OriginHead)));
    }
    if let Some(found) = from_init_default_branch(handle, locals) {
        return Ok(Some((found, Chosen::InitDefaultBranch)));
    }
    for conventional in [b"refs/heads/main".as_slice(), b"refs/heads/master"] {
        if let Some(found) = named(locals, conventional) {
            return Ok(Some((found, Chosen::ConventionalName)));
        }
    }
    Ok(most_merged_into(handle, locals)?.map(|name| (name, Chosen::MostMergedInto)))
}

/// The local branch called `name`, if there is one.
fn named(locals: &[&RefEntry], name: &[u8]) -> Option<RefName> {
    locals
        .iter()
        .find(|entry| entry.name.as_bytes() == name)
        .map(|entry| entry.name.clone())
}

/// What `origin/HEAD` points at, as a local branch.
///
/// A clone writes `refs/remotes/origin/HEAD` as a symbolic ref to
/// `refs/remotes/origin/<branch>`; the branch that matters here is the local
/// one of the same name. A mirror with no local branches falls through to
/// the next rule rather than putting a remote-tracking ref in lane 0, which
/// would be a lane nothing can be committed to.
fn from_origin_head(refs: &[RefEntry], locals: &[&RefEntry]) -> Option<RefName> {
    let head = refs
        .iter()
        .find(|entry| entry.name.as_bytes() == ORIGIN_HEAD)?;
    let RefTarget::Symbolic(target) = &head.target else {
        // A detached `origin/HEAD` names a commit, not a branch, and so
        // says nothing about which branch is the default.
        return None;
    };
    let short = target.as_bytes().strip_prefix(b"refs/remotes/origin/")?;
    named(locals, &[HEADS, short].concat())
}

/// The branch `init.defaultBranch` names, if it exists here.
fn from_init_default_branch(handle: &RepoHandle, locals: &[&RefEntry]) -> Option<RefName> {
    let configured = handle
        .gix()
        .config_snapshot()
        .string("init.defaultBranch")?;
    named(locals, &[HEADS, configured.as_slice()].concat())
}

/// The branch the most first-parent merges land on.
///
/// A tie goes to the first name in ref-name order, so the answer does not
/// depend on how the ref store happened to enumerate.
fn most_merged_into(
    handle: &RepoHandle,
    locals: &[&RefEntry],
) -> Result<Option<RefName>, BranchError> {
    let mut best: Option<(usize, RefName)> = None;
    for entry in locals {
        let RefTarget::Object(tip) = entry.target else {
            // A symbolic branch is a pointer to another branch, which is
            // counted under its own name.
            continue;
        };
        let merges = merges_on(handle, tip)?;
        let better = match &best {
            Some((most, _)) => merges > *most,
            None => true,
        };
        if better {
            best = Some((merges, entry.name.clone()));
        }
    }
    Ok(best.map(|(_, name)| name))
}

/// [`first_parent_merges`], with a missing commit treated as an empty chain.
///
/// A branch whose tip object is not in the repository is a broken ref, not a
/// reason to refuse to draw the graph: it contributes zero merges and every
/// other branch is still counted. Any other failure is reported.
fn merges_on(handle: &RepoHandle, tip: Oid) -> Result<usize, BranchError> {
    match first_parent_merges(handle, tip) {
        Ok(merges) => Ok(merges),
        Err(WalkError::Commit(crate::history::NodeError::Missing { .. })) => Ok(0),
        Err(other) => Err(BranchError::Walk {
            git_dir: handle.git_dir().to_path_buf(),
            source: other,
        }),
    }
}

/// Whether a branch is an integration branch: `develop`, or `release/…`.
fn is_integration(name: &RefName) -> bool {
    let short = name.shorthand();
    short == INTEGRATION || short.starts_with(RELEASE_PREFIX)
}

/// Why the branches could not be resolved.
#[derive(Debug, thiserror::Error)]
pub enum BranchError {
    /// The refs could not be read.
    #[error(transparent)]
    Refs(#[from] RefsError),
    /// A first-parent chain could not be walked.
    #[error("cannot count merges in {}: {source}", git_dir.display())]
    Walk {
        /// The repository being read.
        git_dir: PathBuf,
        /// What went wrong on the chain.
        source: WalkError,
    },
}

#[cfg(test)]
mod tests;
