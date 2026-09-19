//! Text entry: a single-line [`TextField`] and a multi-line [`TextArea`],
//! plus the two widgets that draw them.
//!
//! The state halves are pure — no terminal, no width, no theme — so the
//! commit editor's keys are tested as plain function calls. Everything that
//! depends on how wide the column turned out to be (which slice of a long
//! line is on screen, which body line is at the top) is worked out at draw
//! time from the cursor, so no key handler has to know the layout and no
//! scroll offset can go stale behind a resize.
//!
//! The cursor is a **character** index, never a byte one: both types keep
//! their text as `String` but only ever cut it at character boundaries, so a
//! wide character or a combining mark can never be split.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthChar;

use crate::fit::fit;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// The glyph marking a length guide (50 for the subject, 72 for the body).
pub const GUIDE: char = '┊';

/// The glyph marking whatever has focus. A character rather than a colour or
/// a bold run, so a capture of the character grid still shows it.
pub const FOCUS: char = '▸';

/// Cells `text` occupies, counting a wide character as two.
fn cells(text: &str) -> usize {
    text.chars().filter_map(UnicodeWidthChar::width).sum()
}

/// The byte offset of character `index`, or the end of the string.
fn byte_of(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(offset, _)| offset)
}

/// Characters a person can type into a field.
///
/// A control character would either be invisible or repaint the screen, and
/// it would also make the sanitiser drop a character the cursor index still
/// counted — so it is refused at the door rather than cleaned up later. A
/// paste's newlines are handled by the caller, which knows whether the target
/// has lines at all.
fn typable(character: char) -> bool {
    !character.is_control()
}

/// Where a word boundary sits, walking back from `index` over `chars`:
/// whitespace first, then the word itself.
fn word_start(chars: &[char], index: usize) -> usize {
    let mut at = index;
    while at > 0 && chars[at - 1].is_whitespace() {
        at -= 1;
    }
    while at > 0 && !chars[at - 1].is_whitespace() {
        at -= 1;
    }
    at
}

/// The start of the next word after `index`, or the end.
fn next_word(chars: &[char], index: usize) -> usize {
    let mut at = index;
    while at < chars.len() && !chars[at].is_whitespace() {
        at += 1;
    }
    while at < chars.len() && chars[at].is_whitespace() {
        at += 1;
    }
    at
}

/// One line of editable text: the commit subject, and later the stash
/// message.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TextField {
    text: String,
    /// Character index, `0..=text.chars().count()`.
    cursor: usize,
}

impl TextField {
    /// A field holding `text`, with the cursor at its end — where someone
    /// prefilled a message would carry on typing.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let text: String = text.chars().filter(|&c| typable(c)).collect();
        let cursor = text.chars().count();
        Self { text, cursor }
    }

    /// What has been typed.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The cursor's character index.
    #[must_use]
    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// How many characters long the text is — what the `n/50` counter counts.
    #[must_use]
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Nothing typed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Type one character at the cursor. A control character is refused.
    pub fn insert(&mut self, character: char) {
        if !typable(character) {
            return;
        }
        let at = byte_of(&self.text, self.cursor);
        self.text.insert(at, character);
        self.cursor += 1;
    }

    /// Type a run of characters — a bracketed paste, which lands exactly as
    /// if it had been typed. Newlines and every other control character are
    /// dropped: this field is one line.
    pub fn insert_str(&mut self, text: &str) {
        for character in text.chars() {
            self.insert(character);
        }
    }

    /// Delete the character before the cursor.
    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        self.cursor -= 1;
        let at = byte_of(&self.text, self.cursor);
        self.text.remove(at);
    }

    /// Delete the character under the cursor.
    pub fn delete(&mut self) {
        if self.cursor >= self.len() {
            return;
        }
        let at = byte_of(&self.text, self.cursor);
        self.text.remove(at);
    }

    /// One character left.
    pub fn left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    /// One character right.
    pub fn right(&mut self) {
        self.cursor = (self.cursor + 1).min(self.len());
    }

    /// To the start of the line.
    pub fn home(&mut self) {
        self.cursor = 0;
    }

    /// To the end of the line.
    pub fn end(&mut self) {
        self.cursor = self.len();
    }

    /// To the start of the word the cursor is in, or of the one before it.
    pub fn word_left(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        self.cursor = word_start(&chars, self.cursor);
    }

    /// To the start of the next word, or the end of the line.
    pub fn word_right(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        self.cursor = next_word(&chars, self.cursor);
    }
}

/// Several lines of editable text: the commit body.
///
/// Always holds at least one line, so "the line the cursor is on" is never a
/// question about an empty vector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextArea {
    lines: Vec<String>,
    line: usize,
    /// Character index within `lines[line]`.
    column: usize,
}

impl Default for TextArea {
    fn default() -> Self {
        Self::new("")
    }
}

impl TextArea {
    /// An area holding `text`, split on `\n`, with the cursor at the end.
    #[must_use]
    pub fn new(text: &str) -> Self {
        let lines: Vec<String> = text
            .split('\n')
            .map(|line| line.chars().filter(|&c| typable(c)).collect())
            .collect();
        let line = lines.len() - 1;
        let column = lines[line].chars().count();
        Self {
            lines,
            line,
            column,
        }
    }

    /// The lines joined back with `\n`; round-trips [`TextArea::new`].
    #[must_use]
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// The lines as they are drawn.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// The cursor: which line, and which character within it.
    #[must_use]
    pub fn cursor(&self) -> (usize, usize) {
        (self.line, self.column)
    }

    /// Nothing typed at all — not even a blank line.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(String::is_empty)
    }

    /// Characters in the current line.
    fn line_len(&self) -> usize {
        self.lines[self.line].chars().count()
    }

    /// Type one character at the cursor. A control character is refused;
    /// `\n` goes through [`TextArea::newline`].
    pub fn insert(&mut self, character: char) {
        if !typable(character) {
            return;
        }
        let at = byte_of(&self.lines[self.line], self.column);
        self.lines[self.line].insert(at, character);
        self.column += 1;
    }

    /// Type a run of characters — a bracketed paste. `\n` breaks the line,
    /// exactly as pressing Enter would; `\r\n` counts once.
    pub fn insert_str(&mut self, text: &str) {
        let mut characters = text.chars().peekable();
        while let Some(character) = characters.next() {
            match character {
                '\n' => self.newline(),
                '\r' => {
                    if characters.peek() == Some(&'\n') {
                        characters.next();
                    }
                    self.newline();
                }
                other => self.insert(other),
            }
        }
    }

    /// Break the line at the cursor.
    pub fn newline(&mut self) {
        let at = byte_of(&self.lines[self.line], self.column);
        let rest = self.lines[self.line].split_off(at);
        self.lines.insert(self.line + 1, rest);
        self.line += 1;
        self.column = 0;
    }

    /// Delete backwards; at the start of a line, join it to the one above.
    pub fn backspace(&mut self) {
        if self.column > 0 {
            self.column -= 1;
            let at = byte_of(&self.lines[self.line], self.column);
            self.lines[self.line].remove(at);
            return;
        }
        if self.line == 0 {
            return;
        }
        let tail = self.lines.remove(self.line);
        self.line -= 1;
        self.column = self.line_len();
        self.lines[self.line].push_str(&tail);
    }

    /// Delete forwards; at the end of a line, pull the next one up.
    pub fn delete(&mut self) {
        if self.column < self.line_len() {
            let at = byte_of(&self.lines[self.line], self.column);
            self.lines[self.line].remove(at);
            return;
        }
        if self.line + 1 >= self.lines.len() {
            return;
        }
        let tail = self.lines.remove(self.line + 1);
        self.lines[self.line].push_str(&tail);
    }

    /// One line up, keeping the column where the shorter line allows.
    pub fn up(&mut self) {
        if self.line == 0 {
            self.column = 0;
            return;
        }
        self.line -= 1;
        self.column = self.column.min(self.line_len());
    }

    /// One line down, keeping the column where the shorter line allows.
    pub fn down(&mut self) {
        if self.line + 1 >= self.lines.len() {
            self.column = self.line_len();
            return;
        }
        self.line += 1;
        self.column = self.column.min(self.line_len());
    }

    /// One character left, wrapping to the end of the line above.
    pub fn left(&mut self) {
        if self.column > 0 {
            self.column -= 1;
        } else if self.line > 0 {
            self.line -= 1;
            self.column = self.line_len();
        }
    }

    /// One character right, wrapping to the start of the line below.
    pub fn right(&mut self) {
        if self.column < self.line_len() {
            self.column += 1;
        } else if self.line + 1 < self.lines.len() {
            self.line += 1;
            self.column = 0;
        }
    }

    /// To the start of the line.
    pub fn home(&mut self) {
        self.column = 0;
    }

    /// To the end of the line.
    pub fn end(&mut self) {
        self.column = self.line_len();
    }

    /// To the start of the word the cursor is in, or of the one before it.
    pub fn word_left(&mut self) {
        let chars: Vec<char> = self.lines[self.line].chars().collect();
        self.column = word_start(&chars, self.column);
    }

    /// To the start of the next word, or the end of the line.
    pub fn word_right(&mut self) {
        let chars: Vec<char> = self.lines[self.line].chars().collect();
        self.column = next_word(&chars, self.column);
    }

    /// Put `line` on a line of its own at the end, and leave the cursor at
    /// the end of it — what `Ctrl+Shift+C` does with a co-author trailer.
    ///
    /// An area holding nothing at all gets the line as its first line, not a
    /// blank one followed by it.
    pub fn append_line(&mut self, line: &str) {
        let line: String = line.chars().filter(|&c| typable(c)).collect();
        if self.is_empty() {
            self.lines = vec![line];
        } else {
            self.lines.push(line);
        }
        self.line = self.lines.len() - 1;
        self.column = self.line_len();
    }
}

/// A single-line field as it is drawn: the label, the text, where the cursor
/// is, the length guide and the counter.
///
/// `masked` does not change `text`: the field still holds what was typed, so
/// the caller (a credential prompt) can read it back. Masking is entirely a
/// drawing decision, made once, in [`FieldWidget`] — never by a caller
/// deciding what string to hand the widget (CLAUDE.md, m4-remotes design
/// note choice 2's "must never" list: the secret must never reach a draw
/// path except through this one gate).
#[derive(Clone)]
pub struct FieldView<'a> {
    /// The name shown ahead of the text (`Subject`).
    pub label: &'a str,
    /// The text itself.
    pub text: SanitizedText,
    /// The cursor's **character** index, when this field has focus.
    pub cursor: Option<usize>,
    /// Cells from the start of the text region to the length guide. Ignored
    /// while `masked`: a guide would still leak the exact typed length.
    pub guide_at: Option<u16>,
    /// `(typed, limit)` for the `12/50` counter. Ignored while `masked`: a
    /// counter is the length in digits, which is exactly what masking hides.
    pub counter: Option<(usize, usize)>,
    /// Draws the label bold and the cursor reversed.
    pub focused: bool,
    /// A password or passphrase field: every character draws as a mask
    /// glyph (`•`, or `*` at the ASCII tier), never the real text, in any
    /// render path. Default `false`.
    pub masked: bool,
}

impl std::fmt::Debug for FieldView<'_> {
    /// Redacts `text` while `masked`: a `{:?}` in a log line must not leak a
    /// password any more readily than the screen does (CLAUDE.md choice 2).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = f.debug_struct("FieldView");
        out.field("label", &self.label);
        if self.masked {
            out.field("text", &"<redacted>");
        } else {
            out.field("text", &self.text);
        }
        out.field("cursor", &self.cursor)
            .field("guide_at", &self.guide_at)
            .field("counter", &self.counter)
            .field("focused", &self.focused)
            .field("masked", &self.masked)
            .finish()
    }
}

/// The mask glyph for one character: `•` at the Unicode tier, `*` at the
/// ASCII one (design note "New UI", credential prompt).
const MASK_GLYPH: char = '•';
const MASK_GLYPH_ASCII: char = '*';

/// What a field actually draws for its text: the text itself, or one mask
/// glyph per character while `masked` — the single place that decision is
/// made, so every draw path (this widget, and the credential prompt's own
/// field row) goes through it rather than each deciding for itself.
#[must_use]
pub fn display_text(view: &FieldView<'_>, ascii: bool) -> String {
    if !view.masked {
        return view.text.as_str().to_owned();
    }
    let glyph = if ascii { MASK_GLYPH_ASCII } else { MASK_GLYPH };
    glyph.to_string().repeat(view.text.as_str().chars().count())
}

/// [`FieldView`] drawn into one row.
#[derive(Debug)]
pub struct FieldWidget<'a> {
    /// What to draw.
    pub view: &'a FieldView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// Draw the mask glyph as `*` instead of `•` (the ASCII renderer tier).
    /// Ignored while the field is not `masked`.
    pub ascii: bool,
}

/// Cells the label column takes, including its trailing gap. Fixed so the
/// subject's text starts in the same column as the body's lines.
pub const LABEL_WIDTH: u16 = 9;

/// The counter's `n/limit` text and whether it is over, and the cells its
/// column takes trailing space included. `None` throughout while masked: a
/// counter is the typed length in digits, which is exactly what masking
/// hides (CLAUDE.md choice 2's "must never").
fn counter_column(view: &FieldView<'_>) -> (Option<(String, bool)>, u16) {
    let counter = (!view.masked)
        .then_some(view.counter)
        .flatten()
        .map(|(typed, limit)| (format!("{typed}/{limit}"), typed > limit));
    let width = counter
        .as_ref()
        .map_or(0, |(text, _)| u16::try_from(cells(text) + 1).unwrap_or(0));
    (counter, width)
}

impl Widget for &FieldWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let row = area.y;
        let label_width = LABEL_WIDTH.min(area.width);
        draw_label(
            buf,
            area.x,
            row,
            label_width,
            self.view.label,
            self.view.focused,
            self.theme,
        );

        // A guide marks an exact column past a set number of characters,
        // which would tell an onlooker something about a masked field's
        // length that the mask glyphs are drawn to hide.
        let guide_at = (!self.view.masked).then_some(self.view.guide_at).flatten();
        let (counter, counter_width) = counter_column(self.view);

        let text_x = area.x + label_width;
        let Some(text_width) = area
            .width
            .checked_sub(label_width + counter_width)
            .filter(|width| *width > 0)
        else {
            return;
        };

        let shown = display_text(self.view, self.ascii);
        draw_line(
            buf,
            Rect {
                x: text_x,
                y: row,
                width: text_width,
                height: 1,
            },
            &shown,
            self.view.cursor,
            guide_at,
            self.theme,
        );

        if let Some((text, over)) = counter {
            let token = if over {
                Token::Warning
            } else {
                Token::TextMuted
            };
            let width = usize::from(counter_width).saturating_sub(1);
            buf.set_string(
                text_x + text_width + 1,
                row,
                fit(&text, width),
                foreground(self.theme, token),
            );
        }
    }
}

/// A multi-line area as it is drawn.
#[derive(Clone, Debug)]
pub struct AreaView<'a> {
    /// The name shown on the row above the lines (`Body`).
    pub label: &'a str,
    /// The lines.
    pub lines: Vec<SanitizedText>,
    /// `(line, character)` when this area has focus.
    pub cursor: Option<(usize, usize)>,
    /// Cells from the start of the text region to the length guide.
    pub guide_at: Option<u16>,
    /// Draws the label bold and the cursor reversed.
    pub focused: bool,
}

/// [`AreaView`] drawn into the rows it is given: the label, then as many
/// lines as fit, scrolled so the cursor's line is always one of them.
#[derive(Debug)]
pub struct AreaWidget<'a> {
    /// What to draw.
    pub view: &'a AreaView<'a>,
    /// Colours.
    pub theme: &'a Theme,
}

impl Widget for &AreaWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 || area.width == 0 {
            return;
        }
        let label_width = LABEL_WIDTH.min(area.width);
        draw_label(
            buf,
            area.x,
            area.y,
            label_width,
            self.view.label,
            self.view.focused,
            self.theme,
        );

        let Some(height) = area.height.checked_sub(1).filter(|rows| *rows > 0) else {
            return;
        };
        let text_x = area.x + label_width;
        let Some(width) = area.width.checked_sub(label_width).filter(|w| *w > 0) else {
            return;
        };

        // The top line is worked out here rather than stored, so it cannot
        // go stale behind a resize: the smallest scroll that keeps the
        // cursor's line on screen.
        let cursor_line = self.view.cursor.map_or(0, |(line, _)| line);
        let top = cursor_line.saturating_sub(usize::from(height) - 1);

        for row in 0..height {
            let Some(line) = self.view.lines.get(top + usize::from(row)) else {
                break;
            };
            let cursor = self
                .view
                .cursor
                .filter(|(at, _)| *at == top + usize::from(row))
                .map(|(_, column)| column);
            draw_line(
                buf,
                Rect {
                    x: text_x,
                    y: area.y + 1 + row,
                    width,
                    height: 1,
                },
                line.as_str(),
                cursor,
                self.view.guide_at,
                self.theme,
            );
        }
    }
}

/// A field's label, with `FOCUS` in the cell before it when the field has
/// focus.
///
/// The marker is a character, not a bold attribute, because a capture records
/// the character grid and nothing else (CLAUDE.md): "which field has focus"
/// has to survive into `docs/shots/`. The bold rides along for a real
/// terminal.
fn draw_label(
    buf: &mut Buffer,
    x: u16,
    y: u16,
    width: u16,
    label: &str,
    focused: bool,
    theme: &Theme,
) {
    if width == 0 {
        return;
    }
    let style = if focused {
        foreground(theme, Token::Text).add_modifier(Modifier::BOLD)
    } else {
        foreground(theme, Token::TextMuted)
    };
    let marker = if focused { FOCUS } else { ' ' };
    buf.set_string(x, y, marker.to_string(), style);
    if width > 1 {
        buf.set_string(x + 1, y, fit(label, usize::from(width - 1)), style);
    }
}

/// One row of text with its guide and, when it has one, its cursor.
///
/// Long text scrolls horizontally by the smallest amount that keeps the
/// cursor on screen, worked out here from the cursor and the width for the
/// same reason the body's top line is: a stored offset would go stale.
///
/// `pub(crate)`: [`credential_prompt`](crate::credential_prompt) draws its
/// own field row (no [`LABEL_WIDTH`] column to spare) but still needs this
/// module's scroll-and-cursor placement, so it calls in rather than
/// reimplementing it.
pub(crate) fn draw_line(
    buf: &mut Buffer,
    area: Rect,
    text: &str,
    cursor: Option<usize>,
    guide_at: Option<u16>,
    theme: &Theme,
) {
    let width = usize::from(area.width);
    let chars: Vec<char> = text.chars().collect();
    let cell_of = |index: usize| -> usize {
        chars[..index.min(chars.len())]
            .iter()
            .filter_map(|c| UnicodeWidthChar::width(*c))
            .sum()
    };
    let cursor_cell = cursor.map_or(0, cell_of);
    let scroll = cursor_cell.saturating_sub(width.saturating_sub(1));

    // Skip whole characters until `scroll` cells have gone by; a wide
    // character straddling the edge is dropped rather than half-drawn.
    let mut skipped = 0;
    let mut start = 0;
    while start < chars.len() && skipped < scroll {
        skipped += UnicodeWidthChar::width(chars[start]).unwrap_or(0);
        start += 1;
    }
    let shown: String = chars[start..].iter().collect();

    buf.set_string(
        area.x,
        area.y,
        fit(&shown, width),
        foreground(theme, Token::Text),
    );

    if let Some(column) = guide_at {
        let cell = usize::from(column).checked_sub(skipped);
        if let Some(cell) = cell.filter(|cell| *cell < width) {
            let at = (area.x + u16::try_from(cell).unwrap_or(0), area.y);
            if buf[at].symbol() == " " {
                buf[at].set_char(GUIDE);
            }
            buf[at].set_style(foreground(theme, Token::TextMuted));
        }
    }

    if let Some(cursor) = cursor {
        let cell = cell_of(cursor).saturating_sub(skipped);
        if cell < width {
            let at = (area.x + u16::try_from(cell).unwrap_or(0), area.y);
            buf[at].set_style(Style::new().add_modifier(Modifier::REVERSED));
        }
    }
}

#[cfg(test)]
mod tests;
