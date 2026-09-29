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
use crate::paint::foreground;
use crate::theme::{Theme, Token};
use unicode_width::UnicodeWidthStr;

/// What a row is to `u` and `U`, in words rather than colour.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowMark {
    /// An ordinary entry: in force, but not the one a key would act on.
    #[default]
    None,
    /// `u` takes this one back.
    NextUndo,
    /// `U` puts this one back.
    NextRedo,
    /// A previous run died between the entry's two journal lines, so what
    /// it did is not known from the journal alone.
    Interrupted,
    /// A later entry has already reversed this one.
    Undone,
}

impl RowMark {
    /// The mark as it is drawn, at the right of the row. Empty for
    /// [`RowMark::None`], which is most rows.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            RowMark::None => "",
            RowMark::NextUndo => "← u",
            RowMark::NextRedo => "← U",
            RowMark::Interrupted => "interrupted",
            RowMark::Undone => "undone",
        }
    }
}

/// Whose row it is (J1's distinctions: glyph, source word, indent).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RowKind {
    /// `●` — a journal entry: done in Graft, `u` can undo it.
    #[default]
    Graft,
    /// `$` — a step the shell took, from git's reflog: `u` never touches it.
    Outside,
    /// `└ ✗ lost` — a commit no branch holds, under the step that lost it.
    Lost,
    /// `└ ✗ lost` — an edit an undo replaced, kept only in its backup.
    LostEdit,
}

/// One row, as the browser draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRowView {
    /// How long ago, already in words: `2m ago`. Empty on a `✗` row.
    pub age: SanitizedText,
    /// What happened: `commit "Add OAuth provider"`, `reset main 1 commit
    /// back`, or a `✗` row's whole line.
    pub description: SanitizedText,
    /// A `●` row's second column — what it moved and whether a backup was
    /// kept: `1 ref · backup saved`. Empty on other rows.
    pub summary: SanitizedText,
    /// What this row is to `u`/`U`.
    pub mark: RowMark,
    /// The Command Log: every `git` line the operation ran, shown while the
    /// row is expanded.
    pub commands: Vec<SanitizedText>,
    /// Why an `Aborted` entry failed, in full, shown while the row is
    /// expanded; `None` for an entry that finished.
    pub failure: Option<SanitizedText>,
    /// `Space` opened this row's log.
    pub expanded: bool,
    /// Whose row it is.
    pub kind: RowKind,
    /// A `$` row's own reflog subject, drawn as `git: "<subject>"`: in its
    /// column at 180 columns and wider, else under the row while it is
    /// selected. Sanitised by the caller (the other fields are too).
    pub git_subject: Option<SanitizedText>,
    /// A `✗` row's event: the index of the row it sits under.
    pub under: Option<usize>,
}

impl Default for JournalRowView {
    /// An empty `●` row: the base the caller fills in.
    fn default() -> Self {
        JournalRowView {
            age: graft_term::sanitize::sanitize(""),
            description: graft_term::sanitize::sanitize(""),
            summary: graft_term::sanitize::sanitize(""),
            mark: RowMark::None,
            commands: Vec::new(),
            failure: None,
            expanded: false,
            kind: RowKind::Graft,
            git_subject: None,
            under: None,
        }
    }
}

/// What draws instead of, or with, the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalStatus {
    /// The read is in flight.
    Loading,
    /// The journal is there and empty: nothing has been written yet.
    Empty,
    /// The journal could not be read; the line is git's or the parser's own.
    Failed(SanitizedText),
    /// There are rows.
    Ready,
}

/// J's status row: a refusal or a notice, drawn above the legend only while
/// there is one (the status rule).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalStatusRow {
    /// The line, already fitted by the caller.
    pub words: String,
    /// A refusal or a failure, drawn as a warning; else a notice, muted.
    pub problem: bool,
}

/// Everything the browser draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalView {
    /// Which repository this journal belongs to.
    pub location: SanitizedText,
    /// Newest first; `✗` rows right after their event.
    pub rows: Vec<JournalRowView>,
    /// The row under the cursor, an index into `rows`.
    pub selected: usize,
    /// What to draw when there are no rows.
    pub status: JournalStatus,
    /// After the title, while the lost-commit scan is out or when it could
    /// not finish: `finding lost commits…`. `None` once it is done.
    pub title_suffix: Option<String>,
    /// The refusal or notice to show, if any.
    pub status_row: Option<JournalStatusRow>,
    /// What `⏎` does on the selected row (`⏎ undo this`), or `None` when
    /// the row alone cannot name it.
    pub enter_words: Option<String>,
    /// A box is docked over the browser and names the keys: the bar is
    /// left blank, as the stash list's is under its box.
    pub box_open: bool,
}

/// The browser itself. Carries the theme like every other screen; the
/// caller paints the surface underneath.
#[derive(Clone, Copy, Debug)]
pub struct JournalScreen<'a> {
    /// What to draw.
    pub view: &'a JournalView,
    /// Resolved colours.
    pub theme: &'a Theme,
}

/// The empty state, which is also an instruction: nothing is lost by there
/// being nothing here yet.
const EMPTY: &str = "nothing yet — every write lands here and u takes it back";

/// What each glyph means, on screen for as long as the rows are (J1).
pub const LEGEND: &str = "● done in Graft: u can undo it · $ done outside Graft: u never touches it · ✗ a commit no branch holds";

/// The list's last line, muted: what the list cannot show (`reflog.md`
/// takeaway 12).
pub const NO_TRACE: &str =
    "a branch deleted outside Graft and never checked out leaves no trace here";

/// The heading over an expanded row's Command Log. `$` means "outside
/// Graft" and nothing else (UX §3.6), so the log's lines are `┆ git …`.
pub const COMMANDS_HEADING: &str = "commands Graft ran";

/// The cursor's column: `▸ ` or two blanks.
const CURSOR: usize = 2;
/// The glyph's column: `●  ` / `$  `.
const GLYPH: usize = 3;
/// The source word's column: `Graft` / `outside Graft`, padded.
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
    /// The source word (`Graft` / `outside Graft`): 120 and wider.
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
        title
    }

    /// The keys, per row: what `⏎` does here first (the whole reason the
    /// browser is reachable), then `b` where the row names a commit, the log
    /// where there is one, and how to close. Cut from the right, `close`
    /// kept, when the width is short.
    fn footer(&self, width: usize) -> String {
        let close = "J/esc close";
        let JournalStatus::Ready = self.view.status else {
            return close.to_owned();
        };
        let row = self.view.rows.get(self.view.selected);
        let mut keys: Vec<&str> = Vec::new();
        if let Some(enter) = &self.view.enter_words {
            keys.push(enter);
        }
        match row.map(|row| row.kind) {
            Some(RowKind::Outside | RowKind::Lost) => keys.push("b new branch here"),
            Some(RowKind::Graft) => keys.push("space command log"),
            Some(RowKind::LostEdit) | None => {}
        }
        keys.push(close);
        while keys.len() > 1 && keys.join("  ").width() > width {
            // The optional middle keys go first; `⏎` and `close` stay.
            let drop = if keys.len() > 2 { keys.len() - 2 } else { 0 };
            keys.remove(drop);
        }
        keys.join("  ")
    }

    /// Every body line, in order: each row and what it opens under it, then
    /// the muted last line.
    fn lines(&self, width: u16) -> Vec<Line> {
        let columns = Columns::for_width(width);
        let mut lines = Vec::new();
        for (index, row) in self.view.rows.iter().enumerate() {
            let selected = index == self.view.selected;
            lines.push(Line {
                pieces: row_pieces(row, selected, columns, usize::from(width)),
                row: Some(index),
                selected,
            });
            for pieces in under_pieces(row, selected, columns, usize::from(width)) {
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
            style
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

/// A row's own line: cursor, glyph, source word, age, words, and a `●`
/// row's mark or a `$` row's `git:` column.
fn row_pieces(row: &JournalRowView, selected: bool, columns: Columns, width: usize) -> Vec<Piece> {
    let cursor = if selected { "▸ " } else { "  " };
    let mut pieces = vec![Piece {
        x: 0,
        text: cursor.to_owned(),
        ink: Ink::Text,
    }];
    let words_x = columns.words_x();
    match row.kind {
        RowKind::Graft | RowKind::Outside => {
            let (glyph, source) = if row.kind == RowKind::Graft {
                ("●", "Graft")
            } else {
                ("$", "outside Graft")
            };
            let source_ink = if row.kind == RowKind::Graft {
                Ink::Text
            } else {
                Ink::Muted
            };
            pieces.push(Piece {
                x: CURSOR,
                text: glyph.to_owned(),
                ink: source_ink,
            });
            let mut x = CURSOR + GLYPH;
            if columns.source {
                pieces.push(Piece {
                    x,
                    text: source.to_owned(),
                    ink: source_ink,
                });
                x += SOURCE;
            }
            pieces.push(Piece {
                x,
                text: row.age.as_str().to_owned(),
                ink: Ink::Text,
            });
            pieces.extend(words_pieces(row, columns, width, words_x));
        }
        RowKind::Lost | RowKind::LostEdit => {
            // `└ ✗ lost` under the event's glyph and source, the words in
            // the age's column: the sha7 stands where the time does.
            let lead_x = CURSOR + GLYPH;
            pieces.push(Piece {
                x: lead_x,
                text: "└ ✗ lost".to_owned(),
                ink: Ink::Warning,
            });
            let x = CURSOR + GLYPH + if columns.source { SOURCE } else { AGE };
            pieces.push(Piece {
                x,
                text: cut_at_word(row.description.as_str(), width.saturating_sub(x + 1)),
                ink: Ink::Warning,
            });
        }
    }
    pieces
}

/// The words of a `●` or `$` row, and what stands at their right.
fn words_pieces(
    row: &JournalRowView,
    columns: Columns,
    width: usize,
    words_x: usize,
) -> Vec<Piece> {
    let room = width.saturating_sub(words_x);
    match row.kind {
        RowKind::Graft => graft_pieces(row, room, width, words_x),
        RowKind::Outside => match (&row.git_subject, columns.git) {
            (Some(subject), true) => {
                let words_room = room.saturating_sub(1).min(WORDS_BEFORE_GIT);
                let git_x = words_x + words_room + 1;
                vec![
                    Piece {
                        x: words_x,
                        text: cut_at_word(row.description.as_str(), words_room),
                        ink: Ink::Text,
                    },
                    Piece {
                        x: git_x,
                        text: git_words(subject),
                        ink: Ink::Muted,
                    },
                ]
            }
            _ => vec![Piece {
                x: words_x,
                text: cut_at_word(row.description.as_str(), room.saturating_sub(1)),
                ink: Ink::Text,
            }],
        },
        RowKind::Lost | RowKind::LostEdit => Vec::new(),
    }
}

/// A `●` row's description and summary, and its mark at the right edge.
fn graft_pieces(row: &JournalRowView, room: usize, width: usize, words_x: usize) -> Vec<Piece> {
    // The summaries line up under each other, which is what makes
    // the list scannable; the description's column is capped so a
    // long one does not push every summary off a wide screen.
    let room = room.saturating_sub(MARK);
    let summary = row.summary.as_str();
    let words = if summary.is_empty() {
        row.description.as_str().to_owned()
    } else {
        // The verdict — the summary's first clause — is what must
        // survive a short row (m4 acceptance review M2: `in progress —
        // c continue  a abort`, `not undoable — origin has it`): where
        // it would not fit, that row's description gives way, cut at a
        // word, down to a floor it keeps. The rest of the summary (`2
        // refs · the index …`) is what a short row loses first, so every
        // other row keeps the shared column. (+ 2: the space before the
        // summary, and the `…` a cut after the verdict ends with.)
        const FLOOR: usize = 12;
        let verdict = summary.split(" · ").next().unwrap_or(summary).width();
        let column = room
            .saturating_sub(30)
            .min(44)
            .min(room.saturating_sub(verdict + 2))
            .max(FLOOR);
        format!(
            "{} {summary}",
            pad(&cut_at_word(row.description.as_str(), column), column),
        )
    };
    let mut pieces = vec![Piece {
        x: words_x,
        text: cut_at_word(&words, room),
        ink: Ink::Text,
    }];
    let mark = row.mark.label();
    if !mark.is_empty() {
        pieces.push(Piece {
            x: width.saturating_sub(mark.width() + 1),
            text: mark.to_owned(),
            ink: if row.mark == RowMark::Interrupted {
                Ink::Warning
            } else {
                Ink::Text
            },
        });
    }
    pieces
}

/// `git: "reset: moving to HEAD~1"`.
fn git_words(subject: &SanitizedText) -> String {
    format!("git: \"{}\"", subject.as_str())
}

/// The lines under a row: a selected `$` row's git subject where its column
/// is not drawn, and an expanded `●` row's reason and Command Log.
fn under_pieces(
    row: &JournalRowView,
    selected: bool,
    columns: Columns,
    width: usize,
) -> Vec<Vec<Piece>> {
    let x = columns.words_x();
    let room = width.saturating_sub(x + 1);
    let one = |text: String, ink: Ink| {
        vec![Piece {
            x,
            text: cut_at_word(&text, room),
            ink,
        }]
    };
    let mut lines = Vec::new();
    if row.kind == RowKind::Outside
        && selected
        && !columns.git
        && let Some(subject) = &row.git_subject
    {
        lines.push(one(git_words(subject), Ink::Muted));
    }
    if row.expanded {
        if let Some(failure) = &row.failure {
            lines.push(one(failure.as_str().to_owned(), Ink::Warning));
        }
        if !row.commands.is_empty() {
            lines.push(one(COMMANDS_HEADING.to_owned(), Ink::Muted));
        }
        for command in &row.commands {
            lines.push(one(format!("┆ {}", command.as_str()), Ink::Muted));
        }
    }
    lines
}

impl JournalScreen<'_> {
    /// From the bottom up: the bar, the legend, the status row. The rows
    /// scroll in what is left; these stay (J1, at 24 rows too). Returns the
    /// row above the last one drawn.
    fn draw_bottom(&self, area: Rect, buf: &mut Buffer) -> u16 {
        let muted = foreground(self.theme, Token::TextMuted);
        let width = usize::from(area.width);
        let mut bottom = area.bottom().saturating_sub(1);
        if !self.view.box_open {
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
                &cut_at_word(LEGEND, width),
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
            JournalStatus::Empty => centred(buf, body, EMPTY, muted),
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
mod tests {
    use graft_core::capabilities::ColorDepth;
    use graft_term::sanitize::sanitize;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::test_support::terminal as test_capabilities;
    use crate::theme::ThemeKind;

    fn theme() -> Theme {
        Theme::new(
            ThemeKind::Dark,
            &test_capabilities(ColorDepth::TrueColor, false),
        )
    }

    fn row(age: &str, description: &str, mark: RowMark) -> JournalRowView {
        JournalRowView {
            age: sanitize(age),
            description: sanitize(description),
            summary: sanitize("backup saved"),
            mark,
            commands: vec![sanitize("git commit -F -")],
            ..JournalRowView::default()
        }
    }

    fn outside(age: &str, words: &str, subject: &str) -> JournalRowView {
        JournalRowView {
            age: sanitize(age),
            description: sanitize(words),
            kind: RowKind::Outside,
            git_subject: Some(sanitize(subject)),
            ..JournalRowView::default()
        }
    }

    fn lost(words: &str, under: usize) -> JournalRowView {
        JournalRowView {
            description: sanitize(words),
            kind: RowKind::Lost,
            under: Some(under),
            ..JournalRowView::default()
        }
    }

    /// The drawn grid, one string per line, trailing blanks trimmed.
    fn draw(view: &JournalView, width: u16, height: u16) -> Vec<String> {
        let theme = theme();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("the test backend starts");
        terminal
            .draw(|frame| {
                frame.render_widget(
                    JournalScreen {
                        view,
                        theme: &theme,
                    },
                    frame.area(),
                );
            })
            .expect("the widget draws");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    fn view(rows: Vec<JournalRowView>, status: JournalStatus) -> JournalView {
        JournalView {
            location: sanitize("~/code/api"),
            rows,
            selected: 0,
            status,
            title_suffix: None,
            status_row: None,
            enter_words: None,
            box_open: false,
        }
    }

    /// J1's mock-up rows: a fetch, the shell's reset with its lost commit
    /// under it, a shell switch with the lost spike tip, Graft's switch.
    fn mock_rows() -> Vec<JournalRowView> {
        vec![
            JournalRowView {
                summary: sanitize("remote-tracking refs · not undoable"),
                ..row("2m ago", "fetch all remotes", RowMark::None)
            },
            outside(
                "5m ago",
                "reset main 1 commit back",
                "reset: moving to HEAD~1",
            ),
            lost(
                "9a8b7c6 \"Add rate limiter\" · was main's tip · no branch holds it",
                1,
            ),
            outside(
                "9m ago",
                "switch from spike to main",
                "checkout: moving from spike to main",
            ),
            lost(
                "4d5e6f7 \"Try redis cache\" · was spike's tip · spike is gone",
                3,
            ),
            row("40m ago", "switch from main to spike", RowMark::NextUndo),
        ]
    }

    #[test]
    fn the_next_undo_row_says_so_in_text_not_in_colour() {
        let view = view(
            vec![
                row("2m ago", "commit \"Add OAuth\"", RowMark::NextUndo),
                row("1h ago", "stage src/a.rs", RowMark::None),
            ],
            JournalStatus::Ready,
        );
        let grid = draw(&view, 60, 8);
        assert!(grid[0].starts_with("JOURNAL · "), "{grid:?}");
        assert!(grid[1].contains("2m ago"), "{grid:?}");
        assert!(grid[1].contains("← u"), "{grid:?}");
        assert!(
            !grid[2].contains('←'),
            "only the next one is marked: {grid:?}"
        );
    }

    /// J1: the Command Log is `┆ git …` under `commands Graft ran` — `$`
    /// means only "outside Graft" (UX §3.6). Replaces the `$ git` tests.
    #[test]
    fn the_command_log_uses_the_dotted_rule() {
        let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
        rows[0].expanded = true;
        let grid = draw(&view(rows, JournalStatus::Ready), 60, 10);
        assert!(
            grid[2].trim_start().starts_with(COMMANDS_HEADING),
            "{grid:?}"
        );
        assert!(
            grid[3].trim_start().starts_with("┆ git commit -F -"),
            "{grid:?}"
        );
        assert!(
            !grid.iter().any(|line| line.contains("$ git")),
            "no `$` in the log: {grid:?}"
        );
    }

    #[test]
    fn an_aborted_rows_reason_is_shown_in_full_when_it_is_expanded() {
        let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
        rows[0].expanded = true;
        rows[0].failure = Some(sanitize("failed: hook rejected the commit"));
        let grid = draw(&view(rows, JournalStatus::Ready), 60, 10);
        assert!(grid[2].contains("failed: hook rejected"), "{grid:?}");
        assert!(grid[3].contains(COMMANDS_HEADING), "{grid:?}");
        assert!(grid[4].contains("┆ git commit"), "{grid:?}");
    }

    #[test]
    fn an_empty_journal_says_what_will_fill_it() {
        let grid = draw(&view(Vec::new(), JournalStatus::Empty), 70, 8);
        assert!(grid.iter().any(|line| line.contains(EMPTY)), "{grid:?}");
    }

    #[test]
    fn a_failed_read_shows_the_reason_rather_than_an_empty_list() {
        let status = JournalStatus::Failed(sanitize("line 3: unexpected end of input"));
        let grid = draw(&view(Vec::new(), status), 70, 8);
        assert!(
            grid.iter()
                .any(|line| line.contains("line 3: unexpected end of input")),
            "{grid:?}"
        );
    }

    #[test]
    fn a_one_row_terminal_draws_the_header_and_nothing_else() {
        let view = view(
            vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)],
            JournalStatus::Ready,
        );
        for height in [1, 2, 3] {
            let grid = draw(&view, 40, height);
            assert!(grid[0].starts_with("JOURNAL"), "{grid:?}");
        }
    }

    /// :848: a row too long for its room is cut at a word with `…`, never
    /// run into the mark (`the woundone`).
    #[test]
    fn a_cut_row_cuts_at_a_word() {
        let mut long = row("2m ago", "undo: discard the wound", RowMark::Undone);
        long.summary = sanitize("the index · the working tree · backup saved");
        let grid = draw(&view(vec![long], JournalStatus::Ready), 70, 8);
        let line = &grid[1];
        assert!(line.contains('…'), "{line:?}");
        assert!(line.ends_with("undone"), "{line:?}");
        let before_mark = line.trim_end_matches("undone").trim_end();
        assert!(before_mark.ends_with('…'), "cut before the mark: {line:?}");
        let kept = before_mark.trim_end_matches('…');
        assert!(
            kept.ends_with(|c: char| c.is_alphanumeric()) && !kept.ends_with(' '),
            "{line:?}"
        );
    }

    #[test]
    fn cut_at_word_keeps_whole_words_and_says_where_it_cut() {
        assert_eq!(cut_at_word("put main back", 20), "put main back");
        assert_eq!(cut_at_word("put main back to it", 12), "put main…");
        assert_eq!(cut_at_word("unbreakable", 5), "unbr…");
        assert_eq!(cut_at_word("a   b   c", 6), "a   b…");
    }

    /// J1's breakpoints: at 180 and wider every column, the `git:` column
    /// on `$` rows; the `✗` row indented `└ ✗ lost` under its event.
    #[test]
    fn at_180_every_column_is_drawn() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.selected = 2;
        let grid = draw(&view, 187, 14);
        assert!(grid[1].contains("●  Graft"), "{grid:?}");
        assert!(grid[2].contains("$  outside Graft"), "{grid:?}");
        assert!(
            grid[2].contains("git: \"reset: moving to HEAD~1\""),
            "{grid:?}"
        );
        assert!(grid[3].starts_with("▸    └ ✗ lost"), "{grid:?}");
        assert!(grid[3].contains("9a8b7c6 \"Add rate limiter\""), "{grid:?}");
        assert!(grid.iter().any(|line| line.contains(LEGEND)), "{grid:?}");
        assert!(grid.iter().any(|line| line.contains(NO_TRACE)), "{grid:?}");
    }

    /// 120–179: no `git:` column; the selected `$` row shows it under it.
    #[test]
    fn at_140_the_selected_shell_rows_subject_is_under_it() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.selected = 1;
        let grid = draw(&view, 140, 14);
        assert!(grid[2].contains("outside Graft"), "{grid:?}");
        assert!(!grid[2].contains("git:"), "{grid:?}");
        assert!(
            grid[3]
                .trim_start()
                .starts_with("git: \"reset: moving to HEAD~1\""),
            "{grid:?}"
        );
        assert!(
            !grid.iter().any(|line| line.contains("checkout: moving")),
            "only the selected row's: {grid:?}"
        );
    }

    /// Under 120: no source word; the legend carries it.
    #[test]
    fn under_120_there_is_no_source_word() {
        let view = view(mock_rows(), JournalStatus::Ready);
        let grid = draw(&view, 100, 14);
        assert!(!grid[1].contains("Graft"), "{grid:?}");
        assert!(!grid[2].contains("outside Graft"), "{grid:?}");
        assert!(grid[2].contains("$  5m ago"), "{grid:?}");
        assert!(
            grid.iter().any(|line| line.starts_with("● done in Graft")),
            "{grid:?}"
        );
    }

    /// At 24 rows the title, the legend, the status row and the bar stay;
    /// the rows scroll so the cursor's row is on screen.
    #[test]
    fn at_24_rows_the_frame_stays_and_the_rows_scroll() {
        let mut rows = Vec::new();
        for n in 0..30 {
            rows.push(row(
                &format!("{n}m ago"),
                &format!("stage file{n}"),
                RowMark::None,
            ));
        }
        let mut view = view(rows, JournalStatus::Ready);
        view.selected = 29;
        view.status_row = Some(JournalStatusRow {
            words: "nothing to undo".to_owned(),
            problem: true,
        });
        view.enter_words = Some("⏎ revert to before this".to_owned());
        let grid = draw(&view, 187, 24);
        assert!(grid[0].starts_with("JOURNAL"), "{grid:?}");
        assert!(grid[23].starts_with("⏎ revert to before this"), "{grid:?}");
        assert!(grid[22].starts_with("● done in Graft"), "{grid:?}");
        assert_eq!(grid[21], "nothing to undo", "{grid:?}");
        assert!(
            grid[20].starts_with("▸ ") && grid[20].contains("stage file29"),
            "the cursor's row is on screen: {grid:?}"
        );
    }

    /// The status rule: no status row, no line for it.
    #[test]
    fn no_status_row_when_nothing_is_wrong() {
        let view = view(mock_rows(), JournalStatus::Ready);
        let grid = draw(&view, 187, 14);
        assert!(grid[12].starts_with("● done in Graft"), "{grid:?}");
        assert!(
            grid[11].is_empty() || grid[11].contains(NO_TRACE),
            "{grid:?}"
        );
    }

    /// The bar is per row: `⏎`'s words for this row, `b` where the row
    /// names a commit, the log where there is one.
    #[test]
    fn the_bar_follows_the_selected_row() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.selected = 2;
        view.enter_words = Some("⏎ put main back to it".to_owned());
        let grid = draw(&view, 187, 14);
        assert_eq!(
            grid[13],
            "⏎ put main back to it  b new branch here  J/esc close"
        );
        view.selected = 0;
        view.enter_words = None;
        let grid = draw(&view, 187, 14);
        assert_eq!(grid[13], "space command log  J/esc close");
    }

    /// A box docked over J names the keys; J's bar is blank under it.
    #[test]
    fn the_bar_is_blank_while_a_box_is_open() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.box_open = true;
        view.enter_words = Some("⏎ undo this".to_owned());
        let grid = draw(&view, 187, 14);
        assert!(grid[13].is_empty(), "{grid:?}");
    }

    #[test]
    fn the_title_says_while_the_scan_runs() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.title_suffix = Some("finding lost commits…".to_owned());
        let grid = draw(&view, 187, 14);
        assert_eq!(
            grid[0],
            "JOURNAL · what happened in this repo, newest first · finding lost commits…"
        );
    }
}
