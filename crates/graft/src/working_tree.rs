//! Reading the working tree's full status — staged, unstaged and untracked,
//! one entry per changed path — for the uncommitted-changes row and the
//! changes pane alike (PROMPT.md §1.3, m3-working-tree-undo-and-stash/04).
//!
//! Read by a job, never by `update`: `graft_git::status::read` walks the
//! index and the worktree, which touches disk. No cache: unlike refs'
//! ahead/behind counts, there is nothing here worth remembering between
//! reads.

use std::sync::Arc;

use graft_git::run::Stop;
use graft_git::state::InProgressDetail;
use graft_git::status::WorkingTreeStatus;

use crate::repos::RepoStore;

/// One read of the working tree: what changed, whether the repository is
/// partway through a merge, rebase, cherry-pick, revert or bisect, and which
/// mergetool the user has configured.
///
/// The three travel together because they are read together and used
/// together: the commit editor's refusal preview (ADR-0034) needs the first
/// two to say whether `[ Commit ]` would do anything, and the operation
/// banner needs all three at once — the kind and step to word itself, and
/// `mergetool` to know whether `M` has anything to launch before it offers
/// the key (`m5-history-and-ghost-preview/09`).
///
/// `in_progress` grew from [`graft_git::state::InProgress`] to
/// [`InProgressDetail`] here: the banner says `REBASING step 3 of 7`, and the
/// step counts are in the same `rebase-merge/` directory the kind is read
/// from, so the detail costs the same walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkingTree {
    /// One entry per changed path. `Arc`: shared with the changes pane's
    /// file list rather than copied per event.
    pub status: Arc<WorkingTreeStatus>,
    /// The operation partway through, if there is one, with its step counts.
    pub in_progress: Option<InProgressDetail>,
    /// `merge.tool` from git's config, when the user has set one. `None`
    /// means `M` has nothing to launch and says so rather than running
    /// `git mergetool` and letting git ask the question on a terminal Graft
    /// has just handed over.
    pub mergetool: Option<String>,
}

/// Reads the working tree's status, the repository's in-progress state and
/// the configured mergetool (a job's work).
///
/// # Errors
/// The failure text, ready to show: the repository, its status, its
/// in-progress markers or its config could not be read.
pub fn read(store: &RepoStore, stop: &dyn Stop) -> Result<WorkingTree, String> {
    let repo = store.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let status = graft_git::status::read(&handle, stop).map_err(|error| error.to_string())?;
    // Not swallowed to `None`: an unreadable git directory is a real failure,
    // and reporting "nothing in progress" would let a commit be offered that
    // git will refuse. An incoming commit git cannot name is not such a
    // failure: it rides on the detail's `theirs`, and the banner (with its
    // `--abort`) still draws.
    let in_progress = graft_git::state::in_progress_detail_with_stop(&handle, stop)
        .map_err(|error| error.to_string())?;
    // Likewise not swallowed. `merge.tool` unset is `Ok(None)` — a fact, and
    // the one `M`'s refusal is worded from. A config that could not be *read*
    // is a different thing, and reporting it as "unset" would tell the user
    // to configure a tool they have already configured.
    let mergetool = graft_git::state::mergetool(&handle).map_err(|error| error.to_string())?;
    Ok(WorkingTree {
        status: Arc::new(status),
        in_progress,
        mergetool,
    })
}

#[cfg(test)]
mod tests {
    use graft_fixtures::Shape;
    use graft_git::state::InProgress;

    use super::*;

    /// A `MERGE_HEAD` naming a commit git cannot read (Minor 2 of
    /// `docs/reviews/m6-amend-10-22.md`). Theirs' name is a label; failing
    /// the whole read on it would hide the banner, and with it `--abort`.
    #[test]
    fn an_unreadable_merge_head_still_reads_the_working_tree() {
        let scratch = graft_fixtures::copy_into_temp(Shape::Linear);
        let git_dir = scratch.path().join(".git");
        assert!(git_dir.is_dir(), "the fixture copy is a plain repository");
        std::fs::write(
            git_dir.join("MERGE_HEAD"),
            "1111111111111111111111111111111111111111\n",
        )
        .expect("written");
        let store = RepoStore::new(scratch.path().to_path_buf(), None);

        let read = read(&store, &()).expect("the working tree reads");
        let detail = read.in_progress.expect("the merge is still seen");
        assert_eq!(detail.kind, InProgress::Merge, "its banner, and --abort");
        let error = detail.theirs.expect_err("carried, not swallowed");
        assert_eq!(error.file, "MERGE_HEAD");
    }
}
