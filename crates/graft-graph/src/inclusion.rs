//! Which commits the graph draws: first-parent view, hidden stashes and
//! folded merges, as one inclusion predicate over what the walk emits
//! (DESIGN.md §3.6, last bullet).
//!
//! The layout itself never changes. An [`Included`] source sits between the
//! walk and [`StreamingLayout`], lets through only the commits the view
//! shows, and trims each shown commit's parents to the ones that will be
//! shown too — so every edge the layout draws still reaches its target and
//! the PROMPT.md §1.2 invariants hold unchanged.
//!
//! The decision is made in walk order with a frontier, not a reachability
//! walk: a commit is shown when a shown commit (or a seed) points at it
//! through an edge the view keeps, hidden when only hidden commits or
//! dropped edges point at it, and — outside first-parent view — shown when
//! nothing points at it at all (a fresh tip), unless it is a hidden stash.
//! Folding merge `M` drops `M`'s edges to its second and later parents, so
//! the merged branch's own commits are hidden until they reach a commit a
//! shown lane is still waiting for: the fork point, where they rejoin.
//!
//! Changing the inclusion ([`Included::reinclude`]) alters decisions only
//! from the first commit the change names, so [`reinclude`] keeps every row
//! above that commit and lays out again only from it down.

use std::collections::{BTreeSet, HashMap};

use graft_core::commit::Commit;
use graft_core::commit_handle::Link;
use graft_core::oid::Oid;

use crate::seeds::Seeds;

mod included;
#[cfg(test)]
mod tests;

pub use included::{Included, reinclude};

/// Which parents a view follows. Fixed for the life of a layout: it changes
/// the seeds, and so every row from the first — a different view, laid out
/// on its own, never a truncation of this one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Follow {
    /// Every parent: the full graph.
    #[default]
    AllParents,
    /// Only `parents[0]`, from the default branch's tip (or, with no default
    /// branch, the first commit walked): one straight lane.
    FirstParent,
}

/// What a view hides, beyond what [`Follow`] decides. Can change while a
/// layout lives ([`Included::reinclude`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Inclusion {
    /// Stash commits not drawn. A stash a shown commit points at is still
    /// drawn — which a stash never is, outside a hand-built history.
    pub hidden_stashes: BTreeSet<Oid>,
    /// Merges drawn folded: their second and later parents' branches are
    /// collapsed into them.
    pub folded: BTreeSet<Oid>,
}

impl Inclusion {
    /// The commits whose drawing differs between the two inclusions.
    fn changed_from(&self, other: &Inclusion) -> BTreeSet<Oid> {
        let stashes = self
            .hidden_stashes
            .symmetric_difference(&other.hidden_stashes);
        let folds = self.folded.symmetric_difference(&other.folded);
        stashes.chain(folds).copied().collect()
    }
}

/// The seeds a view lays out from: first-parent view has only lane 0.
#[must_use]
pub fn seeds_for(follow: Follow, seeds: Seeds) -> Seeds {
    match follow {
        Follow::AllParents => seeds,
        Follow::FirstParent => Seeds {
            integration_branches: Vec::new(),
            ..seeds
        },
    }
}

/// Filters a whole walk at once: the commits a view shows, parents trimmed,
/// and the seeds to lay them out from. The slice form of [`Included`].
#[must_use]
pub fn apply(
    commits: &[Commit],
    seeds: Seeds,
    follow: Follow,
    inclusion: &Inclusion,
) -> (Vec<Commit>, Seeds) {
    let seeds = seeds_for(follow, seeds);
    let mut frontier = Frontier::new(&seeds);
    let shown = commits
        .iter()
        .filter_map(|commit| {
            let kept = frontier.decide(commit.id, &commit.parents, follow, inclusion)?;
            let mut drawn = commit.clone();
            drawn.parents = kept;
            Some(drawn)
        })
        .collect();
    (shown, seeds)
}

/// The edges in flight while deciding: how many shown and hidden commits
/// point at each commit still to come. Counts, not sets, so a commit two
/// shown children point at stays wanted when one of them changes.
#[derive(Clone, Debug, Default)]
struct Frontier {
    shown: HashMap<Oid, u32>,
    hidden: HashMap<Oid, u32>,
    /// First-parent view with no default branch: whether the chain has
    /// started at the first commit walked.
    chain_started: bool,
}

impl Frontier {
    fn new(seeds: &Seeds) -> Self {
        let mut frontier = Frontier {
            chain_started: seeds.default_branch.is_some(),
            ..Frontier::default()
        };
        let tips = seeds
            .default_branch
            .iter()
            .chain(&seeds.integration_branches);
        for seed in tips {
            *frontier.shown.entry(seed.tip).or_default() += 1;
        }
        frontier
    }

    /// Decides one commit, in walk order, by id and parents alone: the
    /// parents it keeps if shown, `None` if hidden. Takes `id`/`parents`
    /// rather than a whole [`Commit`] so [`Included::reinclude`] can replay
    /// through [`CommitSource::links`] without re-reading commit bodies.
    fn decide(
        &mut self,
        id: Oid,
        parents: &[Oid],
        follow: Follow,
        inclusion: &Inclusion,
    ) -> Option<Vec<Oid>> {
        let wanted = self.shown.remove(&id).is_some();
        let pointed_at_hidden = self.hidden.remove(&id).is_some();
        let shown = wanted
            || match follow {
                Follow::FirstParent => !std::mem::replace(&mut self.chain_started, true),
                Follow::AllParents => !pointed_at_hidden && !inclusion.hidden_stashes.contains(&id),
            };
        if !shown {
            for &parent in parents {
                *self.hidden.entry(parent).or_default() += 1;
            }
            return None;
        }
        let kept = kept_parents(id, parents, follow, inclusion);
        for &parent in parents {
            let side = if kept.contains(&parent) {
                &mut self.shown
            } else {
                &mut self.hidden
            };
            *side.entry(parent).or_default() += 1;
        }
        Some(kept)
    }
}

/// Which of a commit's parents survive when it is drawn: first parent only
/// in first-parent view or when folded, every parent otherwise. A pure
/// function of the id and its parents, so a re-read draws it the same as the
/// walk did.
fn kept_parents(id: Oid, parents: &[Oid], follow: Follow, inclusion: &Inclusion) -> Vec<Oid> {
    if follow == Follow::FirstParent || inclusion.folded.contains(&id) {
        parents.first().copied().into_iter().collect()
    } else {
        parents.to_vec()
    }
}

/// A shown commit's link as the view draws it: [`kept_parents`], on
/// handles. `id` is the link's commit id.
fn drawn_link(mut link: Link, id: Oid, follow: Follow, inclusion: &Inclusion) -> Link {
    if follow == Follow::FirstParent || inclusion.folded.contains(&id) {
        link.parents = link.parents.first().into_iter().collect();
    }
    link
}

/// How many commits a rebuild of the frontier re-reads at once.
const REPLAY_BATCH: usize = 1_024;
