//! Compaction: which columns a window of rows draws, and which fold behind a
//! single `+N` marker (PROMPT.md §1.2 "Compact output", DESIGN.md §3.5).
//!
//! A decision over laid-out [`Row`]s, never over the layout: every row keeps
//! its real columns, and a [`Fold`] is a lens on them. Expanding the marker
//! means drawing the rows without the lens, so it never re-runs layout.
//!
//! The window is whatever the caller is about to draw — the rows on screen.
//! One fold covers the whole window, so a lane that is drawn is drawn in the
//! same place on every row, and a lane that is folded is folded on every row:
//! a lane never changes column in the middle of its life on screen either.
//!
//! Ranking, least-recently-touched first, in the layout's own top-to-bottom
//! order: a lane no row in the window touches — a pure pass-through — folds
//! before one a row near the top touched, which folds before one a row near
//! the bottom touched. A column is *touched* by a row whose node sits in it
//! or whose edges close into it or open from it. Never folded: lane 0, any
//! column the caller pins (reserved integration lanes, the selected commit's
//! ancestry), and any column a node in the window sits in — a node cannot
//! draw inside a `+N`.

use std::cmp::Reverse;
use std::collections::BTreeSet;

use crate::Row;

/// Which real columns a window of rows draws, and which it folds behind the
/// marker. Built once per window by [`Fold::plan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fold {
    /// Real columns still drawn, ascending; a column's position here is
    /// where it draws.
    kept: Vec<usize>,
    /// Real columns hidden behind the marker, ascending.
    folded: Vec<usize>,
    /// How many columns the window spans as laid out, marker or not.
    columns: usize,
}

impl Fold {
    /// Decide the fold for `rows`, so that at most `max_lanes` columns are
    /// drawn, the marker included — unless only unfoldable columns remain,
    /// in which case the window draws wider than the limit rather than hide
    /// a node or a pinned lane.
    ///
    /// `pinned` are the columns the caller never lets fold, on top of lane 0
    /// and every column a node in the window sits in.
    #[must_use]
    pub fn plan<'r>(
        rows: impl IntoIterator<Item = &'r Row>,
        max_lanes: usize,
        pinned: &BTreeSet<usize>,
    ) -> Fold {
        let rows: Vec<&Row> = rows.into_iter().collect();
        let columns = rows.iter().map(|row| span(row)).max().unwrap_or(0);
        if columns <= max_lanes {
            return Fold {
                kept: (0..columns).collect(),
                folded: Vec::new(),
                columns,
            };
        }

        let mut open = vec![false; columns];
        let mut noded = vec![false; columns];
        let mut last_touch: Vec<Option<usize>> = vec![None; columns];
        for (index, row) in rows.iter().enumerate() {
            for (column, lane) in row.lanes.iter().enumerate() {
                open[column] |= lane.is_some();
            }
            noded[row.lane] = true;
            for column in touched(row) {
                open[column] = true;
                last_touch[column] = Some(index);
            }
        }

        // A column open in no row of the window takes width and shows
        // nothing: it is dropped, not folded, so it never counts in `+N`.
        let open_columns: Vec<usize> = (0..columns).filter(|&c| open[c]).collect();
        let mut candidates: Vec<usize> = open_columns
            .iter()
            .copied()
            .filter(|&c| c != 0 && !noded[c] && !pinned.contains(&c))
            .collect();
        // Untouched first, then the earliest touch; among equals the
        // rightmost column, so the graph stays packed to the left.
        candidates.sort_by_key(|&c| (last_touch[c].is_some(), last_touch[c], Reverse(c)));

        let mut folded = BTreeSet::new();
        let mut drawn = open_columns.len();
        if drawn > max_lanes {
            // Once anything folds, the marker takes a column of its own.
            let mut next = candidates.into_iter();
            while drawn + 1 > max_lanes {
                let Some(column) = next.next() else { break };
                folded.insert(column);
                drawn -= 1;
            }
        }

        Fold {
            kept: open_columns
                .into_iter()
                .filter(|c| !folded.contains(c))
                .collect(),
            folded: folded.into_iter().collect(),
            columns,
        }
    }

    /// Whether the window draws exactly as laid out: nothing folded, nothing
    /// dropped.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.folded.is_empty() && self.kept.len() == self.columns
    }

    /// The real columns hidden behind the marker, ascending.
    #[must_use]
    pub fn folded(&self) -> &[usize] {
        &self.folded
    }

    /// The real columns drawn, ascending, in drawn order.
    #[must_use]
    pub fn kept(&self) -> &[usize] {
        &self.kept
    }

    /// Where a real column draws, or `None` if it is folded or dropped.
    #[must_use]
    pub fn drawn(&self, real: usize) -> Option<usize> {
        self.kept.binary_search(&real).ok()
    }

    /// The real column at a drawn position, or `None` at the marker.
    #[must_use]
    pub fn real(&self, drawn: usize) -> Option<usize> {
        self.kept.get(drawn).copied()
    }

    /// The drawn column the marker takes — the last — or `None` when
    /// nothing is folded.
    #[must_use]
    pub fn marker(&self) -> Option<usize> {
        (!self.folded.is_empty()).then_some(self.kept.len())
    }

    /// How many columns the window draws, the marker included.
    #[must_use]
    pub fn drawn_columns(&self) -> usize {
        self.kept.len() + usize::from(self.marker().is_some())
    }

    /// `row` as drawn through this fold: kept columns renumbered to their
    /// drawn positions, every reference to a folded column redirected to the
    /// marker, and the marker's cell carrying the identity of the first
    /// folded lane the row's edges lead to, so that edge keeps its colour.
    ///
    /// A row whose node sits in a folded column cannot come from the window
    /// this fold was planned for; drawn anyway, its node lands on the marker.
    #[must_use]
    pub fn view(&self, row: &Row) -> Row {
        if self.is_empty() {
            return row.clone();
        }
        let marker = self.kept.len();
        let place = |real: usize| self.drawn(real).unwrap_or(marker);

        let mut incoming: Vec<usize> = row.incoming.iter().map(|&c| place(c)).collect();
        incoming.sort_unstable();
        incoming.dedup();

        let mut lanes: Vec<Option<_>> = self
            .kept
            .iter()
            .map(|&real| row.lanes.get(real).copied().flatten())
            .collect();
        if self.marker().is_some() {
            let leads_to_marker = touched(row)
                .filter(|&c| self.drawn(c).is_none())
                .find_map(|c| row.lanes.get(c).copied().flatten());
            lanes.push(leads_to_marker);
        }
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }

        Row {
            commit: row.commit.clone(),
            lane: place(row.lane),
            incoming,
            outgoing: row
                .outgoing
                .iter()
                .map(|&(c, parent)| (place(c), parent))
                .collect(),
            lanes,
            marker: row.marker,
        }
    }
}

/// How many columns a row spans as laid out: its lanes, and any column an
/// edge reaches past them (a lane closing into a node is not always in
/// `lanes` on that row).
fn span(row: &Row) -> usize {
    let reach = touched(row).map(|c| c + 1).max().unwrap_or(0);
    row.lanes.len().max(reach)
}

/// The columns a row touches: its node's, and every edge's.
fn touched(row: &Row) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(row.lane)
        .chain(row.incoming.iter().copied())
        .chain(row.outgoing.iter().map(|&(c, _)| c))
}

#[cfg(test)]
mod tests;
