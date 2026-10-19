//! The read behind the branch pane (ease-pass 09): one branch's four answers
//! and its route, from `ops::branch_status`. The branch's upstream is read
//! here, not held app-side: the app clears its reads whenever the refs are
//! read again, which is when an upstream can have moved.

use mergewright_git::remote::upstream_of;
use mergewright_ops::branch_status::{BranchStatus, Candidate, branch_status};
use mergewright_ops::operation::Tip;

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::branch_pane::StatusAsk;

/// The work behind `RestRead::BranchStatus`.
///
/// The branch's own upstream is never a candidate base (01 handoff "For
/// B"): a local branch can track another local branch.
///
/// # Errors
/// The repository not opening, its config not reading, or a git read
/// failing (stopped included), as one line.
pub(crate) fn read_branch_status(
    repos: &RepoStore,
    ask: &StatusAsk,
    cancel: &Cancel,
) -> Result<BranchStatus, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let tracked = upstream_of(&handle, &ask.branch.name).map_err(|error| error.to_string())?;
    // An upstream never fetched has no tip to count against: it reads as
    // no upstream, `not pushed yet`, which is what it is to the remote.
    let upstream = tracked.as_ref().and_then(|tracked| {
        tracked.tracking_tip.map(|oid| Tip {
            name: tracked.tracking_ref.clone(),
            oid,
        })
    });
    let candidates: Vec<Candidate> = ask
        .candidates
        .iter()
        .filter(|candidate| {
            tracked
                .as_ref()
                .is_none_or(|tracked| tracked.tracking_ref != candidate.name)
        })
        .cloned()
        .collect();
    branch_status(&handle, &ask.branch, &candidates, upstream.as_ref(), cancel)
        .map_err(|error| error.to_string())
}
