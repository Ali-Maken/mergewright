//! Drawing the rewrite editor: the widget, its glyphs and the commit rows.

use super::{
    ACTION_WIDTH, Buffer, FieldWidget, Frame, InputView, LEAD, Modifier, Rect, RewriteEditorView,
    RowLine, RowMark, SHORT_WIDTH, SUBJECT_MAX, SUBJECT_MIN, Style, Theme, Token, Widget, band,
    draw_message, draw_right, fit, foreground, layout, row_lines,
};

/// Draw the editor into `area` (the whole frame but the hint bar).
pub fn draw_rewrite_editor(frame: &mut Frame, area: Rect, view: &RewriteEditorView, theme: &Theme) {
    frame.render_widget(RewriteEditorWidget { view, theme }, area);
}

/// The editor as a widget, for [`draw_rewrite_editor`] and tests.
#[derive(Debug)]
pub struct RewriteEditorWidget<'a> {
    /// What to draw.
    pub view: &'a RewriteEditorView,
    /// Colours.
    pub theme: &'a Theme,
}

impl Widget for RewriteEditorWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let view = self.view;
        let theme = self.theme;
        let glyphs = Glyphs::new(view.ascii);
        let text = foreground(theme, Token::Text).bg(crate::paint::color(theme.token(Token::Bg)));
        buf.set_style(area, text);
        let at = layout(view, area);

        if let Some(title) = at.title {
            put(
                buf,
                title,
                title.x,
                view.title.as_str(),
                foreground(theme, Token::Accent).add_modifier(Modifier::BOLD),
            );
        }
        if let Some(rows) = at.rows {
            draw_rows(buf, rows, at.top, view, theme, &glyphs);
        }
        if let Some(rule) = at.rule {
            for y in rule.y..rule.bottom() {
                buf.set_string(rule.x, y, glyphs.rule, foreground(theme, Token::Border));
            }
        }
        if let Some(right) = at.right {
            draw_right(buf, right, view, theme, &glyphs);
        }
        if let Some(message) = at.message
            && let Some(block) = &view.message
        {
            draw_message(buf, message, block, area, theme, &glyphs);
        }
        if let Some(notice) = &view.notice {
            put(
                buf,
                at.notice,
                at.notice.x + 1,
                notice.as_str(),
                foreground(theme, Token::Warning),
            );
        }
    }
}

/// The glyphs, Unicode or their ASCII stand-ins.
pub(super) struct Glyphs {
    pub(super) cursor: &'static str,
    pub(super) base: &'static str,
    pub(super) melds: &'static str,
    pub(super) reword: &'static str,
    pub(super) edit: &'static str,
    pub(super) drop: &'static str,
    pub(super) moved: &'static str,
    pub(super) rule: &'static str,
    pub(super) gutter: &'static str,
    pub(super) new_tip: &'static str,
    pub(super) new: &'static str,
    pub(super) more: &'static str,
    pub(super) minus: &'static str,
}

impl Glyphs {
    fn new(ascii: bool) -> Self {
        if ascii {
            Glyphs {
                cursor: ">",
                base: "-",
                melds: "v",
                reword: "*",
                edit: "||",
                drop: "x",
                moved: "<>",
                rule: "|",
                gutter: ":",
                new_tip: "@",
                new: "o",
                more: "...",
                minus: "-",
            }
        } else {
            Glyphs {
                cursor: "\u{25b8}",
                base: "\u{2500}",
                melds: "\u{2193}",
                reword: "\u{270e}",
                edit: "\u{23f8}",
                drop: "\u{2717}",
                moved: "\u{2195}",
                rule: "\u{2502}",
                gutter: "\u{250a}",
                new_tip: "\u{25cf}",
                new: "\u{25cb}",
                more: "\u{2026}",
                minus: "\u{2212}",
            }
        }
    }
}

/// Write `text` at `(x, band.y)`, clipped to the band's right edge.
/// Returns the column after it.
pub(super) fn put(buf: &mut Buffer, band: Rect, x: u16, text: &str, style: Style) -> u16 {
    if x >= band.right() || band.height == 0 {
        return x;
    }
    let room = usize::from(band.right() - x);
    let (end, _) = buf.set_stringn(x, band.y, text, room, style);
    end
}

/// The subject column's width: the longest subject, within bounds.
pub(super) fn subject_width(view: &RewriteEditorView) -> usize {
    view.rows
        .iter()
        .map(|row| row.subject.as_str().chars().count())
        .max()
        .unwrap_or(0)
        .clamp(SUBJECT_MIN, SUBJECT_MAX)
}

/// The action word and glyph, and its style.
pub(super) fn action(mark: &RowMark, glyphs: &Glyphs, theme: &Theme) -> (String, Style) {
    let strong = |token| foreground(theme, token).add_modifier(Modifier::BOLD);
    match mark {
        RowMark::Pick => ("pick".to_owned(), foreground(theme, Token::Text)),
        RowMark::Reword => (format!("reword {}", glyphs.reword), strong(Token::Accent)),
        RowMark::Edit => (format!("edit {}", glyphs.edit), strong(Token::Warning)),
        RowMark::Squash => (format!("squash {}", glyphs.melds), strong(Token::Info)),
        RowMark::Fixup => (format!("fixup {}", glyphs.melds), strong(Token::Info)),
        RowMark::Drop => (format!("drop {}", glyphs.drop), strong(Token::Removed)),
        RowMark::Base { label } => (
            label.as_str().to_owned(),
            foreground(theme, Token::TextMuted),
        ),
    }
}

/// The rows' region.
pub(super) fn draw_rows(
    buf: &mut Buffer,
    rows: Rect,
    top: usize,
    view: &RewriteEditorView,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let subject_width = subject_width(view);
    let lines = row_lines(view);
    for (offset, line) in lines
        .iter()
        .skip(top)
        .take(usize::from(rows.height))
        .enumerate()
    {
        let y = rows.y + u16::try_from(offset).unwrap_or(u16::MAX);
        let band = band(rows, y, 1);
        match *line {
            RowLine::Row(index) => {
                draw_row(buf, band, view, index, subject_width, theme, glyphs);
            }
            RowLine::Field => {
                if let Some(field) = &view.field {
                    let input = InputView {
                        label: "message",
                        text: field.text.clone(),
                        cursor: Some(field.caret),
                        guide_at: None,
                        counter: None,
                        focused: true,
                        secret: None,
                    };
                    let indent = u16::try_from(LEAD + ACTION_WIDTH).unwrap_or(0);
                    let at = Rect::new(
                        band.x + indent.min(band.width),
                        y,
                        band.width.saturating_sub(indent),
                        1,
                    );
                    FieldWidget {
                        view: &input,
                        theme,
                        ascii: view.ascii,
                    }
                    .render(at, buf);
                }
            }
        }
    }
}

/// One row: `▸ squash ↓  7f6e5d4  Handle expired tokens   melds into …`.
pub(super) fn draw_row(
    buf: &mut Buffer,
    band: Rect,
    view: &RewriteEditorView,
    index: usize,
    subject_width: usize,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let Some(row) = view.rows.get(index) else {
        return;
    };
    let is_base = matches!(row.mark, RowMark::Base { .. });
    let on_cursor = index == view.cursor;
    if on_cursor {
        buf.set_style(
            band,
            Style::new().bg(crate::paint::color(theme.token(Token::Selection))),
        );
    }
    let muted = foreground(theme, Token::TextMuted);
    let lead = if is_base {
        glyphs.base
    } else if on_cursor {
        glyphs.cursor
    } else {
        " "
    };
    let lead_style = if is_base {
        muted
    } else {
        foreground(theme, Token::Accent)
    };
    let mut x = put(buf, band, band.x + 1, lead, lead_style);
    x = x.max(band.x + u16::try_from(LEAD).unwrap_or(0));
    let (word, style) = action(&row.mark, glyphs, theme);
    put(buf, band, x, &fit(&word, ACTION_WIDTH), style);
    x += u16::try_from(ACTION_WIDTH).unwrap_or(0);
    put(buf, band, x, &fit(&row.short, SHORT_WIDTH), muted);
    x += u16::try_from(SHORT_WIDTH).unwrap_or(0);
    let subject_style = if is_base || row.mark == RowMark::Drop {
        muted
    } else {
        foreground(theme, Token::Text)
    };
    put(
        buf,
        band,
        x,
        &fit(row.subject.as_str(), subject_width + 2),
        subject_style,
    );
    x += u16::try_from(subject_width + 2).unwrap_or(0);
    // `↕ moved` leads: a long note is cut at its tail, never the carrier.
    let mut note = String::new();
    if row.moved {
        note.push_str(glyphs.moved);
        note.push_str(" moved");
    }
    if let Some(said) = &row.note {
        if !note.is_empty() {
            note.push_str(" \u{b7} ");
        }
        note.push_str(said.as_str());
    }
    if x < band.right() {
        let room = usize::from(band.right() - x);
        put(buf, band, x, &fit(&note, room), muted);
    }
}
