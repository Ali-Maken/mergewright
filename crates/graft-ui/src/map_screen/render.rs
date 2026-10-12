//! Drawing the Map view: [`MapScreen`]'s [`Widget`] impl and every helper it
//! calls. Split out of `map_screen.rs` (m2 review M8) — the parent module
//! keeps the view model ([`MapView`]/[`MapRowView`]/[`MapMarkView`]/
//! [`MapStatus`]) and [`MapScreen`] itself; this file is only how they draw.

use graft_core::capabilities::ColorDepth;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use super::{MapMarkView, MapRowView, MapScreen, MapStatus};
use crate::fit::fit;
use crate::graph_screen::hints::{fit_place_footer, place_close, place_footer_text};
use crate::keymap::ActionId;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

mod footer;

/// Cells the `▸`/blank cursor prefix takes.
const PREFIX_WIDTH: u16 = 2;
/// Cells a row's name is fitted to.
const NAME_WIDTH: u16 = 18;
/// The gap between the name column and the bar.
const GAP: u16 = 1;

/// Unicode density ramp (DESIGN.md §5.1): 8 non-blank levels.
const UNICODE_RAMP: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
/// ASCII density ramp: a blank, then 9 non-blank levels.
const ASCII_RAMP: [char; 10] = [' ', '.', ':', '-', '=', '+', '*', '#', '%', '@'];

/// The density glyph for `count` against `max`: blank for zero, otherwise
/// never blank — the ramp's first non-blank level or higher, ceiling-rounded
/// so a lone commit against a busy branch still shows *something*.
#[must_use]
pub(crate) fn ramp_glyph(count: u32, max: u32, ascii: bool) -> char {
    if count == 0 {
        return ' ';
    }
    let denom = u64::from(max.max(count));
    if ascii {
        let levels = (ASCII_RAMP.len() - 1) as u64;
        let level = (u64::from(count) * levels).div_ceil(denom);
        ASCII_RAMP[usize::try_from(level.clamp(1, levels)).unwrap_or(1)]
    } else {
        let levels = UNICODE_RAMP.len() as u64;
        let level = (u64::from(count) * levels).div_ceil(denom);
        UNICODE_RAMP[usize::try_from(level.clamp(1, levels) - 1).unwrap_or(0)]
    }
}

/// The header's span wording: `last N days` below 60 days, else `last N
/// months` (nearest month, at least one) — see the module doc's
/// undocumented-choices note.
fn span_words(days: u32) -> String {
    if days >= 60 {
        let months = ((days + 15) / 30).max(1);
        format!("last {months} month{}", if months == 1 { "" } else { "s" })
    } else {
        let days = days.max(1);
        format!("last {days} day{}", if days == 1 { "" } else { "s" })
    }
}

/// The severity's glyph and colour token (mirrors
/// [`GraphEntry::marker`](crate::graph_screen::MarkerView)'s convention).
fn radar_glyph(severity: graft_core::mistake_radar::Severity, ascii: bool) -> (char, Token) {
    use graft_core::mistake_radar::Severity;
    match (severity, ascii) {
        (Severity::Warning, false) => ('⚠', Token::Warning),
        (Severity::Warning, true) => ('!', Token::Warning),
        (Severity::Info, false) => ('ⓘ', Token::TextMuted),
        (Severity::Info, true) => ('i', Token::TextMuted),
    }
}

/// Writes `text`, clipped to what is left of `area` from `x`; a no-op past
/// the area's edge.
fn set_text(buf: &mut Buffer, x: u16, y: u16, area: Rect, text: &str, style: Style) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    let width = area.right() - x;
    buf.set_stringn(x, y, text, usize::from(width), style);
}

/// Where the bar starts: past the cursor prefix, the name column and its
/// gap.
fn bar_start_x(area: Rect) -> u16 {
    area.x
        .saturating_add(PREFIX_WIDTH)
        .saturating_add(NAME_WIDTH)
        .saturating_add(GAP)
}

/// A message centred in `area`, one line tall.
fn draw_centered(buf: &mut Buffer, area: Rect, text: &str, style: Style) {
    if area.height == 0 || area.width == 0 {
        return;
    }
    let y = area.y + area.height / 2;
    let text_width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
    let x = area.x + area.width.saturating_sub(text_width) / 2;
    set_text(buf, x, y, area, text, style);
}

impl MapScreen<'_> {
    /// Bars and marks drop the branch/severity colour at 16 colours and
    /// under `NO_COLOR`: the same rule `graph_column` uses to keep lanes
    /// legible without relying on colour alone (m1 12-colour rule).
    fn reduced_color(&self) -> bool {
        matches!(
            self.theme.color_depth(),
            ColorDepth::Palette16 | ColorDepth::None
        )
    }

    fn header_line(&self) -> Line<'static> {
        let text = format!(
            "MAP · {} · {}",
            self.view.title_branch,
            span_words(self.view.span_days)
        );
        Line::styled(text, foreground(self.theme, Token::Text))
    }

    fn draw_column_header(&self, buf: &mut Buffer, area: Rect) {
        let muted = foreground(self.theme, Token::TextMuted);
        let label_x = area.x.saturating_add(PREFIX_WIDTH);
        set_text(buf, label_x, area.y, area, "BRANCH", muted);

        let bar_start = bar_start_x(area);
        let buckets = u16::try_from(self.view.buckets).unwrap_or(u16::MAX);
        let now_width: u16 = 3;
        let now_x = bar_start.saturating_add(buckets.saturating_sub(now_width));
        set_text(buf, now_x, area.y, area, "NOW", muted);
    }

    fn draw_rows(&self, buf: &mut Buffer, area: Rect) {
        let bar_start = bar_start_x(area);
        for (index, row) in self.view.rows.iter().enumerate() {
            let y = area
                .y
                .saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            if y >= area.bottom() {
                break;
            }
            self.draw_row(buf, area, y, bar_start, index, row);
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is the caller's to say; a struct for one caller would only rename them"
    )]
    fn draw_row(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        bar_start: u16,
        index: usize,
        row: &MapRowView,
    ) {
        let selected = index == self.view.selected;
        let reduced = self.reduced_color();
        let mut base_style = foreground(self.theme, Token::Text);
        if selected {
            base_style = base_style.add_modifier(Modifier::BOLD);
            if !reduced {
                let raised = color(self.theme.token(Token::Selection));
                buf.set_style(
                    Rect {
                        y,
                        height: 1,
                        ..area
                    },
                    Style::new().bg(raised),
                );
            }
        }

        let prefix = if row.is_current { "▸ " } else { "  " };
        set_text(buf, area.x, y, area, prefix, base_style);

        let name = fit(row.name.as_str(), usize::from(NAME_WIDTH));
        set_text(
            buf,
            area.x.saturating_add(PREFIX_WIDTH),
            y,
            area,
            &name,
            base_style,
        );

        let branch_style = if reduced {
            base_style
        } else {
            base_style.fg(color(self.theme.branch(row.identity)))
        };
        self.draw_bar(
            buf,
            area,
            y,
            bar_start,
            row,
            base_style,
            branch_style,
            reduced,
        );

        let buckets_u16 = u16::try_from(self.view.buckets).unwrap_or(u16::MAX);
        let bar_end = bar_start.saturating_add(buckets_u16).min(area.right());
        let after_tags = draw_tag_annotations(buf, area, y, bar_end, row, self.theme);
        draw_row_suffix(buf, area, y, after_tags, row, self.theme);
        if selected && !reduced {
            // Every foreground on the fill — name, bar, marks, tags, suffix
            // — lifted to stay legible on it (ADR-0058 §3).
            crate::paint::lift_on_selection(
                buf,
                Rect {
                    y,
                    height: 1,
                    ..area
                },
                self.theme,
            );
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "each argument is the caller's to say; a struct for one caller would only rename them"
    )]
    fn draw_bar(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        bar_start: u16,
        row: &MapRowView,
        base_style: Style,
        branch_style: Style,
        reduced: bool,
    ) {
        let elbow_bucket = row.merged_into.map(|(_, bucket)| bucket);
        for bucket in 0..self.view.buckets {
            let x = bar_start.saturating_add(u16::try_from(bucket).unwrap_or(u16::MAX));
            if x >= area.right() {
                break;
            }
            let count = row.counts.get(bucket).copied().unwrap_or(0);
            let (mut glyph, mut style) = (
                ramp_glyph(count, self.view.max_count, self.view.ascii),
                branch_style,
            );

            if let Some((_, mark)) = row.marks.iter().find(|(b, _)| *b == bucket) {
                (glyph, style) = self.mark_cell(mark, base_style, reduced);
            }
            if elbow_bucket == Some(bucket) {
                (glyph, style) = (if self.view.ascii { 'o' } else { '●' }, branch_style);
            } else if elbow_bucket.map(|b| b + 1) == Some(bucket) {
                (glyph, style) = (if self.view.ascii { '\\' } else { '╮' }, branch_style);
            }

            let mut cell = [0u8; 4];
            set_text(buf, x, y, area, glyph.encode_utf8(&mut cell), style);
        }
    }

    fn mark_cell(&self, mark: &MapMarkView, base_style: Style, reduced: bool) -> (char, Style) {
        match mark {
            MapMarkView::Radar(severity) => {
                let (radar, token) = radar_glyph(*severity, self.view.ascii);
                let style = if reduced {
                    base_style
                } else {
                    base_style.fg(color(self.theme.token(token)))
                };
                (radar, style)
            }
            MapMarkView::Tag(_) => (
                if self.view.ascii { '+' } else { '┴' },
                foreground(self.theme, Token::TextMuted),
            ),
        }
    }
}

/// Draws each tag's `[<name>]` annotation right of the bar, skipping any
/// that would not fit; returns the x past the last one drawn (or `bar_end`
/// if none fit).
fn draw_tag_annotations(
    buf: &mut Buffer,
    area: Rect,
    y: u16,
    bar_end: u16,
    row: &MapRowView,
    theme: &Theme,
) -> u16 {
    let mut x = bar_end;
    for (_, mark) in &row.marks {
        if let MapMarkView::Tag(name) = mark {
            let text = format!(" [{name}]");
            let width = u16::try_from(text.chars().count()).unwrap_or(u16::MAX);
            if x.saturating_add(width) > area.right() {
                continue;
            }
            set_text(buf, x, y, area, &text, foreground(theme, Token::TextMuted));
            x = x.saturating_add(width);
        }
    }
    x
}

/// Draws the current-row ` ●` or active-row ` (active)` suffix, mutually
/// exclusive, starting at `x`.
fn draw_row_suffix(buf: &mut Buffer, area: Rect, y: u16, x: u16, row: &MapRowView, theme: &Theme) {
    if row.is_current {
        let accent = foreground(theme, Token::Accent).add_modifier(Modifier::BOLD);
        set_text(buf, x, y, area, " ●", accent);
    } else if row.active {
        set_text(
            buf,
            x,
            y,
            area,
            " (active)",
            foreground(theme, Token::TextMuted),
        );
    }
}

impl Widget for MapScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        buf.set_line(area.x, area.y, &self.header_line(), area.width);
        if area.height == 1 {
            return;
        }
        self.draw_column_header(
            buf,
            Rect {
                y: area.y.saturating_add(1),
                height: 1,
                ..area
            },
        );
        if area.height == 2 {
            return;
        }

        let footer_y = area.bottom() - 1;
        let body_top = area.y.saturating_add(2);
        let body = Rect {
            y: body_top,
            height: footer_y.saturating_sub(body_top),
            ..area
        };
        match &self.view.status {
            MapStatus::Loading { done, total } => {
                draw_centered(
                    buf,
                    body,
                    &format!("building map… {done}/{total} branches"),
                    foreground(self.theme, Token::Text),
                );
            }
            MapStatus::Empty => {
                draw_centered(
                    buf,
                    body,
                    "no local branches",
                    foreground(self.theme, Token::TextMuted),
                );
            }
            MapStatus::Failed(line) => {
                draw_centered(
                    buf,
                    body,
                    line.as_str(),
                    foreground(self.theme, Token::Warning),
                );
            }
            MapStatus::Ready => self.draw_rows(buf, body),
        }

        buf.set_line(area.x, footer_y, &self.footer_line(area.width), area.width);
    }
}
