//! The graph column in the Unicode tier: lanes, edges and nodes drawn with
//! box-drawing characters, one commit per text row.
//!
//! This is the graph the user sees most days (ADR-0016), so it is drawn as a
//! diagram, not as `git log --graph`: rounded elbows where a branch forks or
//! merges, a node glyph whose *shape* says what kind of commit it is, and a
//! colour per branch taken from the branch's identity, never its column.
//!
//! # Geometry
//!
//! Each lane owns two cells: its own (`2 × column`), where the lane's
//! vertical line or node sits, and a gap to its right that only a horizontal
//! edge crosses. Everything a row draws is read from that one [`Row`] —
//! `lane`, `incoming`, `outgoing` and `lanes` — so any row can be drawn on
//! its own, whatever was above it or scrolled away.
//!
//! In a lane's own cell the glyph is chosen from which of its four sides
//! connect: up (an edge arrives from the row above), down (one leaves for
//! the row below), left and right (a horizontal edge to the node). Corners
//! are rounded, so a fork reads `●─╮` and a merge reads `●─╯`. A lane
//! passing through a horizontal edge keeps its `│`: the edge hops over it
//! rather than appearing to join it.
//!
//! # Lanes without hue
//!
//! Hue says which branch a lane is. Where the terminal cannot show enough
//! hues — twelve collapse to six at 16 colours, and `NO_COLOR` leaves none —
//! shape says which *neighbour* it is: at [`ColorDepth::Palette16`] and
//! [`ColorDepth::None`], every lane in an odd column draws its straight runs
//! dashed (`╎`, `╌`), so two lanes side by side never look alike whatever
//! their colour (DESIGN.md §4.6, ADR-0019). Elbows and junctions have no
//! dashed forms and stay solid. The pattern follows the column, which a lane
//! keeps for its whole life, so it never flips along a line.

use std::borrow::Cow;

use graft_core::capabilities::ColorDepth;
use graft_graph::identity::BranchIdentity;
use graft_graph::row::Row;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::paint::color;
use crate::theme::Theme;

/// Cells each lane takes: its own, then the gap before the next lane.
pub const CELLS_PER_LANE: u16 = 2;

/// What kind of commit a node is, which decides its shape.
///
/// Shape, not colour: a special commit is recognisable with colour off
/// and for any colour vision (PROMPT.md §1.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    /// An ordinary commit with one parent.
    Commit,
    /// A commit with two or more parents.
    Merge,
    /// A commit with no parents.
    Root,
    /// The commit `HEAD` points at. Wins over merge and root: where you are
    /// matters more than how the commit was made.
    Head,
    /// A stash entry.
    Stash,
    /// The working tree's uncommitted changes, drawn above `HEAD`.
    Uncommitted,
}

impl NodeKind {
    /// The kind a row's own parents give it. `HEAD`, stash and uncommitted
    /// are facts about refs, not the graph, so the caller says those.
    #[must_use]
    pub fn of(row: &Row) -> NodeKind {
        match row.commit.parents.len() {
            0 => NodeKind::Root,
            1 => NodeKind::Commit,
            _ => NodeKind::Merge,
        }
    }

    /// The glyph this kind of node draws as.
    #[must_use]
    pub fn glyph(self) -> char {
        match self {
            NodeKind::Commit => '●',
            NodeKind::Merge => '◉',
            NodeKind::Root => '◇',
            NodeKind::Head => '◆',
            NodeKind::Stash => '■',
            NodeKind::Uncommitted => '◌',
        }
    }
}

/// How strongly a row paints, set by selection and search (ticket 16).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowPaint {
    /// As designed.
    #[default]
    Normal,
    /// Pushed back: outside the current focus.
    Dimmed,
    /// Brought forward: the selection, or a search hit.
    Highlighted,
}

impl RowPaint {
    fn modifier(self) -> Modifier {
        match self {
            RowPaint::Normal => Modifier::empty(),
            RowPaint::Dimmed => Modifier::DIM,
            RowPaint::Highlighted => Modifier::BOLD,
        }
    }
}

/// One row of the graph column, as the widget draws it.
///
/// `row` is a [`Cow`] rather than a plain reference so that a virtual row
/// with no commit behind it — the uncommitted-changes row, PROMPT.md §1.3 —
/// can draw with the same connector-line geometry as a real one, by owning a
/// synthesised [`Row`] instead of borrowing one out of the walk.
#[derive(Clone, Debug)]
pub struct GraphLine<'a> {
    /// The laid-out commit, borrowed for a real row or owned for a
    /// synthesised one.
    pub row: Cow<'a, Row>,
    /// Its node's shape.
    pub node: NodeKind,
    /// How strongly it paints.
    pub paint: RowPaint,
}

impl<'a> GraphLine<'a> {
    /// A row drawn normally, its node shape taken from its parents.
    #[must_use]
    pub fn plain(row: &'a Row) -> Self {
        GraphLine {
            node: NodeKind::of(row),
            row: Cow::Borrowed(row),
            paint: RowPaint::Normal,
        }
    }

    /// A row with no commit behind it, drawn normally. For a synthesised row
    /// (the uncommitted-changes row) whose `Row` exists only to be drawn.
    #[must_use]
    pub fn owned(row: Row) -> Self {
        GraphLine {
            node: NodeKind::of(&row),
            row: Cow::Owned(row),
            paint: RowPaint::Normal,
        }
    }
}

/// The graph column for a run of consecutive rows, top row first.
///
/// Fills its whole area: every cell is written, blank where nothing is
/// drawn, so the column is exactly `area.width` wide. Lanes past the right
/// edge are cut; `+N` compaction is ticket 14's.
#[derive(Clone, Copy, Debug)]
pub struct GraphColumn<'a> {
    lines: &'a [GraphLine<'a>],
    theme: &'a Theme,
}

impl<'a> GraphColumn<'a> {
    /// Draw `lines` in `theme`.
    #[must_use]
    pub fn new(lines: &'a [GraphLine<'a>], theme: &'a Theme) -> Self {
        GraphColumn { lines, theme }
    }

    /// The cells needed to show `columns` lanes without cutting any.
    #[must_use]
    pub fn width_for(columns: usize) -> u16 {
        // Saturating on purpose: no terminal is 65,535 cells wide, so a
        // column that wide is simply "wider than any area it is drawn in".
        u16::try_from(columns)
            .unwrap_or(u16::MAX)
            .saturating_mul(CELLS_PER_LANE)
    }
}

impl Widget for GraphColumn<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let dashes = dashes_odd_columns(self.theme.color_depth());
        for (line, y) in self.lines.iter().zip(area.top()..area.bottom()) {
            let cells = row_cells(&line.row, line.node);
            for x in 0..area.width {
                let cell = cells.get(usize::from(x)).copied().unwrap_or(BLANK);
                let mut style = Style::new().add_modifier(line.paint.modifier());
                let mut glyph = cell.glyph;
                if let Some(owner) = cell.owner {
                    style = style.fg(color(self.theme.branch(owner.identity)));
                    if dashes && owner.column % 2 == 1 {
                        glyph = dashed(glyph);
                    }
                }
                buf[(area.x + x, y)].set_char(glyph).set_style(style);
            }
        }
    }
}

/// Whether lanes in odd columns draw dashed at this colour depth: only where
/// hue alone cannot tell neighbours apart.
fn dashes_odd_columns(depth: ColorDepth) -> bool {
    match depth {
        ColorDepth::TrueColor | ColorDepth::Palette256 => false,
        ColorDepth::Palette16 | ColorDepth::None => true,
    }
}

/// The dashed form of a straight run. Anything else — nodes, elbows,
/// junctions — has none and draws as it is.
fn dashed(glyph: char) -> char {
    match glyph {
        '│' => '╎',
        '─' => '╌',
        other => other,
    }
}

/// One drawn cell: its glyph and the lane it belongs to, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Cell {
    glyph: char,
    owner: Option<Owner>,
}

/// The lane a cell is part of: the branch that colours it, and the column
/// whose pattern it draws in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Owner {
    identity: BranchIdentity,
    column: usize,
}

const BLANK: Cell = Cell {
    glyph: ' ',
    owner: None,
};

/// Every cell of one row, from lane 0's cell to the last lane's gap.
fn row_cells(row: &Row, node: NodeKind) -> Vec<Cell> {
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

/// Where one row's edges run, read once from the row.
struct Geometry<'a> {
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
struct Sides {
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

#[cfg(test)]
mod tests;
