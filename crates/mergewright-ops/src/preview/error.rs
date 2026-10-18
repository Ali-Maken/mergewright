//! Why a preview could not be drawn.

use super::{FactsError, Refusal, RunError};

/// Why a preview could not be drawn.
#[derive(Debug, thiserror::Error)]
pub enum PreviewError {
    /// The repository could not be read.
    #[error("{0}")]
    Facts(FactsError),
    /// A `git` read failed.
    #[error("{0}")]
    Read(RunError),
    /// The operation would be refused, and there is no point drawing it: a
    /// `--ff-only` merge of diverged branches, a re-push of a commit that
    /// is not on the branch.
    #[error("{}", .0.explain(&crate::keys::NoKeys))]
    Refused(Refusal),
    /// The caller asked to stop.
    #[error("stopped")]
    Stopped,
}

impl PreviewError {
    /// One line for the box, its keys spelled through `keys` (an unbound
    /// one's clause left out). The `Display` is these words with
    /// [`NoKeys`](crate::keys::NoKeys).
    #[must_use]
    pub fn explain(&self, keys: &dyn crate::keys::KeyNames) -> String {
        match self {
            PreviewError::Refused(refusal) => refusal.explain(keys),
            other => other.to_string(),
        }
    }
}

impl From<RunError> for PreviewError {
    fn from(why: RunError) -> Self {
        match why {
            RunError::Stopped => PreviewError::Stopped,
            other => PreviewError::Read(other),
        }
    }
}
