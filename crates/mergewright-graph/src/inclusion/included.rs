//! [`Included`]: the streaming [`CommitSource`] wrapper that applies
//! an [`Inclusion`] to a walk, plus [`reinclude`] to change it in place and
//! [`repair`] to mend what folding by default got wrong in walk order.

use std::collections::BTreeSet;

use mergewright_core::commit::Commit;
use mergewright_core::commit_handle::{CommitHandle, HandleSet, Link};
use mergewright_core::oid::Oid;

use crate::layout::source::{CommitSource, Pull};
use crate::layout::stream::{Invalidated, StreamError, StreamingLayout};
use crate::seeds::Seeds;

use super::{Admit, FoldWalk, Follow, Frontier, Inclusion, Mark, drawn_link, seeds_for};

mod redecide;
mod whereabouts;

pub use redecide::{advance_folded, repair};
pub use whereabouts::Whereabouts;

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
    /// The fold, when the view folds merged branches by default (never in
    /// first-parent view, where nothing off the first parent is drawn).
    fold: Option<FoldWalk>,
    /// Marks the fold has not been given yet: their commits have no handle
    /// until the walk names them.
    unmarked: Vec<(Oid, Mark)>,
    /// Commits this walk's provisional order hid too early
    /// ([`FoldWalk::take_late`]), shown from now on. A property of the walk,
    /// not of the view: kept across [`Included::reinclude`], and a promoted
    /// order starts a new `Included` without them.
    forced: BTreeSet<Oid>,
    /// Every commit the walk has served, in walk order. Handle order need
    /// not track walk order (a parent can be interned by `links` before it
    /// is itself walked), so this stays a sequence — 4 bytes/commit, not
    /// history-sized enough to be the thing ADR-0026 is after.
    sequence: Vec<CommitHandle>,
    /// Which of `sequence`'s handles the walk has served — same content as
    /// a `HandleSet` built from `sequence`, kept as its own bitset so
    /// [`Included::walked`] is O(1) and membership checks don't scan.
    walked: HandleSet,
    /// Which walked handles were decided shown.
    shown: HandleSet,
    /// How many of `sequence`'s handles have been decided (shown or not) —
    /// a prefix count `shown`'s cardinality alone can't give, since it only
    /// counts the `true` decisions.
    decided: usize,
    /// How many of the decided handles have been served since a rejoin
    /// rewound (`<= decided`): the rest are served again as decided.
    served: usize,
    walk_done: bool,
}

impl<S: CommitSource> Included<S> {
    /// A source showing what `follow` and `inclusion` let through of
    /// `source`'s walk. Lay it out from [`seeds`](Self::seeds), not the
    /// seeds passed in.
    #[must_use]
    pub fn new(source: S, seeds: Seeds, follow: Follow, inclusion: Inclusion) -> Self {
        let seeds = seeds_for(follow, seeds);
        let mut included = Self {
            frontier: Frontier::new(&seeds),
            source,
            follow,
            seeds,
            inclusion,
            fold: None,
            unmarked: Vec::new(),
            forced: BTreeSet::new(),
            sequence: Vec::new(),
            walked: HandleSet::new(),
            shown: HandleSet::new(),
            decided: 0,
            served: 0,
            walk_done: false,
        };
        included.restart_fold();
        included
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

    /// Every commit the walk has served, whether shown or hidden.
    #[must_use]
    pub fn walked(&self) -> &HandleSet {
        &self.walked
    }

    /// The walked commits this view currently draws.
    #[must_use]
    pub fn shown(&self) -> &HandleSet {
        &self.shown
    }

    /// The fold deciding this view, when merged branches fold by default:
    /// which merges fold, which opened merge a row is inside, what a hidden
    /// commit sits behind.
    #[must_use]
    pub fn fold(&self) -> Option<&FoldWalk> {
        self.fold.as_ref()
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
        let marks_changed = self.inclusion.fold != inclusion.fold;
        self.inclusion = inclusion;
        let decided = self.decided;
        let position = match changed {
            None => Some(0),
            Some(changed) => self.sequence[..decided]
                .iter()
                .position(|&handle| changed.contains(&self.source.id_of(handle))),
        };
        match position {
            Some(position) if position < decided => self.redecide(position, &[]).map(Some),
            // Nothing decided changes, but the fold's marks for what is
            // still to come do: rebuilt over everything decided.
            _ if marks_changed => self.redecide(decided, &[]).map(Some),
            _ => Ok(None),
        }
    }

    /// The fold as the inclusion and forced set say it starts: fresh, with
    /// every mark still to give. `None` outside folding by default.
    fn restart_fold(&mut self) {
        self.unmarked.clear();
        self.fold = match (&self.inclusion.fold, self.follow) {
            (Some(folding), Follow::AllParents) => {
                let keep = self
                    .seeds
                    .default_branch
                    .iter()
                    .chain(&self.seeds.integration_branches)
                    .map(|seed| seed.tip)
                    .chain(folding.keep.iter().copied());
                self.unmarked.extend(keep.map(|id| (id, Mark::Keep)));
                self.unmarked
                    .extend(folding.opened.iter().map(|id| (id, Mark::Opened)));
                self.unmarked
                    .extend(self.forced.iter().map(|&id| (id, Mark::Forced)));
                Some(FoldWalk::default())
            }
            _ => None,
        };
        self.resolve_marks();
    }

    /// Gives the fold every mark whose commit now has a handle.
    fn resolve_marks(&mut self) {
        let Some(fold) = self.fold.as_mut() else {
            return;
        };
        let source = &self.source;
        self.unmarked
            .retain(|&(id, mark)| match source.handle_of(&id) {
                Some(handle) => {
                    fold.mark(handle, mark);
                    false
                }
                None => true,
            });
    }

    /// Decides one link in walk order: the link as drawn, or `None` if the
    /// view hides its commit.
    fn decide(&mut self, link: Link) -> Option<Link> {
        let id = self.source.id_of(link.commit);
        let parents: Vec<Oid> = link.parents.iter().map(|p| self.source.id_of(p)).collect();
        // The frontier's kept parents are `drawn_link`'s, by id; only
        // whether it shows the commit is wanted here.
        self.frontier
            .decide(id, &parents, self.follow, &self.inclusion)?;
        let drawn = drawn_link(link, id, self.follow, &self.inclusion);
        let Some(fold) = self.fold.as_mut() else {
            return Some(drawn);
        };
        let parents: Vec<CommitHandle> = drawn.parents.iter().collect();
        match fold.admit(drawn.commit, &parents) {
            Admit::Shown { .. } => Some(fold.trim(drawn)),
            Admit::Hidden { .. } => None,
        }
    }

    /// Whether every decided commit has been served and the walk is over.
    fn is_done(&self) -> bool {
        self.walk_done && self.decided == self.sequence.len() && self.served == self.decided
    }

    /// Serves the decided commits a rewind left unserved, as decided, up to
    /// `limit` links.
    fn serve_again(&mut self, limit: usize, links: &mut Vec<Link>) -> Result<(), S::Error> {
        while self.served < self.decided && links.len() < limit {
            let mut handles = Vec::new();
            while self.served < self.decided && links.len() + handles.len() < limit {
                let handle = self.sequence[self.served];
                if self.shown.contains(handle) {
                    handles.push(handle);
                }
                self.served += 1;
            }
            links.extend(self.links(&handles)?);
        }
        Ok(())
    }
}

impl<S: CommitSource> CommitSource for Included<S> {
    type Error = S::Error;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        let mut links = Vec::new();
        self.serve_again(limit, &mut links)?;
        if !links.is_empty() {
            return Ok(Pull {
                links,
                done: self.is_done(),
                stopped: false,
            });
        }
        loop {
            let done = self.is_done();
            if limit == 0 || done {
                return Ok(Pull {
                    links,
                    done,
                    stopped: false,
                });
            }
            // Walked before a `reinclude` (or past an earlier limit) but not
            // decided under the inclusion in force: re-served by handle,
            // never re-walked.
            let (batch, stopped) = if self.decided < self.sequence.len() {
                let end = (self.decided + limit).min(self.sequence.len());
                (self.source.links(&self.sequence[self.decided..end])?, false)
            } else {
                let pull = self.source.pull(limit, stop)?;
                for link in &pull.links {
                    self.sequence.push(link.commit);
                    self.walked.insert(link.commit);
                }
                self.walk_done = pull.done;
                (pull.links, pull.stopped)
            };
            self.resolve_marks();
            for link in batch {
                if links.len() == limit {
                    break;
                }
                let commit = link.commit;
                let drawn = self.decide(link);
                self.decided += 1;
                self.served = self.decided;
                if let Some(drawn) = drawn {
                    self.shown.insert(commit);
                    links.push(drawn);
                }
            }
            let done = self.is_done();
            if !links.is_empty() || done || stopped {
                return Ok(Pull {
                    links,
                    done,
                    stopped,
                });
            }
        }
    }

    // Untrimmed: a row takes its parents from its shape, which the trimmed
    // links laid out.
    fn read(&mut self, commits: &[CommitHandle]) -> Result<Vec<Commit>, Self::Error> {
        self.source.read(commits)
    }

    fn links(&mut self, commits: &[CommitHandle]) -> Result<Vec<Link>, Self::Error> {
        let links = self.source.links(commits)?;
        Ok(links
            .into_iter()
            .map(|link| {
                let id = self.source.id_of(link.commit);
                let drawn = drawn_link(link, id, self.follow, &self.inclusion);
                match &self.fold {
                    Some(fold) => fold.trim(drawn),
                    None => drawn,
                }
            })
            .collect())
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
