//! The Mistake Radar's context job: the default branch's first-parent chain
//! and its previous reflog tips — the two facts nothing else already reads,
//! and the only ones `view::radar`'s rules need that are not already sitting
//! in `AppState` (refs, branches, `HEAD`).

use std::collections::HashSet;
use std::sync::Arc;

use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::{CommitTable, first_parent_chain, previous_tips};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_ops::mistake_radar::probes::{
    EvilVerdict, ForcePushVerdict, probe_evil_merge, probe_force_push,
};

/// How many of the default branch's previous reflog tips are kept: enough
/// for the force-push probe to recognise a recent rewrite, not so many a
/// long-lived branch's whole reflog is read every time.
const PREVIOUS_TIPS_CAP: usize = 20;

/// The default branch's first-parent ancestry and its previous tips.
#[derive(Debug, PartialEq, Eq)]
pub struct RadarContext {
    /// Every commit on the default branch's first-parent chain, from its tip
    /// to the root. `view::radar::findings_for` uses membership in this set
    /// as `CommitFacts::on_default_chain`.
    pub default_chain: HashSet<Oid>,
    /// The default branch's previous reflog tips, newest first, deduped: a
    /// merge that carries one of these as a non-first parent has pushed it
    /// off the first-parent line (`Evidence::Backward`).
    pub previous_default_tips: Vec<Oid>,
}

/// The work behind [`Command::RadarContext`](crate::update::Command::RadarContext).
///
/// # Errors
/// Either `graft-git` read's failure, joined to one line.
pub fn run_radar_context(
    repo: &RepoHandle,
    table: &Arc<CommitTable>,
    default_tip: Oid,
    default_ref: &RefName,
    stop: &dyn Stop,
) -> Result<Arc<RadarContext>, String> {
    let chain =
        first_parent_chain(repo, table, default_tip, stop).map_err(|error| error.to_string())?;
    let default_chain = chain
        .into_iter()
        .map(|handle| table.id_of(handle))
        .collect();
    let previous_default_tips =
        previous_tips(repo, default_ref, PREVIOUS_TIPS_CAP).map_err(|error| error.to_string())?;
    Ok(Arc::new(RadarContext {
        default_chain,
        previous_default_tips,
    }))
}

/// The work behind [`Command::EvilProbe`](crate::update::Command::EvilProbe).
///
/// Never a `Result`: `EvilVerdict::NotCheckable` already carries every
/// reason the check could not run (`graft_ops::mistake_radar::probes`).
pub fn run_evil_probe(repo: &RepoHandle, merge: Oid, stop: &dyn Stop) -> EvilVerdict {
    probe_evil_merge(repo, merge, stop)
}

/// The work behind
/// [`Command::ForcePushProbe`](crate::update::Command::ForcePushProbe).
///
/// Resolves the remote-tracking ref here rather than in `update`: which ref
/// a branch pulls from is config, and `update` reads nothing from disk. A
/// branch whose upstream has gone (or was never there) is `NotCheckable`,
/// not an error — the same rule the verdict follows everywhere else.
pub fn run_force_push_probe(
    repo: &RepoHandle,
    branch: &str,
    current: Oid,
    stop: &dyn Stop,
) -> ForcePushVerdict {
    let local = match RefName::new(format!("refs/heads/{branch}").into_bytes()) {
        Ok(name) => name,
        Err(error) => return ForcePushVerdict::NotCheckable(error.to_string()),
    };
    match repo.upstream(&local) {
        Ok(Some(remote_ref)) => probe_force_push(repo, branch, &remote_ref, current, stop),
        Ok(None) => ForcePushVerdict::NotCheckable("no upstream".into()),
        Err(error) => ForcePushVerdict::NotCheckable(error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use graft_fixtures::Shape;

    use super::*;
    use crate::repos::RepoStore;

    struct NeverStop;
    impl Stop for NeverStop {
        fn should_stop(&self) -> bool {
            false
        }
    }

    fn store(shape: Shape) -> RepoStore {
        RepoStore::new(graft_fixtures::borrow(shape), None)
    }

    #[test]
    fn the_default_chain_holds_the_first_parent_line_and_a_fresh_branch_has_no_previous_tips() {
        let store = store(Shape::FoxtrotMerge);
        let repo = store.shared().expect("the fixture opens");
        let handle = repo.handle();
        let table = store.table().expect("the table opens");
        let branches = store.branches().expect("branches resolve");
        let default_ref = branches
            .default_branch()
            .expect("the fixture has a default branch")
            .clone();
        let refs = store.refs().expect("refs read");
        let default_tip = refs
            .peeled
            .iter()
            .find(|(name, _)| *name == default_ref)
            .map(|&(_, id)| id)
            .expect("the default branch has a tip");

        let context = run_radar_context(&handle, &table, default_tip, &default_ref, &NeverStop)
            .expect("the context reads");
        assert!(context.default_chain.contains(&default_tip));
        assert_eq!(context.previous_default_tips, Vec::new());
    }
}
