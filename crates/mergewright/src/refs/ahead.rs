//! What a local branch has to push, read off its label (ease-pass/07a).

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
