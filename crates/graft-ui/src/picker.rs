//! The branch picker (`C`, m7-power-tools/10; keymap §2.16): a box over the
//! lower part of the graph listing his branches and the remote's, a typed
//! query above them.
//!
//! Pure drawing. `graft` owns the state, the filter and the keys, and hands
//! over a [`PickerView`] with every string already worded and sanitized.
//!
//! Different kinds of row look different without colour: a glyph and a word
//! in their own column (`↩ before`, `● local`, `◌ remote`, `· here`), and the
//! relation (`tracks …`, `built on …`, `no copy of yours yet`) in the right
//! column. Below 120 columns only the glyph, the name and the age.

use graft_term::SanitizedText;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;

use crate::fit::fit;
use crate::frame_box::{border_row, cells, content_row};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The box's title, in its top border.
pub const TITLE: &str = "SWITCH BRANCH \u{b7} yours and the remote's \u{b7} type to find";

/// The widest the box gets.
const MAX_WIDTH: u16 = 150;

/// Rows that are not branch rows: the two borders, the query and the
/// bottom line.
const CHROME_ROWS: u16 = 4;

/// Which kind of branch a row is: its own glyph and word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickerKind {
    /// The branch he was on before (`@{-1}`).
    Before,
    /// A local branch.
    Local,
    /// A remote branch with no local copy yet.
    Remote,
    /// The branch he is on.
    Here,
}

impl PickerKind {
    /// The glyph, as the narrow layout draws it alone.
    #[must_use]
    pub fn glyph(self) -> char {
        match self {
            PickerKind::Before => '\u{21a9}',
            PickerKind::Local => '\u{25cf}',
            PickerKind::Remote => '\u{25cc}',
            PickerKind::Here => '\u{b7}',
        }
    }

    /// Glyph and word, as the wide layout draws them.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            PickerKind::Before => "\u{21a9} before",
            PickerKind::Local => "\u{25cf} local",
            PickerKind::Remote => "\u{25cc} remote",
            PickerKind::Here => "\u{b7} here",
        }
    }
}

/// One branch row, worded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerRow {
    /// Its kind.
    pub kind: PickerKind,
    /// The short name (`main`, `origin/sara/login-page`).
    pub name: SanitizedText,
    /// Who made the tip.
    pub who: SanitizedText,
    /// The tip's age (`1h`).
    pub age: String,
    /// The tip's subject; the `here` row draws `you're on it` instead.
    pub subject: SanitizedText,
    /// `tracks origin/main`, `built on feature-a`, `no copy of yours yet`,
    /// or empty.
    pub relation: String,
}

/// What stands under the query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PickerBody {
    /// The read has not answered.
    Reading,
    /// The read failed, and why (one line).
    Failed(SanitizedText),
    /// The repository has no branch at all (an unborn `HEAD`).
    NoBranches,
    /// Nothing matches the query, which is carried.
    NoMatch(SanitizedText),
    /// The rows in drawn order, and the one under the cursor.
    Rows {
        /// Every row the query keeps.
        rows: Vec<PickerRow>,
        /// Index into `rows`.
        selected: usize,
    },
}

/// Everything the box draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PickerView {
    /// What he has typed.
    pub query: SanitizedText,
    /// The rows, or what stands for them.
    pub body: PickerBody,
    /// The bottom line: `15 branches`, `2 of 15 branches`.
    pub footer: String,
    /// Why `⏎` did nothing (a write running, git mid-operation, `you're on
    /// it`): drawn on the bottom line in place of the footer.
    pub note: Option<SanitizedText>,
    /// At least 120 columns: every column; else glyph, name and age.
    pub wide: bool,
}

/// Where the box goes over `graph`: as wide as `graph` less 4, at most
/// [`MAX_WIDTH`], centred; the lower 60% of it, so the graph above stays in
/// view.
#[must_use]
pub fn picker_area(graph: Rect) -> Rect {
    let (x, y, width, height) = picker_box((graph.x, graph.y, graph.width, graph.height));
    Rect {
        x,
        y,
        width,
        height,
    }
}

/// [`picker_area`] in plain numbers, `(x, y, width, height)`, for a caller
/// that must not name the rendering library (`graft::update::geometry`
/// mirrors the frame so a click can find the row it landed on).
#[must_use]
pub fn picker_box(graph: (u16, u16, u16, u16)) -> (u16, u16, u16, u16) {
    let (x, y, graph_width, graph_height) = graph;
    let width = graph_width.saturating_sub(4).min(MAX_WIDTH);
    let height = u16::try_from(u32::from(graph_height) * 3 / 5)
        .unwrap_or(graph_height)
        .max(CHROME_ROWS + 1)
        .min(graph_height);
    (
        x + graph_width.saturating_sub(width) / 2,
        (y + graph_height).saturating_sub(height),
        width,
        height,
    )
}

/// The first row drawn in a box `height` rows tall with `selected` under
/// the cursor: the list scrolls only as far as keeps the cursor in view.
#[must_use]
pub fn first_drawn(height: u16, selected: usize) -> usize {
    (selected + 1).saturating_sub(usize::from(height.saturating_sub(CHROME_ROWS)))
}

/// The row a click `line` rows below the box's top border landed on, in a
/// box `height` rows tall showing `count` rows with `selected` under the
/// cursor. `None` on the border, the query, the bottom line, or below the
/// last row.
#[must_use]
pub fn row_at(height: u16, count: usize, selected: usize, line: u16) -> Option<usize> {
    let visible = height.saturating_sub(CHROME_ROWS);
    let offset = line.checked_sub(2).filter(|&offset| offset < visible)?;
    Some(first_drawn(height, selected) + usize::from(offset)).filter(|&row| row < count)
}

/// Draws `view` into `area` (from [`picker_area`]), clearing it first.
pub fn draw_picker(frame: &mut Frame, area: Rect, view: &PickerView, theme: &Theme) {
    render_picker(frame.buffer_mut(), area, view, theme);
}

/// [`draw_picker`] into a bare buffer.
pub fn render_picker(buf: &mut Buffer, area: Rect, view: &PickerView, theme: &Theme) {
    if area.width < 4 || area.height < CHROME_ROWS {
        return;
    }
    let width = usize::from(area.width);
    let text = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    for y in area.y..area.bottom() {
        buf.set_string(area.x, y, content_row("", width), text);
    }
    buf.set_string(
        area.x,
        area.y,
        border_row('\u{250c}', '\u{2510}', TITLE, width),
        text.add_modifier(Modifier::BOLD),
    );
    buf.set_string(
        area.x,
        area.y + 1,
        content_row(&format!("> {}\u{258f}", view.query.as_str()), width),
        text,
    );
    draw_body(buf, area, view, theme);
    let bottom_line = area.bottom() - 2;
    match &view.note {
        Some(note) => buf.set_string(
            area.x,
            bottom_line,
            content_row(note.as_str(), width),
            foreground(theme, Token::Warning),
        ),
        None => buf.set_string(area.x, bottom_line, content_row(&view.footer, width), muted),
    }
    buf.set_string(
        area.x,
        area.bottom() - 1,
        border_row('\u{2514}', '\u{2518}', "", width),
        text,
    );
}

/// The rows under the query, or the one line that stands for them.
fn draw_body(buf: &mut Buffer, area: Rect, view: &PickerView, theme: &Theme) {
    let width = usize::from(area.width);
    let text = foreground(theme, Token::Text);
    let muted = foreground(theme, Token::TextMuted);
    let list_rows = area.height - CHROME_ROWS;
    let inner = width.saturating_sub(4);
    match &view.body {
        PickerBody::Rows { rows, selected } => {
            let columns = Columns::of(rows, inner, view.wide);
            let visible = usize::from(list_rows);
            let top = first_drawn(area.height, *selected);
            for (offset, (at, row)) in rows.iter().enumerate().skip(top).take(visible).enumerate() {
                let y = area.y + 2 + u16::try_from(offset).unwrap_or(0);
                let chosen = at == *selected;
                let style = if chosen {
                    foreground(theme, Token::Accent).add_modifier(Modifier::BOLD)
                } else if row.kind == PickerKind::Here {
                    muted
                } else {
                    text
                };
                buf.set_string(
                    area.x,
                    y,
                    content_row(&columns.line(row, chosen), width),
                    style,
                );
            }
        }
        body => {
            let words = match body {
                PickerBody::Reading => "reading branches\u{2026}".to_owned(),
                PickerBody::Failed(why) => format!("can't list branches \u{b7} {}", why.as_str()),
                PickerBody::NoBranches => closes("no branches yet"),
                PickerBody::NoMatch(query) => {
                    closes(&format!("no branch matches \"{}\"", query.as_str()))
                }
                PickerBody::Rows { .. } => String::new(),
            };
            let style = if matches!(body, PickerBody::Failed(_)) {
                foreground(theme, Token::Warning)
            } else {
                muted
            };
            buf.set_string(area.x, area.y + 2, content_row(&words, width), style);
        }
    }
}

/// `words · esc closes`. The picker is a text field — every press goes on
/// as typed (ADR-0055) — so its key spells from the shipped table whatever
/// his config says (ticket m8/10, decision 5).
fn closes(words: &str) -> String {
    match Keymap::shipped().spell_keys(&[ActionId("app.back")]) {
        Some(key) => format!("{words} \u{b7} {key} closes"),
        None => words.to_owned(),
    }
}

/// The words a `here` row draws where the subject would be.
const ON_IT: &str = "you're on it";

/// Column widths, measured once over every row so the columns line up.
struct Columns {
    wide: bool,
    name: usize,
    who: usize,
    age: usize,
    subject: usize,
}

impl Columns {
    fn of(rows: &[PickerRow], inner: usize, wide: bool) -> Columns {
        let widest = |measure: &dyn Fn(&PickerRow) -> usize| rows.iter().map(measure).max();
        let age = widest(&|row| cells(&row.age)).unwrap_or(0);
        // `▸ ` + the kind column + a gap.
        let lead = 2 + if wide { 8 + 2 } else { 1 + 1 };
        if !wide {
            let name = inner.saturating_sub(lead + 2 + age);
            return Columns {
                wide,
                name,
                who: 0,
                age,
                subject: 0,
            };
        }
        let who = widest(&|row| cells(row.who.as_str())).unwrap_or(0).min(8);
        let relation = widest(&|row| cells(&row.relation)).unwrap_or(0);
        let room = inner.saturating_sub(lead + 2 + who + 2 + age + 2 + 2 + relation);
        let name = widest(&|row| cells(row.name.as_str()))
            .unwrap_or(0)
            .min(room * 2 / 5);
        Columns {
            wide,
            name,
            who,
            age,
            subject: room.saturating_sub(name),
        }
    }

    fn line(&self, row: &PickerRow, chosen: bool) -> String {
        let cursor = if chosen { "\u{25b8} " } else { "  " };
        let name = pad(row.name.as_str(), self.name);
        if !self.wide {
            return format!(
                "{cursor}{} {name}  {}",
                row.kind.glyph(),
                pad(&row.age, self.age)
            );
        }
        let subject = if row.kind == PickerKind::Here {
            ON_IT
        } else {
            row.subject.as_str()
        };
        format!(
            "{cursor}{}  {name}  {}  {}  {}  {}",
            pad(row.kind.label(), 8),
            pad(row.who.as_str(), self.who),
            pad(&row.age, self.age),
            pad(subject, self.subject),
            row.relation
        )
    }
}

/// `text` cut or padded to exactly `width` cells.
fn pad(text: &str, width: usize) -> String {
    let cut = fit(text, width);
    let used = cells(&cut);
    format!("{cut}{}", " ".repeat(width.saturating_sub(used)))
}

#[cfg(test)]
mod tests;
