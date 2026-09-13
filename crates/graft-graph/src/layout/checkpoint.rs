//! Layout checkpoints: DESIGN.md §3.6's resumption points.
//!
//! A checkpoint is a [`LayoutCursor`] as it stood at a row that is a multiple
//! of the interval. Rebuilding evicted rows replays from the nearest
//! checkpoint at or before them instead of from row 0. Checkpoints are
//! cache-only and bounded: past the limit the least recently used goes, and a
//! rebuild that needed it replays from the one before, re-recording it on the
//! way.

use crate::layout::LayoutCursor;

/// The checkpoints taken so far, in row order.
#[derive(Debug)]
pub(crate) struct Checkpoints {
    every: usize,
    max: usize,
    taken: Vec<Checkpoint>,
    clock: u64,
}

#[derive(Debug)]
struct Checkpoint {
    row: usize,
    cursor: LayoutCursor,
    last_used: u64,
}

impl Checkpoints {
    /// No checkpoints yet; one is due every `every` rows, at most `max` kept.
    pub(crate) fn new(every: usize, max: usize) -> Self {
        Self {
            every,
            max,
            taken: Vec::new(),
            clock: 0,
        }
    }

    /// Whether a cursor that has emitted `rows` rows is a checkpoint to keep.
    /// Row 0 never is: the caller holds the fresh cursor itself.
    pub(crate) fn is_due(&self, rows: usize) -> bool {
        rows > 0 && rows.is_multiple_of(self.every)
    }

    /// Keeps `cursor` as the checkpoint at `row`, unless one is already held
    /// there. Evicts the least recently used checkpoint if that puts the
    /// count over the limit — never the one just recorded.
    pub(crate) fn record(&mut self, row: usize, cursor: LayoutCursor) {
        let Err(position) = self.taken.binary_search_by_key(&row, |taken| taken.row) else {
            return;
        };
        self.clock += 1;
        self.taken.insert(
            position,
            Checkpoint {
                row,
                cursor,
                last_used: self.clock,
            },
        );
        if self.taken.len() > self.max
            && let Some(oldest) = self
                .taken
                .iter()
                .enumerate()
                .min_by_key(|(_, taken)| taken.last_used)
                .map(|(index, _)| index)
        {
            self.taken.remove(oldest);
        }
    }

    /// The nearest checkpoint at or before `row`, as a fresh cursor to
    /// replay from, and the row it stands at. Touches it. `None` means
    /// replay from the start.
    pub(crate) fn nearest_at_or_before(&mut self, row: usize) -> Option<(usize, LayoutCursor)> {
        let count = self.taken.partition_point(|taken| taken.row <= row);
        let found = self.taken.get_mut(count.checked_sub(1)?)?;
        self.clock += 1;
        found.last_used = self.clock;
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
