//! The scan behind [`Fold::plan`](super::Fold::plan): which columns a window
//! opens, which kind each one is, and the order each kind ranks in
//! (DESIGN.md §3.5, ADR-0025).

use std::cmp::Reverse;
use std::collections::HashMap;

use graft_core::oid::Oid;

use super::Keep;
use crate::Row;

/// The columns a window opens, sorted into the kinds a fold treats apart.
pub(super) struct Scan {
    /// How many columns are open in some row of the window. A column open
    /// in no row takes width and shows nothing; a pass-through shows a
    /// branch with nothing of it in view. Both are dropped, not folded, so
    /// neither counts as folded.
    pub(super) open: usize,
    /// Open columns holding a node in the window, and lane 0 while open,
    /// ascending: every column drawn or folded before any connector.
    pub(super) nodes: Vec<usize>,
    /// Every column in `nodes` but lane 0, lowest priority first: the order
    /// they fold in once the budget runs out (ADR-0025's priority list,
    /// reversed).
    pub(super) fold_order: Vec<usize>,
    /// Connectors, highest priority first: the order they are admitted in,
    /// after every node column.
    pub(super) connectors: Vec<Connector>,
}

/// A column with no node in the window that carries at least one edge whose
/// two ends are both nodes in the window (ease-pass 06u).
pub(super) struct Connector {
    /// The connector's real column.
    pub(super) column: usize,
    /// For each such edge, the real columns its upper and lower end nodes
    /// sit in.
    pub(super) ends: Vec<(usize, usize)>,
}

/// Scans `rows` for which columns are open, which hold a node, which only
/// connect two nodes in view, and how each kind ranks under `keep`.
pub(super) fn scan(rows: &[Row], columns: usize, keep: &Keep) -> Scan {
    let mut open = vec![false; columns];
    let mut has_node = vec![false; columns];
    let mut last_touch: Vec<Option<usize>> = vec![None; columns];
    for (index, row) in rows.iter().enumerate() {
        for (column, lane) in row.lanes.iter().enumerate() {
            open[column] |= lane.is_some();
        }
        has_node[row.lane] = true;
        for column in touched(row) {
            open[column] = true;
            last_touch[column] = Some(index);
        }
    }

    let nodes: Vec<usize> = (0..columns)
        .filter(|&c| open[c] && (c == 0 || has_node[c]))
        .collect();

    let mut ends: Vec<Vec<(usize, usize)>> = vec![Vec::new(); columns];
    for (column, upper, lower) in edges_between_nodes_in_view(rows) {
        if column != 0 && !has_node[column] {
            ends[column].push((upper, lower));
        }
    }

    let fold_order = rank(nodes.iter().copied().filter(|&c| c != 0), keep, &last_touch);
    let connected = (0..columns).filter(|&c| !ends[c].is_empty());
    let connectors = rank(connected, keep, &last_touch)
        .into_iter()
        .rev()
        .map(|column| Connector {
            column,
            ends: std::mem::take(&mut ends[column]),
        })
        .collect();

    Scan {
        open: open.iter().filter(|&&o| o).count(),
        nodes,
        fold_order,
        connectors,
    }
}

/// `columns` lowest priority first under `keep` (ADR-0025's priority list,
/// reversed): unclassed, preferred, reserved, then selected. Within a class
/// the earliest touch first; among equals the rightmost column, so the graph
/// stays packed to the left. Every column ranked here was touched: a node
/// or an edge end sits in it.
fn rank(
    columns: impl Iterator<Item = usize>,
    keep: &Keep,
    last_touch: &[Option<usize>],
) -> Vec<usize> {
    let (mut other, mut preferred, mut reserved, mut selected) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for c in columns {
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
        class.sort_by_key(|&c| (last_touch[c], Reverse(c)));
    }
    other
        .into_iter()
        .chain(preferred)
        .chain(reserved)
        .chain(selected)
        .collect()
}

/// Every edge both of whose ends are nodes on window rows, and which runs
/// down a column other than its upper node's: `(column, upper node's column,
/// lower node's column)`. The edge leaves the upper node into `column`
/// (`outgoing`) and closes into its parent's node from `column`
/// (`incoming`); a lane never changes column on the way down.
fn edges_between_nodes_in_view(rows: &[Row]) -> Vec<(usize, usize, usize)> {
    let row_of: HashMap<Oid, &Row> = rows.iter().map(|row| (row.commit.id, row)).collect();
    let mut edges = Vec::new();
    for upper in rows {
        for &(column, parent) in &upper.outgoing {
            if column == upper.lane {
                continue;
            }
            if let Some(lower) = row_of.get(&parent)
                && lower.incoming.contains(&column)
            {
                edges.push((column, upper.lane, lower.lane));
            }
        }
    }
    edges
}

/// How many columns a row spans as laid out: its lanes, and any column an
/// edge reaches past them (a lane closing into a node is not always in
/// `lanes` on that row).
pub(super) fn span(row: &Row) -> usize {
    let reach = touched(row).map(|c| c + 1).max().unwrap_or(0);
    row.lanes.len().max(reach)
}

/// The columns a row touches: its node's, and every edge's.
pub(super) fn touched(row: &Row) -> impl Iterator<Item = usize> + '_ {
    std::iter::once(row.lane)
        .chain(row.incoming.iter().copied())
        .chain(row.outgoing.iter().map(|&(c, _)| c))
}
