//! The journal browser (`J`, DESIGN.md §1.10; m7 note J1): what happened in
//! this repository, newest first — Graft's own steps (`●`), the shell's
//! (`$`) and the commits and edits nothing holds any more (`✗`) — and which
//! one `u` would take back (m3-working-tree-undo-and-stash/06,
//! m7-power-tools/24).
//!
//! An overlay over the graph area, like `map_screen`: it owns no state and
//! no I/O, and `view::journal::journal_view` (crate `graft`) builds its
//! input. Every field-relevant fact is in the text — the glyphs, the source
//! word, the `← u` mark, the `interrupted` word — because a capture carries
//! no colour and neither does a `NO_COLOR` terminal.

use graft_term::sanitize::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::graph_screen::hints::place_footer_text;
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};
use unicode_width::UnicodeWidthStr;

mod pieces;
mod view;
use pieces::{row_pieces, under_pieces};
pub use view::{
    JournalFindView, JournalRowView, JournalStatus, JournalStatusRow, JournalView, RowKind, RowMark,
};

/// The browser itself. Carries the theme like every other screen; the
/// caller paints the surface underneath.
#[derive(Clone, Copy, Debug)]
pub struct JournalScreen<'a> {
    /// What to draw.
    pub view: &'a JournalView,
    /// Resolved colours.
    pub theme: &'a Theme,
    /// His keymap: the footer, legend and empty state spell keys from it.
    pub keys: &'a Keymap,
}

/// The empty state, which is also an instruction: nothing is lost by there
/// being nothing here yet. `app.undo`'s key spelled from `keys`.
fn empty(keys: &Keymap) -> String {
    match keys.spell_keys(&[ActionId("app.undo")]) {
        Some(undo) => format!("nothing yet — every write lands here and {undo} takes it back"),
        None => "nothing yet — every write lands here".to_owned(),
    }
}

/// What each glyph means, on screen for as long as the rows are (J1), with
/// `app.undo`'s key spelled from `keys` (`u`, or `u/⌃Z`'s first — the bar's
/// one key per action). Unbound, the legend says it in words.
#[must_use]
pub fn legend(keys: &Keymap) -> String {
    let (can, never) = match keys.spell_keys(&[ActionId("app.undo")]) {
        Some(undo) => (
            format!("{undo} can undo it"),
            format!("{undo} never touches it"),
        ),
        None => (
            "undo can take it back".to_owned(),
            "undo never touches it".to_owned(),
        ),
    };
    format!("● done in Graft: {can} · $ done outside Graft: {never} · ✗ a commit no branch holds")
}

/// The list's last line, muted: what the list cannot show (`reflog.md`
/// takeaway 12).
pub const NO_TRACE: &str =
    "a branch deleted outside Tributary and never checked out leaves no trace here";

/// The heading over an expanded row's Command Log. `$` means "outside
/// Graft" and nothing else (UX §3.6), so the log's lines are `┆ git …`.
pub const COMMANDS_HEADING: &str = "commands Tributary ran";

/// The heading over an expanded row's moved refs (ADR-0062): `refs the
/// fetch moved · u does not move them back`, `app.undo`'s key spelled from
/// `keys` — they mirror the remote, and `u` steps over a fetch. Unbound,
/// the first clause alone: with no key to name, the second has no subject.
#[must_use]
pub fn moved_heading(keys: &Keymap) -> String {
    match keys.spell_keys(&[ActionId("app.undo")]) {
        Some(undo) => format!("refs the fetch moved · {undo} does not move them back"),
        None => "refs the fetch moved".to_owned(),
    }
}

/// The cursor's column: `▸ ` or two blanks.
const CURSOR: usize = 2;
/// The glyph's column: `●  ` / `$  `.
const GLYPH: usize = 3;
/// The source word's column: `Graft` / `outside Tributary`, padded.
const SOURCE: usize = 15;
/// The age's column: `2m ago`, padded.
const AGE: usize = 10;
/// Room kept at the right of a `●` row for its mark, so the words never run
/// into it (`the woundone`, M6 usability :848).
const MARK: usize = 13;
/// A `$` row's words before its `git:` column, at 180 columns and wider.
const WORDS_BEFORE_GIT: usize = 50;

/// Which columns the width has room for (J1's breakpoints).
#[derive(Clone, Copy, Debug)]
struct Columns {
    /// The source word (`Graft` / `outside Tributary`): 120 and wider.
    source: bool,
    /// The `git: "…"` column on `$` rows: 180 and wider.
    git: bool,
}

impl Columns {
    fn for_width(width: u16) -> Columns {
        Columns {
            source: width >= 120,
            git: width >= 180,
        }
    }

    /// Where the words start on every row.
    fn words_x(self) -> usize {
        CURSOR + GLYPH + if self.source { SOURCE } else { 0 } + AGE
    }
}

/// `text` cut to at most `width` cells at a word boundary, with `…` where
/// it was cut; whole when it fits. The spacing inside the kept part is left
/// as it was (a padded column stays padded). A first word wider than
/// `width` is cut mid-word, the only cut left.
///
/// `blame`'s own `cut_at_word` (private, `Option`-returning, `None` when no
/// word fits) is the same rule for a subject alone; this one never gives
/// up, because a row must draw something.
#[must_use]
pub fn cut_at_word(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let room = width.saturating_sub(1);
    let mut best: Option<&str> = None;
    for (index, character) in text.char_indices() {
        if character.is_whitespace() {
            let kept = text[..index].trim_end();
            if kept.width() > room {
                break;
            }
            if !kept.is_empty() {
                best = Some(kept);
            }
        }
    }
    match best {
        Some(kept) => format!("{kept}…"),
        None => fit(text, width).trim_end().to_owned(),
    }
}

/// `text` padded to `columns` display columns, or left alone when it is
/// already wider.
fn pad(text: &str, columns: usize) -> String {
    let mut padded = text.to_owned();
    for _ in text.width()..columns {
        padded.push(' ');
    }
    padded
}

/// One text write, clipped to `area` and dropped when it would fall outside
/// it. Mirrors `map_screen::render`'s own guard.
fn set_text(buf: &mut Buffer, x: u16, y: u16, area: Rect, text: &str, style: Style) {
    if x >= area.right() || y >= area.bottom() {
        return;
    }
    let width = usize::from(area.right() - x);
    buf.set_stringn(x, y, fit(text, width), width, style);
}

/// A line centred in `area`, for the three states that have no rows.
fn centred(buf: &mut Buffer, area: Rect, text: &str, style: Style) {
    if area.is_empty() {
        return;
    }
    let width = usize::from(area.width);
    let drawn = fit(text, width);
    let columns = u16::try_from(drawn.trim_end().width()).unwrap_or(area.width);
    let x = area.x + area.width.saturating_sub(columns) / 2;
    let y = area.y + area.height / 2;
    set_text(buf, x, y, area, &drawn, style);
}

/// The column `x` cells into `area`, as a buffer coordinate.
fn column(area: Rect, x: usize) -> u16 {
    area.x
        .saturating_add(u16::try_from(x).unwrap_or(u16::MAX))
        .min(area.right())
}

/// What a line of the body is drawn in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ink {
    Text,
    Muted,
    Warning,
}

/// One piece of a body line: text from column `x`.
#[derive(Clone, Debug)]
struct Piece {
    x: usize,
    text: String,
    ink: Ink,
}

/// One line of the body, before scrolling.
#[derive(Clone, Debug)]
struct Line {
    pieces: Vec<Piece>,
    /// The row the line belongs to; `None` for the list's last line.
    row: Option<usize>,
    /// The cursor's row: painted to the right edge.
    selected: bool,
}

impl JournalScreen<'_> {
    /// `JOURNAL · what happened in this repo, newest first`, and the scan's
    /// word while it has one.
    fn header(&self) -> String {
        let mut title = "JOURNAL · what happened in this repo, newest first".to_owned();
        if let Some(suffix) = &self.view.title_suffix {
            title.push_str(" · ");
            title.push_str(suffix);
        }
        // The kept find, worded as blame's `FindTitle` tail
        // (`blame::title`): the same words for the same key.
        if let Some(find) = &self.view.find {
            match find.at {
                Some(at) if find.of > 0 => title.push_str(&format!(
                    " · find \"{}\" {at} of {}",
                    find.query.as_str(),
                    find.of
                )),
                _ => title.push_str(&format!(" · find \"{}\" · no match", find.query.as_str())),
            }
        }
        title
    }

    /// The keys, per row: what `⏎` does here first (the whole reason the
    /// browser is reachable), then `b` where the row names a commit, the log
    /// where there is one, how to close, and the `? keys` doorway (m8/11e
    /// F2). Short of width the middle goes first and `close` stays: the one
    /// rule the stash list and the Map share (`hints::fit_place_footer`).
    fn footer(&self, width: usize) -> String {
        place_footer_text(&self.footer_clauses(), self.keys, width)
    }

    /// Every clause [`JournalScreen::footer`] can draw, in order, `close`
    /// last; the doorway is the fitting rule's to add.
    fn footer_clauses(&self) -> Vec<String> {
        let keys = self.keys;
        // `J` and `esc` both close: one hint, both keys (`J/esc close`).
        let close = keys.key_verb(&[ActionId("journal.close"), ActionId("app.back")], "close");
        let JournalStatus::Ready = self.view.status else {
            return close.into_iter().collect();
        };
        let row = self.view.rows.get(self.view.selected);
        let mut parts: Vec<String> = Vec::new();
        if let Some(enter) = &self.view.enter_words {
            parts.extend(keys.key_verb(&[ActionId("journal.revert")], enter));
        }
        if self.view.branch_here {
            parts.extend(keys.key_verb(&[ActionId("journal.branch")], "new branch here"));
        }
        // As blame's bar (`hints::blame_place_list`): `/ find` always,
        // `n/N match` after it while a find is kept.
        parts.extend(keys.key_verb(&[ActionId("app.find")], "find"));
        if self.view.find.is_some() {
            parts.extend(keys.key_verb(
                &[ActionId("app.next_match"), ActionId("app.prev_match")],
                "match",
            ));
        }
        if row.is_some_and(|row| row.kind == RowKind::Graft) {
            parts.extend(keys.key_verb(&[ActionId("journal.expand")], "command log"));
        }
        parts.extend(close);
        parts
    }

    /// Every body line, in order: each row and what it opens under it, then
    /// the muted last line.
    fn lines(&self, width: u16) -> Vec<Line> {
        let columns = Columns::for_width(width);
        let mut lines = Vec::new();
        for (index, row) in self.view.rows.iter().enumerate() {
            let selected = index == self.view.selected;
            lines.push(Line {
                pieces: row_pieces(row, selected, columns, usize::from(width), self.keys),
                row: Some(index),
                selected,
            });
            for pieces in under_pieces(row, selected, columns, usize::from(width), self.keys) {
                lines.push(Line {
                    pieces,
                    row: Some(index),
                    selected: false,
                });
            }
        }
        lines.push(Line {
            pieces: vec![Piece {
                x: CURSOR,
                text: cut_at_word(NO_TRACE, usize::from(width).saturating_sub(CURSOR)),
                ink: Ink::Muted,
            }],
            row: None,
            selected: false,
        });
        lines
    }

    /// The rows in `area`, scrolled so the cursor's row and the lines under
    /// it are on screen.
    fn draw_body(&self, buf: &mut Buffer, area: Rect) {
        let lines = self.lines(area.width);
        let height = usize::from(area.height);
        let last_of_selected = lines
            .iter()
            .rposition(|line| line.row == Some(self.view.selected))
            .unwrap_or(0);
        let top = (last_of_selected + 1).saturating_sub(height);
        for (offset, line) in lines.iter().skip(top).take(height).enumerate() {
            let y = area
                .y
                .saturating_add(u16::try_from(offset).unwrap_or(u16::MAX));
            self.draw_line(buf, area, y, line);
        }
    }

    fn style(&self, ink: Ink, selected: bool) -> Style {
        let style = match ink {
            Ink::Text => foreground(self.theme, Token::Text),
            Ink::Muted => foreground(self.theme, Token::TextMuted),
            Ink::Warning => foreground(self.theme, Token::Warning),
        };
        if selected {
            crate::paint::on_selection(self.theme, style)
                .bg(crate::paint::color(self.theme.token(Token::Selection)))
                .add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    fn draw_line(&self, buf: &mut Buffer, area: Rect, y: u16, line: &Line) {
        if line.selected {
            // Painted first, so the cursor's background reaches the right
            // edge rather than stopping where the text does.
            let blanks = " ".repeat(usize::from(area.width));
            set_text(buf, area.x, y, area, &blanks, self.style(Ink::Text, true));
        }
        for piece in &line.pieces {
            let x = column(area, piece.x);
            let room = usize::from(area.right().saturating_sub(x));
            set_text(
                buf,
                x,
                y,
                area,
                &cut_at_word(&piece.text, room),
                self.style(piece.ink, line.selected),
            );
        }
    }
}

impl JournalScreen<'_> {
    /// From the bottom up: the bar, the legend, the status row. The rows
    /// scroll in what is left; these stay (J1, at 24 rows too). Returns the
    /// row above the last one drawn.
    fn draw_bottom(&self, area: Rect, buf: &mut Buffer) -> u16 {
        let muted = foreground(self.theme, Token::TextMuted);
        let width = usize::from(area.width);
        let mut bottom = area.bottom().saturating_sub(1);
        if let Some(typed) = &self.view.find_prompt {
            // The `/` prompt in place of the bar, in text ink, as blame's
            // (`view::frame::overlays`'s blame bar).
            let prompt = fit(&format!("/{}", typed.as_str()), width);
            set_text(
                buf,
                area.x,
                bottom,
                area,
                &prompt,
                foreground(self.theme, Token::Text),
            );
        } else if !self.view.box_open {
            set_text(buf, area.x, bottom, area, &self.footer(width), muted);
        }
        let ready = self.view.status == JournalStatus::Ready;
        if ready && area.height >= 4 {
            bottom = bottom.saturating_sub(1);
            set_text(
                buf,
                area.x,
                bottom,
                area,
                &cut_at_word(&legend(self.keys), width),
                muted,
            );
        }
        if let Some(status) = &self.view.status_row
            && area.height >= 5
        {
            bottom = bottom.saturating_sub(1);
            let ink = if status.problem {
                foreground(self.theme, Token::Warning)
            } else {
                muted
            };
            set_text(buf, area.x, bottom, area, &status.words, ink);
        }
        bottom
    }
}

impl Widget for JournalScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        let width = usize::from(area.width);
        set_text(
            buf,
            area.x,
            area.y,
            area,
            &cut_at_word(&self.header(), width),
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        if area.height == 1 {
            return;
        }
        let bottom = self.draw_bottom(area, buf);
        let body = Rect {
            x: area.x,
            y: area.y.saturating_add(1),
            width: area.width,
            height: bottom.saturating_sub(area.y.saturating_add(1)),
        };
        if body.is_empty() {
            return;
        }
        match &self.view.status {
            JournalStatus::Ready => self.draw_body(buf, body),
            JournalStatus::Loading => centred(buf, body, "reading the journal…", muted),
            JournalStatus::Empty => centred(buf, body, &empty(self.keys), muted),
            JournalStatus::Failed(why) => centred(
                buf,
                body,
                &format!("the journal could not be read: {}", why.as_str()),
                foreground(self.theme, Token::Warning),
            ),
        }
    }
}

#[cfg(test)]
mod tests;
