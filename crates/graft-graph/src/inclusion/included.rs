//! [`Included`]: the streaming [`CommitSource`] wrapper that applies
//! an [`Inclusion`] to a walk, plus [`reinclude`] to change it in place.

use graft_core::commit::Commit;
use graft_core::commit_handle::{CommitHandle, HandleSet, Link};
use graft_core::oid::Oid;

use crate::layout::source::{CommitSource, Pull};
use crate::layout::stream::{Invalidated, StreamError, StreamingLayout};
use crate::seeds::Seeds;

use super::{Follow, Frontier, Inclusion, REPLAY_BATCH, drawn_link, seeds_for};

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
            sequence: Vec::new(),
            walked: HandleSet::new(),
            shown: HandleSet::new(),
            decided: 0,
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
        let decided = self.decided;
        let Some(position) = self.sequence[..decided]
            .iter()
            .position(|&handle| changed.contains(&self.source.id_of(handle)))
        else {
            return Ok(None);
        };
        let row = self.sequence[..position]
            .iter()
            .filter(|&&handle| self.shown.contains(handle))
            .count();

        // Everything before `position` decides as it did (none of it is
        // named by the change), so replaying it rebuilds the frontier the
        // new inclusion would have had there. Structure only — parents by
        // handle, through `links` — since the outcome is thrown away here;
        // only the frontier's edge counts are kept.
        self.frontier = Frontier::new(&self.seeds);
        for &handle in &self.sequence[position..decided] {
            self.shown.remove(handle);
        }
        self.decided = position;
        let mut start = 0;
        while start < position {
            let end = (start + REPLAY_BATCH).min(position);
            let links = self.source.links(&self.sequence[start..end])?;
            for link in &links {
                let id = self.source.id_of(link.commit);
                let parents: Vec<Oid> = link.parents.iter().map(|p| self.source.id_of(p)).collect();
                let _kept = self
                    .frontier
                    .decide(id, &parents, self.follow, &self.inclusion);
            }
            start = end;
        }
        Ok(Some(row))
    }
}

impl<S: CommitSource> CommitSource for Included<S> {
    type Error = S::Error;

    fn pull(&mut self, limit: usize, stop: &dyn Fn() -> bool) -> Result<Pull, Self::Error> {
        let mut links = Vec::new();
        loop {
            let done = self.walk_done && self.decided == self.sequence.len();
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
            for link in batch {
                if links.len() == limit {
                    break;
                }
                let id = self.source.id_of(link.commit);
                let parents: Vec<Oid> = link.parents.iter().map(|p| self.source.id_of(p)).collect();
                let shown = self
                    .frontier
                    .decide(id, &parents, self.follow, &self.inclusion)
                    .is_some();
                self.decided += 1;
                if shown {
                    self.shown.insert(link.commit);
                    links.push(drawn_link(link, id, self.follow, &self.inclusion));
                }
            }
            let done = self.walk_done && self.decided == self.sequence.len();
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
                drawn_link(link, id, self.follow, &self.inclusion)
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
