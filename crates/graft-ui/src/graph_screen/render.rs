//! Drawing: the [`Widget`] entry point, the rows, and their badges.

use graft_core::capabilities::ColorDepth;
use graft_core::mistake_radar::Severity;
use graft_graph::fold::Fold;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use super::layout::{Columns, graph_width};
use super::{Badge, BadgeKind, GraphEntry, GraphScreen, Icons, SUBJECT_GAP, badges};
use crate::fit::fit;
use crate::graph_column::{GraphColumn, GraphLine, RowPaint};
use crate::paint::{color, foreground};
use crate::theme::Token;

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
        let (body, rows, applied) = self.body_layout(area);
        let folded_lanes = applied.as_ref().map_or(0, Fold::folded_count);
        if chrome.header.height > 0 {
            buf.set_line(
                area.x,
                chrome.header.y,
                &self.header(width, folded_lanes),
                area.width,
            );
        }
        for (offset, line) in self
            .view
            .banner
            .iter()
            .take(usize::from(chrome.banner.height))
            .enumerate()
        {
            let y = chrome
                .banner
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

        if body.height > 0 {
            self.body(body, rows, applied.as_ref(), buf);
        }

        if chrome.footer.height > 0 {
            let (text, token) = self.footer_line(body, rows, applied.as_ref(), width);
            let footer = Line::styled(fit(&text, width), foreground(self.theme, token));
            buf.set_line(area.x, chrome.footer.y, &footer, area.width);
        }
        if let Some(line) = self.view.commit_graph
            && chrome.offer.height > 0
        {
            buf.set_line(
                area.x,
                chrome.offer.y,
                &self.commit_graph_line(line, width),
                area.width,
            );
        }
    }
}

impl GraphScreen<'_> {
    /// The rows: graph, the reserved marker slot, the subject, and — when
    /// there is room — the author chip, the relative date and the short SHA.
    fn body(&self, area: Rect, rows: &[GraphEntry<'_>], fold: Option<&Fold>, buf: &mut Buffer) {
        let graph_width = graph_width(area, rows, fold);
        // At the pixel tier the cells stay blank: text drawn there would sit
        // on top of the terminal's images.
        if !self.view.pixel {
            let graph_lines: Vec<GraphLine<'_>> =
                rows.iter().map(|entry| entry.line.clone()).collect();
            let mut column = GraphColumn::new(&graph_lines, self.theme).ascii(self.view.ascii);
            if let Some(fold) = fold {
                column = column.through(fold);
            }
            column.render(
                Rect {
                    width: graph_width,
                    ..area
                },
                buf,
            );
        }

        let columns = Columns::compute(area, graph_width);
        for (entry, y) in rows.iter().zip(area.top()..) {
            self.draw_row(entry, y, area, &columns, buf);
        }
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
        let (style, trailing_style, palette16_edge) = self.row_emphasis(entry, y, area, buf);

        let (glyph, glyph_style) = if let Some((glyph, token)) = self.marker_glyph(entry) {
            (glyph, foreground(self.theme, token))
        } else if entry.compare_marked || palette16_edge {
            // The pinned compare base (`x`, m2-details-and-diffs/11) draws
            // the same accent `▏` the Palette16 fallback uses, unconditional
            // of colour depth: it is a mark the user set, not a substitute
            // for a background this terminal can't show.
            (
                "▏ ",
                foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
            )
        } else {
            ("  ", style)
        };
        draw_cell(
            buf,
            columns.marker_x,
            y,
            columns.marker_width,
            glyph,
            glyph_style,
        );
        let taken = self.draw_badges(entry, y, columns, style, buf);
        draw_cell(
            buf,
            columns.subject_x + taken,
            y,
            columns.subject_width - taken,
            entry.subject.as_str(),
            style,
        );
        let author_style = trailing_style.fg(color(self.theme.branch(entry.author.identity)));
        draw_cell(
            buf,
            columns.author_x,
            y,
            columns.author_width,
            entry.author.initials.as_str(),
            author_style,
        );
        draw_cell(
            buf,
            columns.when_x,
            y,
            columns.when_width,
            &entry.when,
            trailing_style,
        );
        draw_cell(
            buf,
            columns.sha_x,
            y,
            columns.sha_width,
            &entry.sha,
            trailing_style,
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
        Some(match (marker.severity, self.view.ascii) {
            (Severity::Warning, false) => ("⚠ ", Token::Warning),
            (Severity::Warning, true) => ("! ", Token::Warning),
            (Severity::Info, false) => ("ⓘ ", Token::TextMuted),
            (Severity::Info, true) => ("i ", Token::TextMuted),
        })
    }

    /// The text style, the trailing-column style, and whether the row falls
    /// back to the Palette16 rule instead of a background fill: `Highlighted`
    /// paints bold and, where the terminal can show it, a raised background;
    /// `Dimmed` paints faint; anything else is plain.
    ///
    /// At [`ColorDepth::Palette16`], `bg`/`surface`/`surface-raised` collapse
    /// to the same index (KNOWN_GAPS.md M0), so the raised background step
    /// would paint nothing a viewer can see — the marker slot's rule
    /// (DESIGN.md §4.6) carries the highlight there instead, so the
    /// background step is skipped rather than drawn for nothing. `NO_COLOR`
    /// ([`ColorDepth::None`]) has no background step either, so it takes the
    /// same rule — uncoloured, its shape alone marks the row.
    fn row_emphasis(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        area: Rect,
        buf: &mut Buffer,
    ) -> (Style, Style, bool) {
        let mut style = foreground(self.theme, Token::Text);
        let mut trailing_style = foreground(self.theme, Token::TextMuted);
        let highlighted = entry.line.paint == RowPaint::Highlighted;
        let palette16_edge = highlighted
            && matches!(
                self.theme.color_depth(),
                ColorDepth::Palette16 | ColorDepth::None
            );

        if highlighted {
            style = style.add_modifier(Modifier::BOLD);
            trailing_style = trailing_style.add_modifier(Modifier::BOLD);
            if !palette16_edge {
                let raised = color(self.theme.token(Token::SurfaceRaised));
                buf.set_style(
                    Rect {
                        y,
                        height: 1,
                        ..area
                    },
                    Style::new().bg(raised),
                );
            }
        } else if entry.line.paint == RowPaint::Dimmed {
            style = style.add_modifier(Modifier::DIM);
            trailing_style = trailing_style.add_modifier(Modifier::DIM);
        }

        (style, trailing_style, palette16_edge)
    }
}

impl GraphScreen<'_> {
    /// The row's ref badges, left of the subject, and how many cells they
    /// took (their trailing gap included).
    ///
    /// Badges never take more than half the subject column: at 80 columns a
    /// commit with a pile of refs still says what it is. What does not fit is
    /// cut with `…` rather than pushing the subject off the row.
    fn draw_badges(
        &self,
        entry: &GraphEntry<'_>,
        y: u16,
        columns: &Columns,
        style: Style,
        buf: &mut Buffer,
    ) -> u16 {
        let icons = self.view.icons;
        for (span, index) in badge_spans(entry, columns, icons) {
            let badge = &entry.badges[index];
            draw_cell(
                buf,
                span.start,
                y,
                span.end - span.start,
                &badge.text(icons),
                style.patch(self.badge_style(badge)),
            );
        }
        badges_width(entry, icons, columns)
    }

    /// The colour and weight one badge draws in: branches and remotes in
    /// their lane's colour, so a badge and its line read as one thing.
    fn badge_style(&self, badge: &Badge) -> Style {
        match &badge.kind {
            BadgeKind::Branch {
                identity, current, ..
            } => {
                let style = Style::new().fg(color(self.theme.branch(*identity)));
                if *current {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                }
            }
            BadgeKind::Remote { identity } => Style::new().fg(color(self.theme.branch(*identity))),
            BadgeKind::Tag => foreground(self.theme, Token::Warning),
            BadgeKind::Head => foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD),
            BadgeKind::Stash => foreground(self.theme, Token::Info),
        }
    }
}

/// The screen columns each of `entry`'s badges occupies, as drawn: one
/// `(range, index)` per badge that had room, `index` naming its place in
/// [`GraphEntry::badges`]. Ascending, never overlapping, all inside the
/// subject column's badge share.
///
/// **This is the packing rule itself, not a description of it.**
/// [`GraphScreen::draw_badges`] draws from exactly these ranges, and
/// `graft`'s mouse hit-test reads the same call to say which badge a cell
/// belongs to (`graft::view::hit::badge_at`). A second, mirrored copy of the
/// packing in the app crate is what this function exists to prevent: the
/// `detail_pane` mirror in `graft::update::geometry` is a standing drift
/// risk, and one was enough (m5/10 decision).
///
/// Pure arithmetic over the badges' own text widths — no buffer, no theme,
/// no `Rect`.
#[must_use]
pub fn badge_spans(
    entry: &GraphEntry<'_>,
    columns: &Columns,
    icons: Icons,
) -> Vec<(std::ops::Range<u16>, usize)> {
    if entry.badges.is_empty() {
        return Vec::new();
    }
    let separator = u16::try_from(badges::separator(icons).width()).unwrap_or(u16::MAX);
    // Room for the badges themselves; the gap after them stays blank.
    let room = badges_width(entry, icons, columns).saturating_sub(SUBJECT_GAP);
    let end = columns.subject_x + room;

    let mut spans = Vec::with_capacity(entry.badges.len());
    let mut x = columns.subject_x;
    for (index, badge) in entry.badges.iter().enumerate() {
        if index > 0 {
            x = x.saturating_add(separator).min(end);
        }
        let width = u16::try_from(badge.text(icons).width())
            .unwrap_or(u16::MAX)
            .min(end - x);
        if width == 0 {
            break;
        }
        spans.push((x..x + width, index));
        x += width;
    }
    spans
}

/// Cells a row's badges take from the subject column's left edge, the
/// trailing gap included — half the subject column at most. The same sum
/// [`GraphScreen::draw_badges`] draws to, so it and the footer reveal's
/// truncation check ([`subject_truncated`]) always agree on where the
/// subject itself actually starts.
fn badges_width(entry: &GraphEntry<'_>, icons: Icons, columns: &Columns) -> u16 {
    if entry.badges.is_empty() {
        return 0;
    }
    let texts: Vec<String> = entry.badges.iter().map(|badge| badge.text(icons)).collect();
    let separator = badges::separator(icons);
    let wanted = texts.iter().map(|text| text.width()).sum::<usize>()
        + separator.width() * (texts.len() - 1)
        + usize::from(SUBJECT_GAP);
    let budget = columns.subject_width / 2;
    u16::try_from(wanted).unwrap_or(u16::MAX).min(budget)
}

/// Whether `entry`'s subject needed a `…` to fit the subject column it is
/// actually drawn into — badges' share subtracted first, the same as
/// [`GraphScreen::draw_badges`]. What the footer reveal
/// ([`GraphView::reveal`]) keys off.
pub(super) fn subject_truncated(entry: &GraphEntry<'_>, icons: Icons, columns: &Columns) -> bool {
    let taken = badges_width(entry, icons, columns);
    let available = usize::from(columns.subject_width.saturating_sub(taken));
    super::layout::text_width(entry.subject.as_str()) > available
}

/// Draws `text`, clamped and padded to exactly `width` cells, at `(x, y)`.
/// Does nothing when `width` is zero — the column has dropped for this
/// area's size.
fn draw_cell(buf: &mut Buffer, x: u16, y: u16, width: u16, text: &str, style: Style) {
    if width == 0 {
        return;
    }
    let text = fit(text, usize::from(width));
    buf.set_stringn(x, y, text, usize::from(width), style);
}
