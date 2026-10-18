//! Layout checkpoints: DESIGN.md §3.6's resumption points.
//!
//! A checkpoint is a [`LayoutCursor`] as it stood at a row on the grid: a
//! multiple of the current `stride`. Rebuilding evicted rows replays from the
//! nearest checkpoint at or before them instead of from row 0. Checkpoints
//! are cache-only and bounded: when recording one more would pass `max`, the
//! stride doubles and every checkpoint not on the new, sparser grid is
//! dropped. Spacing stays even across all of history — never a dense band
//! near wherever was visited last — and the stride never shrinks again
//! (m1-the-graph/60: least-recently-used left a long checkpoint-less stretch
//! once history passed `max` checkpoints' worth of rows).

use crate::layout::LayoutCursor;

/// The checkpoints taken so far, in row order, on an even grid.
#[derive(Debug)]
pub(crate) struct Checkpoints {
    stride: usize,
    max: usize,
    taken: Vec<Checkpoint>,
}

#[derive(Debug)]
struct Checkpoint {
    row: usize,
    cursor: LayoutCursor,
}

impl Checkpoints {
    /// No checkpoints yet; one is due every `every` rows, at most `max` kept.
    /// `every` is the grid's starting stride.
    pub(crate) fn new(every: usize, max: usize) -> Self {
        Self {
            stride: every,
            max,
            taken: Vec::new(),
        }
    }

    /// Whether a cursor that has emitted `rows` rows stands on the grid.
    /// Row 0 never is: the caller holds the fresh cursor itself.
    pub(crate) fn is_due(&self, rows: usize) -> bool {
        rows > 0 && rows.is_multiple_of(self.stride)
    }

    /// Rows between kept checkpoints right now.
    pub(crate) fn stride(&self) -> usize {
        self.stride
    }

    /// Keeps `cursor` as the checkpoint at `row`, unless one is already held
    /// there or `row` is not on the current grid. If that would leave more
    /// than `max` held, the stride doubles and every checkpoint whose row is
    /// not a multiple of the new stride is dropped — `row`'s own checkpoint
    /// included, if it falls off the new grid.
    pub(crate) fn record(&mut self, row: usize, cursor: LayoutCursor) {
        if !row.is_multiple_of(self.stride) {
            return;
        }
        let Err(position) = self.taken.binary_search_by_key(&row, |taken| taken.row) else {
            return;
        };
        self.taken.insert(position, Checkpoint { row, cursor });
        while self.taken.len() > self.max {
            self.stride *= 2;
            let stride = self.stride;
            self.taken.retain(|taken| taken.row.is_multiple_of(stride));
        }
    }

    /// The nearest checkpoint at or before `row`, as a fresh cursor to
    /// replay from, and the row it stands at. `None` means replay from the
    /// start.
    pub(crate) fn nearest_at_or_before(&self, row: usize) -> Option<(usize, LayoutCursor)> {
        let count = self.taken.partition_point(|taken| taken.row <= row);
        let found = self.taken.get(count.checked_sub(1)?)?;
        Some((found.row, found.cursor.clone()))
    }

    /// Drops every checkpoint standing past `row`, returning their rows,
    /// ascending. One *at* `row` is kept: it is the lane state before `row`
    /// is laid out, which nothing past `row` can change.
    pub(crate) fn drop_after(&mut self, row: usize) -> Vec<usize> {
        let keep = self.taken.partition_point(|taken| taken.row <= row);
        self.taken.drain(keep..).map(|taken| taken.row).collect()
    }

    /// How many checkpoints are held.
    pub(crate) fn count(&self) -> usize {
        self.taken.len()
    }

    /// The rows the held checkpoints stand at, ascending.
    pub(crate) fn rows(&self) -> Vec<usize> {
        self.taken.iter().map(|taken| taken.row).collect()
    }

    /// Roughly how much memory the held checkpoints cost between them.
    pub(crate) fn approximate_bytes(&self) -> usize {
        self.taken
            .iter()
            .map(|taken| std::mem::size_of::<Checkpoint>() + taken.cursor.approximate_bytes())
            .sum()
    }
}
