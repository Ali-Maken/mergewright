//! [`ResolverScreen`]'s per-row drawers: one method per [`ScreenRow`] kind,
//! the gutter and number column they share, and the two text helpers.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::{
    BlockState, BlockView, GUTTER, LabelOf, RegionView, ResolverScreen, SIDE_BY_SIDE_MIN,
    ScreenRow, SideKind, TakeView, number_width,
};
use graft_term::SanitizedText;

use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::Token;

impl ResolverScreen<'_> {
    pub(super) fn rule(&self) -> char {
        if self.ascii { '-' } else { '─' }
    }

    pub(super) fn divider(&self) -> char {
        if self.ascii { '|' } else { '│' }
    }

    fn join(&self) -> char {
        if self.ascii { '+' } else { '┴' }
    }

    pub(super) fn cursor_mark(&self) -> &'static str {
        if self.ascii { "|" } else { "\u{258f}" }
    }

    /// The separator the widget's own labels use (`OURS · HEAD`,
    /// ` conflict 1 of 3 · open`, …) — ASCII `-` at the ASCII tier, mirroring
    /// every box-drawing glyph here. App-supplied text (`doc.title`,
    /// `ours_label`, …) is drawn verbatim either way: sanitizing *that* is
    /// `term::sanitize`'s job, not this widget's.
    pub(super) fn dot(&self) -> &'static str {
        if self.ascii { "-" } else { "\u{b7}" }
    }

    /// `n` right-aligned in the number column, then a space; a blank column
    /// when `None`.
    fn numbered(&self, n: Option<usize>, text: &str) -> String {
        let width = number_width(self.doc);
        match n {
            Some(n) => format!("{n:>width$} {text}"),
            None => format!("{:width$} {text}", ""),
        }
    }

    pub(super) fn is_cursor_block(&self, block: usize) -> bool {
        self.doc.cursor_block == Some(block)
    }

    /// `None` when `doc` carries no card for `index` — a stale `Layout` built
    /// from a previous `DocView` (a re-read after `e`/`M` can shrink
    /// `blocks()`, `retake_replaces` shows 3 → 2). Draw call sites must stay
    /// total: they draw a blank row rather than unwrap this.
    pub(super) fn block(&self, index: usize) -> Option<&BlockView> {
        self.doc.regions.iter().find_map(|region| match region {
            RegionView::Block(block) if block.index == index => Some(block),
            _ => None,
        })
    }

    /// A blank row, `width` cells, for a row whose block or line no longer
    /// exists in `doc` — the render stays total instead of panicking or
    /// reading stale data (CLAUDE.md rule 3).
    fn draw_blank(&self, buf: &mut Buffer, area: Rect, y: u16) {
        let content = Self::content_area(area);
        set_text(
            buf,
            content.x,
            y,
            content,
            &" ".repeat(usize::from(content.width)),
            foreground(self.theme, Token::TextMuted),
        );
    }

    /// `area` with the gutter's cells removed from the left, whether or not
    /// this row's block has the cursor — the document column stays put
    /// whichever block does.
    fn content_area(area: Rect) -> Rect {
        Rect {
            x: area.x.saturating_add(GUTTER),
            width: area.width.saturating_sub(GUTTER),
            ..area
        }
    }

    fn draw_gutter(&self, buf: &mut Buffer, area: Rect, y: u16, block: usize) {
        if self.is_cursor_block(block) {
            set_text(
                buf,
                area.x,
                y,
                area,
                self.cursor_mark(),
                foreground(self.theme, Token::Accent),
            );
        }
    }

    /// The plain content style: never reversed. Only [`Self::draw_header`]
    /// reverses (Major 3, m6-02-03 review) — every other row of the cursor
    /// block carries the `▏` gutter instead ([`Self::draw_gutter`]), which
    /// is the design note's own split ("the cursor block's header (reversed,
    /// `▏` rule on its lines)"). A whole reversed card is a worse version of
    /// the m5/44 problem this design already fixed once.
    pub(super) fn base_style(&self) -> ratatui::style::Style {
        foreground(self.theme, Token::Text)
    }

    /// [`Self::base_style`], reversed when `cursor` — the header's own
    /// style, and the only place reversal happens.
    pub(super) fn header_style(&self, cursor: bool) -> ratatui::style::Style {
        if cursor {
            self.base_style().add_modifier(Modifier::REVERSED)
        } else {
            self.base_style()
        }
    }

    pub(super) fn draw_row(&self, buf: &mut Buffer, area: Rect, y: u16, row: &ScreenRow) {
        match row {
            ScreenRow::Plain { line_no, text } => {
                // Drawn through `content_area` like every block row, so the
                // number column sits in the same cells whether the row is
                // inside a card or not (Major 2, m6-02-03 review) — a plain
                // line never carries the cursor gutter, but it still leaves
                // room for one so the numbers line up.
                let content = Self::content_area(area);
                let line = self.numbered(Some(*line_no), text.as_str());
                set_text(
                    buf,
                    content.x,
                    y,
                    content,
                    &fit(&line, usize::from(content.width)),
                    foreground(self.theme, Token::Text),
                );
            }
            ScreenRow::Note { text } => {
                // In the text column, under the lines above it, with the
                // number column left blank: it is about the file, not in it.
                let content = Self::content_area(area);
                let line = self.numbered(None, text.as_str());
                set_text(
                    buf,
                    content.x,
                    y,
                    content,
                    &fit(&line, usize::from(content.width)),
                    foreground(self.theme, Token::TextMuted),
                );
            }
            ScreenRow::WholeFile { how, keys } => {
                let line = format!("{how} {} {keys}", self.dot());
                set_text(
                    buf,
                    area.x,
                    y,
                    area,
                    &fit(&line, usize::from(area.width)),
                    foreground(self.theme, Token::Warning),
                );
            }
            ScreenRow::Header { block } => self.draw_header(buf, area, y, *block),
            ScreenRow::Labels { block } => self.draw_labels(buf, area, y, *block),
            ScreenRow::Pair {
                block,
                ours,
                theirs,
            } => self.draw_pair(buf, area, y, *block, *ours, *theirs),
            ScreenRow::Side { block, side, i } => self.draw_side(buf, area, y, *block, *side, *i),
            ScreenRow::End { block } => self.draw_end(buf, area, y, *block),
            ScreenRow::PaneTitles | ScreenRow::Aligned { .. } => {
                self.draw_pane_row(buf, area, y, row);
            }
        }
    }

    fn draw_header(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
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

    fn draw_labels(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
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

    fn draw_pair(
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

    fn draw_side(
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

    fn draw_end(&self, buf: &mut Buffer, area: Rect, y: u16, block_index: usize) {
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

    pub(super) fn side_style(&self, muted: bool) -> ratatui::style::Style {
        if muted {
            foreground(self.theme, Token::TextMuted)
        } else {
            foreground(self.theme, Token::Text)
        }
    }
}

/// `middle` centred in a rule line `width` cells wide, the rule character
/// filling what is left; cut with `…` if `middle` alone overflows `width`.
fn ruled_line(middle: &str, rule: char, width: usize) -> String {
    let mut s = format!("{rule}{rule}{middle}");
    let used = s.width();
    if used < width {
        s.push_str(&rule.to_string().repeat(width - used));
    }
    fit(&s, width)
}

/// Writes `text`, clipped to what is left of `area` from `(x, y)`; a no-op
/// past the area's edge (mirrors `map_screen::render::set_text`).
pub(super) fn set_text(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    area: Rect,
    text: &str,
    style: ratatui::style::Style,
) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    let width = area.right() - x;
    buf.set_stringn(x, y, text, usize::from(width), style);
}
