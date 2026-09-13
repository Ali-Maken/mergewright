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
use graft_core::commit_handle::{CommitHandle, Link};
use graft_core::oid::Oid;

use crate::layout::source::{CommitSource, Pull};
use crate::layout::stream::{Invalidated, StreamError, StreamingLayout};
use crate::seeds::Seeds;

#[cfg(test)]
mod tests;

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
        .filter_map(|commit| frontier.decide(commit, follow, inclusion))
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

    /// Decides one commit, in walk order: the commit as drawn, or `None`.
    fn decide(&mut self, commit: &Commit, follow: Follow, inclusion: &Inclusion) -> Option<Commit> {
        let wanted = self.shown.remove(&commit.id).is_some();
        let pointed_at_hidden = self.hidden.remove(&commit.id).is_some();
        let shown = wanted
            || match follow {
                Follow::FirstParent => !std::mem::replace(&mut self.chain_started, true),
                Follow::AllParents => {
                    !pointed_at_hidden && !inclusion.hidden_stashes.contains(&commit.id)
                }
            };
        if !shown {
            for &parent in &commit.parents {
                *self.hidden.entry(parent).or_default() += 1;
            }
            return None;
        }
        let drawn = drawn_as(commit.clone(), follow, inclusion);
        for &parent in &commit.parents {
            let kept = drawn.parents.contains(&parent);
            let side = if kept {
                &mut self.shown
            } else {
                &mut self.hidden
            };
            *side.entry(parent).or_default() += 1;
        }
        Some(drawn)
    }
}

/// A shown commit as the view draws it: first parent only in first-parent
/// view or when folded. A pure function of the commit and the view, so a
/// re-read by id draws it the same as the walk did.
fn drawn_as(mut commit: Commit, follow: Follow, inclusion: &Inclusion) -> Commit {
    if follow == Follow::FirstParent || inclusion.folded.contains(&commit.id) {
        commit.parents.truncate(1);
    }
    commit
}

/// How many commits a rebuild of the frontier re-reads at once.
const REPLAY_BATCH: usize = 1_024;

/// A [`CommitSource`] that lets through only what a view shows.
///
/// Remembers the id of every commit the underlying walk has served, in walk
/// order — the one cost that grows with history, like
/// [`EmittedOrder`](crate::layout::EmittedOrder) — so a change of inclusion
/// can re-decide from any point by re-reading, never by walking again.
#[derive(Debug)]
pub struct Included<S> {
    source: S,
    follow: Follow,
    seeds: Seeds,
    inclusion: Inclusion,
    frontier: Frontier,
    /// Every commit the walk has served, in walk order.
    walked: Vec<Oid>,
    /// Whether each commit decided so far was shown, in walk order: a
    /// prefix of `walked`.
    shown: Vec<bool>,
    walk_done: bool,
}

impl<S: CommitSource> Included<S> {
    /// A source showing what `follow` and `inclusion` let through of
    /// `source`'s walk. Lay it out from [`seeds`](Self::seeds), not the
    /// seeds passed in.
    #[must_use]
    pub fn new(source: S, seeds: Seeds, follow: Follow, inclusion: Inclusion) -> Self {
        let seeds = seeds_for(follow, seeds);
        Self {
            frontier: Frontier::new(&seeds),
            source,
            follow,
            seeds,
            inclusion,
            walked: Vec::new(),
            shown: Vec::new(),
            walk_done: false,
        }
    }

    /// The seeds this view lays out from.
    #[must_use]
    pub fn seeds(&self) -> &Seeds {
        &self.seeds
    }

    /// Which parents this view follows.
    #[must_use]
    pub fn follow(&self) -> Follow {
        self.follow
    }

    /// What this view hides now.
    #[must_use]
    pub fn inclusion(&self) -> &Inclusion {
        &self.inclusion
    }

    /// The source underneath.
    #[must_use]
    pub fn inner(&self) -> &S {
        &self.source
    }

    /// Switches to `inclusion`, returning the first row whose drawing can
    /// change — `None` if the change names no commit decided so far, in
    /// which case nothing already served differs. Afterwards the walk is
    /// served again from that commit on, decided under `inclusion`.
    ///
    /// # Errors
    ///
    /// If re-reading the commits before the change fails.
    pub fn reinclude(&mut self, inclusion: Inclusion) -> Result<Option<usize>, S::Error> {
        let changed = self.inclusion.changed_from(&inclusion);
        self.inclusion = inclusion;
        let decided = self.shown.len();
        let Some(position) = self.walked[..decided]
            .iter()
            .position(|id| changed.contains(id))
        else {
            return Ok(None);
        };
        let row = self.shown[..position]
            .iter()
            .filter(|&&shown| shown)
            .count();

        // Everything before `position` decides as it did (none of it is
        // named by the change), so replaying it rebuilds the frontier the
        // new inclusion would have had there.
        self.frontier = Frontier::new(&self.seeds);
        self.shown.truncate(position);
        let mut start = 0;
        while start < position {
            let end = (start + REPLAY_BATCH).min(position);
            let commits = self.source.read(&self.walked[start..end])?;
            for commit in &commits {
                // Decided only for the edges it leaves in the frontier.
                let _drawn = self.frontier.decide(commit, self.follow, &self.inclusion);
            }
            start = end;
        }
        Ok(Some(row))
    }
}

impl<S: CommitSource> CommitSource for Included<S> {
    type Error = S::Error;

    fn next(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        let mut commits = Vec::new();
        loop {
            let decided = self.shown.len();
            let done = self.walk_done && decided == self.walked.len();
            if limit == 0 || done {
                return Ok(Pull {
                    commits,
                    done,
                    stopped: false,
                });
            }
            // Walked before a `reinclude` (or past an earlier limit) but not
            // decided under the inclusion in force: re-read, never re-walked.
            let (batch, stopped) = if decided < self.walked.len() {
                let end = (decided + limit).min(self.walked.len());
                (self.source.read(&self.walked[decided..end])?, false)
            } else {
                let pull = self.source.next(limit, stop)?;
                self.walked
                    .extend(pull.commits.iter().map(|commit| commit.id));
                self.walk_done = pull.done;
                (pull.commits, pull.stopped)
            };
            for commit in &batch {
                if commits.len() == limit {
                    break;
                }
                let drawn = self.frontier.decide(commit, self.follow, &self.inclusion);
                self.shown.push(drawn.is_some());
                commits.extend(drawn);
            }
            let done = self.walk_done && self.shown.len() == self.walked.len();
            if !commits.is_empty() || done || stopped {
                return Ok(Pull {
                    commits,
                    done,
                    stopped,
                });
            }
        }
    }

    fn read(&mut self, ids: &[Oid]) -> Result<Vec<Commit>, Self::Error> {
        let commits = self.source.read(ids)?;
        Ok(commits
            .into_iter()
            .map(|commit| drawn_as(commit, self.follow, &self.inclusion))
            .collect())
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Self::Error> {
        let mut links = self.source.links(commits)?;
        for link in &mut links {
            // As `drawn_as`, on handles.
            if self.follow == Follow::FirstParent
                || self
                    .inclusion
                    .folded
                    .contains(&self.source.id_of(link.commit))
            {
                link.parents = link.parents.first().into_iter().collect();
            }
        }
        Ok(links)
    }

    fn handle_of(&self, id: &Oid) -> Option<CommitHandle> {
        self.source.handle_of(id)
    }

    fn id_of(&self, handle: CommitHandle) -> Oid {
        self.source.id_of(handle)
    }
}

/// Switches `layout`'s view to `inclusion` and throws away only the rows
/// that can change: from the first commit the change names, down. The
/// caller advances the layout again to lay them out.
///
/// # Errors
///
/// If re-reading history to replay the kept state fails.
pub fn reinclude<S: CommitSource>(
    layout: &mut StreamingLayout<Included<S>>,
    inclusion: Inclusion,
) -> Result<Invalidated, StreamError<S::Error>> {
    match layout
        .source_mut()
        .reinclude(inclusion)
        .map_err(StreamError::Source)?
    {
        Some(row) => layout.truncate(row),
        None => layout.truncate(usize::MAX),
    }
}
