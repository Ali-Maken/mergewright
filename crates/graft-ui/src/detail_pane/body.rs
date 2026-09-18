//! The pane widget itself, and its body region: message text, the diff (in
//! either mode), or the Explain panel in place of files+body.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use graft_core::mistake_radar::Severity;

use crate::detail::{BodyView, DetailFocus, DetailView, DiffMode, ExplainView};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

use super::files::render_files;
use super::header::render_header;
use super::layout::{DetailLayout, detail_areas};
use super::line_render::{DiffFocus, draw_line};
use super::side_by_side::diff_body_side_by_side;
use super::unified::diff_body;

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
        if let Some(explain) = &self.view.explain {
            // Same area the files+body split otherwise occupies, at every
            // breakpoint (`m2-details-and-diffs/17`).
            render_explain(explain, areas.files.union(areas.body), self.theme, buf);
        } else {
            render_files(self.view, areas.files, self.theme, buf);
            render_body(self.view, areas.body, self.theme, buf);
        }
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
        BodyView::Diff(diff) => render_diff_body(view, diff, focused, area, theme, buf, muted),
    }
}

/// The [`BodyView::Diff`] arm of [`render_body`], split out to keep that
/// function under clippy's line-count limit.
fn render_diff_body(
    view: &DetailView,
    diff: &crate::detail::DiffView,
    focused: bool,
    area: Rect,
    theme: &Theme,
    buf: &mut Buffer,
    muted: Style,
) {
    if diff.untracked {
        draw_line(buf, area, 0, "untracked \u{b7} no diff yet (M3)", muted);
        return;
    }
    if let Some(binary) = &diff.binary {
        draw_line(
            buf,
            area,
            0,
            &format!("binary \u{b7} {}", binary.as_str()),
            muted,
        );
        return;
    }
    let focus = DiffFocus {
        focused,
        word_tint: view.options.word_tint,
    };
    let _rows = match view.mode {
        DiffMode::Unified => diff_body(
            &diff.hunks,
            view.body_top,
            diff.truncated,
            area,
            buf,
            theme,
            focus,
        ),
        DiffMode::SideBySide => diff_body_side_by_side(
            &diff.hunks,
            view.body_top,
            area,
            buf,
            theme,
            focus.focused,
            focus.word_tint,
        ),
    };
}

/// The Explain panel: title (severity glyph + `title`, severity colour),
/// paragraphs wrapped to width, a blank line, the sketch verbatim, footer
/// `Esc close` (`m2-details-and-diffs/17`; design note "Explain panel").
fn render_explain(explain: &ExplainView, area: Rect, theme: &Theme, buf: &mut Buffer) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let (glyph, severity_token) = match explain.severity {
        Severity::Warning => ("\u{26a0} ", Token::Warning),
        Severity::Info => ("\u{24d8} ", Token::TextMuted),
    };
    let title_style = foreground(theme, severity_token);
    let text_style = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    let width = usize::from(area.width);

    let mut offset: u16 = 0;
    draw_line(
        buf,
        area,
        offset,
        &format!("{glyph}{}", explain.title.as_str()),
        title_style,
    );
    offset += 1;

    'paragraphs: for paragraph in &explain.paragraphs {
        for line in wrap_to_width(paragraph.as_str(), width) {
            if offset >= area.height {
                break 'paragraphs;
            }
            draw_line(buf, area, offset, &line, text_style);
            offset += 1;
        }
    }

    offset += 1; // blank line between the paragraphs and the sketch.

    for line in &explain.sketch {
        if offset >= area.height {
            return;
        }
        draw_line(buf, area, offset, line.as_str(), text_style);
        offset += 1;
    }

    if offset < area.height {
        draw_line(buf, area, offset, "Esc close", muted);
    }
}

/// Greedy word wrap at `width` cells (`unicode-width`, matching `fit`'s own
/// notion of a cell). An empty `text` wraps to no lines.
fn wrap_to_width(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let extra = usize::from(!current.is_empty());
        if !current.is_empty() && current.width() + extra + word.width() > width {
            lines.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
