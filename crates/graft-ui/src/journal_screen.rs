//! The undo journal browser (`J`, DESIGN.md §1.10): every write this
//! repository has taken, newest first, and which one `u` would take back
//! (m3-working-tree-undo-and-stash/06).
//!
//! An overlay over the graph area, like `map_screen`: it owns no state and
//! no I/O, and `view::journal::journal_view` (crate `graft`) builds its
//! input. Every field-relevant fact is in the text — the `← u` mark, the
//! `interrupted` word, the failure line — because a capture carries no
//! colour and neither does a `NO_COLOR` terminal.

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

/// One entry, as the browser draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRowView {
    /// How long ago, already in words: `2m ago`.
    pub age: SanitizedText,
    /// What the operation was: `commit "Add OAuth provider"`.
    pub description: SanitizedText,
    /// What it moved and whether a backup was kept: `1 ref · backup saved`.
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

/// Everything the browser draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalView {
    /// Which repository this journal belongs to, for the header.
    pub location: SanitizedText,
    /// Newest first.
    pub rows: Vec<JournalRowView>,
    /// The row under the cursor, an index into `rows`.
    pub selected: usize,
    /// What to draw when there are no rows.
    pub status: JournalStatus,
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

/// The description column's width: what is left once the prefix, the age
/// and the mark have had theirs, capped so a long description does not push
/// every summary off a wide screen. The summaries line up under each other,
/// which is what makes the list scannable.
fn description_columns(width: u16) -> usize {
    const PREFIX_AND_AGE: usize = 2 + 9 + 1;
    const MARK: usize = 13;
    usize::from(width)
        .saturating_sub(PREFIX_AND_AGE + MARK)
        .saturating_sub(30)
        .min(44)
}

/// `text` padded to `columns` display columns, or left alone when it is
/// already wider — the row clips at the right edge either way.
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
    let columns = u16::try_from(drawn.chars().count()).unwrap_or(area.width);
    let x = area.x + area.width.saturating_sub(columns) / 2;
    let y = area.y + area.height / 2;
    set_text(buf, x, y, area, &drawn, style);
}

impl JournalScreen<'_> {
    /// `UNDO JOURNAL · ~/code/api`.
    fn header(&self) -> String {
        format!("UNDO JOURNAL · {}", self.view.location.as_str())
    }

    /// The keys, spelled out: `Enter` is the whole reason the browser is
    /// reachable at all, so it goes first.
    fn footer(&self) -> &'static str {
        match self.view.status {
            JournalStatus::Ready => "⏎ undo this  space command log  J/esc close",
            JournalStatus::Loading | JournalStatus::Empty | JournalStatus::Failed(_) => {
                "J/esc close"
            }
        }
    }

    /// Draws the rows from `area.y` down, stopping at the bottom of `area`.
    /// A row that does not fit is simply not drawn; keeping the cursor row
    /// in view is the caller's scrolling to do.
    fn draw_rows(&self, buf: &mut Buffer, area: Rect) {
        let mut y = area.y;
        for (index, row) in self.view.rows.iter().enumerate() {
            if y >= area.bottom() {
                return;
            }
            y = self.draw_row(buf, area, y, row, index == self.view.selected);
        }
    }

    /// One row, plus its Command Log and failure line while it is expanded.
    /// Returns the `y` the next row starts at.
    fn draw_row(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        row: &JournalRowView,
        on_cursor: bool,
    ) -> u16 {
        let text = foreground(self.theme, Token::Text);
        let warning = foreground(self.theme, Token::Warning);
        let style = if on_cursor {
            text.bg(crate::paint::color(self.theme.token(Token::Selection)))
                .add_modifier(Modifier::BOLD)
        } else {
            text
        };
        if on_cursor {
            // Painted first, so the cursor's background reaches the right
            // edge rather than stopping where the text does.
            let blanks = " ".repeat(usize::from(area.width));
            set_text(buf, area.x, y, area, &blanks, style);
        }
        let prefix = if on_cursor { "▸ " } else { "  " };
        let line = format!(
            "{prefix}{:<9} {} {}",
            row.age.as_str(),
            pad(row.description.as_str(), description_columns(area.width)),
            row.summary.as_str()
        );
        set_text(buf, area.x, y, area, &line, style);
        self.draw_mark(buf, area, y, row, style);
        self.draw_log(buf, area, y.saturating_add(1), row, warning)
    }

    /// The `← u` / `interrupted` mark, right-aligned so the eye finds it in
    /// the same column on every row.
    fn draw_mark(&self, buf: &mut Buffer, area: Rect, y: u16, row: &JournalRowView, style: Style) {
        let mark = row.mark.label();
        if mark.is_empty() {
            return;
        }
        let columns = u16::try_from(mark.chars().count()).unwrap_or(area.width);
        let x = area
            .right()
            .saturating_sub(columns.saturating_add(1))
            .max(area.x);
        let style = match row.mark {
            RowMark::Interrupted => foreground(self.theme, Token::Warning),
            _ => style,
        };
        set_text(buf, x, y, area, mark, style);
    }

    /// The expanded row's reason and Command Log. Returns the `y` after it.
    fn draw_log(
        &self,
        buf: &mut Buffer,
        area: Rect,
        y: u16,
        row: &JournalRowView,
        warning: Style,
    ) -> u16 {
        if !row.expanded {
            return y;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        let mut y = y;
        if let Some(failure) = &row.failure {
            if y >= area.bottom() {
                return y;
            }
            set_text(
                buf,
                area.x,
                y,
                area,
                &format!("    {}", failure.as_str()),
                warning,
            );
            y = y.saturating_add(1);
        }
        for command in &row.commands {
            if y >= area.bottom() {
                return y;
            }
            set_text(
                buf,
                area.x,
                y,
                area,
                &format!("    $ {}", command.as_str()),
                muted,
            );
            y = y.saturating_add(1);
        }
        y
    }
}

impl Widget for JournalScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        set_text(
            buf,
            area.x,
            area.y,
            area,
            &self.header(),
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        if area.height == 1 {
            return;
        }
        let footer_y = area.bottom().saturating_sub(1);
        set_text(buf, area.x, footer_y, area, self.footer(), muted);
        if area.height == 2 {
            return;
        }
        let body = Rect {
            x: area.x,
            y: area.y.saturating_add(1),
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        match &self.view.status {
            JournalStatus::Ready => self.draw_rows(buf, body),
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
            failure: None,
            expanded: false,
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
        }
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
        assert!(grid[0].starts_with("UNDO JOURNAL · ~/code/api"), "{grid:?}");
        assert!(grid[1].contains("2m ago"), "{grid:?}");
        assert!(grid[1].contains("← u"), "{grid:?}");
        assert!(
            !grid[2].contains('←'),
            "only the next one is marked: {grid:?}"
        );
    }

    #[test]
    fn an_expanded_row_shows_its_command_log_under_it() {
        let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
        rows[0].expanded = true;
        let grid = draw(&view(rows, JournalStatus::Ready), 60, 8);
        assert!(grid[2].contains("$ git commit -F -"), "{grid:?}");
    }

    #[test]
    fn an_aborted_rows_reason_is_shown_in_full_when_it_is_expanded() {
        let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
        rows[0].expanded = true;
        rows[0].failure = Some(sanitize("failed: hook rejected the commit"));
        let grid = draw(&view(rows, JournalStatus::Ready), 60, 8);
        assert!(grid[2].contains("failed: hook rejected"), "{grid:?}");
        assert!(grid[3].contains("$ git commit"), "{grid:?}");
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
            assert!(grid[0].starts_with("UNDO JOURNAL"), "{grid:?}");
        }
    }
}
