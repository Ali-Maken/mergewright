//! Drawing the rewrite editor's right side and message: the after-state, the changes, the message block.

use super::{
    AfterMark, AfterRow, AfterView, Buffer, ChangesView, Glyphs, MessageBlock, Modifier, Rect,
    RewriteEditorView, RightSide, SHORT_HEIGHT, SHORT_MESSAGE_LINES, SHORT_WIDTH, SUBJECT_MAX,
    SUBJECT_MIN, SanitizedText, Style, Theme, Token, band, fit, foreground, put,
};

/// One drawn line: its pieces, each in its own style.
pub(super) type Spans = Vec<(String, Style)>;

/// A landed AFTER plan: one line per row (branch, mark, subject, note),
/// then, after a blank, its facts; a fact starting `!` is a warning.
pub(super) fn after_lines(
    rows: &[AfterRow],
    facts: &[SanitizedText],
    theme: &Theme,
    glyphs: &Glyphs,
) -> Vec<Spans> {
    let muted = foreground(theme, Token::TextMuted);
    let branch_width = rows
        .iter()
        .filter_map(|row| row.branch.as_ref())
        .map(|branch| branch.as_str().chars().count())
        .max()
        .unwrap_or(0)
        + 2;
    let subject_width = rows
        .iter()
        .map(|row| row.subject.as_str().chars().count())
        .max()
        .unwrap_or(0)
        .clamp(SUBJECT_MIN, SUBJECT_MAX)
        + 2;
    let mut lines = Vec::new();
    for row in rows {
        let branch = row.branch.as_ref().map_or("", SanitizedText::as_str);
        let (mark, mark_style) = match &row.mark {
            AfterMark::NewTip => (
                format!("{} new", glyphs.new_tip),
                foreground(theme, Token::Accent).add_modifier(Modifier::BOLD),
            ),
            AfterMark::New => (
                format!("{} new", glyphs.new),
                foreground(theme, Token::Info),
            ),
            AfterMark::Unchanged { short } => (short.clone(), muted),
        };
        let subject_style = if matches!(row.mark, AfterMark::Unchanged { .. }) {
            muted
        } else {
            foreground(theme, Token::Info)
        };
        lines.push(vec![
            (fit(branch, branch_width), foreground(theme, Token::Text)),
            (fit(&mark, SHORT_WIDTH), mark_style),
            (fit(row.subject.as_str(), subject_width), subject_style),
            (
                row.note
                    .as_ref()
                    .map_or_else(String::new, |note| note.as_str().to_owned()),
                muted,
            ),
        ]);
    }
    if !facts.is_empty() {
        lines.push(Vec::new());
    }
    for fact in facts {
        let style = if fact.as_str().starts_with('!') {
            foreground(theme, Token::Warning)
        } else {
            foreground(theme, Token::Text)
        };
        lines.push(vec![(fact.as_str().to_owned(), style)]);
    }
    lines
}

/// A row's changed files: path, `+added`, `-removed`, the paths padded to
/// one column no wider than `width` leaves room for.
pub(super) fn changes_lines(
    files: &[(SanitizedText, Option<(u32, u32)>)],
    width: u16,
    theme: &Theme,
    glyphs: &Glyphs,
) -> Vec<Spans> {
    let path_width = files
        .iter()
        .map(|(path, _)| path.as_str().chars().count())
        .max()
        .unwrap_or(0)
        .min(usize::from(width).saturating_sub(16))
        + 2;
    files
        .iter()
        .map(|(path, counts)| {
            let path = (
                fit(path.as_str(), path_width),
                foreground(theme, Token::Text),
            );
            let Some((added, removed)) = counts else {
                return vec![
                    path,
                    ("binary".to_owned(), foreground(theme, Token::TextMuted)),
                ];
            };
            vec![
                path,
                (format!("+{added}"), foreground(theme, Token::Added)),
                (" ".to_owned(), foreground(theme, Token::TextMuted)),
                (
                    format!("{}{removed}", glyphs.minus),
                    foreground(theme, Token::Removed),
                ),
            ]
        })
        .collect()
}

/// The right side: its header, a blank, its lines.
pub(super) fn draw_right(
    buf: &mut Buffer,
    right: Rect,
    view: &RewriteEditorView,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let header_style = foreground(theme, Token::Accent).add_modifier(Modifier::BOLD);
    let muted = foreground(theme, Token::TextMuted);
    let warning = foreground(theme, Token::Warning);
    let mut lines: Vec<Spans> = Vec::new();
    let header = match &view.right {
        RightSide::After(AfterView::Checking) => "AFTER y \u{b7} checking\u{2026}".to_owned(),
        RightSide::After(_) => "AFTER y \u{b7} not written yet".to_owned(),
        RightSide::Changes { short, .. } => {
            format!("CHANGES of {short} \u{b7} \u{23ce} back to AFTER")
        }
    };
    lines.push(vec![(header, header_style)]);
    lines.push(Vec::new());
    match &view.right {
        RightSide::After(AfterView::Checking) => {}
        RightSide::After(AfterView::Failed(words) | AfterView::Refused(words)) => {
            lines.push(vec![(words.as_str().to_owned(), warning)]);
        }
        RightSide::After(AfterView::Ready { rows, facts }) => {
            lines.extend(after_lines(rows, facts, theme, glyphs));
        }
        RightSide::Changes { files, .. } => match files {
            ChangesView::Reading => lines.push(vec![("reading\u{2026}".to_owned(), muted)]),
            ChangesView::Failed(words) => lines.push(vec![(words.as_str().to_owned(), warning)]),
            ChangesView::Files(files) if files.is_empty() => {
                lines.push(vec![("no files changed".to_owned(), muted)]);
            }
            ChangesView::Files(files) => {
                lines.extend(changes_lines(files, right.width, theme, glyphs));
            }
        },
    }
    for (offset, spans) in lines.iter().take(usize::from(right.height)).enumerate() {
        let y = right.y + u16::try_from(offset).unwrap_or(u16::MAX);
        let line = band(right, y, 1);
        let mut x = line.x;
        for (text, style) in spans {
            x = put(buf, line, x, text, *style);
        }
    }
}

/// The message block: a blank, its label, then each line behind `┊`; on a
/// short screen [`SHORT_MESSAGE_LINES`] lines and `…`, and whatever the
/// room, a cut ends in `…`.
pub(super) fn draw_message(
    buf: &mut Buffer,
    at: Rect,
    block: &MessageBlock,
    area: Rect,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let muted = foreground(theme, Token::TextMuted);
    let label_style = foreground(theme, Token::Text).add_modifier(Modifier::BOLD);
    if at.height < 2 {
        return;
    }
    put(
        buf,
        band(at, at.y + 1, 1),
        at.x + 1,
        block.label.as_str(),
        label_style,
    );
    let room = usize::from(at.height - 2);
    let cap = if area.height <= SHORT_HEIGHT {
        SHORT_MESSAGE_LINES
    } else {
        usize::MAX
    };
    let fits = block.lines.len() <= room && block.lines.len() <= cap;
    let shown = if fits {
        block.lines.len()
    } else {
        room.saturating_sub(1).min(cap)
    };
    let mut y = at.y + 2;
    for line in block.lines.iter().take(shown) {
        let row = band(at, y, 1);
        let x = put(buf, row, row.x + 1, glyphs.gutter, muted);
        put(
            buf,
            row,
            x + 1,
            line.as_str(),
            foreground(theme, Token::Text),
        );
        y += 1;
    }
    if !fits && y < at.bottom() {
        let row = band(at, y, 1);
        let x = put(buf, row, row.x + 1, glyphs.gutter, muted);
        put(buf, row, x + 1, glyphs.more, muted);
    }
}
