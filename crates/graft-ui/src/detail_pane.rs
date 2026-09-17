//! The detail pane's regions, breakpoints and states.
//!
//! Draws a [`DetailView`] (`crate::detail`, view-model only) beside, below or
//! in place of the graph, per DESIGN.md §1.1-1.3's breakpoints. This is the
//! *plain* diff renderer: [`diff_body`] paints one row per line, sign glyph
//! and soft background only, no gutter, no pinned heading, no word/syntax
//! tint. `diff-widget` (m2-details-and-diffs) replaces its body under the
//! same signature.
//!
//! Deviation from the ticket's literal signature: [`DetailPane`] also carries
//! `theme` (not listed in the ticket's `Signatures` block). Without it the
//! widget has no colour source at render time — every other widget in this
//! crate (`GraphScreen`, `StatusScreen`) is handed a `&Theme` the same way.
//! Reported as a spec gap rather than silently worked around.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::SanitizedText;

use crate::detail::{
    BodyView, ContainedIn, DetailFocus, DetailHeader, DetailView, FileRow, FileRowKind,
    FileStatusView, HunkView, LineKindView, LineView, RadarLine, Subject,
};
use crate::fit::fit;
use crate::paint::{color, foreground};
use crate::theme::{Theme, Token};

/// Where the detail pane sits, per DESIGN.md §1.1-1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailLayout {
    /// 160+ columns: a right column.
    Right {
        /// Column width, 42% of the screen width, min 48.
        columns: u16,
    },
    /// 100..160 columns: a strip below the graph.
    Below {
        /// Strip height, 40% of the screen height, min 8.
        rows: u16,
    },
    /// Under 100 columns: a `[Graph] Detail` tab strip.
    Tab {
        /// Follows `focus != DetailFocus::Graph` (no separate "which tab"
        /// state).
        shown: bool,
    },
}

impl DetailLayout {
    const RIGHT_BREAKPOINT: u16 = 160;
    const BELOW_BREAKPOINT: u16 = 100;
    const RIGHT_MIN: u16 = 48;
    const BELOW_MIN: u16 = 8;

    /// The layout for a screen of `width` by `height` cells. `override_cells`
    /// (a divider drag or `<`/`>`, `detail-keys-and-focus`) replaces the
    /// percentage figure, still clamped to the breakpoint's minimum.
    #[must_use]
    pub fn compute(
        width: u16,
        height: u16,
        focus: DetailFocus,
        override_cells: Option<u16>,
    ) -> DetailLayout {
        if width >= Self::RIGHT_BREAKPOINT {
            let pct = percent(width, 42);
            DetailLayout::Right {
                columns: override_cells.unwrap_or(pct).max(Self::RIGHT_MIN),
            }
        } else if width >= Self::BELOW_BREAKPOINT {
            let pct = percent(height, 40);
            DetailLayout::Below {
                rows: override_cells.unwrap_or(pct).max(Self::BELOW_MIN),
            }
        } else {
            DetailLayout::Tab {
                shown: focus != DetailFocus::Graph,
            }
        }
    }
}

/// `cells * pct / 100`, saturating rather than overflowing for any `u16`.
fn percent(cells: u16, pct: u32) -> u16 {
    u16::try_from(u32::from(cells) * pct / 100).unwrap_or(u16::MAX)
}

/// The pane's three regions: header (fixed rows), files (a list) and body
/// (message or diff).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailAreas {
    /// Fixed-height rows at the top: subject, author, parents, contained,
    /// radar (as many as the breakpoint keeps).
    pub header: Rect,
    /// The file list.
    pub files: Rect,
    /// The message or diff.
    pub body: Rect,
}

/// Split `pane_area` into [`DetailAreas`] for `layout`. `files_count` caps
/// how much of the files region the file list actually needs (`Right`/`Tab`
/// only): it never grows past that even when there is room, and never goes
/// negative when there are no files.
#[must_use]
pub fn detail_areas(pane_area: Rect, layout: DetailLayout, files_count: usize) -> DetailAreas {
    let header_rows = match layout {
        DetailLayout::Right { .. } => 5,
        DetailLayout::Below { .. } => 2,
        DetailLayout::Tab { .. } => 3,
    };
    let header = Rect {
        height: header_rows.min(pane_area.height),
        ..pane_area
    };
    let rest = Rect {
        y: pane_area.y.saturating_add(header.height),
        height: pane_area.height.saturating_sub(header.height),
        ..pane_area
    };

    if matches!(layout, DetailLayout::Below { .. }) {
        let files_width = 30.min(rest.width);
        let files = Rect {
            width: files_width,
            ..rest
        };
        let body = Rect {
            x: rest.x.saturating_add(files_width),
            width: rest.width.saturating_sub(files_width),
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    } else {
        let cap = percent(rest.height, 40);
        let files_height = cap
            .min(u16::try_from(files_count).unwrap_or(u16::MAX))
            .min(rest.height);
        let files = Rect {
            height: files_height,
            ..rest
        };
        let body = Rect {
            y: rest.y.saturating_add(files_height),
            height: rest.height.saturating_sub(files_height),
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    }
}

/// A row `diff_body` draws: a hunk's own heading, or one of its lines.
enum DiffRow<'a> {
    Heading(&'a SanitizedText),
    Line(&'a LineView),
}

fn flatten(hunks: &[HunkView]) -> Vec<DiffRow<'_>> {
    let mut rows = Vec::new();
    for hunk in hunks {
        rows.push(DiffRow::Heading(&hunk.heading));
        rows.extend(hunk.lines.iter().map(DiffRow::Line));
    }
    rows
}

/// Draw `hunks` from content row `top`, plainly: one screen row per line,
/// sign glyph and kind soft background only, text fit to width. No gutter
/// numbers, no pinned heading, no word/syntax tint, no `␍`, no truncation
/// footer — `diff-widget` replaces this function's body under the same
/// signature to add those. Returns the total content rows, for scroll
/// clamping.
#[must_use]
pub fn diff_body(
    hunks: &[HunkView],
    top: usize,
    area: Rect,
    buf: &mut Buffer,
    theme: &Theme,
    focused: bool,
) -> usize {
    let rows = flatten(hunks);
    if area.width == 0 || area.height == 0 {
        return rows.len();
    }
    let palette16 = theme.color_depth() == ColorDepth::Palette16;
    let style_ctx = DiffRowStyle { focused, palette16 };
    for (offset, row) in rows
        .iter()
        .skip(top)
        .take(usize::from(area.height))
        .enumerate()
    {
        let y = area
            .y
            .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
        draw_diff_row(row, area.x, y, area.width, buf, theme, style_ctx);
    }
    rows.len()
}

/// The flags [`draw_diff_row`] needs beyond the row itself and the theme,
/// bundled to keep the function under clippy's argument-count limit.
#[derive(Clone, Copy)]
struct DiffRowStyle {
    /// The diff region has the keys.
    focused: bool,
    /// The 16-colour rule is in force: bold/dim, no background tint.
    palette16: bool,
}

fn draw_diff_row(
    row: &DiffRow<'_>,
    x: u16,
    y: u16,
    width: u16,
    buf: &mut Buffer,
    theme: &Theme,
    style_ctx: DiffRowStyle,
) {
    let (sign, kind_token, text) = match row {
        DiffRow::Heading(heading) => (' ', None, heading.as_str().to_string()),
        DiffRow::Line(line) => {
            let (sign, token) = match line.kind {
                LineKindView::Added => ('+', Some(Token::Added)),
                LineKindView::Removed => ('-', Some(Token::Removed)),
                LineKindView::Context | LineKindView::HunkHeading => (' ', None),
            };
            (sign, token, line.text.as_str().to_string())
        }
    };

    let style = if !style_ctx.focused || matches!(row, DiffRow::Heading(_)) {
        foreground(theme, Token::TextMuted)
    } else if let Some(token) = kind_token {
        if style_ctx.palette16 {
            let modifier = if token == Token::Added {
                ratatui::style::Modifier::BOLD
            } else {
                ratatui::style::Modifier::DIM
            };
            foreground(theme, token).add_modifier(modifier)
        } else {
            Style::new()
                .fg(color(theme.token(Token::Text)))
                .bg(color(theme.token(token)))
        }
    } else {
        foreground(theme, Token::Text)
    };

    let line = format!("{sign} {text}");
    let fitted = fit(&line, usize::from(width));
    buf.set_stringn(x, y, fitted, usize::from(width), style);
}

/// The detail pane widget: header, files and body, drawn for one frame.
pub struct DetailPane<'a> {
    /// What to draw.
    pub view: &'a DetailView,
    /// Where the pane's regions land.
    pub layout: &'a DetailLayout,
    /// Not in the ticket's literal signature; see the module doc.
    pub theme: &'a Theme,
}

impl Widget for DetailPane<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let files_count = self.view.files.as_ref().map_or(0, |files| files.rows.len());
        let areas = detail_areas(area, *self.layout, files_count);
        render_header(self.view, *self.layout, areas.header, self.theme, buf);
        render_files(self.view, areas.files, self.theme, buf);
        render_body(self.view, areas.body, self.theme, buf);
    }
}

fn draw_line(buf: &mut Buffer, area: Rect, offset: u16, text: &str, style: Style) {
    if offset >= area.height || area.width == 0 {
        return;
    }
    let y = area.y.saturating_add(offset);
    let fitted = fit(text, usize::from(area.width));
    buf.set_stringn(area.x, y, fitted, usize::from(area.width), style);
}

fn title_text(subject: &Subject) -> String {
    match subject {
        Subject::None => String::new(),
        Subject::Commit { sha } => format!("COMMIT {}", sha.as_str()),
        Subject::Compare { base, head } => {
            format!("COMPARE {} \u{2192} {}", base.as_str(), head.as_str())
        }
        Subject::WorkingTree {
            staged,
            unstaged,
            untracked,
        } => {
            let mut parts = Vec::new();
            if *staged > 0 {
                parts.push(format!("{staged} staged"));
            }
            if *unstaged > 0 {
                parts.push(format!("{unstaged} unstaged"));
            }
            if *untracked > 0 {
                parts.push(format!("{untracked} untracked"));
            }
            if parts.is_empty() {
                "UNCOMMITTED".to_string()
            } else {
                format!("UNCOMMITTED \u{b7} {}", parts.join(" "))
            }
        }
    }
}

fn radar_text(line: &RadarLine) -> String {
    match line {
        RadarLine::Finding { title, .. } => title.as_str().to_string(),
        RadarLine::EvilUnchecked => "evil merge: unchecked".to_string(),
        RadarLine::EvilChecking => "evil merge: checking\u{2026}".to_string(),
        RadarLine::EvilClean => "evil merge: clean".to_string(),
        RadarLine::EvilNotCheckable(reason) => format!("evil merge: {}", reason.as_str()),
        RadarLine::ForcePushNotCheckable(reason) => format!("force-push: {}", reason.as_str()),
    }
}

/// The author line: `name <email> · date · relative`, plus `committed by`
/// and `with:` (co-authors) when the header carries them.
fn author_line(header: &DetailHeader) -> String {
    let mut line = String::new();
    if !header.author.as_str().is_empty() {
        line.push_str(header.author.as_str());
        line.push_str(" \u{b7} ");
        line.push_str(header.author_date.as_str());
        line.push_str(" \u{b7} ");
        line.push_str(header.relative.as_str());
    }
    if let Some(committer) = &header.committer {
        line.push_str(" \u{b7} committed by ");
        line.push_str(committer.as_str());
    }
    if !header.co_authors.is_empty() {
        line.push_str(" \u{b7} with: ");
        let names: Vec<&str> = header
            .co_authors
            .iter()
            .map(SanitizedText::as_str)
            .collect();
        line.push_str(&names.join(", "));
    }
    line
}

/// The header's five canonical lines (title, author, parents/against,
/// contained, radar), before a narrower breakpoint compresses them.
fn header_lines(view: &DetailView) -> [String; 5] {
    let header = &view.header;
    let title = title_text(&header.subject);

    if let Subject::WorkingTree { .. } = header.subject {
        let against = header.against.as_ref().map_or("", |sha| sha.as_str());
        return [
            title,
            format!("diff vs {against}"),
            String::new(),
            String::new(),
            String::new(),
        ];
    }

    let author_line = author_line(header);

    let parents_line = if header.parents.is_empty() {
        header.against.as_ref().map_or(String::new(), |against| {
            format!("diff vs {}", against.as_str())
        })
    } else {
        let names: Vec<&str> = header.parents.iter().map(SanitizedText::as_str).collect();
        format!("parents {} \u{b7} diff vs first parent", names.join(", "))
    };

    let contained_line = match &header.contained {
        ContainedIn::NotAsked => String::new(),
        ContainedIn::Checking => "Contained in: checking\u{2026}".to_string(),
        ContainedIn::Known {
            branches,
            tags,
            more,
        } => {
            let mut names: Vec<&str> = branches.iter().map(SanitizedText::as_str).collect();
            names.extend(tags.iter().map(SanitizedText::as_str));
            let mut line = format!("Contained in: {}", names.join(", "));
            if *more > 0 {
                line.push_str(&format!(" +{more} more"));
            }
            line
        }
        ContainedIn::Failed(message) => format!("Contained in: {}", message.as_str()),
    };

    let radar_line = header.radar.first().map(radar_text).unwrap_or_default();

    [title, author_line, parents_line, contained_line, radar_line]
}

fn title_style(view: &DetailView, theme: &Theme) -> Style {
    if view.focus == DetailFocus::Graph {
        foreground(theme, Token::TextMuted)
    } else {
        foreground(theme, Token::Accent)
    }
}

fn render_header(
    view: &DetailView,
    layout: DetailLayout,
    area: Rect,
    theme: &Theme,
    buf: &mut Buffer,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let lines = header_lines(view);
    let is_working_tree = matches!(view.header.subject, Subject::WorkingTree { .. });
    let meta = [&lines[1], &lines[2], &lines[3]]
        .into_iter()
        .filter(|line| !line.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" \u{b7} ");
    let mut subject_radar = view.header.summary.as_str().to_string();
    if !lines[4].is_empty() {
        subject_radar.push_str(" \u{b7} ");
        subject_radar.push_str(&lines[4]);
    }

    let displayed: Vec<String> = match layout {
        DetailLayout::Right { .. } => lines.to_vec(),
        DetailLayout::Below { .. } => {
            if is_working_tree {
                vec![lines[0].clone(), lines[1].clone()]
            } else {
                let meta_line = if meta.is_empty() {
                    lines[0].clone()
                } else {
                    format!("{} \u{b7} {meta}", lines[0])
                };
                vec![meta_line, subject_radar]
            }
        }
        DetailLayout::Tab { .. } => {
            if is_working_tree {
                vec![lines[0].clone(), lines[1].clone(), String::new()]
            } else {
                vec![lines[0].clone(), meta, subject_radar]
            }
        }
    };

    let title_style = title_style(view, theme);
    let text_style = foreground(theme, Token::Text);
    for (index, line) in displayed.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        let style = if index == 0 { title_style } else { text_style };
        draw_line(buf, area, offset, line, style);
    }
}

fn status_letter(status: FileStatusView) -> char {
    match status {
        FileStatusView::Added => 'A',
        FileStatusView::Deleted => 'D',
        FileStatusView::Modified => 'M',
        FileStatusView::Renamed => 'R',
        FileStatusView::Copied => 'C',
        FileStatusView::TypeChanged => 'T',
        FileStatusView::Unmerged => 'U',
        FileStatusView::Untracked => '?',
    }
}

fn file_row_text(row: &FileRow, width: usize) -> String {
    let indent = "  ".repeat(usize::from(row.depth));
    let (left, right) = match &row.kind {
        FileRowKind::Message { lines } => {
            (format!("\u{2709} message ({lines} lines)"), String::new())
        }
        FileRowKind::Dir { collapsed } => {
            let marker = if *collapsed { "\u{25b8}" } else { "\u{25be}" };
            (
                format!("{indent}{marker} {}", row.name.as_str()),
                String::new(),
            )
        }
        FileRowKind::File {
            status,
            added,
            removed,
        } => {
            let letter = status_letter(*status);
            let counts = match (added, removed) {
                (Some(added), Some(removed)) => format!("+{added} \u{2212}{removed}"),
                _ => "bin".to_string(),
            };
            (format!("{indent}{letter} {}", row.name.as_str()), counts)
        }
    };
    if right.is_empty() {
        left
    } else {
        let gap = width
            .saturating_sub(left.chars().count() + right.chars().count())
            .max(1);
        format!("{left}{}{right}", " ".repeat(gap))
    }
}

fn render_files(view: &DetailView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let Some(files) = view.files.as_ref() else {
        return;
    };
    let text_style = if view.focus == DetailFocus::Files {
        foreground(theme, Token::Accent)
    } else {
        foreground(theme, Token::TextMuted)
    };
    let cursor_bg = color(theme.token(Token::SurfaceRaised));

    for (offset, (index, row)) in (0_u16..).zip(files.visible().skip(files.top)) {
        if offset >= area.height {
            break;
        }
        let style = if index == files.cursor {
            text_style.bg(cursor_bg)
        } else {
            text_style
        };
        let text = file_row_text(row, usize::from(area.width));
        draw_line(buf, area, offset, &text, style);
    }
}

fn render_body(view: &DetailView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let focused = view.focus == DetailFocus::Diff;
    let muted = foreground(theme, Token::TextMuted);
    let warning = foreground(theme, Token::Warning);
    let text_style = if focused {
        foreground(theme, Token::Text)
    } else {
        muted
    };

    match &view.body {
        BodyView::Empty(text) => draw_line(buf, area, 0, text.as_str(), muted),
        BodyView::Loading(text) => draw_line(buf, area, 0, text.as_str(), text_style),
        BodyView::Failed(text) => draw_line(buf, area, 0, text.as_str(), warning),
        BodyView::Message { lines } => {
            for (index, line) in lines.iter().enumerate() {
                let Ok(offset) = u16::try_from(index) else {
                    break;
                };
                draw_line(buf, area, offset, line.as_str(), text_style);
            }
        }
        BodyView::Diff(diff) => {
            if diff.untracked {
                draw_line(buf, area, 0, "untracked \u{b7} no diff yet (M3)", muted);
            } else if let Some(binary) = &diff.binary {
                draw_line(
                    buf,
                    area,
                    0,
                    &format!("binary \u{b7} {}", binary.as_str()),
                    muted,
                );
            } else {
                let _rows = diff_body(&diff.hunks, view.body_top, area, buf, theme, focused);
            }
        }
    }
}

#[cfg(test)]
mod tests;
