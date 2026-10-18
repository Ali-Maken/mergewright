//! [`CommitGraphOffer`]: where the offer to build a missing commit-graph
//! file is — split out of `events` (ease-pass/08s M6).

use graft_core::job::JobId;

/// Where the offer to build a missing commit-graph file is (m1-the-graph/54).
///
/// The job ids are `None` from the key press until the runner announces the
/// job: `B` and `X` must leave `Offered` at once, so a second press does not
/// start a second job, and the id is not known until `Started`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CommitGraphOffer {
    /// Nothing to offer: a commit-graph is there, the user declined before,
    /// the facts are not in yet, or a written line was cleared.
    #[default]
    NotNeeded,
    /// No commit-graph and no decline: `B` builds, `X` declines. The line
    /// says there is no git to build with when the git check did not pass.
    Offered,
    /// `git commit-graph write` is running as this job.
    Writing(Option<JobId>),
    /// The write finished; cleared by the next key.
    Written {
        /// Commits the new file holds.
        commits: u32,
    },
    /// The decline is being written as this job.
    Declining(Option<JobId>),
    /// Declined and remembered; no line.
    Declined,
    /// The write failed; the problem line says why. No retry this run.
    Failed,
    /// A harness asked for no offer (`Harness::offer_commit_graph`).
    Suppressed,
}
