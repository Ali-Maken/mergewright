//! Drawing: the [`Widget`] entry point and the rows; the row's text columns
//! are `row_cells`'.

mod blocks;
mod emphasis;
mod second_line;

use mergewright_core::mistake_radar::Severity;
use mergewright_graph::fold::Fold;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Widget;

use super::lanes::graph_width;
use super::row_columns::Columns;
use super::{GraphEntry, GraphScreen, LaneTally, RULE_WIDTH};
use crate::fit::fit;
use crate::graph_column::{GraphColumn, GraphLine, RowPaint};
use crate::paint::{color, foreground};
use crate::theme::{Token, TokenColor};

impl GraphScreen<'_> {
    /// The rule over the bar (ADR-0058 §4), across the whole row as the
    /// mock-up draws it; `-` at the ASCII tier, which has no box-drawing
    /// glyphs. Nothing when `rule` is zero-height (under 30 rows).
    fn rule(&self, rule: Rect, buf: &mut Buffer) {
        if rule.height == 0 {
            return;
        }
        let glyph = if self.view.ascii { "-" } else { "\u{2500}" };
        buf.set_line(
            rule.x,
            rule.y,
            &Line::styled(
                glyph.repeat(usize::from(rule.width)),
                foreground(self.theme, Token::Border),
            ),
            rule.width,
        );
    }
}

impl GraphScreen<'_> {
    /// The banner's lines into `banner` (its carved rows): the first in the
    /// accent, the rest as information.
    fn banner(&self, area: Rect, banner: Rect, buf: &mut Buffer) {
        let width = usize::from(area.width);
        for (offset, line) in self
            .view
            .banner
            .iter()
            .take(usize::from(banner.height))
            .enumerate()
        {
            let y = banner
                .y
                .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            let token = if offset == 0 {
                Token::Accent
            } else {
                Token::Info
            };
            let styled = Line::styled(
                fit(&format!(" {line}"), width),
                foreground(self.theme, token),
            );
            buf.set_line(area.x, y, &styled, area.width);
        }
    }
}

impl Widget for GraphScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            // A pty that has not been sized yet (ADR-0012).
            return;
        }
        let width = usize::from(area.width);
        // One carve for the whole screen, so no two of these draws can claim
        // the same row at any height (`GraphScreen::screen_rows`).
        let chrome = self.screen_rows(area);
        let (body, rows, fold) = self.body_layout(area);
        if chrome.header.height > 0 {
            buf.set_line(
                area.x,
                chrome.header.y,
                &self.header(area.width, LaneTally::of(&fold, rows)),
                area.width,
            );
            // A failed search or walk in the header's tail leads with the mark.
            crate::failure::ink_marks(buf, chrome.header, self.theme, self.view.ascii);
        }
        self.rule(chrome.rule, buf);
        self.banner(area, chrome.banner, buf);
        if let Some(card) = &self.view.tour
            && chrome.tour.height > 0
        {
            crate::tour_card::TourCard {
                view: card,
                theme: self.theme,
            }
            .render(chrome.tour, buf);
        }

        self.pending_block(chrome.pending, buf);
        self.branch_pane(chrome.pane, buf);

        if body.height > 0 {
            // Gutter cell 0, where the cursor's `▸` goes — absent on a screen
            // too narrow to spare the gutter (`layout::inset`).
            let gutter = (body.x >= area.x + super::GUTTER).then(|| body.x - super::GUTTER);
            self.body(body, gutter, (rows, &fold), area.width, buf);
        }

        // One cell of right margin on the bottom rows, as on every other.
        let bar_width = width
            .saturating_sub(usize::from(super::RIGHT_MARGIN))
            .max(1);
        self.bottom_rows(&chrome, (body, rows, &fold), bar_width, area, buf);
        if let Some(line) = self.view.commit_graph
            && chrome.offer.height > 0
        {
            buf.set_line(
                area.x,
                chrome.offer.y,
                &self.commit_graph_line(line, bar_width),
                area.width,
            );
        }
    }
}

impl GraphScreen<'_> {
    /// The rows: graph, the reserved marker slot, the subject, and — when
    /// there is room — the author chip, the relative date and the short SHA.
    fn body(
        &self,
        area: Rect,
        gutter: Option<u16>,
        (rows, fold): (&[GraphEntry<'_>], &Fold),
        screen: u16,
        buf: &mut Buffer,
    ) {
        let graph_width = graph_width(area, fold);
        // At the pixel tier the cells stay blank: text drawn there would sit
        // on top of the terminal's images. Not on a screen with no gutter:
        // a strip starts in the gutter's gap cell (ease-pass/29 D5), so none
        // is placed there and the column draws as text.
        if !self.view.pixel || gutter.is_none() {
            let graph_lines: Vec<GraphLine<'_>> =
                rows.iter().map(|entry| entry.line.clone()).collect();
            let column = GraphColumn::new(&graph_lines, self.theme)
                .ascii(self.view.ascii)
                .second_line_after(rows.iter().position(|entry| entry.second.is_some()));
            column.through(fold).render(
                Rect {
                    width: graph_width,
                    ..area
                },
                buf,
            );
        }

        let columns = Columns::compute(area, graph_width, screen);
        // The cursor row's second line takes the line under it (07b).
        let mut ys = area.top()..area.bottom();
        for entry in rows {
            let Some(y) = ys.next() else { break };
            if let Some(x) = gutter {
                self.draw_gutter(entry, x, y, buf);
            }
            self.draw_row(entry, y, area, &columns, buf);
            if let Some(y) = entry.second.as_ref().and_then(|_| ys.next()) {
                self.draw_second_line(entry, y, area, &columns, buf);
            }
        }
    }

    /// The cursor's `▸` (ASCII `>`) in gutter cell 0, accent and bold
    /// (ADR-0058 §2, DESIGN §1.0): the row's mark at every depth, on the
    /// ground, never on the fill. Cell 1 is the gap. Only the cursor row
    /// draws one — a drop target is marked in the slot (`→`, [`row_rule`]).
    fn draw_gutter(&self, entry: &GraphEntry<'_>, x: u16, y: u16, buf: &mut Buffer) {
        if entry.line.paint != RowPaint::Highlighted {
            return;
        }
        let mark = if self.view.ascii { ">" } else { "\u{25b8}" };
        draw_cell(
            buf,
            x,
            y,
            1,
            mark,
            foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
        );
    }

    /// `ink` as this row draws it: lifted to stay legible on the `selection`
    /// fill on a cursor or drop-target row ([`Theme::on_selection`], ADR-0058
    /// §3), as is everywhere else.
    ///
    /// [`Theme::on_selection`]: crate::theme::Theme::on_selection
    pub(super) fn ink(&self, entry: &GraphEntry<'_>, ink: TokenColor) -> Style {
        let ink = if raised(entry) {
            self.theme.on_selection(ink)
        } else {
            ink
        };
        Style::new().fg(color(ink))
    }

    /// One row's text columns: highlight/dim first, then the marker slot,
    /// the subject, and whichever trailing columns fit.
    fn draw_row(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        columns: &Columns,
        buf: &mut Buffer,
    ) {
        debug_assert!(
            entry.line.paint != RowPaint::Ghost
                || (!entry.compare_marked && entry.marker.is_none()),
            "a Ghost Preview row never carries the pinned-compare accent or a Mistake Radar \
             marker — the caller builds ghost rows fresh, never off a real row that had one"
        );
        let (style, trailing_style) = self.row_emphasis(entry, y, area, columns, buf);

        self.draw_marker_slot(entry, y, columns, style, buf);
        self.draw_branch(entry, y, columns, style, buf);
        // The gap after the branch column carries the row's emphasis too.
        draw_cell(
            buf,
            columns.branch_x + columns.branch_width,
            y,
            columns.subject_x - columns.branch_x - columns.branch_width,
            "",
            style,
        );
        self.draw_message(entry, y, columns, style, buf);
        self.draw_who_age(entry, y, columns, trailing_style, buf);
    }

    /// The marker slot, which holds two marks that **do not queue** and the
    /// gap that keeps them apart: the cursor's rule ([`row_rule`]) takes the
    /// first cell, the Mistake Radar's glyph the second, and the third stays
    /// blank so neither ever touches the subject.
    ///
    /// The rule used to lose that contest outright, so on any row carrying a
    /// `⚠` the cursor fell back to the background fill alone — which is the
    /// signal m5-history-and-ghost-preview/35 exists because the user could
    /// not see. Found by taking `docs/shots/m5-ui-dragging-80x24.txt`, not by
    /// reading the code. Giving the rule the first cell then spent the gap
    /// instead (`▏⚠Merge branch 'feature'`), so the slot is three cells wide
    /// now and [`RULE_WIDTH`] is reserved on every row — see
    /// [`MARKER_WIDTH`](super::MARKER_WIDTH).
    /// Reserving it rather than taking it only when a rule draws is what
    /// keeps the glyph in the same column on a cursor row and its neighbour.
    ///
    /// The marker's text is the bare glyph and the no-marker case is the
    /// empty string, because [`draw_cell`] pads to the width it is given:
    /// a `"⚠ "` or `"  "` literal would *truncate*, to `…`, on the narrow
    /// terminals where the slot is clamped below its full width.
    fn draw_marker_slot(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        style: Style,
        buf: &mut Buffer,
    ) {
        let rule_width = RULE_WIDTH.min(columns.marker_width);
        let (rule, rule_style) =
            match row_rule(entry.line.paint, entry.compare_marked, self.view.ascii) {
                Some(rule) => (
                    rule,
                    style
                        .patch(self.ink(entry, self.theme.token(Token::Accent)))
                        .add_modifier(Modifier::BOLD),
                ),
                // Still drawn, so the cell carries the row's own emphasis
                // rather than whatever the buffer held before it.
                None => ("", style),
            };
        draw_cell(buf, columns.marker_x, y, rule_width, rule, rule_style);
        let (glyph, glyph_style) = match self.marker_glyph(entry) {
            Some((glyph, token)) => (glyph, self.ink(entry, self.theme.token(token))),
            None => ("", style),
        };
        draw_cell(
            buf,
            columns.marker_x.saturating_add(rule_width),
            y,
            columns.marker_width.saturating_sub(rule_width),
            glyph,
            glyph_style,
        );
    }

    /// The marker slot's glyph and colour token for this row's Mistake Radar
    /// marker, or `None` when there is none to draw — either the row carries
    /// no finding, or the column has not yet earned its keep this session
    /// (`GraphView::marker_column`). ASCII (`!`/`i`) mirrors the same
    /// `view.ascii` switch the graph column itself draws with, rather than a
    /// second one (badges have none to mirror: they carry no ASCII form).
    fn marker_glyph(&self, entry: &GraphEntry<'_>) -> Option<(&'static str, Token)> {
        if !self.view.marker_column {
            return None;
        }
        let marker = entry.marker?;
        // The bare glyph: `draw_cell` pads it out to whatever the slot has
        // left, which since m5-history-and-ghost-preview/35 is one cell on
        // a row that also carries the cursor's rule and two otherwise. A
        // trailing space baked in here truncated to `…` in the one-cell
        // case.
        Some(match (marker.severity, self.view.ascii) {
            (Severity::Warning, false) => ("⚠", Token::Warning),
            (Severity::Warning, true) => ("!", Token::Warning),
            (Severity::Info, false) => ("ⓘ", Token::TextMuted),
            (Severity::Info, true) => ("i", Token::TextMuted),
        })
    }
}

/// Draws `text`, clamped and padded to exactly `width` cells, at `(x, y)`.
/// Does nothing when `width` is zero — the column has dropped for this
/// area's size.
pub(super) fn draw_cell(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let text = fit(text, usize::from(width));
    buf.set_stringn(x, y, text, usize::from(width), style);
}

/// Whether `entry` is drawn on the `selection` fill: the cursor row, or the
/// row a dragged badge would land on.
fn raised(entry: &GraphEntry<'_>) -> bool {
    matches!(
        entry.line.paint,
        RowPaint::Highlighted | RowPaint::DropTarget
    )
}

/// The accent mark the marker slot draws for this row, or `None` when the
/// slot is the row's own to leave blank.
///
/// **ADR-0058 moved the cursor's mark out of the slot**: the cursor is `▸`
/// in the gutter ([`GraphScreen::draw_gutter`]), and the slot keeps `▏` for
/// the pinned compare base only — on the cursor row too, so a cursor resting
/// on the compare base still shows it. A drop target draws `→` (ASCII `=`),
/// so a cursor and a drop target on screen together never share a glyph
/// (m5-history-and-ghost-preview/35's rule, ADR-0058 §5).
///
/// History, kept because the reason still holds for the gutter glyph:
/// **a shape, at every colour depth** (m5-history-and-ghost-preview/35).
/// The cursor row used to be marked by `Modifier::BOLD` plus a background
/// one step off the pane's own — `#14161B` → `#232730` in the dark theme —
/// and nothing else, with the `▏` rule drawn only where the terminal had no
/// background to give (`ColorDepth::Palette16` and `NO_COLOR`). Both were
/// applied, and both were correct, and the user still could not find his own
/// cursor: *"really and I mean really hard to notice which row I'm on."* A
/// six-percent lightness step is a real attribute and an unreadable signal,
/// and a test that asserts `bg == SurfaceRaised` cannot tell the difference.
///
/// So the rule is unconditional now. It costs the two cells the slot already
/// reserves, it survives every theme, every colour depth and a screenshot in
/// black and white, and it is the one cue that does not depend on the
/// terminal's palette being kind.
///
/// The marks, distinct **by shape**, so two on screen at once still read as
/// two different things:
///
/// | Unicode | ASCII | where | |
/// |---|---|---|---|
/// | `▸` | `>` | gutter | the cursor row |
/// | `▏` | `\|` | slot | the pinned compare base (`x`) |
/// | `→` | `=` | slot | the row a dragged badge would land on |
///
/// The ASCII column follows [`GraphScreen::marker_glyph`]'s switch on the
/// same `view.ascii` flag.
///
/// The mark takes the **first** of the slot's cells and a Mistake Radar
/// marker the second, so a row can carry both (`▏⚠`). `Ghost` and `Dimmed`
/// rows draw no drop mark: a ghost is not somewhere a badge can land, and a
/// dimmed row is one pushed back.
fn row_rule(paint: RowPaint, compare_marked: bool, ascii: bool) -> Option<&'static str> {
    let compare = if ascii { "|" } else { "\u{258f}" };
    match paint {
        RowPaint::DropTarget => Some(if ascii { "=" } else { "\u{2192}" }),
        RowPaint::Highlighted | RowPaint::Normal | RowPaint::Dimmed | RowPaint::Ghost => {
            compare_marked.then_some(compare)
        }
    }
}
