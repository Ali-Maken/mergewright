//! The merge review's reads (m6-conflicts/19, ease-pass/12b): what the
//! merge about to be committed changes, and the read-only look `⏎` opens
//! ([`look`]).
//!
//! All run here, off the loop's thread; `update` receives one finished `Arc`.

use std::sync::Arc;

use mergewright_ops::finish::SummaryError;

use crate::repos::RepoStore;
use crate::update::finish::{ReviewFailed, Reviewed};

/// The merge review (ease-pass/05b): what the merge in progress will
/// commit and the files solved on disk that its `y` stages, read with
/// `mergewright_ops::finish::merge_review` — nothing is staged by reading.
/// Files with a clash still open come back as
/// [`ReviewFailed::StillOpen`] so the app can say how many, files and
/// clashes; any other failure in `mergewright_ops`' own words.
pub(super) fn summary(repos: &RepoStore) -> Result<Reviewed, ReviewFailed> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    match mergewright_ops::finish::merge_review(&handle, &()) {
        Ok(review) => Ok(Reviewed {
            summary: Arc::new(review.summary),
            solved: review.solved,
            // A failed count is not a failed review: the head line draws
            // `? commits from …`, as a folded merge's unknown count does
            // (ease-pass 06). The review is still right without it.
            incoming: incoming_count(&handle).ok(),
        }),
        Err(SummaryError::ClashesOpen { files, clashes }) => {
            Err(ReviewFailed::StillOpen { files, clashes })
        }
        Err(error) => Err(ReviewFailed::Failed(error.to_string())),
    }
}

/// The commits the merge brings in: `HEAD..MERGE_HEAD`, the review's
/// `N commits from <branch>` (ease-pass/12), counted with the read a folded
/// merge's tail uses (`count_between`).
///
/// # Errors
/// `HEAD` or `MERGE_HEAD` not reading, or `git` failing, as one line.
fn incoming_count(handle: &mergewright_git::repo::RepoHandle) -> Result<u32, String> {
    let head = handle
        .head_commit()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "HEAD names no commit".to_owned())?;
    let detail = mergewright_git::state::in_progress_detail(handle)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "no merge is in progress".to_owned())?;
    let theirs = detail
        .theirs
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "MERGE_HEAD names no commit".to_owned())?;
    let count = mergewright_git::history_reads::count_between(handle, head, theirs.id, &())
        .map_err(|error| error.to_string())?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

pub mod look;

#[cfg(test)]
mod tests;
