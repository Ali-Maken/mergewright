//! The Map view (`M`, DESIGN.md §5.1): one line per local branch, plotted as
//! a density sparkline against a shared time axis (`map-view-screen`,
//! m2-details-and-diffs/19).
//!
//! Draws whatever [`crate::graft::view::map::map_view`] (a later crate)
//! builds from [`crate::graft::map::MapData`] — this module owns no I/O and
//! no state, only the widget.
//!
//! Text glyphs at every tier in M2 (design note, Proposed changes): the
//! pixel tier's real density curve is out of scope here (M8 polish gap).
//! Mouse is also out of scope (M2 gap row) — every interaction here is a
//! key, `update::map::map_key`'s.
//!
//! **Undocumented choices, not fully pinned by the design note:**
//! - The header's day/month wording switches at 60 days (`span_words`):
//!   below it reads `last N days`, at or above `last N months` (rounded to
//!   the nearest month, at least one).
//! - The ASCII tier's tag mark (`┴` in Unicode) draws `+`, mirroring
//!   `graph_column`'s ASCII junction glyph — the design note gives no ASCII
//!   form for it.
//! - When a Radar mark and a Tag mark land in the same bucket (both name
//!   the same bucket index), the later one in `MapRowView::marks` wins the
//!   glyph; unspecified by the design note, and rare (a tag and a finding on
//!   the same commit).

use graft_core::capabilities::ColorDepth;
use graft_core::mistake_radar::Severity;
use graft_graph::identity::BranchIdentity;
use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

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

/// A mark on one bucket of one row, once sanitised for drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapMarkView {
    /// A tag's short name.
    Tag(SanitizedText),
    /// The loudest Mistake Radar finding in the bucket.
    Radar(Severity),
}

/// One row: a branch's density sparkline, ready to draw.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapRowView {
    /// The branch's name, sanitised.
    pub name: SanitizedText,
    /// Lane colour: the same identity the main graph's lanes use.
    pub identity: BranchIdentity,
    /// Commits per bucket; [`MapView::buckets`] long.
    pub counts: Vec<u32>,
    /// `HEAD` is on this branch: prefixed `▸`, suffixed ` ●`.
    pub is_current: bool,
    /// `Some((row, bucket))`: this branch merged into the row at `row`, its
    /// own line ending at `bucket` — where the elbow (`●╮`) draws.
    pub merged_into: Option<(usize, usize)>,
    /// Tag and Radar marks, by bucket.
    pub marks: Vec<(usize, MapMarkView)>,
    /// The tip is in the newest bucket and this row is not `merged_into`
    /// anything: draws ` (active)`. Never set on the current row (it draws
    /// ` ●` instead).
    pub active: bool,
    /// This row's own walk did not hit its cap; `false` colours nothing here
    /// — only [`MapView::complete`] draws the footer's ` · partial`.
    pub complete: bool,
}

/// What the Map view is doing right now, in place of its rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MapStatus {
    /// Still walking branches: `done` of `total` have a row so far.
    Loading {
        /// Rows built so far.
        done: u32,
        /// Rows the build expects to visit.
        total: u32,
    },
    /// No local branches at all.
    Empty,
    /// The build failed; the line to show, already sanitised.
    Failed(SanitizedText),
    /// Rows are ready to draw.
    Ready,
}

/// Everything the Map view draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MapView {
    /// The branch named in the header: current if there is one, else the
    /// default branch.
    pub title_branch: SanitizedText,
    /// The shared time span's width, in days, for the header's wording.
    pub span_days: u32,
    /// Rows, in the order they draw (current first, then default, then the
    /// rest by tip date).
    pub rows: Vec<MapRowView>,
    /// The largest bucket count over every row: what a bar's glyphs scale
    /// against.
    pub max_count: u32,
    /// Columns every row's bar has.
    pub buckets: usize,
    /// The row under the cursor.
    pub selected: usize,
    /// Local branches beyond what `rows` holds; the footer's `+N more`.
    pub more: usize,
    /// `false` if any row's walk hit its cap; the footer's ` · partial`.
    pub complete: bool,
    /// What to draw instead of, or alongside, the rows.
    pub status: MapStatus,
    /// Draw bars and marks with the ASCII glyph table rather than Unicode
    /// block density (`graph.renderer`, mirroring [`GraphView::ascii`]).
    ///
    /// [`GraphView::ascii`]: crate::graph_screen::GraphView::ascii
    pub ascii: bool,
}

/// The Map view, ready to render.
#[derive(Clone, Copy, Debug)]
pub struct MapScreen<'a> {
    /// What to draw.
    pub view: &'a MapView,
    /// The theme to draw it in.
    pub theme: &'a Theme,
}

/// The density glyph for `count` against `max`: blank for zero, otherwise
/// never blank — the ramp's first non-blank level or higher, ceiling-rounded
/// so a lone commit against a busy branch still shows *something*.
#[must_use]
pub fn ramp_glyph(count: u32, max: u32, ascii: bool) -> char {
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
fn radar_glyph(severity: Severity, ascii: bool) -> (char, Token) {
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

    fn footer_line(&self) -> Line<'static> {
        let muted = foreground(self.theme, Token::TextMuted);
        if let MapStatus::Loading { .. } = self.view.status {
            return Line::styled("Esc cancels".to_owned(), muted);
        }
        let mut spans = vec![Span::styled(
            "↑↓ select  ⏎ zoom to commits  M/Esc back".to_owned(),
            muted,
        )];
        if self.view.more > 0 {
            spans.push(Span::styled(format!(" · +{} more", self.view.more), muted));
        }
        if !self.view.complete {
            spans.push(Span::styled(
                " · partial".to_owned(),
                foreground(self.theme, Token::Warning),
            ));
        }
        Line::from(spans)
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

    #[allow(clippy::too_many_arguments)]
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
    }

    #[allow(clippy::too_many_arguments)]
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

        buf.set_line(area.x, footer_y, &self.footer_line(), area.width);
    }
}

#[cfg(test)]
mod tests {
    use graft_core::capabilities::ColorDepth;
    use graft_term::sanitize_line;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::test_support::terminal as test_capabilities;
    use crate::theme::ThemeKind;

    fn draw(view: &MapView, columns: u16, rows: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(columns, rows))
            .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
        let capabilities = test_capabilities(ColorDepth::TrueColor, false);
        let theme = Theme::new(ThemeKind::Dark, &capabilities);
        terminal
            .draw(|frame| {
                frame.render_widget(
                    MapScreen {
                        view,
                        theme: &theme,
                    },
                    frame.area(),
                );
            })
            .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
        terminal.backend().to_string()
    }

    fn row(name: &str, identity: u8, counts: Vec<u32>) -> MapRowView {
        MapRowView {
            name: sanitize_line(name),
            identity: BranchIdentity::of_name(&[identity]),
            counts,
            is_current: false,
            merged_into: None,
            marks: Vec::new(),
            active: false,
            complete: true,
        }
    }

    fn sample_view(ascii: bool) -> MapView {
        let mut main = row("main", 1, vec![0, 1, 2, 7, 0, 0, 3, 0, 1, 0]);
        main.is_current = true;
        let mut feature = row("feature/auth", 2, vec![0, 0, 3, 4, 0, 0, 0, 0, 0, 0]);
        feature.merged_into = Some((0, 3));
        let mut release = row("release/2.3", 3, vec![0, 0, 0, 0, 0, 0, 0, 1, 2, 0]);
        release.marks = vec![
            (7, MapMarkView::Tag(sanitize_line("v1.0"))),
            (8, MapMarkView::Radar(Severity::Warning)),
        ];
        MapView {
            title_branch: sanitize_line("main"),
            span_days: 40,
            rows: vec![main, feature, release],
            max_count: 7,
            buckets: 10,
            selected: 0,
            more: 0,
            complete: true,
            status: MapStatus::Ready,
            ascii,
        }
    }

    #[test]
    fn ramp_glyph_zero_is_blank_never_for_a_nonzero_count() {
        assert_eq!(ramp_glyph(0, 1000, false), ' ');
        assert_eq!(ramp_glyph(0, 1000, true), ' ');
        assert_eq!(ramp_glyph(1000, 1000, false), '█');
        assert_eq!(ramp_glyph(1000, 1000, true), '@');
        assert_eq!(ramp_glyph(1, 1000, false), '▁');
        assert_eq!(ramp_glyph(1, 1000, true), '.');
        for count in 1..=1000u32 {
            assert_ne!(ramp_glyph(count, 1000, false), ' ');
            assert_ne!(ramp_glyph(count, 1000, true), ' ');
        }
    }

    #[test]
    fn ready_snapshot_80x24_unicode() {
        insta::assert_snapshot!(draw(&sample_view(false), 80, 24));
    }

    #[test]
    fn ready_snapshot_80x24_ascii() {
        insta::assert_snapshot!(draw(&sample_view(true), 80, 24));
    }

    #[test]
    fn ready_snapshot_120x40_unicode() {
        insta::assert_snapshot!(draw(&sample_view(false), 120, 40));
    }

    #[test]
    fn loading_snapshot() {
        let mut view = sample_view(false);
        view.status = MapStatus::Loading { done: 2, total: 5 };
        insta::assert_snapshot!(draw(&view, 80, 24));
    }

    #[test]
    fn empty_snapshot() {
        let mut view = sample_view(false);
        view.rows = Vec::new();
        view.status = MapStatus::Empty;
        insta::assert_snapshot!(draw(&view, 80, 24));
    }

    #[test]
    fn failed_snapshot() {
        let mut view = sample_view(false);
        view.status = MapStatus::Failed(sanitize_line("git for-each-ref: not a repository"));
        insta::assert_snapshot!(draw(&view, 80, 24));
    }
}
