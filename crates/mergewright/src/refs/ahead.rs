//! What a local branch has to push, read off its label (ease-pass/07a), and
//! the counts behind it, kept across refs reads ([`AheadBehindCache`]).

use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use mergewright_core::oid::Oid;
use mergewright_git::history::AheadBehind;

use super::{RefLabel, Upstream};

impl RefLabel {
    /// What a local branch has to push: `By(n)` commits its upstream lacks
    /// (`By(0)`: level or only behind), `NotPushed` when it has no
    /// upstream. `None` for anything that is not a local branch: a remote,
    /// a tag, a stash or a detached `HEAD` has nothing to push.
    ///
    /// Read from the count [`super::read`] already stored on the label,
    /// once per refs read; asking costs nothing and walks nothing.
    #[must_use]
    pub fn ahead_of_upstream(&self) -> Option<AheadOfUpstream> {
        match self {
            Self::Branch { upstream, .. } => Some(match upstream {
                Upstream::None => AheadOfUpstream::NotPushed,
                Upstream::InSync { .. } => AheadOfUpstream::By(0),
                Upstream::Apart(counts) => AheadOfUpstream::By(counts.ahead),
            }),
            Self::DetachedHead | Self::Remote(_) | Self::Tag(_) | Self::Stash(_) => None,
        }
    }
}

/// How far a local branch is ahead of its upstream: what `↑N to push` and
/// `not pushed yet` draw from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AheadOfUpstream {
    /// No upstream configured, or the ref it names does not exist.
    NotPushed,
    /// Commits the branch has that its upstream lacks; `0` when in sync.
    By(usize),
}

/// Ahead/behind counts already worked out, keyed by (branch tip, upstream
/// tip): a pair that has not moved is never walked twice.
///
/// Held by the job runner for the life of the run and shared by every refs
/// job, so a refresh after a fetch that moved one remote walks only that
/// branch.
#[derive(Debug, Default)]
pub struct AheadBehindCache {
    counts: Mutex<HashMap<(Oid, Oid), AheadBehind>>,
    #[cfg(test)]
    walked_on: Mutex<Vec<std::thread::ThreadId>>,
}

impl AheadBehindCache {
    /// The counts for `(local, upstream)`, walking only when this pair has
    /// not been counted before.
    ///
    /// # Errors
    /// The walk's failure, when it had to walk and could not.
    pub fn get_or_count(
        &self,
        local: Oid,
        upstream: Oid,
        walk: impl FnOnce() -> Result<AheadBehind, String>,
    ) -> Result<AheadBehind, String> {
        if let Some(&known) = self.lock().get(&(local, upstream)) {
            return Ok(known);
        }
        #[cfg(test)]
        self.walked_on
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(std::thread::current().id());
        // Not held across the walk: two jobs counting the same pair at once
        // both walk and agree, which beats one blocking the other.
        let counted = walk()?;
        self.lock().insert((local, upstream), counted);
        Ok(counted)
    }

    /// Every thread a walk ran on, in order (tests: proving it was never the
    /// loop's).
    #[cfg(test)]
    pub(crate) fn walked_on(&self) -> Vec<std::thread::ThreadId> {
        self.walked_on
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(Oid, Oid), AheadBehind>> {
        // A poisoned lock means a walk panicked mid-insert; the map itself is
        // still a valid set of finished counts, so it is safe to keep using.
        self.counts.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
