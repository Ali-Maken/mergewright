//! The pane widget itself, and its body region: message text, the diff (in
//! either mode), or the Explain panel in place of files+body.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use graft_core::mistake_radar::Severity;

use crate::detail::{
    BodyView, DetailFocus, DetailView, DiffMode, ExplainView, FileRowKind, FileStatusView,
    SubmoduleLogView, SubmoduleView,
};
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
    if let FileStatusView::Submodule(submodule) = &diff.status {
        // The inside facts come from git's status, which the row carries;
        // the diff job reads only the pointer's move. Line 2 explains the
        // row's marks, so it takes them from the row under the cursor
        // (m7-power-tools/30i).
        let mut submodule = submodule.as_ref().clone();
        if let Some(row) = cursor_submodule(view) {
            submodule.changed_inside = row.changed_inside;
            submodule.untracked_inside = row.untracked_inside;
        }
        render_submodule_diff(diff.path.as_str(), &submodule, area, theme, buf);
        return;
    }
    // An untracked file is read as all added lines (m6-conflicts/81); only
    // one with no bytes has nothing to draw. Binary goes to the line below.
    if diff.untracked && diff.hunks.is_empty() && diff.binary.is_none() {
        draw_line(buf, area, 0, "empty file", muted);
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
        cursor: view.body_cursor,
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

/// The most commits the diff pane lists for one submodule move; the rest
/// are counted in a `+N more` line (m7-power-tools/13).
pub(crate) const SUBMODULE_COMMITS_MAX: usize = 50;

/// The diff pane on a submodule path (m7-power-tools/13): the header
/// `<path> · submodule · <old7> → <new7>`, then one row per commit — `+`
/// pulled in, `−` rewound — capped at [`SUBMODULE_COMMITS_MAX`], or the one
/// line saying the commits cannot be listed. Never a content diff.
fn render_submodule_diff(
    path: &str,
    submodule: &SubmoduleView,
    area: Rect,
    theme: &Theme,
    buf: &mut Buffer,
) {
    let text = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    let ids = pointer_ids(submodule);
    draw_line(
        buf,
        area,
        0,
        &format!("{path} \u{b7} submodule{ids}"),
        foreground(theme, Token::Accent),
    );
    // Line 2, only when true: what the submodule's own tree holds — the
    // words behind the row's `±` / `?` marks (m7-power-tools/30i).
    let inside = submodule_inside_words(submodule);
    let first: u16 = if inside.is_some() { 2 } else { 1 };
    if let Some(inside) = inside {
        draw_line(buf, area, 1, &inside, text);
    }
    match &submodule.log {
        Some(SubmoduleLogView::NotHere) => draw_line(
            buf,
            area,
            first,
            "the submodule's commits aren't here \u{b7} not set up",
            muted,
        ),
        Some(SubmoduleLogView::Commits(commits)) if commits.is_empty() => {
            let why = if submodule.move_words(false).is_some() {
                "no commits to list"
            } else {
                "the pointer did not move"
            };
            draw_line(buf, area, first, why, muted);
        }
        Some(SubmoduleLogView::Commits(commits)) => {
            let mut offset: u16 = first;
            for commit in commits.iter().take(SUBMODULE_COMMITS_MAX) {
                let (sign, token) = if commit.pulled_in {
                    ('+', Token::Added)
                } else {
                    ('\u{2212}', Token::Removed)
                };
                draw_line(
                    buf,
                    area,
                    offset,
                    &format!("{sign} {} {}", commit.id.as_str(), commit.subject.as_str()),
                    foreground(theme, token),
                );
                offset = offset.saturating_add(1);
            }
            let more = commits.len().saturating_sub(SUBMODULE_COMMITS_MAX);
            if more > 0 {
                draw_line(buf, area, offset, &format!("+{more} more"), muted);
            }
        }
        None => draw_line(buf, area, first, "reading\u{2026}", text),
    }
}

/// The header's pointer after `submodule`: ` · <old7> → <new7>`, ` · new
/// at <id>`, ` · removed (was <id>)`, ` · <id>` unmoved, or nothing.
fn pointer_ids(submodule: &SubmoduleView) -> String {
    match (&submodule.from, &submodule.to) {
        (Some(from), Some(to)) if from != to => {
            format!(" \u{b7} {} \u{2192} {}", from.as_str(), to.as_str())
        }
        (None, Some(to)) => format!(" \u{b7} new at {}", to.as_str()),
        (Some(from), None) => format!(" \u{b7} removed (was {})", from.as_str()),
        (Some(at), Some(_)) => format!(" \u{b7} {}", at.as_str()),
        (None, None) => String::new(),
    }
}

/// The submodule facts of the file row under the cursor, if it is one.
fn cursor_submodule(view: &DetailView) -> Option<&SubmoduleView> {
    let files = view.files.as_ref()?;
    match &files.rows.get(files.cursor)?.kind {
        FileRowKind::File {
            status: FileStatusView::Submodule(row),
            ..
        } => Some(row),
        _ => None,
    }
}

/// The diff pane's line 2 on a submodule (m7-power-tools/30i): each true
/// fact about the submodule's own tree, joined by ` · ` — `changed inside`,
/// `untracked inside`. `None` when neither is true (no line, no blank).
pub(crate) fn submodule_inside_words(view: &SubmoduleView) -> Option<String> {
    let facts: Vec<&str> = [
        (view.changed_inside, "changed inside"),
        (view.untracked_inside, "untracked inside"),
    ]
    .into_iter()
    .filter_map(|(true_, words)| true_.then_some(words))
    .collect();
    (!facts.is_empty()).then(|| facts.join(" \u{b7} "))
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
