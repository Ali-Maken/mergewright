//! Drawing the three panes' rows: the titles row and the aligned rows.
//!
//! Each pane is `mark column · number · space · text`. The mark column is
//! the cursor block's `▏`, drawn in the yours pane only; the number column
//! is as wide as that pane's widest number ([`pane_number_widths`]). A
//! block's rows carry the band ([`super::band`]); the titles row and plain
//! rows keep `│` in `border`.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::super::draw::set_text;
use super::super::{BlockState, BlockView, RegionView, ResolverScreen, ScreenRow, TakeView};
use super::band::{self, RowKind, Side, Stand};
use super::{Cell, LineRef, Pane, pane_number_widths, three_panes};
use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::Token;

/// Where a cell sits: its block (if any), its pane, and the digits that
/// pane's number column takes.
struct CellAt {
    block: Option<usize>,
    pane: Pane,
    number_width: usize,
}

impl ResolverScreen<'_> {
    /// A [`ScreenRow::PaneTitles`] or [`ScreenRow::Aligned`] row across
    /// `area`, the document's own width.
    pub(in super::super) fn draw_pane_row(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        row: &ScreenRow,
    ) {
        let panes = three_panes(area);
        match row {
            ScreenRow::PaneTitles => {
                self.draw_plain_separators(buf, area, y, &panes);
                // By position: the left title in `ours`, the right in
                // `theirs`, whoever's side each is (ADR-0044 §1).
                let names = &self.doc.names;
                let titles = [
                    (names.ours.as_str(), Token::Ours),
                    ("result", Token::Text),
                    (names.theirs.as_str(), Token::Theirs),
                ];
                for (pane, (title, token)) in panes.iter().zip(titles) {
                    let style = foreground(self.theme, token).add_modifier(Modifier::BOLD);
                    Self::draw_in_pane(buf, *pane, y, title, style);
                }
            }
            ScreenRow::Aligned {
                block,
                ours,
                result,
                theirs,
            } => {
                match block.and_then(|b| self.block(b)) {
                    Some(view) => self.draw_band(buf, area, y, &panes, view, *result),
                    None => self.draw_plain_separators(buf, area, y, &panes),
                }
                if block.is_some_and(|b| self.is_cursor_block(b)) {
                    let accent = foreground(self.theme, Token::Accent);
                    set_text(buf, panes[0].x, y, panes[0], self.cursor_mark(), accent);
                }
                let widths = pane_number_widths(self.doc);
                let cells = [
                    (Pane::Yours, ours),
                    (Pane::Result, result),
                    (Pane::Theirs, theirs),
                ];
                for (area, (pane, cell)) in panes.into_iter().zip(cells) {
                    let at = CellAt {
                        block: *block,
                        pane,
                        number_width: widths.of(pane),
                    };
                    self.draw_cell(buf, area, y, &at, cell);
                }
            }
            _ => {}
        }
    }

    /// `│` in `border` between the panes: the titles row and plain rows.
    fn draw_plain_separators(&self, buf: &mut Buffer, area: Rect, y: u16, panes: &[Rect; 3]) {
        let bar = self.divider().to_string();
        let border = foreground(self.theme, Token::Border);
        for pane in &panes[..2] {
            set_text(buf, pane.right(), y, area, &bar, border);
        }
    }

    /// A block's row of the band: each pane's tint across its full width,
    /// then the two separators. The text is drawn over it afterwards, with
    /// foreground-only styles, so the tint stays under it.
    fn draw_band(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        panes: &[Rect; 3],
        view: &BlockView,
        result: Cell,
    ) {
        // Which kept line of the result this row is: a `both` take's tint
        // turns from ours to theirs after `ours.len()` of them. The rule row
        // counts as the first; a pad past the kept lines as the last.
        let result_row = match result {
            Cell::Line {
                text: LineRef::Taken { i, .. },
                ..
            } => i,
            Cell::Rule => 0,
            _ => usize::MAX,
        };
        let kinds = [Pane::Yours, Pane::Result, Pane::Theirs];
        for (rect, pane) in panes.iter().zip(kinds) {
            let tint = band::row_tint(view.state, pane, result_row, view.ours.len(), self.theme);
            if let Some(bg) = tint {
                buf.set_style(
                    Rect {
                        y,
                        height: 1,
                        ..*rect
                    },
                    Style::new().bg(bg),
                );
            }
        }
        let kind = if result == Cell::Rule {
            RowKind::Rule
        } else {
            RowKind::Body
        };
        let cursor = self.is_cursor_block(view.index);
        for (x, side) in [
            (panes[0].right(), Side::Left),
            (panes[1].right(), Side::Right),
        ] {
            let (glyph, style) =
                band::separator(view.state, side, kind, cursor, self.theme, self.ascii);
            set_text(buf, x, y, area, &glyph.to_string(), style);
        }
    }

    /// A line's style in `pane`: `text`, muted for BASE and for the side
    /// not taken.
    fn line_style(&self, block: Option<usize>, pane: Pane, text: LineRef) -> Style {
        let not_taken = block
            .and_then(|b| self.block(b))
            .is_some_and(|view| band::stand(view.state, pane) == Stand::NotTaken);
        if not_taken {
            band::not_taken(self.theme)
        } else {
            self.side_style(matches!(text, LineRef::Base { .. }))
        }
    }

    /// A rule's words' style in `pane`: the side's hue while open or kept,
    /// the result's by what it took, `text-muted` for the side not taken;
    /// reversed on the cursor block, as built.
    fn rule_style(&self, view: &BlockView, pane: Pane) -> Style {
        let style = match (pane, view.state) {
            (Pane::Result, BlockState::Open) => foreground(self.theme, Token::Open),
            (Pane::Result, BlockState::Took(TakeView::Ours)) => foreground(self.theme, Token::Ours),
            (Pane::Result, BlockState::Took(TakeView::Theirs)) => {
                foreground(self.theme, Token::Theirs)
            }
            (Pane::Result, BlockState::Took(TakeView::Both)) => foreground(self.theme, Token::Text),
            (side, state) if band::stand(state, side) == Stand::NotTaken => {
                band::not_taken(self.theme)
            }
            (side, _) => foreground(self.theme, band::side_hue(side)),
        };
        if self.is_cursor_block(view.index) {
            style.add_modifier(Modifier::REVERSED)
        } else {
            style
        }
    }

    /// `text` in `pane`'s text column (after the mark column), cut with `…`.
    fn draw_in_pane(buf: &mut Buffer, pane: Rect, y: u16, text: &str, style: Style) {
        let x = pane.x.saturating_add(1);
        let width = usize::from(pane.width.saturating_sub(1));
        if width > 0 {
            set_text(buf, x, y, pane, &fit(text, width), style);
        }
    }

    fn draw_cell(&self, buf: &mut Buffer, area: Rect, y: u16, at: &CellAt, cell: &Cell) {
        let CellAt {
            block,
            pane,
            number_width: width,
        } = *at;
        let blank = || format!("{:width$} ", "");
        match cell {
            Cell::Pad => {}
            Cell::Line { line_no, text } => {
                let words = self.line_text(*text);
                let line = match line_no {
                    Some(n) => format!("{n:>width$} {words}"),
                    None => format!("{}{words}", blank()),
                };
                let style = self.line_style(block, pane, *text);
                Self::draw_in_pane(buf, area, y, &line, style);
            }
            Cell::Placeholder => {
                let words = self.placeholder(block, pane);
                let line = format!("{}{words}", blank());
                // An open block's `? open · o t b` is the work still to do:
                // `open`, BOLD (BOLD alone under `NO_COLOR`). Its `base`
                // and a whole-file card's words stay muted.
                let open = block.and_then(|b| self.block(b)).is_some_and(|view| {
                    view.state == BlockState::Open && !(self.doc.base_shown && view.base.is_some())
                });
                let style = if open {
                    foreground(self.theme, Token::Open).add_modifier(Modifier::BOLD)
                } else {
                    self.side_style(true)
                };
                Self::draw_in_pane(buf, area, y, &line, style);
            }
            Cell::Rule => {
                let Some(view) = block.and_then(|b| self.block(b)) else {
                    return;
                };
                let words = self.rule_words(view, pane);
                // Padded to the pane, so the cursor block's reversal spans
                // all three panes (colour unverified, CLAUDE.md rule 5).
                let room = usize::from(area.width.saturating_sub(1));
                let line = format!("{words:room$}");
                Self::draw_in_pane(buf, area, y, &line, self.rule_style(view, pane));
            }
        }
    }

    /// The text a [`LineRef`] names; empty if a stale layout names a line
    /// `doc` no longer has (the draw stays total).
    fn line_text(&self, text: LineRef) -> &str {
        let line = match text {
            LineRef::Plain { region, i } => match self.doc.regions.get(region) {
                Some(RegionView::Plain { lines, .. } | RegionView::WholeFile { lines, .. }) => {
                    lines.get(i)
                }
                _ => None,
            },
            LineRef::Ours { block, i } => self.block(block).and_then(|b| b.ours.get(i)),
            LineRef::Theirs { block, i } => self.block(block).and_then(|b| b.theirs.get(i)),
            LineRef::Taken { block, i } => self.block(block).and_then(|b| taken_line(b, i)),
            LineRef::Base { block, i } => self
                .block(block)
                .and_then(|b| b.base.as_ref())
                .and_then(|(_, lines)| lines.get(i)),
        };
        line.map_or("", |text| text.as_str())
    }

    /// What a [`Cell::Placeholder`] says in `pane`.
    fn placeholder(&self, block: Option<usize>, pane: Pane) -> String {
        let dot = self.dot();
        if let Some(view) = block.and_then(|b| self.block(b)) {
            return if self.doc.base_shown && view.base.is_some() {
                "base".to_owned()
            } else {
                format!("? open {dot} o t b")
            };
        }
        let absent = self.doc.regions.iter().find_map(|region| match region {
            RegionView::WholeFile {
                ours_absent,
                theirs_absent,
                ..
            } => Some((ours_absent, theirs_absent)),
            _ => None,
        });
        let Some((ours, theirs)) = absent else {
            return String::new();
        };
        match pane {
            Pane::Yours => ours.as_ref().map_or("", |t| t.as_str()).to_owned(),
            Pane::Theirs => theirs.as_ref().map_or("", |t| t.as_str()).to_owned(),
            Pane::Result if ours.is_some() || theirs.is_some() => {
                format!("? keep or delete {dot} o t")
            }
            Pane::Result => format!("? open {dot} o t"),
        }
    }

    /// A block's rule, worded for `pane`.
    fn rule_words(&self, view: &BlockView, pane: Pane) -> String {
        let rule = self.rule().to_string().repeat(2);
        let dot = self.dot();
        let check = if self.ascii { '*' } else { '✓' };
        let kept = |side: Pane| match view.state {
            BlockState::Open => false,
            BlockState::Took(TakeView::Both) => true,
            BlockState::Took(TakeView::Ours) => side == Pane::Yours,
            BlockState::Took(TakeView::Theirs) => side == Pane::Theirs,
        };
        let names = &self.doc.names;
        match (pane, view.state) {
            (Pane::Yours, BlockState::Open) => format!("{rule} o take"),
            (Pane::Theirs, BlockState::Open) => format!("{rule} t take"),
            (Pane::Yours | Pane::Theirs, _) if kept(pane) => format!("{rule} taken {check}"),
            (Pane::Yours | Pane::Theirs, _) => format!("{rule} not taken"),
            (Pane::Result, state) => {
                let (n, of) = (view.index + 1, view.of);
                let what = match state {
                    BlockState::Open => "open".to_owned(),
                    BlockState::Took(TakeView::Ours) => format!("took {}", names.ours),
                    BlockState::Took(TakeView::Theirs) => format!("took {}", names.theirs),
                    BlockState::Took(TakeView::Both) => "took both".to_owned(),
                };
                format!("{rule} {n} of {of} {dot} {what}")
            }
        }
    }
}

/// Line `i` of what `block` kept: ours, theirs, or ours then theirs.
fn taken_line(block: &BlockView, i: usize) -> Option<&graft_term::SanitizedText> {
    match block.state {
        BlockState::Open => None,
        BlockState::Took(TakeView::Ours) => block.ours.get(i),
        BlockState::Took(TakeView::Theirs) => block.theirs.get(i),
        BlockState::Took(TakeView::Both) => block
            .ours
            .get(i)
            .or_else(|| block.theirs.get(i.checked_sub(block.ours.len())?)),
    }
}
