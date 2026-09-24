//! A conflict block's rows in the card arrangements: its header, the side
//! labels, the pair, each side's lines and the end rule.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{
    BlockState, LabelOf, ResolverScreen, SIDE_BY_SIDE_MIN, SideKind, TakeView, ruled_line, set_text,
};
use graft_term::SanitizedText;

use crate::fit::fit;

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
        let rule2 = self.rule().to_string().repeat(2);
        let middle = format!(
            " conflict {} of {} {} {state_word} {rule2} {keys} ",
            block.index + 1,
            block.of,
            self.dot(),
        );
        // No number: a marker row is not a line of any side (ADR-0043 §5).
        let line = ruled_line(&middle, self.rule(), usize::from(content.width));
        let cursor = self.is_cursor_block(block_index);
        set_text(buf, content.x, y, content, &line, self.header_style(cursor));
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
