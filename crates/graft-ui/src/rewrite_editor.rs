//! The rewrite editor (`R`, m7-power-tools/08, note R1): his branch as rows,
//! newest first, git's letters on them, and beside (or below) them what the
//! branches will be after `y`, with the message the rewrite will write.
//!
//! Owns no state and reads no repository. `graft::update::rewrite_editor`
//! holds the state and `graft::view::rewrite_editor` turns it into the
//! [`RewriteEditorView`] this draws. The layout is one pure function over
//! the view and the area ([`layout`]); the drawing and the app's mouse
//! ([`rewrite_row_at`]) both ask it, so what is drawn and what a click lands
//! on cannot disagree.
//!
//! Every distinction is carried by **characters** (R1's distinctions table):
//! the action word and its glyph (`↓ ✎ ⏸ ✗`), `↕ moved`, `─` on the base
//! row, the `│` rule between now and after (≥180 columns), `○ new` / `● new`
//! on the AFTER rows, the `┊` gutter of the message. A capture records the
//! grid and nothing else (CLAUDE.md rule 5); colour ships unverified.
//!
//! Breakpoints (R1):
//! - ≥180: rows left ([`LEFT_WIDTH`]), `│`, AFTER the rest; the message
//!   under the rows.
//! - 120–179: rows full width; AFTER below them, labelled; the message
//!   below that.
//! - <120: rows only, or AFTER only while `Tab` has swapped to it; the
//!   message under the rows.
//!
//! The last row of the area is the notice row: a refusal in words, above
//! the app's hint bar.

use graft_term::SanitizedText;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{FieldView as InputView, FieldWidget};
use crate::theme::{Theme, Token};

#[cfg(test)]
mod tests;

/// From this width the AFTER side stands right of the rows, past a `│`.
pub const WIDE_MIN: u16 = 180;

/// Below this width the AFTER side is a card `Tab` swaps to.
pub const NARROW_BELOW: u16 = 120;

/// The rows' width at [`WIDE_MIN`] and above; the `│` is the next column.
pub const LEFT_WIDTH: u16 = 96;

/// At this many area rows or fewer the message shows
/// [`SHORT_MESSAGE_LINES`] lines and `…`.
pub const SHORT_HEIGHT: u16 = 24;

/// The message lines shown on a short screen.
pub const SHORT_MESSAGE_LINES: usize = 4;

/// Cells before the action word: a space, the cursor (`▸`) or the base's
/// `─`, a space.
const LEAD: usize = 3;

/// Cells the action word and its glyph take, gap included (`squash ↓  `).
const ACTION_WIDTH: usize = 10;

/// Cells a short sha takes, gap included.
const SHORT_WIDTH: usize = 9;

/// The subject column's bounds: as wide as the longest subject, within
/// these.
const SUBJECT_MIN: usize = 12;
const SUBJECT_MAX: usize = 30;

/// The rows that keep their natural height before anything below them does:
/// fewer than this and the rows scroll.
const ROWS_MIN: u16 = 3;

/// Everything the editor draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RewriteEditorView {
    /// `REWRITE feature-a · 5 commits since it left main · newest first ·
    /// nothing written until y`.
    pub title: SanitizedText,
    /// His rows newest first, then the base row last.
    pub rows: Vec<EditorRow>,
    /// Index into `rows`.
    pub cursor: usize,
    /// `r`'s message field, drawn on the line under its row.
    pub field: Option<FieldView>,
    /// The message the rewrite will write, when a squash group or a reword
    /// exists.
    pub message: Option<MessageBlock>,
    /// The AFTER plan or `⏎`'s file list.
    pub right: RightSide,
    /// Below [`NARROW_BELOW`]: `Tab` has swapped the rows for the right side.
    pub narrow_shows_right: bool,
    /// A refusal in words, on the area's last row.
    pub notice: Option<SanitizedText>,
    /// The ASCII renderer tier: every glyph has an ASCII stand-in.
    pub ascii: bool,
}

/// A row's action, which picks its word and glyph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowMark {
    /// `pick`.
    Pick,
    /// `reword ✎`.
    Reword,
    /// `edit ⏸`.
    Edit,
    /// `squash ↓`.
    Squash,
    /// `fixup ↓`.
    Fixup,
    /// `drop ✗`, subject muted.
    Drop,
    /// The row under his: `─ <label>`, not editable.
    Base {
        /// The upstream's or the default branch's short name, or a sha.
        label: SanitizedText,
    },
}

/// One row on the left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EditorRow {
    /// Its action.
    pub mark: RowMark,
    /// Its short sha.
    pub short: String,
    /// Its subject as it stands.
    pub subject: SanitizedText,
    /// What the row says about itself (`melds into the row below · marked by
    /// its name`, `✗ nothing below to meld into`, `stays`).
    pub note: Option<SanitizedText>,
    /// `↕ moved`, after the note.
    pub moved: bool,
}

/// `r`'s one-line field.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldView {
    /// The row it edits (index into `rows`); the field is the line under it.
    pub row: usize,
    /// The subject being typed.
    pub text: SanitizedText,
    /// The caret, a character index.
    pub caret: usize,
}

/// The message block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageBlock {
    /// `MESSAGE of the squashed commit · r on "<kept subject>" changes it`.
    pub label: SanitizedText,
    /// The message, one entry per line, each drawn behind `┊`.
    pub lines: Vec<SanitizedText>,
}

/// What the right-hand side shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RightSide {
    /// The AFTER plan.
    After(AfterView),
    /// `⏎`: one row's changes.
    Changes {
        /// The row's short sha.
        short: String,
        /// Its files.
        files: ChangesView,
    },
}

/// The AFTER plan's state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfterView {
    /// `checking…` in the header.
    Checking,
    /// The plan could not be read, in words.
    Failed(SanitizedText),
    /// The plan (or the todo) refuses, in words; `y` gives the same.
    Refused(SanitizedText),
    /// The plan landed.
    Ready {
        /// The branches after `y`, top down, base last.
        rows: Vec<AfterRow>,
        /// `built on yours: …`, `on origin: …`, `no conflicts expected`.
        facts: Vec<SanitizedText>,
    },
}

/// One AFTER row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AfterRow {
    /// The branch label, on the first row of each branch's run.
    pub branch: Option<SanitizedText>,
    /// New (and whether his new tip) or unchanged.
    pub mark: AfterMark,
    /// The commit's subject.
    pub subject: SanitizedText,
    /// `built on yours · follows`, `3 in one`, `unchanged`, ….
    pub note: Option<SanitizedText>,
}

/// An AFTER row's mark.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfterMark {
    /// His branch's new tip: `● new`.
    NewTip,
    /// A new commit: `○ new`.
    New,
    /// Kept as it is, by its short sha.
    Unchanged {
        /// The short sha.
        short: String,
    },
}

/// `⏎`'s file list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangesView {
    /// Asked for, not arrived.
    Reading,
    /// The read failed, in words.
    Failed(SanitizedText),
    /// Each file: path, lines added, lines removed.
    Files(Vec<(SanitizedText, u32, u32)>),
}

/// One line in the rows' region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RowLine {
    /// A row (index into `rows`).
    Row(usize),
    /// The field, under its row.
    Field,
}

/// Where everything goes: the one answer the draw and the hit test share.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Layout {
    /// The title's row, over the rows.
    title: Option<Rect>,
    /// The rows' region: one line per [`RowLine`], from `top`.
    rows: Option<Rect>,
    /// The first [`RowLine`] drawn (the rows scroll to keep the cursor in).
    top: usize,
    /// The `│` column (wide only).
    rule: Option<Rect>,
    /// The right side, its header on its first row.
    right: Option<Rect>,
    /// The message block: a blank line, the label, the `┊` lines.
    message: Option<Rect>,
    /// The area's last row.
    notice: Rect,
}

/// The rows' lines, in order.
fn row_lines(view: &RewriteEditorView) -> Vec<RowLine> {
    let mut lines = Vec::with_capacity(view.rows.len() + 1);
    for index in 0..view.rows.len() {
        lines.push(RowLine::Row(index));
        if view.field.as_ref().is_some_and(|field| field.row == index) {
            lines.push(RowLine::Field);
        }
    }
    lines
}

/// Rows the message block wants: a blank, the label, its lines (capped on
/// a short screen, `…` after the cap).
fn message_height(view: &RewriteEditorView, area: Rect) -> u16 {
    let Some(message) = &view.message else {
        return 0;
    };
    let lines = if area.height <= SHORT_HEIGHT && message.lines.len() > SHORT_MESSAGE_LINES {
        SHORT_MESSAGE_LINES + 1
    } else {
        message.lines.len()
    };
    u16::try_from(lines + 2).unwrap_or(u16::MAX)
}

/// Rows the right side wants: its header, a blank, its lines.
fn right_height(view: &RewriteEditorView) -> u16 {
    let lines = match &view.right {
        RightSide::After(AfterView::Ready { rows, facts }) => {
            rows.len() + usize::from(!facts.is_empty()) + facts.len()
        }
        RightSide::After(_) => 1,
        RightSide::Changes { files, .. } => match files {
            ChangesView::Files(files) => files.len().max(1),
            _ => 1,
        },
    };
    u16::try_from(lines + 2).unwrap_or(u16::MAX)
}

/// The first line to draw so the cursor's row (and its field) is on screen.
fn scroll_top(lines: &[RowLine], cursor: usize, height: u16) -> usize {
    let height = usize::from(height);
    let Some(at) = lines.iter().position(|line| *line == RowLine::Row(cursor)) else {
        return 0;
    };
    let last = if lines.get(at + 1) == Some(&RowLine::Field) {
        at + 1
    } else {
        at
    };
    (last + 1).saturating_sub(height)
}

/// Rect helper: `height` rows from `y` inside `within`'s columns.
fn band(within: Rect, y: u16, height: u16) -> Rect {
    Rect::new(within.x, y, within.width, height)
}

/// Where everything goes in `area`.
fn layout(view: &RewriteEditorView, area: Rect) -> Layout {
    let notice = Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1);
    let body = Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1));
    let lines = row_lines(view);
    let natural = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let empty = Layout {
        title: None,
        rows: None,
        top: 0,
        rule: None,
        right: None,
        message: None,
        notice,
    };
    if body.height < 3 {
        return empty;
    }
    let wide = area.width >= WIDE_MIN;
    let narrow = area.width < NARROW_BELOW;

    if narrow && view.narrow_shows_right {
        return Layout {
            right: Some(body),
            ..empty
        };
    }

    // The left column: the whole width, or the rows' width left of `│`.
    let left = if wide {
        Rect::new(body.x, body.y, LEFT_WIDTH.min(body.width), body.height)
    } else {
        body
    };
    let (rule, right_wide) = if wide {
        let rule = Rect::new(body.x + LEFT_WIDTH, body.y, 1, body.height);
        let right = Rect::new(
            body.x + LEFT_WIDTH + 2,
            body.y,
            body.width.saturating_sub(LEFT_WIDTH + 2),
            body.height,
        );
        (Some(rule), Some(right))
    } else {
        (None, None)
    };

    let title = band(left, left.y, 1);
    let below = left.y + 2;
    let room = left.bottom().saturating_sub(below);
    let right_need = if wide || narrow { 0 } else { right_height(view) + 1 };
    let message_need = message_height(view, area);
    let rows_height = natural
        .min(room.saturating_sub(right_need + message_need))
        .max(natural.min(ROWS_MIN))
        .min(room);
    let rows = band(left, below, rows_height);
    let mut y = rows.bottom();

    let right = if let Some(right) = right_wide {
        Some(right)
    } else if narrow {
        None
    } else {
        // 120–179: a blank line, then the right side under the rows.
        let height = right_need.min(left.bottom().saturating_sub(y));
        let right = band(left, y + 1, height.saturating_sub(1));
        y += height;
        (right.height > 0).then_some(right)
    };
    let message_height = message_need.min(left.bottom().saturating_sub(y));
    let message = (message_height > 0).then(|| band(left, y, message_height));

    Layout {
        title: Some(title),
        rows: Some(rows),
        top: scroll_top(&lines, view.cursor, rows.height),
        rule,
        right,
        message,
        notice,
    }
}

/// The row (index into `view.rows`) drawn at `(column, row)`, as
/// [`draw_rewrite_editor`] laid it out in `area`; `None` off the rows (the
/// field's line included).
#[must_use]
pub fn rewrite_row_at(
    area: Rect,
    view: &RewriteEditorView,
    column: u16,
    row: u16,
) -> Option<usize> {
    let at = layout(view, area);
    let rows = at.rows?;
    if column < rows.x || column >= rows.right() || row < rows.y || row >= rows.bottom() {
        return None;
    }
    match row_lines(view).get(at.top + usize::from(row - rows.y))? {
        RowLine::Row(index) => Some(*index),
        RowLine::Field => None,
    }
}

/// Draw the editor into `area` (the whole frame but the hint bar).
pub fn draw_rewrite_editor(frame: &mut Frame, area: Rect, view: &RewriteEditorView, theme: &Theme) {
    frame.render_widget(
        RewriteEditorWidget {
            view,
            theme,
        },
        area,
    );
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
struct Glyphs {
    cursor: &'static str,
    base: &'static str,
    melds: &'static str,
    reword: &'static str,
    edit: &'static str,
    drop: &'static str,
    moved: &'static str,
    rule: &'static str,
    gutter: &'static str,
    new_tip: &'static str,
    new: &'static str,
    more: &'static str,
    minus: &'static str,
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
fn put(buf: &mut Buffer, band: Rect, x: u16, text: &str, style: Style) -> u16 {
    if x >= band.right() || band.height == 0 {
        return x;
    }
    let room = usize::from(band.right() - x);
    let (end, _) = buf.set_stringn(x, band.y, text, room, style);
    end
}

/// The subject column's width: the longest subject, within bounds.
fn subject_width(view: &RewriteEditorView) -> usize {
    view.rows
        .iter()
        .map(|row| row.subject.as_str().chars().count())
        .max()
        .unwrap_or(0)
        .clamp(SUBJECT_MIN, SUBJECT_MAX)
}

/// The action word and glyph, and its style.
fn action(mark: &RowMark, glyphs: &Glyphs, theme: &Theme) -> (String, Style) {
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
fn draw_rows(
    buf: &mut Buffer,
    rows: Rect,
    top: usize,
    view: &RewriteEditorView,
    theme: &Theme,
    glyphs: &Glyphs,
) {
    let subject_width = subject_width(view);
    let lines = row_lines(view);
    for (offset, line) in lines.iter().skip(top).take(usize::from(rows.height)).enumerate() {
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
fn draw_row(
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
        buf.set_style(band, Style::new().bg(crate::paint::color(theme.token(Token::Selection))));
    }
    let muted = foreground(theme, Token::TextMuted);
    let lead = if is_base {
        glyphs.base
    } else if on_cursor {
        glyphs.cursor
    } else {
        " "
    };
    let lead_style = if is_base { muted } else { foreground(theme, Token::Accent) };
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

/// The right side: its header, a blank, its lines.
fn draw_right(buf: &mut Buffer, right: Rect, view: &RewriteEditorView, theme: &Theme, glyphs: &Glyphs) {
    let header_style = foreground(theme, Token::Accent).add_modifier(Modifier::BOLD);
    let muted = foreground(theme, Token::TextMuted);
    let warning = foreground(theme, Token::Warning);
    let mut lines: Vec<Vec<(String, Style)>> = Vec::new();
    let header = match &view.right {
        RightSide::After(AfterView::Checking) => "AFTER y \u{b7} checking\u{2026}".to_owned(),
        RightSide::After(_) => "AFTER y \u{b7} not written yet".to_owned(),
        RightSide::Changes { short, .. } => format!("CHANGES of {short} \u{b7} \u{23ce} back to AFTER"),
    };
    lines.push(vec![(header, header_style)]);
    lines.push(Vec::new());
    match &view.right {
        RightSide::After(AfterView::Checking) => {}
        RightSide::After(AfterView::Failed(words) | AfterView::Refused(words)) => {
            lines.push(vec![(words.as_str().to_owned(), warning)]);
        }
        RightSide::After(AfterView::Ready { rows, facts }) => {
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
            for row in rows {
                let branch = row.branch.as_ref().map_or("", SanitizedText::as_str);
                let (mark, mark_style) = match &row.mark {
                    AfterMark::NewTip => (
                        format!("{} new", glyphs.new_tip),
                        foreground(theme, Token::Accent).add_modifier(Modifier::BOLD),
                    ),
                    AfterMark::New => (format!("{} new", glyphs.new), foreground(theme, Token::Info)),
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
                let style = if fact.as_str().starts_with('!') { warning } else { foreground(theme, Token::Text) };
                lines.push(vec![(fact.as_str().to_owned(), style)]);
            }
        }
        RightSide::Changes { files, .. } => match files {
            ChangesView::Reading => lines.push(vec![("reading\u{2026}".to_owned(), muted)]),
            ChangesView::Failed(words) => lines.push(vec![(words.as_str().to_owned(), warning)]),
            ChangesView::Files(files) if files.is_empty() => {
                lines.push(vec![("no files changed".to_owned(), muted)]);
            }
            ChangesView::Files(files) => {
                let path_width = files
                    .iter()
                    .map(|(path, _, _)| path.as_str().chars().count())
                    .max()
                    .unwrap_or(0)
                    .min(usize::from(right.width).saturating_sub(16))
                    + 2;
                for (path, added, removed) in files {
                    lines.push(vec![
                        (fit(path.as_str(), path_width), foreground(theme, Token::Text)),
                        (format!("+{added}"), foreground(theme, Token::Added)),
                        (" ".to_owned(), muted),
                        (format!("{}{removed}", glyphs.minus), foreground(theme, Token::Removed)),
                    ]);
                }
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
fn draw_message(
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
    put(buf, band(at, at.y + 1, 1), at.x + 1, block.label.as_str(), label_style);
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
        put(buf, row, x + 1, line.as_str(), foreground(theme, Token::Text));
        y += 1;
    }
    if !fits && y < at.bottom() {
        let row = band(at, y, 1);
        let x = put(buf, row, row.x + 1, glyphs.gutter, muted);
        put(buf, row, x + 1, glyphs.more, muted);
    }
}
