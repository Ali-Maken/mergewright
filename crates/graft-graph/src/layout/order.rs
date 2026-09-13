//! The order commits were laid out in, as a list of handles.
//!
//! This is the one piece of layout state that grows with history, and it is
//! kept *outside* [`LayoutCursor`](super::LayoutCursor) on purpose: a
//! checkpoint (DESIGN.md §3.6) is a clone of the cursor, so anything the
//! cursor carries is paid for once per checkpoint. The order is paid for once,
//! shared by the frontier and every checkpoint alike, and answers the two
//! questions that used to need a per-cursor set: "was this commit drawn
//! already?" and "which commit is at row *n*?".
//!
//! ADR-0026: everything here holds a [`CommitHandle`] (a `u32`), never an
//! `Oid` (33 bytes) — the id-sized state that made a million-commit history
//! cost most of a gigabyte. The reverse lookup (handle → row) is a dense
//! `Vec<u32>`, `u32::MAX` marking a handle never emitted: handles are dense
//! and stable for the repository's life, so this is a plain array, not a
//! hash table.

use std::ops::Range;

use graft_core::commit_handle::CommitHandle;

/// A handle with no row: `handle_at`/`row_by_handle` never emitted it.
const UNSET: u32 = u32::MAX;

/// Every commit laid out so far, in row order, with the reverse lookup.
#[derive(Clone, Debug, Default)]
pub struct EmittedOrder {
    /// `rows[row]` is the commit drawn on `row`.
    rows: Vec<CommitHandle>,
    /// `row_by_handle[handle.index()]` is the row `handle` was drawn on, or
    /// [`UNSET`]. Grown to the largest handle seen so far.
    row_by_handle: Vec<u32>,
}

/// A commit the walk order names twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the walk yielded commit {commit:?} twice: at row {first}, then again at row {again}")]
pub struct RepeatedCommit {
    /// The commit.
    pub commit: CommitHandle,
    /// The row it was first seen at.
    pub first: usize,
    /// The row it turned up at again.
    pub again: usize,
}

impl EmittedOrder {
    /// An order with nothing laid out yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// An order known in full before any row is laid out — the product of a
    /// finished background topological walk (ADR-0008 promotion).
    ///
    /// Fails if `order` repeats a commit: the layout has no meaning for a
    /// sequence that draws the same commit twice.
    pub fn from_handles(order: Vec<CommitHandle>) -> Result<Self, RepeatedCommit> {
        let mut built = EmittedOrder {
            rows: order,
            row_by_handle: Vec::new(),
        };
        for row in 0..built.rows.len() {
            let commit = built.rows[row];
            if let Some(first) = built.row_of(commit) {
                return Err(RepeatedCommit {
                    commit,
                    first,
                    again: row,
                });
            }
            built.set_row_by_handle(commit, row);
        }
        Ok(built)
    }

    /// Records that `commit` is the next commit laid out, returning the row
    /// it took. Idempotent: a `commit` already recorded — a checkpoint replay
    /// feeding rows the order already knows — returns the row it already has,
    /// without recording it again. A rebuild reproducing a different sequence
    /// is caught above this, at the chunk level, by DESIGN.md §3.6's
    /// eviction fingerprint (`RebuildDiffers`); nothing here re-derives that
    /// row by row.
    ///
    /// # Errors
    ///
    /// If a fresh append would give `commit` a second, different row — the
    /// walk itself named a commit twice, never a replay.
    pub fn note(&mut self, commit: CommitHandle) -> Result<usize, RepeatedCommit> {
        if let Some(row) = self.row_of(commit) {
            return Ok(row);
        }
        Ok(self.append(commit))
    }

    /// Appends `commit` as the next row and grows the reverse lookup to
    /// cover it. Only called once a caller has confirmed `commit` has no row
    /// yet.
    fn append(&mut self, commit: CommitHandle) -> usize {
        let row = self.rows.len();
        self.rows.push(commit);
        self.set_row_by_handle(commit, row);
        row
    }

    /// Grows the reverse lookup to cover `commit` and records `row` for it.
    fn set_row_by_handle(&mut self, commit: CommitHandle, row: usize) {
        let index = commit.index();
        if index >= self.row_by_handle.len() {
            self.row_by_handle.resize(index + 1, UNSET);
        }
        self.row_by_handle[index] =
            u32::try_from(row).unwrap_or_else(|_| panic!("more than u32::MAX rows in one order"));
    }

    /// Forgets every row from `row` on, so the layout can lay them out
    /// again under a different inclusion (`graph::inclusion`). Rows before
    /// `row` are untouched.
    pub(crate) fn truncate(&mut self, row: usize) {
        if row >= self.rows.len() {
            return;
        }
        for commit in self.rows.drain(row..) {
            self.row_by_handle[commit.index()] = UNSET;
        }
    }

    /// The row `commit` was drawn on, if it has been.
    #[must_use]
    pub fn row_of(&self, commit: CommitHandle) -> Option<usize> {
        let row = *self.row_by_handle.get(commit.index())?;
        (row != UNSET).then_some(row as usize)
    }

    /// The commit drawn on `row`, if that row exists.
    #[must_use]
    pub fn handle_at(&self, row: usize) -> Option<CommitHandle> {
        self.rows.get(row).copied()
    }

    /// The commits on `rows`, clamped to the rows that exist.
    #[must_use]
    pub fn handles(&self, rows: Range<usize>) -> &[CommitHandle] {
        let end = rows.end.min(self.rows.len());
        let start = rows.start.min(end);
        &self.rows[start..end]
    }

    /// How many rows have been laid out.
    #[must_use]
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether nothing has been laid out yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Roughly how much memory the order holds, counting the space its
    /// containers have reserved rather than only what they use — the number
    /// a memory budget (PROMPT.md §7.1) actually pays for.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.rows.capacity() * std::mem::size_of::<CommitHandle>()
            + self.row_by_handle.capacity() * std::mem::size_of::<u32>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn handle(n: u32) -> CommitHandle {
        CommitHandle(n)
    }

    #[test]
    fn appending_records_both_directions_of_the_lookup() {
        let mut order = EmittedOrder::new();
        assert_eq!(order.note(handle(7)), Ok(0));
        assert_eq!(order.note(handle(3)), Ok(1));

        assert_eq!(order.len(), 2);
        assert_eq!(order.row_of(handle(3)), Some(1));
        assert_eq!(order.handle_at(0), Some(handle(7)));
        assert_eq!(order.row_of(handle(9)), None);
        assert_eq!(order.handle_at(2), None);
    }

    #[test]
    fn noting_the_same_commit_again_is_idempotent() {
        let mut order = EmittedOrder::new();
        assert_eq!(order.note(handle(7)), Ok(0));
        assert_eq!(order.note(handle(3)), Ok(1));
        assert_eq!(order.note(handle(7)), Ok(0));
        assert_eq!(order.len(), 2);
    }

    #[test]
    fn a_full_order_rejects_a_repeated_commit() {
        assert_eq!(
            EmittedOrder::from_handles(vec![handle(1), handle(2), handle(1)])
                .map(|order| order.len()),
            Err(RepeatedCommit {
                commit: handle(1),
                first: 0,
                again: 2
            })
        );
        let order = EmittedOrder::from_handles(vec![handle(1), handle(2)]).expect("distinct");
        assert_eq!(order.row_of(handle(2)), Some(1));
    }

    #[test]
    fn handles_clamps_to_the_rows_that_exist() {
        let order =
            EmittedOrder::from_handles(vec![handle(1), handle(2), handle(3)]).expect("distinct");
        assert_eq!(order.handles(1..10), &[handle(2), handle(3)]);
        assert_eq!(order.handles(5..10), &[]);
        let backwards = Range { start: 2, end: 1 };
        assert_eq!(order.handles(backwards), &[]);
    }

    #[test]
    fn truncate_forgets_the_reverse_lookup_too() {
        let mut order = EmittedOrder::new();
        order.note(handle(7)).unwrap();
        order.note(handle(3)).unwrap();
        order.truncate(1);
        assert_eq!(order.len(), 1);
        assert_eq!(order.row_of(handle(3)), None);
        assert_eq!(order.row_of(handle(7)), Some(0));
    }

    #[test]
    fn sparse_handles_do_not_allocate_the_gap_in_rows() {
        let mut order = EmittedOrder::new();
        order.note(handle(1_000)).unwrap();
        assert_eq!(order.len(), 1);
        assert!(order.approximate_bytes() < 5_000);
    }
}
