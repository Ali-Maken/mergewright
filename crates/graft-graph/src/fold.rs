//! Compaction: which columns a window of rows draws, and which fold behind a
//! single rail column (PROMPT.md §1.2 "Compact output", DESIGN.md §3.5,
//! ADR-0022). An edge into a folded lane is not drawn to the rail: the row
//! lists it as a [`FoldedEdge`], which renderers draw as a stub on the node.
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
//! A lane is drawn only while its branch has a commit in view (ease-pass
//! 06b, ADR-0063): a column open in the window with no node in it is
//! *dropped* — not drawn, not behind the rail, not counted as folded —
//! whatever the cap and whatever [`Keep`] says of it. Lane 0 is the one
//! exception: it stays while open, so trunk scrolling out of view never
//! shifts every lane left. An edge into a dropped column is a stub, as an
//! edge into a folded one is.
//!
//! The cap is a budget, not a set of vetoes (ADR-0025): [`Fold::plan`] draws
//! at most `max_lanes` columns, the rail included, admitting the columns
//! left in priority order until the budget is spent. Highest first: lane 0;
//! the selected commit's own column; reserved integration lanes; the
//! selection's thread ([`Keep::preferred`]); every other column holding a
//! node. Within a class, ranking is least-recently-touched first, in the
//! layout's own top-to-bottom order: a lane a row near the top touched folds
//! before one a row near the bottom touched. A column is *touched* by a row
//! whose node sits in it or whose edges close into it or open from it.

use std::borrow::Cow;
use std::cmp::Reverse;
use std::collections::BTreeSet;

use crate::Row;
use crate::identity::BranchIdentity;

/// Columns a fold weighs above the ordinary least-recently-touched rank
/// (ADR-0025). Each is a budget priority, not a veto: a column named here
/// still folds once the budget cannot hold it — only lane 0 is never
/// folded. A column with no node in the window is dropped whatever this
/// says of it: these rank node-holding columns only.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Keep {
    /// The selected commit's own real column; `None` when its row is not in
    /// the window this fold plans for.
    pub selected: Option<usize>,
    /// Reserved integration lanes, real columns.
    pub reserved: BTreeSet<usize>,
    /// The selection's thread ([`crate::Ancestry::columns`]), real columns.
    pub preferred: BTreeSet<usize>,
}

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
    /// How many columns are open in the window, dropped ones included.
    open: usize,
}

impl Fold {
    /// Decide the fold for `rows`, so that at most `max_lanes` columns are
    /// drawn, the marker included — unless only lane 0 remains, in which
    /// case the window draws lane 0 and the rail. Columns open with no node
    /// in the window are dropped first, under the cap as over it (module
    /// docs); `usize::MAX` drops them and folds nothing.
    ///
    /// `keep`'s columns are admitted ahead of the other node columns, in
    /// priority order (module docs), but still fold once the budget runs
    /// out — they are a priority, not a veto (ADR-0025).
    #[must_use]
    pub fn plan(max_lanes: usize, keep: &Keep, rows: &[Row]) -> Fold {
        let columns = rows.iter().map(span).max().unwrap_or(0);
        let Scan {
            open,
            drawable,
            candidates,
        } = scan(rows, columns, keep);

        let mut folded = BTreeSet::new();
        let mut drawn = drawable.len();
        if drawn > max_lanes {
            // Once anything folds, the marker takes a column of its own.
            let mut next = candidates.into_iter();
            while drawn + 1 > max_lanes {
                let Some(column) = next.next() else { break };
                folded.insert(column);
                drawn -= 1;
            }
        }
        debug_assert!(!folded.contains(&0), "lane 0 never folds");

        Fold {
            kept: drawable
                .into_iter()
                .filter(|c| !folded.contains(c))
                .collect(),
            folded: folded.into_iter().collect(),
            columns,
            open,
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

    /// How many lanes fold behind the rail: `0` when nothing folds. A
    /// dropped column is not one of them. No row carries it (ADR-0022).
    #[must_use]
    pub fn folded_count(&self) -> usize {
        self.folded.len()
    }

    /// Columns open in the window (a lane present or touched on any row),
    /// node-less pass-throughs included: the M of the header's
    /// `lanes drawn: N of M`. `kept().len() + folded_count() <= open_columns()`.
    #[must_use]
    pub fn open_columns(&self) -> usize {
        self.open
    }

    /// `row` as drawn through this fold: kept columns renumbered to their
    /// drawn positions, and every edge into a folded column taken out of the
    /// row and listed in [`DrawnRow::folded`] instead, so a renderer draws it
    /// as a stub on the node rather than a line to the rail (ADR-0022). The
    /// rail column itself is never part of the row: `lanes` stops before it.
    ///
    /// Borrows `row` untouched when the fold changes nothing.
    ///
    /// A row whose node sits in a folded column — node columns overflowing
    /// the cap — has its node on the rail column, with no lane of its own;
    /// its edges into drawn columns are drawn from there.
    #[must_use]
    pub fn view<'a>(&self, row: &'a Row) -> DrawnRow<'a> {
        if self.is_empty() {
            return DrawnRow {
                row: Cow::Borrowed(row),
                folded: Vec::new(),
            };
        }
        let rail = self.kept.len();
        let identity_at = |real: usize| {
            // Layout does not always record a lane closing into the node in
            // `lanes` on that row (KNOWN_GAPS); the node's own lane, then the
            // commit, still give the stub a stable colour.
            row.lanes
                .get(real)
                .copied()
                .flatten()
                .or_else(|| row.lanes.get(row.lane).copied().flatten())
                .unwrap_or_else(|| BranchIdentity::of_short_sha(row.commit.id))
        };

        let mut folded = Vec::new();
        let mut outgoing = Vec::with_capacity(row.outgoing.len());
        for &(real, parent) in &row.outgoing {
            match self.drawn(real) {
                Some(drawn) => outgoing.push((drawn, parent)),
                None => folded.push(FoldedEdge {
                    identity: identity_at(real),
                    toward: Toward::Below,
                }),
            }
        }
        let mut incoming = Vec::with_capacity(row.incoming.len());
        for &real in &row.incoming {
            match self.drawn(real) {
                Some(drawn) => incoming.push(drawn),
                None => folded.push(FoldedEdge {
                    identity: identity_at(real),
                    toward: Toward::Above,
                }),
            }
        }

        let mut lanes: Vec<Option<BranchIdentity>> = self
            .kept
            .iter()
            .map(|&real| row.lanes.get(real).copied().flatten())
            .collect();
        while lanes.last().is_some_and(Option::is_none) {
            lanes.pop();
        }

        DrawnRow {
            row: Cow::Owned(Row {
                commit: row.commit.clone(),
                drawn_parents: row.drawn_parents.clone(),
                lane: self.drawn(row.lane).unwrap_or(rail),
                incoming,
                outgoing,
                lanes,
                marker: row.marker,
            }),
            folded,
        }
    }
}

/// Which way a folded edge leaves its node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Toward {
    /// An outgoing edge: the branch continues below this row, behind the fold.
    Below,
    /// An incoming edge: the branch came from above, behind the fold.
    Above,
}

/// An edge a fold took out of a row: drawn as a stub on the node, never as a
/// line to the rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FoldedEdge {
    /// The branch the edge belongs to: the stub's colour.
    pub identity: BranchIdentity,
    /// Whether it leads down (outgoing) or came from above (incoming).
    pub toward: Toward,
}

/// A row as a fold draws it: the row in drawn columns, and the edges it lost
/// to the fold, outgoing ones first in parent order, then incoming ascending.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DrawnRow<'a> {
    /// The row in drawn columns; borrowed when the fold changes nothing.
    pub row: Cow<'a, Row>,
    /// Edges into folded columns, taken out of `row`.
    pub folded: Vec<FoldedEdge>,
}

/// The columns a window opens, which of them it may draw, and those it may
/// fold in fold order: lowest priority first, so the least protected column
/// is the first candidate to fold once the budget runs out (ADR-0025's
/// priority list, reversed).
struct Scan {
    /// How many columns are open in some row of the window. A column open
    /// in no row takes width and shows nothing; one open with no node in
    /// the window shows a branch with nothing in view. Both are dropped,
    /// not folded, so neither counts as folded.
    open: usize,
    /// Open columns holding a node in the window, and lane 0 while open,
    /// ascending: every column the window may draw.
    drawable: Vec<usize>,
    /// Every drawable column but lane 0, ordered lowest priority to fold
    /// first.
    candidates: Vec<usize>,
}

/// Scans `rows` for which columns are open, which hold a node, and how the
/// node columns rank for folding under `keep` (ADR-0025).
fn scan(rows: &[Row], columns: usize, keep: &Keep) -> Scan {
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

    let drawable: Vec<usize> = (0..columns)
        .filter(|&c| open[c] && (c == 0 || noded[c]))
        .collect();

    // Fold order within a class: the earliest touch first; among equals the
    // rightmost column, so the graph stays packed to the left. Every
    // candidate holds a node, so every candidate was touched.
    let fold_key = |&c: &usize| (last_touch[c], Reverse(c));

    let (mut other, mut preferred, mut reserved, mut selected) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for &c in drawable.iter().filter(|&&c| c != 0) {
        if keep.selected == Some(c) {
            selected.push(c);
        } else if keep.reserved.contains(&c) {
            reserved.push(c);
        } else if keep.preferred.contains(&c) {
            preferred.push(c);
        } else {
            other.push(c);
        }
    }
    for class in [&mut other, &mut preferred, &mut reserved, &mut selected] {
        class.sort_by_key(fold_key);
    }
    let candidates: Vec<usize> = other
        .into_iter()
        .chain(preferred)
        .chain(reserved)
        .chain(selected)
        .collect();

    Scan {
        open: open.iter().filter(|&&o| o).count(),
        drawable,
        candidates,
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
