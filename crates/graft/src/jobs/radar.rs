//! The Mistake Radar's context job: the default branch's first-parent chain
//! and its previous reflog tips — the two facts nothing else already reads,
//! and the only ones `view::radar`'s rules need that are not already sitting
//! in `AppState` (refs, branches, `HEAD`).

use std::sync::Arc;

use graft_core::commit_handle::HandleSet;
use graft_core::oid::Oid;
use graft_core::ref_name::RefName;
use graft_git::history::{CommitTable, first_parent_chain, previous_tips};
use graft_git::repo::RepoHandle;
use graft_git::run::Stop;
use graft_ops::mistake_radar::probes::{
    Duplicates, EvilVerdict, ForcePushVerdict, probe_duplicates, probe_evil_merge, probe_force_push,
};

/// How many of the default branch's previous reflog tips are kept: enough
/// for the force-push probe to recognise a recent rewrite, not so many a
/// long-lived branch's whole reflog is read every time.
const PREVIOUS_TIPS_CAP: usize = 20;

/// The default branch's first-parent ancestry and its previous tips.
///
/// The chain is as long as history, so it is held the way ADR-0026 holds
/// everything history-sized: one bit per commit in a [`HandleSet`], keyed by
/// the [`CommitTable`] handle, not a `HashSet<Oid>`. A set of ids cost 44 B
/// per commit on the chain (33-byte `Oid`, hashed, over-allocated) and was
/// 43 of the heavy tier's 76 B/commit heap slope (housekeeping/02); the
/// bitset costs an eighth of a byte. The table the handles belong to is held
/// here too, so [`on_default_chain`](Self::on_default_chain) answers about an
/// `Oid` — the only thing `view::radar` has — and a table rebuilt since
/// cannot make these handles mean something else.
pub struct RadarContext {
    /// Every commit on the default branch's first-parent chain, from its tip
    /// to the root, by handle in [`RadarContext::table`].
    default_chain: HandleSet,
    /// The table `default_chain`'s handles belong to.
    table: Arc<CommitTable>,
    /// The default branch's previous reflog tips, newest first, deduped: a
    /// merge that carries one of these as a non-first parent has pushed it
    /// off the first-parent line (`Evidence::Backward`).
    pub previous_default_tips: Vec<Oid>,
}

impl RadarContext {
    /// Whether `id` is on the default branch's first-parent chain — what
    /// `view::radar` reads as `CommitFacts::on_default_chain`. A commit the
    /// table has never seen is not on it.
    #[must_use]
    pub fn on_default_chain(&self, id: &Oid) -> bool {
        self.table
            .handle_of(id)
            .is_some_and(|handle| self.default_chain.contains(handle))
    }

    /// How many commits are on the chain.
    #[must_use]
    pub fn chain_len(&self) -> usize {
        self.default_chain.len()
    }

    /// A context over the ids given, interned into a table of its own: the
    /// shape [`run_radar_context`] produces, for callers that hold ids rather
    /// than a repository (tests, and `update::context_menu`'s probe).
    #[must_use]
    pub fn of_ids(
        chain: impl IntoIterator<Item = Oid>,
        previous_default_tips: Vec<Oid>,
    ) -> RadarContext {
        let mut default_chain = HandleSet::new();
        let table = CommitTable::without_a_commit_graph(Oid::SHA1_BYTES);
        for id in chain {
            default_chain.insert(table.intern(&id));
        }
        RadarContext {
            default_chain,
            table: Arc::new(table),
            previous_default_tips,
        }
    }
}

/// Two contexts are the same when they say the same thing about the same
/// commits: the same chain in the same table, and the same previous tips.
/// Handles only mean anything against their own table, so the tables are
/// compared by identity.
impl PartialEq for RadarContext {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.table, &other.table)
            && self.default_chain == other.default_chain
            && self.previous_default_tips == other.previous_default_tips
    }
}

impl Eq for RadarContext {}

/// The chain by size, not by member: a hundred thousand handles are not a
/// debug line. [`CommitTable`] has no `Debug` of its own and needs none here.
impl std::fmt::Debug for RadarContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RadarContext")
            .field(
                "default_chain",
                &format_args!("{} commits", self.chain_len()),
            )
            .field("previous_default_tips", &self.previous_default_tips)
            // `table` is left out on purpose: it is the repository's whole id
            // table, shared by everything, and has no `Debug` of its own.
            .finish_non_exhaustive()
    }
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
    let mut default_chain = HandleSet::new();
    for handle in chain {
        default_chain.insert(handle);
    }
    let previous_default_tips =
        previous_tips(repo, default_ref, PREVIOUS_TIPS_CAP).map_err(|error| error.to_string())?;
    Ok(Arc::new(RadarContext {
        default_chain,
        table: Arc::clone(table),
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

/// The work behind
/// [`Command::DuplicatesProbe`](crate::update::Command::DuplicatesProbe).
///
/// # Errors
/// Any of the probe's three `git` spawns failing, joined to one line.
pub fn run_duplicates_probe(
    repo: &RepoHandle,
    window: u32,
    stop: &dyn Stop,
) -> Result<Arc<Duplicates>, String> {
    probe_duplicates(repo, window, stop)
        .map(Arc::new)
        .map_err(|error| error.to_string())
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
        assert!(context.on_default_chain(&default_tip));
        assert_eq!(context.previous_default_tips, Vec::new());
    }
}
