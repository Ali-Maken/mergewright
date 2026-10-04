//! [`ResolverScreen`]'s per-row drawers: one method per [`ScreenRow`] kind,
//! the gutter and number column they share, and the two text helpers.

mod block;

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use unicode_width::UnicodeWidthStr;

use super::{
    BlockState, BlockView, GUTTER, LabelOf, RegionView, ResolverScreen, SIDE_BY_SIDE_MIN,
    ScreenRow, SideKind, TakeView, number_width,
};

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

    /// The take keys alone, space apart (`o t b`), spelled from his keymap;
    /// an unbound one left out.
    pub(super) fn take_keys(&self, actions: &[&'static str]) -> String {
        actions
            .iter()
            .filter_map(|action| self.keys.spell_keys(&[crate::keymap::ActionId(action)]))
            .collect::<Vec<_>>()
            .join(" ")
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
