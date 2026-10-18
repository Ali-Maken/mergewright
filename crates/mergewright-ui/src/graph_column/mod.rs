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
//! keeps for its whole life, so it never flips along a line — the *real*
//! column, even when compaction ([`GraphColumn::through`]) draws the lane
//! somewhere else.
//!
//! Compaction (DESIGN.md §3.5, ADR-0022): drawn through a [`Fold`], the kept
//! lanes take their drawn columns and the folded ones become a quiet rail
//! ([`RAIL`], `text-muted`) in the last column, on every row. An edge into a
//! folded lane is never drawn to the rail: it is a stub in the gap right of
//! its node — [`STUB_BELOW`] for an edge leading down, [`STUB_ABOVE`] for one
//! that came from above — in the branch's colour. How many lanes fold is the
//! screen header's to say, never a row's.

use std::borrow::Cow;
use std::collections::BTreeSet;

use mergewright_graph::fold::{DrawnRow, Fold, Toward};
use mergewright_graph::row::Row;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
#[cfg(test)]
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

mod geometry;
mod kinds;
mod style;

pub use kinds::{NodeKind, RowPaint};

use geometry::{BLANK, row_cells};
use style::{
    dashed, dashes_odd_columns, ghost_edge_glyph, ghost_node_glyph, lane_x, off_thread, on_thread,
    to_ascii,
};

/// A folded outgoing edge: the branch continues below, behind the fold.
pub const STUB_BELOW: char = '▾';
/// A folded incoming edge: the branch came from above, behind the fold.
pub const STUB_ABOVE: char = '▴';
/// The rail: the column folded lanes sit behind.
pub const RAIL: char = '⋮';

/// Cells each lane takes: its own, then the gap before the next lane.
pub const CELLS_PER_LANE: u16 = 2;

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
    /// The real columns on this row that carry the selection's thread
    /// (`mergewright_graph::Ancestry::lit`): those lanes draw bold, every other
    /// lane dim. `None` when nothing is selected: every lane as designed.
    pub thread: Option<BTreeSet<usize>>,
    /// The real columns that belong to the preview, on a
    /// [`RowPaint::Ghost`] row. Only these lanes dash and mute; the node's
    /// own cell always does, whether or not its column is listed. Every
    /// other lane on the row is a real branch passing through and draws
    /// exactly as it would on any other row — solid, in its own colour
    /// (DESIGN.md §1.7, `docs/design/05-visualization.md` "real nodes stay
    /// solid").
    ///
    /// Empty on a non-ghost row, and meaningless there. Empty on a ghost
    /// row means only the anchor itself is drawn as a ghost — the
    /// conservative reading: a real lane is never made to look unreal by
    /// omission.
    pub ghost_lanes: BTreeSet<usize>,
}

impl<'a> GraphLine<'a> {
    /// A row drawn normally, its node shape taken from its parents.
    #[must_use]
    pub fn plain(row: &'a Row) -> Self {
        GraphLine {
            node: NodeKind::of(row),
            row: Cow::Borrowed(row),
            paint: RowPaint::Normal,
            thread: None,
            ghost_lanes: BTreeSet::new(),
        }
    }

    /// The same row drawn as a Ghost Preview row: `lanes` are the real
    /// columns the preview owns, and nothing else on the row changes
    /// appearance. Setting [`GraphLine::paint`] to [`RowPaint::Ghost`] by
    /// hand and leaving `ghost_lanes` empty ghosts only the anchor — this
    /// is the way to say which edges are the preview's.
    #[must_use]
    pub fn ghost(mut self, lanes: BTreeSet<usize>) -> Self {
        self.paint = RowPaint::Ghost;
        self.ghost_lanes = lanes;
        self
    }

    /// A row with no commit behind it, drawn normally. For a synthesised row
    /// (the uncommitted-changes row) whose `Row` exists only to be drawn.
    #[must_use]
    pub fn owned(row: Row) -> Self {
        GraphLine {
            node: NodeKind::of(&row),
            row: Cow::Owned(row),
            paint: RowPaint::Normal,
            thread: None,
            ghost_lanes: BTreeSet::new(),
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
    /// The compaction the rows draw through, if any (DESIGN.md §3.5).
    fold: Option<&'a Fold>,
    /// Draw with the ASCII glyph table (ticket m1-the-graph/19) instead of
    /// Unicode box-drawing. The same widget, the same geometry — only the
    /// glyph each cell resolves to changes.
    ascii: bool,
}

impl<'a> GraphColumn<'a> {
    /// Draw `lines` in `theme`, every lane in its real column.
    #[must_use]
    pub fn new(lines: &'a [GraphLine<'a>], theme: &'a Theme) -> Self {
        GraphColumn {
            lines,
            theme,
            fold: None,
            ascii: false,
        }
    }

    /// Draw with the ASCII tier's glyph table (`* | / \ - +`) rather than
    /// Unicode box-drawing (DESIGN.md §4.2, ADR-0016). Node shape is lost —
    /// six glyphs leave no room for six node kinds and a graph too — so
    /// every node draws as `*`; paint (bold selection, dim off-thread) and
    /// the row's own text carry what shape used to. The 16-colour/`NO_COLOR`
    /// dash convention (`dashes_odd_columns`) has no ASCII equivalent in
    /// this glyph set either, so odd-column lanes draw solid like every
    /// other lane: at this tier, lane identity leans on column position and
    /// the row's rather than a per-cell mark.
    #[must_use]
    pub fn ascii(mut self, ascii: bool) -> Self {
        self.ascii = ascii;
        self
    }

    /// Draw the lines through `fold`: kept lanes in their drawn columns, the
    /// folded ones behind a rail in the last column, their edges as stubs. A fold that
    /// changes nothing draws exactly as [`GraphColumn::new`] does.
    #[must_use]
    pub fn through(mut self, fold: &'a Fold) -> Self {
        self.fold = (!fold.is_empty()).then_some(fold);
        self
    }

    /// The cells needed to show a window through `fold` without cutting any
    /// drawn lane or the rail.
    #[must_use]
    pub fn width_through(fold: &Fold) -> u16 {
        Self::width_for(fold.drawn_columns())
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
        // The dash pattern follows a lane's *real* column, so folding never
        // flips a lane between solid and dashed.
        let real_column = |drawn: usize| match self.fold {
            Some(fold) => fold.real(drawn),
            None => Some(drawn),
        };
        for (line, y) in self.lines.iter().zip(area.top()..area.bottom()) {
            let drawn = self.fold.map(|fold| fold.view(&line.row));
            let row = drawn.as_ref().map_or(&*line.row, |drawn| &*drawn.row);
            let cells = row_cells(row, line.node);
            let ghost = line.paint == RowPaint::Ghost;
            let node_x = lane_x(row.lane);
            for x in 0..area.width {
                let cell = cells.get(usize::from(x)).copied().unwrap_or(BLANK);
                let mut style = Style::new().add_modifier(line.paint.modifier());
                let mut glyph = cell.glyph;
                // A cell is the ghost's if it is the anchor itself or its
                // lane is one the caller named. A real lane crossing a
                // ghost row is *not*: it keeps its glyph and its colour, or
                // the user cannot tell what is real (DESIGN.md §1.7).
                let ghost_cell = ghost
                    && (x == node_x
                        || cell.owner.is_some_and(|owner| {
                            real_column(owner.column)
                                .is_some_and(|column| line.ghost_lanes.contains(&column))
                        }));
                if ghost_cell {
                    // The ghost's own cells paint muted regardless of the
                    // lane's branch colour and dash regardless of the
                    // 16-colour/`NO_COLOR` parity rule below (ADR-0038 §5).
                    glyph = if x == node_x {
                        ghost_node_glyph(line.node)
                    } else {
                        ghost_edge_glyph(glyph)
                    };
                    if cell.owner.is_some() {
                        style = foreground(self.theme, Token::TextMuted)
                            .add_modifier(line.paint.modifier());
                    }
                } else if let Some(owner) = cell.owner {
                    style = self.threaded(style, line, owner.identity, real_column(owner.column));
                    if dashes && real_column(owner.column).is_some_and(|c| c % 2 == 1) {
                        glyph = dashed(glyph);
                    }
                }
                self.put(buf, area, x, y, glyph, style);
            }
            if let (Some(fold), Some(drawn)) = (self.fold, &drawn) {
                self.fold_overlay(buf, area, y, line, fold, drawn);
            }
        }
    }
}

impl GraphColumn<'_> {
    /// What a fold adds to row `y`: the rail, and one stub for the row's
    /// folded edges.
    fn fold_overlay(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        line: &GraphLine<'_>,
        fold: &Fold,
        drawn: &DrawnRow<'_>,
    ) {
        // A node whose column folded sits on the rail (ease-pass 06b): the
        // node shows there, not the rail, so no row loses its commit.
        let node_on_rail = fold.drawn(line.row.lane).is_none();
        if let Some(rail) = fold.marker().filter(|_| !node_on_rail) {
            let style = off_thread(foreground(self.theme, Token::TextMuted), line, None);
            self.put(buf, area, lane_x(rail), y, RAIL, style);
        }
        // Several folded edges on one row draw one stub, in the first edge's
        // colour (ADR-0022). It takes the gap cell even when a kept edge runs
        // through it: the edge reads on either side.
        if let Some(first) = drawn.folded.first() {
            let glyph = match first.toward {
                Toward::Below => STUB_BELOW,
                Toward::Above => STUB_ABOVE,
            };
            let style = off_thread(
                Style::new().fg(color(self.theme.branch(first.identity))),
                line,
                self.faded(first.identity),
            );
            let gap = lane_x(drawn.row.lane).saturating_add(1);
            self.put(buf, area, gap, y, glyph, style);
        }
    }

    /// A lane cell in its branch's colour, under the selection's thread
    /// ([`on_thread`]).
    fn threaded(
        &self,
        style: Style,
        line: &GraphLine<'_>,
        identity: mergewright_graph::identity::BranchIdentity,
        real: Option<usize>,
    ) -> Style {
        let style = style.fg(color(self.theme.branch(identity)));
        on_thread(style, line, real, self.faded(identity))
    }

    /// A lane's off-thread colour where the depth fades by hue
    /// ([`Theme::lane_muted`]); `None` where the fade stays DIM.
    fn faded(
        &self,
        identity: mergewright_graph::identity::BranchIdentity,
    ) -> Option<ratatui::style::Color> {
        self.theme
            .fades_by_hue()
            .then(|| color(self.theme.branch_muted(identity)))
    }

    /// Write one glyph at `x` cells into the column, if it fits, through the
    /// ASCII table when this is the ASCII tier.
    fn put(&self, buf: &mut Buffer, area: Rect, x: u16, y: u16, glyph: char, style: Style) {
        if x >= area.width {
            return;
        }
        let glyph = if self.ascii { to_ascii(glyph) } else { glyph };
        buf[(area.x + x, y)].set_char(glyph).set_style(style);
    }
}

#[cfg(test)]
mod tests;
