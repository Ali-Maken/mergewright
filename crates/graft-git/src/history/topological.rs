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

//! [`Sorted`] is the other topological sort, for when there is no
//! commit-graph: ADR-0008's background promotion, which has to read the whole
//! range first anyway, so it does exactly that — once, into dense arrays
//! indexed by handle, freed when the sort ends.
//!
//! Neither keeps a map keyed by commit id. Per-commit state is a `u32`
//! in-degree per handle and one bit per question (discovered, expanded,
//! emitted); a node is held only while its commit is discovered and not yet
//! emitted, and only when it cannot be read again from the commit-graph for
//! the price of holding it (ADR-0026).

use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};

use graft_core::commit_handle::{CommitHandle, HandleSet, Parents};

use super::node::{Key, Node, NodeError, Nodes};

/// A commit waiting to be expanded, ordered by generation first so the
/// highest generations come off the heap before the lower ones.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Pending {
    generation: u32,
    key: Key,
}

/// A commit whose in-degree has reached zero, most recent first. Carries its
/// generation so confirming it needs no read.
#[derive(Clone, Copy)]
struct Candidate {
    key: Key,
    generation: u32,
}

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for Candidate {}

impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key.cmp(&other.key)
    }
}

impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Everything the incremental sort knows per commit, dense by handle.
struct State {
    /// How many discovered children have not been emitted yet, by handle.
    indegree: Vec<u32>,
    /// Read once; everything after is a lookup.
    discovered: HandleSet,
    /// Its parents have been counted, so a commit with many children is
    /// expanded once.
    expanded: HandleSet,
    /// Already handed out.
    emitted: HandleSet,
    /// Nodes discovered and not yet emitted that the commit-graph cannot give
    /// back: commits parsed out of an object. Empty on a graph-covered walk.
    frontier: HashMap<CommitHandle, Node>,
}

impl State {
    fn indegree_mut(&mut self, handle: CommitHandle) -> &mut u32 {
        let index = handle.index();
        if index >= self.indegree.len() {
            self.indegree.resize(index + 1, 0);
        }
        &mut self.indegree[index]
    }

    fn indegree(&self, handle: CommitHandle) -> u32 {
        self.indegree.get(handle.index()).copied().unwrap_or(0)
    }
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
    candidates: BinaryHeap<Candidate>,
    state: State,
}

impl Topological {
    /// Seeds the sort with the commits a walk starts from.
    pub(crate) fn new(
        nodes: &impl Nodes,
        tips: &[CommitHandle],
        reads: &mut u64,
    ) -> Result<Self, NodeError> {
        let mut sort = Topological {
            explore: BinaryHeap::new(),
            candidates: BinaryHeap::new(),
            state: State {
                indegree: Vec::new(),
                discovered: HandleSet::new(),
                expanded: HandleSet::new(),
                emitted: HandleSet::new(),
                frontier: HashMap::new(),
            },
        };
        for &tip in tips {
            if sort.state.discovered.contains(tip) {
                continue;
            }
            let (key, generation) = sort.discover(nodes, tip, reads)?;
            sort.explore.push(Pending { generation, key });
            // A tip has no children until exploration finds one, so it is a
            // candidate from the start. One tip being an ancestor of another
            // is normal — the in-degree check at pop time catches it.
            sort.candidates.push(Candidate { key, generation });
        }
        Ok(sort)
    }

    /// The next commit in topological order with its parents, or [`None`]
    /// when the walk is complete.
    pub(crate) fn next(
        &mut self,
        nodes: &impl Nodes,
        reads: &mut u64,
    ) -> Result<Option<(CommitHandle, Parents)>, NodeError> {
        loop {
            let Some(top) = self.candidates.pop() else {
                return Ok(None);
            };
            let handle = top.key.handle;

            // Everything that could still turn out to be a child of `top`
            // has to be expanded before `top`'s in-degree is final.
            self.expand_down_to(nodes, top.generation, reads)?;

            if self.state.emitted.contains(handle) {
                continue;
            }
            if self.state.indegree(handle) > 0 {
                // A child turned up while exploring. Dropping the candidate
                // is safe: emitting that child pushes this commit back.
                continue;
            }

            self.state.emitted.insert(handle);
            let parents = self.release_parents(nodes, handle, reads)?;
            return Ok(Some((handle, parents)));
        }
    }

    /// Reads a commit for the first time, keeping its node only if the
    /// commit-graph cannot give it back.
    fn discover(
        &mut self,
        nodes: &impl Nodes,
        handle: CommitHandle,
        reads: &mut u64,
    ) -> Result<(Key, u32), NodeError> {
        self.state.discovered.insert(handle);
        let node = nodes.node(handle)?;
        *reads += 1;
        let found = (Key::of(nodes, handle, &node), node.generation);
        if !nodes.rereadable(handle) {
            self.state.frontier.insert(handle, node);
        }
        Ok(found)
    }

    /// The node of a commit already discovered and not yet emitted.
    fn look(&self, nodes: &impl Nodes, handle: CommitHandle) -> Result<Node, NodeError> {
        match self.state.frontier.get(&handle) {
            Some(node) => Ok(node.clone()),
            None => nodes.node(handle),
        }
    }

    /// Expands every discovered commit whose generation is at least
    /// `generation`.
    fn expand_down_to(
        &mut self,
        nodes: &impl Nodes,
        generation: u32,
        reads: &mut u64,
    ) -> Result<(), NodeError> {
        while let Some(pending) = self.explore.peek().copied() {
            if pending.generation < generation {
                break;
            }
            self.explore.pop();
            self.expand(nodes, pending.key.handle, reads)?;
        }
        Ok(())
    }

    /// Counts a commit against each of its parents and queues any parent not
    /// seen before.
    fn expand(
        &mut self,
        nodes: &impl Nodes,
        handle: CommitHandle,
        reads: &mut u64,
    ) -> Result<(), NodeError> {
        if !self.state.expanded.insert(handle) {
            return Ok(());
        }
        let node = self.look(nodes, handle)?;
        for parent in node.parents.iter() {
            if !self.state.discovered.contains(parent) {
                let (key, generation) = self.discover(nodes, parent, reads)?;
                self.explore.push(Pending { generation, key });
            }
            *self.state.indegree_mut(parent) += 1;
        }
        Ok(())
    }

    /// Emitting a commit frees its parents by one child each; a parent with
    /// no unemitted children left becomes a candidate.
    fn release_parents(
        &mut self,
        nodes: &impl Nodes,
        handle: CommitHandle,
        reads: &mut u64,
    ) -> Result<Parents, NodeError> {
        // A candidate is always expanded by the time it is emitted, because
        // `expand_down_to` covers its own generation. Calling again is a
        // no-op and keeps the invariant local rather than assumed.
        self.expand(nodes, handle, reads)?;
        // Emitted: nothing asks for this node again.
        let node = match self.state.frontier.remove(&handle) {
            Some(node) => node,
            None => nodes.node(handle)?,
        };
        for parent in node.parents.iter() {
            let indegree = self.state.indegree_mut(parent);
            *indegree = indegree.saturating_sub(1);
            if *indegree == 0 && !self.state.emitted.contains(parent) {
                let parent_node = self.look(nodes, parent)?;
                self.candidates.push(Candidate {
                    key: Key::of(nodes, parent, &parent_node),
                    generation: parent_node.generation,
                });
            }
        }
        Ok(node.parents)
    }
}

/// The topological sort with no generation numbers to go on: read the whole
/// range, then emit.
///
/// Emits exactly what [`Topological`] emits when every generation is
/// unknown — there the first confirmation expands everything too, and from
/// then on the next commit is always the most recent one with no unemitted
/// child — without the per-commit map that sort would need for nodes parsed
/// out of objects.
pub(crate) struct Sorted {
    /// The walk's tips until the range is read, then nothing.
    tips: Vec<CommitHandle>,
    /// Filled by the read; dropped as soon as the last commit is out.
    range: Option<Range>,
}

/// The whole range, dense by handle.
///
/// Parents are held compressed-row style: every parent handle end to end in
/// `links`, in the order the read met the commits, and per handle where its
/// run starts and how long it is. That is about 9 bytes a commit where a
/// [`Parents`] per handle is 24 (m1-the-graph/59b).
struct Range {
    /// Where each commit's parents start in `links`.
    start: Paged<u32>,
    /// How many parents each commit has, up to [`WIDE`]; a commit with more
    /// keeps its count in `wide`.
    count: Paged<u8>,
    /// Every commit's parent handles, one run per commit.
    links: Paged<u32>,
    /// Entries used in `links`.
    links_len: u32,
    /// Parent counts of octopus merges too wide for a `u8`.
    wide: HashMap<CommitHandle, u32>,
    time: Paged<i64>,
    indegree: Paged<u32>,
    emitted: HandleSet,
    /// Commits with no unemitted child. Most recent first.
    ready: BinaryHeap<Key>,
}

/// The count that says "look in `wide`".
const WIDE: u8 = u8::MAX;

impl Sorted {
    /// A sort from `tips`. Reads nothing until the first [`next`](Self::next).
    pub(crate) fn new(tips: &[CommitHandle]) -> Self {
        Sorted {
            tips: tips.to_vec(),
            range: None,
        }
    }

    /// The next commit in topological order with its parents, or [`None`]
    /// when the walk is complete.
    pub(crate) fn next(
        &mut self,
        nodes: &impl Nodes,
        reads: &mut u64,
    ) -> Result<Option<(CommitHandle, Parents)>, NodeError> {
        if self.range.is_none() {
            let tips = std::mem::take(&mut self.tips);
            self.range = Some(Range::read(nodes, &tips, reads)?);
        }
        let Some(range) = self.range.as_mut() else {
            return Ok(None);
        };
        let Some(top) = range.ready.pop() else {
            // Done: the arrays go now, not when the walk is dropped.
            range.start = Paged::default();
            range.count = Paged::default();
            range.links = Paged::default();
            range.wide = HashMap::new();
            range.time = Paged::default();
            range.indegree = Paged::default();
            return Ok(None);
        };
        let handle = top.handle;
        range.emitted.insert(handle);
        let parents = range.parents(handle);
        for parent in parents.iter() {
            let indegree = range.indegree.get_mut(parent.0);
            *indegree = indegree.saturating_sub(1);
            if *indegree == 0 && !range.emitted.contains(parent) {
                range.ready.push(Key {
                    time: *range.time.get_mut(parent.0),
                    id: nodes.id(parent),
                    handle: parent,
                });
            }
        }
        Ok(Some((handle, parents)))
    }
}

impl Range {
    /// A range with nothing read yet.
    fn empty() -> Self {
        Range {
            start: Paged::default(),
            count: Paged::default(),
            links: Paged::default(),
            links_len: 0,
            wide: HashMap::new(),
            time: Paged::default(),
            indegree: Paged::default(),
            emitted: HandleSet::new(),
            ready: BinaryHeap::new(),
        }
    }

    /// Reads every commit `tips` reach, once each, counting in-degrees.
    fn read(nodes: &impl Nodes, tips: &[CommitHandle], reads: &mut u64) -> Result<Self, NodeError> {
        let mut range = Range::empty();
        let mut seen = HandleSet::new();
        let mut stack: Vec<CommitHandle> = Vec::new();
        for &tip in tips {
            if seen.insert(tip) {
                stack.push(tip);
            }
        }
        while let Some(handle) = stack.pop() {
            let node = nodes.node(handle)?;
            *reads += 1;
            for parent in node.parents.iter() {
                *range.indegree.get_mut(parent.0) += 1;
                if seen.insert(parent) {
                    stack.push(parent);
                }
            }
            *range.time.get_mut(handle.0) = node.time;
            range.store_parents(handle, &node.parents);
        }
        // Only a tip can have no child inside the range.
        let mut tips_ready = HandleSet::new();
        for &tip in tips {
            if *range.indegree.get_mut(tip.0) == 0 && tips_ready.insert(tip) {
                range.ready.push(Key {
                    time: *range.time.get_mut(tip.0),
                    id: nodes.id(tip),
                    handle: tip,
                });
            }
        }
        Ok(range)
    }

    /// Appends `handle`'s parents to `links` and records where they are.
    fn store_parents(&mut self, handle: CommitHandle, parents: &Parents) {
        *self.start.get_mut(handle.0) = self.links_len;
        let len = parents.len();
        match u8::try_from(len) {
            Ok(short) if short < WIDE => *self.count.get_mut(handle.0) = short,
            _ => {
                *self.count.get_mut(handle.0) = WIDE;
                self.wide.insert(handle, len_u32(len));
            }
        }
        for parent in parents.iter() {
            *self.links.get_mut(self.links_len) = parent.0;
            self.links_len = len_u32(self.links_len as usize + 1);
        }
    }

    /// `handle`'s parents, first parent first, as the read stored them.
    fn parents(&mut self, handle: CommitHandle) -> Parents {
        let start = *self.start.get_mut(handle.0);
        let len = match *self.count.get_mut(handle.0) {
            WIDE => self.wide.get(&handle).copied().unwrap_or_else(|| {
                panic!("a wide parent count was stored for every count of {WIDE} or more")
            }),
            short => u32::from(short),
        };
        (start..start + len)
            .map(|at| CommitHandle(*self.links.get_mut(at)))
            .collect()
    }
}

/// A parent-link count as the `u32` the arrays index by.
fn len_u32(len: usize) -> u32 {
    u32::try_from(len).unwrap_or_else(|_| {
        panic!("a range of 2^32 parent links or more has no index for the last")
    })
}

/// A dense array by index, grown a fixed page at a time.
///
/// Not a `Vec`: growing a million-entry `Vec` by doubling holds the old and
/// the new allocation at once and leaves up to half of it unused, which is
/// the whole per-commit budget of the sort. A page is 2¹⁶ entries, so the
/// slack is one page and nothing is ever copied.
struct Paged<T> {
    pages: Vec<Box<[T]>>,
}

impl<T> Default for Paged<T> {
    fn default() -> Self {
        Paged { pages: Vec::new() }
    }
}

const PAGE_BITS: u32 = 16;

impl<T: Default> Paged<T> {
    /// The entry at `index` (a handle's number, or a place in `links`),
    /// created (as `T::default()`) with its page.
    fn get_mut(&mut self, index: u32) -> &mut T {
        let page = (index >> PAGE_BITS) as usize;
        let slot = (index & ((1 << PAGE_BITS) - 1)) as usize;
        while self.pages.len() <= page {
            self.pages.push(
                std::iter::repeat_with(T::default)
                    .take(1 << PAGE_BITS)
                    .collect(),
            );
        }
        &mut self.pages[page][slot]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parents_come_back_as_stored_whatever_their_count() {
        let wide: Parents = (0..300).map(CommitHandle).collect();
        let cases = [
            Parents::None,
            Parents::One(CommitHandle(7)),
            Parents::Two(CommitHandle(1), CommitHandle(0)),
            (10..264).map(CommitHandle).collect(),
            wide,
        ];
        let mut range = Range::empty();
        // Handles far apart, so both the per-handle arrays and `links` cross
        // a page.
        let handles: Vec<CommitHandle> = (0..5).map(|i| CommitHandle(i * 70_000)).collect();
        for (&handle, parents) in handles.iter().zip(&cases) {
            range.store_parents(handle, parents);
        }
        for (&handle, parents) in handles.iter().zip(&cases) {
            assert_eq!(&range.parents(handle), parents);
        }
    }
}
