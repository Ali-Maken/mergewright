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

use std::collections::{BinaryHeap, HashMap};

use graft_core::commit_handle::{CommitHandle, HandleSet, Parents};

use super::node::{Key, NodeError, Nodes};

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
