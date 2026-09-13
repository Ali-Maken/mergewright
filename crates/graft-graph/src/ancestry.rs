//! The selected commit's thread through a window of laid-out rows: which rows
//! are its ancestors or descendants, and which lane cells carry the edges
//! between them (DESIGN.md §5.2).
//!
//! A paint-time lens, like [`crate::Fold`]: it reads rows already laid out and
//! never asks the layout or the walk for anything. The cost is one pass over
//! the window's rows per selection change.

use std::collections::{BTreeSet, HashMap, HashSet};

use graft_core::oid::Oid;

use crate::row::Row;

/// How a row stands to the selected commit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kin {
    /// The selected commit itself.
    Selected,
    /// Reachable from the selection through parents.
    Ancestor,
    /// Reaches the selection through its parents.
    Descendant,
    /// Neither: off the thread.
    Unrelated,
}

/// The selection's thread over one window of consecutive rows.
///
/// Exact for every row in the window: a path between two rows only passes
/// through rows between them. Lane cells are exact too, except a descendant
/// edge that enters from above the window's top, which is lit up the column
/// from its node until the column stops carrying a lane.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ancestry {
    kin: Vec<Kin>,
    lit: Vec<BTreeSet<usize>>,
}

impl Ancestry {
    /// The thread of `selected` through `rows`, or `None` when the selection
    /// is not among them.
    #[must_use]
    pub fn of(rows: &[Row], selected: Oid) -> Option<Ancestry> {
        let at = rows.iter().position(|row| row.commit.id == selected)?;
        let (kin, descendants) = kin_of(rows, at);
        let mut lit = edges_between_kin(rows, &kin, &descendants);
        // Descendant edges from children above the window: into the
        // selection or a descendant, through the node's own lane or a lane
        // closing into it.
        for index in 0..=at {
            if !matches!(kin[index], Kin::Selected | Kin::Descendant) {
                continue;
            }
            let row = &rows[index];
            for column in std::iter::once(row.lane).chain(row.incoming.iter().copied()) {
                if index == 0 || !lit[index - 1].contains(&column) {
                    light_upwards(rows, &kin, &mut lit, index, column);
                }
            }
        }
        Some(Ancestry { kin, lit })
    }

    /// How row `row` of the window stands to the selection; off the window
    /// reads as unrelated.
    #[must_use]
    pub fn kin(&self, row: usize) -> Kin {
        self.kin.get(row).copied().unwrap_or(Kin::Unrelated)
    }

    /// The real columns on row `row` that carry the thread; empty off the
    /// window.
    #[must_use]
    pub fn lit(&self, row: usize) -> &BTreeSet<usize> {
        static NONE: BTreeSet<usize> = BTreeSet::new();
        self.lit.get(row).unwrap_or(&NONE)
    }

    /// Every column the thread lights anywhere in the window: the preferred
    /// set a fold weighs above ordinary columns, [`crate::fold::Keep::preferred`]
    /// (ADR-0025) — a priority, not a veto.
    #[must_use]
    pub fn columns(&self) -> BTreeSet<usize> {
        self.lit.iter().flatten().copied().collect()
    }
}

/// Each row's relation to the selection at row `at`, and the ids of the
/// selection and its descendants.
fn kin_of(rows: &[Row], at: usize) -> (Vec<Kin>, HashSet<Oid>) {
    let mut kin = vec![Kin::Unrelated; rows.len()];
    kin[at] = Kin::Selected;
    let mut ancestors: HashSet<Oid> = rows[at].commit.parents.iter().copied().collect();
    for (index, row) in rows.iter().enumerate().skip(at + 1) {
        if ancestors.contains(&row.commit.id) {
            kin[index] = Kin::Ancestor;
            ancestors.extend(row.commit.parents.iter().copied());
        }
    }
    let mut descendants: HashSet<Oid> = HashSet::from([rows[at].commit.id]);
    for index in (0..at).rev() {
        let row = &rows[index];
        if row.commit.parents.iter().any(|p| descendants.contains(p)) {
            kin[index] = Kin::Descendant;
            descendants.insert(row.commit.id);
        }
    }
    (kin, descendants)
}

/// The columns lit on each row by nodes on the thread and the edges between
/// them: an edge runs down its column to its parent's row, or off the
/// bottom of the window.
fn edges_between_kin(
    rows: &[Row],
    kin: &[Kin],
    descendants: &HashSet<Oid>,
) -> Vec<BTreeSet<usize>> {
    let row_of: HashMap<Oid, usize> = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (row.commit.id, index))
        .collect();
    let mut lit = vec![BTreeSet::new(); rows.len()];
    for (index, row) in rows.iter().enumerate() {
        let on_thread = |parent: &Oid| match kin[index] {
            // Every parent of the selection or an ancestor is an ancestor.
            Kin::Selected | Kin::Ancestor => true,
            Kin::Descendant => descendants.contains(parent),
            Kin::Unrelated => false,
        };
        if kin[index] == Kin::Unrelated {
            continue;
        }
        lit[index].insert(row.lane);
        for (column, parent) in row.outgoing.iter().filter(|(_, parent)| on_thread(parent)) {
            let end = row_of.get(parent).copied().unwrap_or(rows.len() - 1);
            for cells in &mut lit[index..=end.max(index)] {
                cells.insert(*column);
            }
        }
    }
    lit
}

/// Lights `column` from the row above `index` upwards, while the column
/// carries a lane and no other commit sits in it — for an edge whose child
/// is above the window.
fn light_upwards(
    rows: &[Row],
    kin: &[Kin],
    lit: &mut [BTreeSet<usize>],
    index: usize,
    column: usize,
) {
    for above in (0..index).rev() {
        let row = &rows[above];
        let carries = row.lanes.get(column).copied().flatten().is_some();
        let own_node = row.lane == column;
        if !carries || (own_node && kin[above] == Kin::Unrelated) {
            return;
        }
        lit[above].insert(column);
        if own_node {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
