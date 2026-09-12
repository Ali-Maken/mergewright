//! The order commits were laid out in, as a list of ids.
//!
//! This is the one piece of layout state that grows with history, and it is
//! kept *outside* [`LayoutCursor`](super::LayoutCursor) on purpose: a
//! checkpoint (DESIGN.md §3.6) is a clone of the cursor, so anything the
//! cursor carries is paid for once per checkpoint. The order is paid for once,
//! shared by the frontier and every checkpoint alike, and answers the two
//! questions that used to need a per-cursor set: "was this commit drawn
//! already?" and "which commit is at row *n*?".

use std::collections::HashMap;
use std::ops::Range;

use graft_core::oid::Oid;

/// Every commit laid out so far, in row order, with the reverse lookup.
#[derive(Clone, Debug, Default)]
pub struct EmittedOrder {
    /// `ids[row]` is the commit drawn on `row`.
    ids: Vec<Oid>,
    /// The row each commit was drawn on.
    rows: HashMap<Oid, usize>,
}

/// A commit the walk order names twice.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("the walk yielded commit {id} twice: at row {first}, then again at row {again}")]
pub struct RepeatedCommit {
    /// The commit.
    pub id: Oid,
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
    /// Fails if `ids` repeats a commit: the layout has no meaning for a
    /// sequence that draws the same commit twice.
    pub fn from_ids(ids: Vec<Oid>) -> Result<Self, RepeatedCommit> {
        let mut rows = HashMap::with_capacity(ids.len());
        for (row, &id) in ids.iter().enumerate() {
            if let Some(first) = rows.insert(id, row) {
                return Err(RepeatedCommit {
                    id,
                    first,
                    again: row,
                });
            }
        }
        Ok(Self { ids, rows })
    }

    /// Records that `id` is the commit on `row`.
    ///
    /// Appending (`row == len()`) adds a commit never seen before. Revisiting
    /// (`row < len()`) is a replay from a checkpoint and must agree with what
    /// the order already holds — the streaming layer checks the ids it got
    /// back from the repository before it lays them out, so a disagreement
    /// here is a bug in that check, not bad data.
    ///
    /// # Panics
    ///
    /// If `row` is past the end, if an appended `id` is already in the
    /// order, or if a revisited `row` holds a different commit. Each is a
    /// broken contract between the layout and its caller, never a state a
    /// repository can put it in.
    pub fn note(&mut self, row: usize, id: Oid) {
        match row.cmp(&self.ids.len()) {
            std::cmp::Ordering::Less => assert_eq!(
                self.ids[row], id,
                "replaying row {row}: the order holds {} but {id} was laid out",
                self.ids[row]
            ),
            std::cmp::Ordering::Equal => {
                let previous = self.rows.insert(id, row);
                assert!(
                    previous.is_none(),
                    "commit {id} laid out twice: on row {} and again on row {row}",
                    previous.unwrap_or(row)
                );
                self.ids.push(id);
            }
            std::cmp::Ordering::Greater => {
                panic!("row {row} laid out ahead of row {}", self.ids.len());
            }
        }
    }

    /// The row `id` was drawn on, if it has been.
    #[must_use]
    pub fn row_of(&self, id: Oid) -> Option<usize> {
        self.rows.get(&id).copied()
    }

    /// The commit drawn on `row`, if that row exists.
    #[must_use]
    pub fn id_at(&self, row: usize) -> Option<Oid> {
        self.ids.get(row).copied()
    }

    /// The commits on `rows`, clamped to the rows that exist.
    #[must_use]
    pub fn ids(&self, rows: Range<usize>) -> &[Oid] {
        let end = rows.end.min(self.ids.len());
        let start = rows.start.min(end);
        &self.ids[start..end]
    }

    /// How many rows have been laid out.
    #[must_use]
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// Whether nothing has been laid out yet.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Roughly how much memory the order holds, counting the space its
    /// containers have reserved rather than only what they use — the number
    /// a memory budget (PROMPT.md §7.1) actually pays for.
    #[must_use]
    pub fn approximate_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.ids.capacity() * std::mem::size_of::<Oid>()
            + self.rows.capacity() * (std::mem::size_of::<Oid>() + std::mem::size_of::<usize>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oid(byte: u8) -> Oid {
        Oid::from_bytes(&[byte; Oid::SHA1_BYTES]).expect("20 bytes is a valid oid length")
    }

    #[test]
    fn appending_records_both_directions_of_the_lookup() {
        let mut order = EmittedOrder::new();
        order.note(0, oid(7));
        order.note(1, oid(3));

        assert_eq!(order.len(), 2);
        assert_eq!(order.row_of(oid(3)), Some(1));
        assert_eq!(order.id_at(0), Some(oid(7)));
        assert_eq!(order.row_of(oid(9)), None);
        assert_eq!(order.id_at(2), None);
    }

    #[test]
    fn revisiting_a_row_with_the_same_commit_is_a_no_op() {
        let mut order = EmittedOrder::new();
        order.note(0, oid(7));
        order.note(1, oid(3));
        order.note(0, oid(7));
        assert_eq!(order.len(), 2);
    }

    #[test]
    #[should_panic(expected = "replaying row 1")]
    fn revisiting_a_row_with_a_different_commit_is_a_broken_contract() {
        let mut order = EmittedOrder::new();
        order.note(0, oid(7));
        order.note(1, oid(3));
        order.note(1, oid(4));
    }

    #[test]
    #[should_panic(expected = "laid out twice")]
    fn appending_a_commit_already_drawn_is_a_broken_contract() {
        let mut order = EmittedOrder::new();
        order.note(0, oid(7));
        order.note(1, oid(7));
    }

    #[test]
    #[should_panic(expected = "ahead of row 0")]
    fn skipping_a_row_is_a_broken_contract() {
        let mut order = EmittedOrder::new();
        order.note(1, oid(7));
    }

    #[test]
    fn a_full_order_rejects_a_repeated_commit() {
        assert_eq!(
            EmittedOrder::from_ids(vec![oid(1), oid(2), oid(1)]).map(|order| order.len()),
            Err(RepeatedCommit {
                id: oid(1),
                first: 0,
                again: 2
            })
        );
        let order = EmittedOrder::from_ids(vec![oid(1), oid(2)]).expect("distinct ids");
        assert_eq!(order.row_of(oid(2)), Some(1));
    }

    #[test]
    fn ids_clamps_to_the_rows_that_exist() {
        let order = EmittedOrder::from_ids(vec![oid(1), oid(2), oid(3)]).expect("distinct ids");
        assert_eq!(order.ids(1..10), &[oid(2), oid(3)]);
        assert_eq!(order.ids(5..10), &[]);
        let backwards = Range { start: 2, end: 1 };
        assert_eq!(order.ids(backwards), &[]);
    }
}
