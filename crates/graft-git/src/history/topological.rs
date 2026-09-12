//! Topological order, emitted while the walk is still running.
//!
//! The obvious topological sort reads the whole range, counts in-degrees and
//! only then emits. On a large repository that means the first row appears
//! when the last one does, and PROMPT.md's 150 ms first paint is impossible.
//!
//! This is git's own incremental sort instead, and it rests on one fact about
//! generation numbers: a commit's generation is strictly greater than every
//! parent's. So to be sure no unseen child of a commit `c` exists, it is
//! enough to have expanded everything with a generation of at least
//! `generation(c)` — anything still unexpanded is at a lower generation, and
//! nothing reachable from it can be higher. That turns "have I seen all its
//! children?" from a question about the whole repository into a question
//! about a bounded prefix of it, and the first commit can be emitted after a
//! handful of reads.
//!
//! Commits the commit-graph does not cover have an unknown generation
//! ([`Node::UNKNOWN_GENERATION`]), which sorts first and so is explored
//! before anything can be confirmed. That is correct, and it is also why
//! ADR-0008 does not run this path at all when there is no commit-graph
//! file: with every generation unknown it degenerates into the whole-range
//! walk above.

use std::collections::{BinaryHeap, HashMap, HashSet};

use graft_core::oid::Oid;

use super::node::{Key, Node, NodeError, Nodes};

/// A commit waiting to be expanded, ordered by generation first so the
/// highest generations come off the heap before the lower ones.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pending {
    generation: u64,
    key: Key,
}

/// The incremental topological sort.
pub(crate) struct Topological {
    /// Discovered, not yet expanded. Highest generation first.
    explore: BinaryHeap<Pending>,
    /// Commits whose in-degree has reached zero. Most recent first.
    ///
    /// Entries here are candidates, not decisions: a commit can be pushed
    /// with an in-degree of zero and then gain a child as exploration goes
    /// on, which is what the check at pop time is for.
    candidates: BinaryHeap<Key>,
    /// How many discovered children of each commit have not been emitted
    /// yet. An absent entry is zero.
    indegree: HashMap<Oid, usize>,
    /// Every commit discovered so far, read exactly once.
    known: HashMap<Oid, Node>,
    /// Commits whose parents have been counted, so a commit with many
    /// children is expanded once.
    expanded: HashSet<Oid>,
    /// Commits already handed out.
    emitted: HashSet<Oid>,
}

impl Topological {
    /// Seeds the sort with the commits a walk starts from.
    pub(crate) fn new(
        nodes: &impl Nodes,
        tips: &[Oid],
        reads: &mut u64,
    ) -> Result<Self, NodeError> {
        let mut sort = Topological {
            explore: BinaryHeap::new(),
            candidates: BinaryHeap::new(),
            indegree: HashMap::new(),
            known: HashMap::new(),
            expanded: HashSet::new(),
            emitted: HashSet::new(),
        };
        for &tip in tips {
            if sort.known.contains_key(&tip) {
                continue;
            }
            let key = sort.discover(nodes, tip, reads)?;
            sort.explore.push(Pending {
                generation: sort.generation(tip),
                key,
            });
            // A tip has no children until exploration finds one, so it is a
            // candidate from the start. One tip being an ancestor of another
            // is normal — the in-degree check at pop time catches it.
            sort.candidates.push(key);
        }
        Ok(sort)
    }

    /// The next commit in topological order, or [`None`] when the walk is
    /// complete.
    pub(crate) fn next(
        &mut self,
        nodes: &impl Nodes,
        reads: &mut u64,
    ) -> Result<Option<Oid>, NodeError> {
        loop {
            let Some(&top) = self.candidates.peek() else {
                return Ok(None);
            };
            self.candidates.pop();

            // Everything that could still turn out to be a child of `top`
            // has to be expanded before `top`'s in-degree is final.
            self.expand_down_to(nodes, self.generation(top.id), reads)?;

            if self.emitted.contains(&top.id) {
                continue;
            }
            if self.indegree.get(&top.id).is_some_and(|count| *count > 0) {
                // A child turned up while exploring. Dropping the candidate
                // is safe: emitting that child pushes this commit back.
                continue;
            }

            self.emitted.insert(top.id);
            self.release_parents(nodes, top.id, reads)?;
            return Ok(Some(top.id));
        }
    }

    /// The generation of a commit already discovered.
    ///
    /// An unknown commit cannot reach here — everything in either heap was
    /// discovered on the way in — and treating one as uncovered would only
    /// make the sort explore more, never emit too early.
    fn generation(&self, id: Oid) -> u64 {
        self.known
            .get(&id)
            .map_or(Node::UNKNOWN_GENERATION, |node| node.generation)
    }

    /// Reads a commit once and remembers it.
    fn discover(&mut self, nodes: &impl Nodes, id: Oid, reads: &mut u64) -> Result<Key, NodeError> {
        if let Some(node) = self.known.get(&id) {
            return Ok(Key::of(node));
        }
        let node = nodes.node(id)?;
        *reads += 1;
        let key = Key::of(&node);
        self.known.insert(id, node);
        Ok(key)
    }

    /// Expands every discovered commit whose generation is at least
    /// `generation`.
    fn expand_down_to(
        &mut self,
        nodes: &impl Nodes,
        generation: u64,
        reads: &mut u64,
    ) -> Result<(), NodeError> {
        while let Some(pending) = self.explore.peek().copied() {
            if pending.generation < generation {
                break;
            }
            self.explore.pop();
            self.expand(nodes, pending.key.id, reads)?;
        }
        Ok(())
    }

    /// Counts a commit against each of its parents and queues any parent not
    /// seen before.
    fn expand(&mut self, nodes: &impl Nodes, id: Oid, reads: &mut u64) -> Result<(), NodeError> {
        if !self.expanded.insert(id) {
            return Ok(());
        }
        self.discover(nodes, id, reads)?;
        let parents = self.parents_of(id);
        for parent in parents {
            *self.indegree.entry(parent).or_insert(0) += 1;
            if !self.known.contains_key(&parent) {
                let key = self.discover(nodes, parent, reads)?;
                self.explore.push(Pending {
                    generation: self.generation(parent),
                    key,
                });
            }
        }
        Ok(())
    }

    /// Emitting a commit frees its parents by one child each; a parent with
    /// no unemitted children left becomes a candidate.
    fn release_parents(
        &mut self,
        nodes: &impl Nodes,
        id: Oid,
        reads: &mut u64,
    ) -> Result<(), NodeError> {
        // A candidate is always expanded by the time it is emitted, because
        // `expand_down_to` covers its own generation. Calling again is a
        // no-op and keeps the invariant local rather than assumed.
        self.expand(nodes, id, reads)?;
        for parent in self.parents_of(id) {
            let remaining = self.indegree.entry(parent).or_insert(0);
            *remaining = remaining.saturating_sub(1);
            if *remaining == 0 && !self.emitted.contains(&parent) {
                let key = self.discover(nodes, parent, reads)?;
                self.candidates.push(key);
            }
        }
        Ok(())
    }

    /// A discovered commit's parents. Empty for anything not discovered,
    /// which cannot happen on the paths that call it.
    fn parents_of(&self, id: Oid) -> Vec<Oid> {
        match self.known.get(&id) {
            Some(node) => node.parents.clone(),
            // Only reachable for a commit that was never discovered, and
            // every caller discovers first. "No parents" is the safe answer
            // if it ever happens: it under-counts in-degrees, which can
            // emit early, rather than leaving a commit blocked for ever.
            None => Vec::new(),
        }
    }
}
