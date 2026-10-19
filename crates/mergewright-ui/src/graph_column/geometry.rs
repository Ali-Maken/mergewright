//! One row's edge geometry: which cells a lane, gap or node occupies, read
//! once from a [`super::GraphLine`]'s [`Row`] and turned into [`Cell`]s for
//! [`super::GraphColumn`]'s `Widget::render` to paint.

use mergewright_graph::identity::BranchIdentity;
use mergewright_graph::row::Row;

use super::{CELLS_PER_LANE, NodeKind};

/// One drawn cell: its glyph and the lane it belongs to, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Cell {
    pub(super) glyph: char,
    pub(super) owner: Option<Owner>,
}

/// The lane a cell is part of: the branch that colours it, and the column
/// whose pattern it draws in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Owner {
    pub(super) identity: BranchIdentity,
    pub(super) column: usize,
}

pub(super) const BLANK: Cell = Cell {
    glyph: ' ',
    owner: None,
};

/// Every cell of one row, from lane 0's cell to the last lane's gap.
pub(super) fn row_cells(row: &Row, node: NodeKind) -> Vec<Cell> {
    let geometry = Geometry::of(row);
    let columns = row.lanes.len().max(row.lane + 1);
    let mut cells = Vec::with_capacity(columns * usize::from(CELLS_PER_LANE));
    for column in 0..columns {
        cells.push(if column == row.lane {
            Cell {
                glyph: node.glyph(),
                owner: geometry.owner_at(column),
            }
        } else {
            geometry.lane_cell(column)
        });
        cells.push(geometry.gap_cell(column));
    }
    cells
}

/// The cells of the line drawn under `row` when it is the cursor row
/// (ease-pass/07b, mock G9): every lane leaving the row downward continues
/// straight, `│` in its lane's cell, and nothing else — no node, no
/// connector. A lane leaves downward where [`Geometry::lane_cell`] gives it
/// a `down` side: present and not closing here; the node's own column where
/// an edge leaves it in that column.
pub(super) fn continuation_cells(row: &Row) -> Vec<Cell> {
    let columns = row.lanes.len().max(row.lane + 1);
    let mut cells = Vec::with_capacity(columns * usize::from(CELLS_PER_LANE));
    for column in 0..columns {
        let down = if column == row.lane {
            row.outgoing.iter().any(|&(to, _)| to == column)
        } else {
            !row.incoming.contains(&column)
        };
        let owner = row.lanes.get(column).copied().flatten();
        cells.push(match owner.filter(|_| down) {
            Some(identity) => Cell {
                glyph: '│',
                owner: Some(Owner { identity, column }),
            },
            None => BLANK,
        });
        cells.push(BLANK);
    }
    cells
}

/// Where one row's edges run, read once from the row.
pub(super) struct Geometry<'a> {
    row: &'a Row,
    /// Columns whose edge starts on this row. Merge lanes always open a
    /// fresh column, never an open one, and the node's own lane is drawn by
    /// its glyph, so these are exactly the non-node `outgoing` columns.
    opened: Vec<usize>,
    /// Columns joined to the node by a horizontal edge: closing or opening.
    elbows: Vec<usize>,
    /// The farthest elbow right of the node, and left of it.
    right_end: Option<usize>,
    left_end: Option<usize>,
}

impl<'a> Geometry<'a> {
    fn of(row: &'a Row) -> Self {
        let opened: Vec<usize> = row
            .outgoing
            .iter()
            .map(|&(column, _)| column)
            .filter(|&column| column != row.lane)
            .collect();
        let elbows: Vec<usize> = row.incoming.iter().chain(&opened).copied().collect();
        let right_end = elbows.iter().copied().filter(|&c| c > row.lane).max();
        let left_end = elbows.iter().copied().filter(|&c| c < row.lane).min();
        Geometry {
            row,
            opened,
            elbows,
            right_end,
            left_end,
        }
    }

    fn owner_at(&self, column: usize) -> Option<Owner> {
        let identity = self.row.lanes.get(column).copied().flatten()?;
        Some(Owner { identity, column })
    }

    /// The lane a horizontal edge at `column` belongs to: the nearest elbow
    /// at or beyond it, looking away from the node.
    fn horizontal_owner(&self, column: usize) -> Option<Owner> {
        let elbows = self.elbows.iter().copied();
        let nearest = if column > self.row.lane {
            elbows.filter(|&c| c >= column).min()
        } else {
            elbows.filter(|&c| c <= column).max()
        };
        nearest.and_then(|c| self.owner_at(c))
    }

    /// A lane's own cell, for any column but the node's.
    fn lane_cell(&self, column: usize) -> Cell {
        let lane = self.row.lane;
        let right_span = self
            .right_end
            .is_some_and(|end| column > lane && column <= end);
        let left_span = self
            .left_end
            .is_some_and(|end| column < lane && column >= end);
        let present = self.owner_at(column).is_some();
        let sides = Sides {
            up: present && !self.opened.contains(&column),
            down: present && !self.row.incoming.contains(&column),
            left: right_span || (left_span && self.left_end != Some(column)),
            right: left_span || (right_span && self.right_end != Some(column)),
        };
        let owner = if sides.up || sides.down {
            self.owner_at(column)
        } else if right_span || left_span {
            self.horizontal_owner(column)
        } else {
            None
        };
        Cell {
            glyph: sides.glyph(self.elbows.contains(&column)),
            owner,
        }
    }

    /// The gap right of `column`'s cell: part of a horizontal edge if one
    /// runs from that cell to the next.
    fn gap_cell(&self, column: usize) -> Cell {
        let lane = self.row.lane;
        let rightward = self
            .right_end
            .is_some_and(|end| column >= lane && column < end);
        let leftward = self
            .left_end
            .is_some_and(|end| column < lane && column >= end);
        if rightward {
            Cell {
                glyph: '─',
                owner: self.horizontal_owner(column + 1),
            }
        } else if leftward {
            Cell {
                glyph: '─',
                owner: self.horizontal_owner(column),
            }
        } else {
            BLANK
        }
    }
}

/// Which sides of a lane's cell an edge leaves through.
#[derive(Clone, Copy, Debug)]
pub(super) struct Sides {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

impl Sides {
    /// The box-drawing character joining these sides.
    ///
    /// `elbow` is whether this lane connects to the horizontal edge (it
    /// closes into, or opens from, the node). A lane that only passes
    /// through keeps a plain `│`, so it never reads as joined to an edge
    /// that merely crosses it.
    fn glyph(self, elbow: bool) -> char {
        let vertical = self.up || self.down;
        let horizontal = self.left || self.right;
        if vertical && horizontal && !elbow {
            return '│';
        }
        match (self.up, self.down, self.left, self.right) {
            (false, false, false, false) => ' ',
            (true, _, false, false) | (false, true, false, false) => '│',
            (false, false, _, _) => '─',
            (true, false, true, false) => '╯',
            (true, false, false, true) => '╰',
            (false, true, true, false) => '╮',
            (false, true, false, true) => '╭',
            (true, true, true, false) => '┤',
            (true, true, false, true) => '├',
            (true, false, true, true) => '┴',
            (false, true, true, true) => '┬',
            (true, true, true, true) => '┼',
        }
    }
}
