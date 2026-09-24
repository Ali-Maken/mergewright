//! A conflict block's rows in the card arrangements: its header, the side
//! labels, the pair, each side's lines and the end rule.
//!
//! Colour (ADR-0044 §2, "Card"): no glyph changes. An open block's left
//! side sits on `ours-tint`, its right on `theirs-tint`, each label in its
//! hue, BOLD; a taken block's lines on the taken side's tint, with the
//! header's name in that hue; the header's `open` in `open`. Tints come
//! from [`Theme::tint`](crate::theme::Theme::tint), so they drop out at 16
//! colours, in high contrast and under `NO_COLOR`. Colour unverified by
//! capture (CLAUDE.md rule 5).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use unicode_width::UnicodeWidthStr;

use super::{
    BlockState, LabelOf, ResolverScreen, SIDE_BY_SIDE_MIN, SideKind, TakeView, ruled_line, set_text,
};
use graft_term::SanitizedText;

use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::Token;

impl ResolverScreen<'_> {
    pub(super) fn draw_header(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let names = &self.doc.names;
        let (ours, theirs) = (names.roles.ours(), names.roles.theirs());
        let (state_word, keys) = match block.state {
            BlockState::Open => ("open".to_owned(), format!("o {ours}  t {theirs}  b both")),
            BlockState::Took(TakeView::Ours) => (
                format!("took {}", names.ours),
                format!("t {theirs}  b both"),
            ),
            BlockState::Took(TakeView::Theirs) => (
                format!("took {}", names.theirs),
                format!("o {ours}  b both"),
            ),
            BlockState::Took(TakeView::Both) => {
                ("took both".to_owned(), format!("o {ours}  t {theirs}"))
            }
        };
        // Where the state words' coloured part starts in them, how wide it
        // is, and its hue: `open`, or the taken side's name.
        let hue = match block.state {
            BlockState::Open => Some((0, "open".width(), Token::Open)),
            BlockState::Took(TakeView::Ours) => {
                Some(("took ".width(), names.ours.as_str().width(), Token::Ours))
            }
            BlockState::Took(TakeView::Theirs) => Some((
                "took ".width(),
                names.theirs.as_str().width(),
                Token::Theirs,
            )),
            BlockState::Took(TakeView::Both) => None,
        };
        let rule2 = self.rule().to_string().repeat(2);
        let lead = format!(
            " conflict {} of {} {} ",
            block.index + 1,
            block.of,
            self.dot()
        );
        let middle = format!("{lead}{state_word} {rule2} {keys} ");
        // No number: a marker row is not a line of any side (ADR-0043 §5).
        let line = ruled_line(&middle, self.rule(), usize::from(content.width));
        let cursor = self.is_cursor_block(block_index);
        set_text(buf, content.x, y, content, &line, self.header_style(cursor));
        if let Some((skip, width, token)) = hue {
            // `ruled_line` leads with two rule cells. Restyled, not redrawn:
            // the text (and its cut, if any) stays as drawn, and the
            // cursor's reversal stays under the new colour.
            let x = 2 + lead.width() + skip;
            restyle(buf, content, x, y, width, foreground(self.theme, token));
        }
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

    pub(super) fn draw_labels(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        if self.block(block_index).is_none() {
            self.draw_blank(buf, area, y);
            return;
        }
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        let left = fit(self.doc.names.ours.as_str(), half);
        let right = fit(
            self.doc.names.theirs.as_str(),
            width.saturating_sub(half + 1),
        );
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.side_style(false));
        let rest = width.saturating_sub(half + 1);
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

    pub(super) fn draw_pair(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        ours: Option<usize>,
        theirs: Option<usize>,
    ) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let half = width.saturating_sub(1) / 2;
        // `Pair` rows only exist for an `Open` block (`layout_block`): each
        // side counts in its own projection.
        let left_text = ours
            .and_then(|i| block.ours.get(i))
            .map_or("", |t| t.as_str());
        let left = fit(
            &self.numbered(ours.map(|i| block.numbers.ours + i), left_text),
            half,
        );
        let right_text = theirs
            .and_then(|i| block.theirs.get(i))
            .map_or("", |t| t.as_str());
        let right = fit(
            &self.numbered(theirs.map(|i| block.numbers.theirs + i), right_text),
            width.saturating_sub(half + 1),
        );
        let line = format!("{left}{}{right}", self.divider());
        set_text(buf, content.x, y, content, &line, self.base_style());
        self.tint(buf, content, 0, y, half, Token::OursTint);
        let rest = width.saturating_sub(half + 1);
        self.tint(buf, content, half + 1, y, rest, Token::TheirsTint);
    }

    pub(super) fn draw_side(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        block_index: usize,
        side: SideKind,
        i: usize,
    ) {
        let Some(block) = self.block(block_index) else {
            self.draw_blank(buf, area, y);
            return;
        };
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let (line, muted) = match side {
            SideKind::Label(LabelOf::Ours) => (self.doc.names.ours.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Theirs) => (self.doc.names.theirs.as_str().to_owned(), true),
            SideKind::Label(LabelOf::Base) => {
                (format!("BASE {} merged common ancestor", self.dot()), true)
            }
            SideKind::Ours => {
                // Open: the ours projection. Taken: the result, where the
                // kept lines start at `numbers.result`.
                let n = match block.state {
                    BlockState::Open => block.numbers.ours + i,
                    BlockState::Took(_) => block.numbers.result + i,
                };
                let text = block.ours.get(i).map_or("", |t| t.as_str());
                (self.numbered(Some(n), text), false)
            }
            SideKind::Theirs => {
                // Open: the theirs projection. Taken: the result, after the
                // ours lines a `Both` take kept first.
                let n = match block.state {
                    BlockState::Open => block.numbers.theirs + i,
                    BlockState::Took(TakeView::Both) => block.numbers.result + block.ours.len() + i,
                    BlockState::Took(_) => block.numbers.result + i,
                };
                let text = block.theirs.get(i).map_or("", |t| t.as_str());
                (self.numbered(Some(n), text), false)
            }
            SideKind::Base => {
                let base_lines = block
                    .base
                    .as_ref()
                    .map_or(&[][..], |(_, lines)| lines.as_slice());
                (
                    self.numbered(None, base_lines.get(i).map_or("", SanitizedText::as_str)),
                    false,
                )
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
        // Stacked, or a taken block's kept lines: each side in its tint,
        // its label in its hue. BASE stays as built.
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

    pub(super) fn draw_end(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
        self.draw_gutter(buf, area, y, block_index);
        let content = Self::content_area(area);
        let width = usize::from(content.width);
        let rule = self.rule();
        let line = if self.layout.columns >= SIDE_BY_SIDE_MIN {
            let half = width.saturating_sub(1) / 2;
            let mut s = rule.to_string().repeat(half);
            s.push(self.join());
            s.push_str(&rule.to_string().repeat(width.saturating_sub(half + 1)));
            s
        } else {
            rule.to_string().repeat(width)
        };
        set_text(
            buf,
            content.x,
            y,
            content,
            &fit(&line, width),
            self.base_style(),
        );
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
