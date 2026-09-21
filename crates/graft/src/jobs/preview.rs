//! The reading half of Ghost Preview (m5-history-and-ghost-preview/08).
//!
//! `update` decides *what* to preview and hands it over as
//! [`Command::PlanPreview`](crate::update::Command::PlanPreview); this module
//! does the reading. Two things happen here and nowhere else:
//!
//! 1. [`graft_ops::preview::plan`] draws the picture — ghosts, ref moves,
//!    the working-tree effect, the conflict forecast.
//! 2. The commits the picture points at that are **not already on screen**
//!    are read, so the renderer can lay out a bounded slice without walking
//!    history again (design note hard choice 1: a preview never loads more
//!    history mid-preview).
//!
//! Step 2 is capped at [`PREVIEW_ANCHOR_CAP`]. Past the cap the answer is an
//! error the box prints, not a silently shorter picture: a preview that
//! quietly drops the part the user is looking for is worse than one that
//! says it cannot draw it.

use std::collections::HashSet;
use std::sync::Arc;

use graft_core::commit::Commit;
use graft_core::oid::Oid;
use graft_ops::preview::{PreviewPlan, Target};

use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_git::version::system_git;

use super::Cancel;
use crate::repos::RepoStore;
use crate::update::preview::{PREVIEW_ANCHOR_CAP, Planned, PreviewRequest, PreviewSubject};

/// Draws one preview, and reads the commits it needs to be drawable.
///
/// The `Err` is the line the preview box prints, so it is written for the
/// user, not for a log.
pub(super) fn run_preview(
    repos: &RepoStore,
    request: &PreviewRequest,
    clock: crate::session::Clock,
    cancel: &Cancel,
) -> Result<Arc<Planned>, String> {
    let repo = repos.shared().map_err(|error| error.to_string())?;
    let handle = repo.handle();
    let git = system_git().map_err(|error| error.to_string())?.version();

    let plan = match &request.subject {
        PreviewSubject::Operation { op, .. } => {
            let scope = op.facts_scope();
            let facts = graft_ops::facts::read_facts(&handle, git, cancel, &scope)
                .map_err(|why| why.to_string())?;
            graft_ops::preview::plan(&handle, &git, op, &facts, cancel)
                .map_err(|why| why.explain())?
        }
        PreviewSubject::RevertTo(entry) => {
            let entries = super::undo::journal_entries(repos, clock, cancel)?;
            graft_ops::preview::plan_revert_to(&entries, entry).map_err(|why| why.explain())?
        }
    };

    let anchors = read_anchors(&handle, &plan, &request.resident, cancel)?;
    Ok(Arc::new(Planned { plan, anchors }))
}

/// Every real commit the picture points at, in the order it was found.
///
/// A ghost's parents anchor it in the slice; a ref that moves has to be
/// drawn where it came from and where it lands; `HEAD` detaching onto a real
/// commit has to have somewhere to land. Anything already resident is
/// skipped — it is on screen, and re-reading it would only make a second
/// copy of the same row.
fn wanted(plan: &PreviewPlan, resident: &HashSet<Oid>) -> Vec<Oid> {
    let mut seen: HashSet<Oid> = resident.clone();
    let mut out = Vec::new();
    let mut want = |oid: Oid, out: &mut Vec<Oid>| {
        if seen.insert(oid) {
            out.push(oid);
        }
    };
    for ghost in &plan.ghosts {
        for parent in &ghost.parents {
            if let Target::Real(oid) = parent {
                want(*oid, &mut out);
            }
        }
    }
    for moved in &plan.ref_moves {
        if let Some(from) = moved.from {
            want(from, &mut out);
        }
        if let Some(Target::Real(oid)) = moved.to {
            want(oid, &mut out);
        }
    }
    if let Some(graft_ops::preview::HeadMove::Detached(Target::Real(oid))) = plan.head {
        want(oid, &mut out);
    }
    out
}

/// Reads [`wanted`]'s commits, refusing rather than truncating past the cap.
fn read_anchors(
    handle: &RepoHandle,
    plan: &PreviewPlan,
    resident: &HashSet<Oid>,
    cancel: &Cancel,
) -> Result<Vec<Commit>, String> {
    let wanted = wanted(plan, resident);
    if wanted.len() > PREVIEW_ANCHOR_CAP {
        return Err(format!(
            "this one reaches {} commits outside the view — more than a preview reads ({PREVIEW_ANCHOR_CAP})",
            wanted.len()
        ));
    }
    let mut anchors = Vec::with_capacity(wanted.len());
    for oid in wanted {
        if cancel.should_stop() {
            return Err("stopped".to_owned());
        }
        if let Some(commit) = keep(graft_git::history::read_commit(handle, oid))? {
            anchors.push(commit);
        }
    }
    Ok(anchors)
}

/// What one anchor read means for the picture: a row, nothing, or a refusal.
///
/// **Nothing** is only ever the one safe case: a ref pointing at an object
/// this repository no longer has (a pruned remote-tracking tip). The slice
/// refuses later, **by name**, if the missing one was holding a ghost up —
/// which is a sentence the user can act on, where "failed to read 7f3a…" is
/// not.
///
/// Every other failure — a corrupt object, an object database that will not
/// open — is a refusal. Dropping one would draw a picture with a piece
/// silently missing and no sign that it is missing, which is the one thing a
/// preview must not do.
fn keep(read: Result<Commit, graft_git::history::WalkError>) -> Result<Option<Commit>, String> {
    use graft_git::history::{NodeError, WalkError};
    match read {
        Ok(commit) => Ok(Some(commit)),
        Err(WalkError::Commit(NodeError::Missing { .. })) => Ok(None),
        Err(other) => Err(format!("cannot draw this one: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::keep;
    use graft_core::oid::Oid;
    use graft_git::history::{NodeError, WalkError};

    /// An id to hang an error on. Which one it is does not matter here.
    fn some_oid() -> Oid {
        Oid::from_sha1([0x7f; 20])
    }

    #[test]
    fn a_pruned_tip_is_left_out_of_the_picture_rather_than_refusing_it() {
        let missing = Err(WalkError::Commit(NodeError::Missing { id: some_oid() }));
        assert_eq!(keep(missing), Ok(None));
    }

    #[test]
    fn a_commit_that_is_there_but_will_not_read_refuses_the_whole_picture() {
        let unreadable = Err(WalkError::Commit(NodeError::Unreadable {
            id: some_oid(),
            source: Box::new(std::io::Error::other("the object database is on fire")),
        }));
        let refused = keep(unreadable).expect_err("an unreadable object is not a missing one");
        assert!(
            refused.contains("the object database is on fire"),
            "the box has to say what went wrong, got {refused:?}"
        );
    }
}
