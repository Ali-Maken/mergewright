//! A conflict block's rows inline, as the merge mock draws a clash
//! (ease-pass/11, M7–M8): its mark in the gutter, an open clash's two
//! sides (side by side, or stacked), a taken one shrunk to the lines the
//! pick wrote, and the outline round the cursor's clash.
//!
//! **Geometry.** Every block row keeps the same cells whether or not it
//! has the cursor: column 0 for the outline's left edge, column 1 for the
//! mark, the last column for the outline's right edge, the text between.
//! Plain lines start in the same column, so a kept line's number sits
//! under a plain line's.
//!
//! Colour (ADR-0044 §2): an open clash's left side on `ours-tint`, its
//! right on `theirs-tint`, each label in its hue, BOLD; the mark and a
//! taken clash's lines in the pick's hue; the outline in `open`. Tints come
//! from [`Theme::tint`](crate::theme::Theme::tint), so they drop out at 16
//! colours, in high contrast and under `NO_COLOR`. Colour unverified by
//! capture (CLAUDE.md rule 5).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use unicode_width::UnicodeWidthStr;

use super::{BlockState, LabelOf, ResolverScreen, SideKind, set_text};
use crate::conflict_resolver::{BlockView, GUTTER, Mark};
use graft_term::SanitizedText;

use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::Token;

/// Which of a block's rows an outline edge is drawn on.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Edge {
    Top,
    Side,
    Bottom,
}

impl ResolverScreen<'_> {
    /// A block row's text cells: right of the gutter, left of the
    /// outline's right edge — the same whichever block has the cursor.
    fn block_content(area: Rect) -> Rect {
        Rect {
            x: area.x.saturating_add(GUTTER),
            width: area.width.saturating_sub(GUTTER + 1),
            ..area
        }
    }

    /// The outline's cells on row `y`, only round the cursor's block.
    fn draw_outline(&self, buf: &mut Buffer, area: Rect, y: u16, block: usize, edge: Edge) {
        if !self.is_cursor_block(block) || area.width < 2 {
            return;
        }
        let (h, v) = if self.ascii {
            ('-', '|')
        } else {
            ('─', '│')
        };
        let (left, right) = match (edge, self.ascii) {
            (Edge::Side, _) => (v, v),
            (Edge::Top | Edge::Bottom, true) => ('+', '+'),
            (Edge::Top, false) => ('┌', '┐'),
            (Edge::Bottom, false) => ('└', '┘'),
        };
        let style = foreground(self.theme, Token::Open);
        let inner = usize::from(area.width - 2);
        if edge == Edge::Side {
            set_text(buf, area.x, y, area, &left.to_string(), style);
        } else {
            let line = format!("{left}{}{right}", h.to_string().repeat(inner));
            set_text(buf, area.x, y, area, &line, style);
        }
        set_text(buf, area.right() - 1, y, area, &right.to_string(), style);
    }

    /// `mark` in the gutter's second cell, in its hue.
    fn draw_mark(&self, buf: &mut Buffer, area: Rect, y: u16, mark: Mark) {
        let style = foreground(self.theme, mark.token()).add_modifier(Modifier::BOLD);
        set_text(
            buf,
            area.x.saturating_add(1),
            y,
            area,
            &mark.glyph(self.ascii).to_string(),
            style,
        );
    }

    /// The block's first row: the outline's top round the cursor's block.
    pub(super) fn draw_header(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        self.draw_outline(buf, area, y, block_index, Edge::Top);
    }

    /// The block's last row: the outline's bottom round the cursor's block.
    pub(super) fn draw_end(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        self.draw_outline(buf, area, y, block_index, Edge::Bottom);
    }

    /// Lays `token`'s tint under `width` cells from `x` cells into
    /// `content` on row `y` — nothing where the theme drops tints.
    fn tint(&self, buf: &mut Buffer, content: Rect, x: usize, y: u16, width: usize, token: Token) {
        if let Some(tint) = self.theme.tint(token) {
            restyle(buf, content, x, y, width, Style::new().bg(color(tint)));
        }
    }

    /// A side's label style: its hue, BOLD.
    fn label_style(&self, token: Token) -> Style {
        foreground(self.theme, token).add_modifier(Modifier::BOLD)
    }

    /// The block `block_index` names, its outline edges drawn on row `y`;
    /// `None` (a blank row) when a stale layout names a block `doc` lacks.
    fn block_row(&self, buf: &mut Buffer, area: Rect, y: u16, index: usize) -> Option<&BlockView> {
        let Some(block) = self.block(index) else {
            self.draw_blank(buf, area, y);
            return None;
        };
        self.draw_outline(buf, area, y, index, Edge::Side);
        Some(block)
    }

    pub(super) fn draw_labels(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        if self.block_row(buf, area, y, block_index).is_none() {
            return;
        }
        self.draw_mark(buf, area, y, Mark::Open);
        let content = Self::block_content(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        let rest = width.saturating_sub(half + 1);
        let left = fit(self.doc.names.ours.as_str(), half);
        let right = fit(self.doc.names.theirs.as_str(), rest);
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.side_style(false));
        restyle(buf, content, 0, y, half, self.label_style(Token::Ours));
        restyle(
            buf,
            content,
            half + 1,
            y,
            rest,
            self.label_style(Token::Theirs),
        );
        self.tint(buf, content, 0, y, half, Token::OursTint);
        self.tint(buf, content, half + 1, y, rest, Token::TheirsTint);
    }

    /// One row of an open clash's two sides, side by side and unnumbered:
    /// the number column stays blank so the code lines up with the plain
    /// lines around it.
    pub(super) fn draw_pair(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        ours: Option<usize>,
        theirs: Option<usize>,
    ) {
        let Some(block) = self.block_row(buf, area, y, block_index) else {
            return;
        };
        let content = Self::block_content(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        let rest = width.saturating_sub(half + 1);
        let text = |lines: &[SanitizedText], i: Option<usize>| {
            i.and_then(|i| lines.get(i))
                .map_or("", SanitizedText::as_str)
                .to_owned()
        };
        let left = fit(&self.numbered(None, &text(&block.ours, ours)), half);
        let right = fit(&self.numbered(None, &text(&block.theirs, theirs)), rest);
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.base_style());
        self.tint(buf, content, 0, y, half, Token::OursTint);
        self.tint(buf, content, half + 1, y, rest, Token::TheirsTint);
    }

    /// A stacked row of an open clash: a section's label (yours' carries
    /// the `?`), or one of its lines, unnumbered.
    pub(super) fn draw_side(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        side: SideKind,
        i: usize,
    ) {
        let Some(block) = self.block_row(buf, area, y, block_index) else {
            return;
        };
        if side == SideKind::Label(LabelOf::Ours) {
            self.draw_mark(buf, area, y, Mark::Open);
        }
        let content = Self::block_content(area);
        let width = usize::from(content.width);
        let line_of = |lines: &[SanitizedText]| {
            self.numbered(None, lines.get(i).map_or("", SanitizedText::as_str))
        };
        let (line, muted) = match side {
            SideKind::Label(LabelOf::Ours) => (self.doc.names.ours.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Theirs) => (self.doc.names.theirs.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Base) => {
                (format!("BASE {} merged common ancestor", self.dot()), true)
            }
            SideKind::Ours => (line_of(&block.ours), false),
            SideKind::Theirs => (line_of(&block.theirs), false),
            SideKind::Base => {
                let base_lines = block
                    .base
                    .as_ref()
                    .map_or(&[][..], |(_, lines)| lines.as_slice());
                (line_of(base_lines), false)
            }
        };
        set_text(
            buf,
            content.x,
            y,
            content,
            &fit(&line, width),
            self.side_style(muted),
        );
        match side {
            SideKind::Label(LabelOf::Ours) => {
                restyle(buf, content, 0, y, width, self.label_style(Token::Ours));
            }
            SideKind::Label(LabelOf::Theirs) => {
                restyle(buf, content, 0, y, width, self.label_style(Token::Theirs));
            }
            SideKind::Ours => self.tint(buf, content, 0, y, width, Token::OursTint),
            SideKind::Theirs => self.tint(buf, content, 0, y, width, Token::TheirsTint),
            SideKind::Label(LabelOf::Base) | SideKind::Base => {}
        }
    }

    /// One line a pick wrote, numbered as a plain line from the result's
    /// count: the first carries the mark, the last ends with its words
    /// (`kept yours`), muted.
    pub(super) fn draw_kept(&self, buf: &mut Buffer, area: Rect, y: u16, index: usize, i: usize) {
        let Some(block) = self.block_row(buf, area, y, index) else {
            return;
        };
        let BlockState::Took(take) = block.state else {
            // A stale layout: the block was re-opened since.
            return;
        };
        let mark = Mark::of(BlockState::Took(take));
        if i == 0 {
            self.draw_mark(buf, area, y, mark);
        }
        let content = Self::block_content(area);
        let width = usize::from(content.width);
        let line = match block.kept.get(i) {
            Some(text) => self.numbered(Some(block.numbers.result + i), text.as_str()),
            None => String::new(),
        };
        let line_w = line.width().min(width);
        let hue = foreground(self.theme, mark.token());
        set_text(buf, content.x, y, content, &fit(&line, width), hue);
        if i + 1 >= block.kept.len().max(1) {
            let words = mark.words(self.doc.names.roles);
            let gap = if line.is_empty() { "" } else { "  " };
            let x = content
                .x
                .saturating_add(u16::try_from(line_w).unwrap_or(u16::MAX));
            set_text(
                buf,
                x,
                y,
                content,
                &format!("{gap}{words}"),
                foreground(self.theme, Token::TextMuted),
            );
        }
    }
}

/// Patches `style` onto `width` cells from `x` cells into `content` on row
/// `y`, clipped to `content`: only the fields `style` sets change.
fn restyle(buf: &mut Buffer, content: Rect, x: usize, y: u16, width: usize, style: Style) {
    // Past `u16::MAX` is past any terminal: the clip below leaves nothing.
    let x = content
        .x
        .saturating_add(u16::try_from(x).unwrap_or(u16::MAX));
    let width = u16::try_from(width).unwrap_or(u16::MAX);
    let span = Rect::new(x, y, width, 1).intersection(content);
    buf.set_style(span, style);
}
