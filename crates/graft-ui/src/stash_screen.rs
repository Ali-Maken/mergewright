//! The stash overlay (`S`, DESIGN.md §1.11): every stash entry this
//! repository holds, what each one touched, and the save prompt
//! (m3-working-tree-undo-and-stash/06).
//!
//! An overlay over the graph area, like `journal_screen`: it owns no state
//! and no I/O, and `view::stash::stash_view` (crate `graft`) builds its
//! input. Every fact is in the text — `stash@{0}`, `3 files`, the chosen
//! scope's `(•)` — because a capture carries no colour and neither does a
//! `NO_COLOR` terminal.

use graft_term::sanitize::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

use crate::detail::{FileListView, FileRowKind};
use crate::fit::fit;
use crate::paint::foreground;
use crate::text_input::{FieldView, FieldWidget};
use crate::theme::{Theme, Token};

/// The empty state, which is also the instruction for leaving it.
const EMPTY: &str = "no stashes — s stashes the working tree";

/// One stash entry, as the overlay draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StashRowView {
    /// `stash@{0}`, git's own name for the entry.
    pub name: SanitizedText,
    /// The message git recorded, already quoted by the builder.
    pub message: SanitizedText,
    /// How long ago, in words: `2h ago`.
    pub age: SanitizedText,
    /// How many paths the stash touched.
    pub files: usize,
}

/// The save prompt (`s`): a message line and the scope toggle.
#[derive(Debug)]
pub struct StashPromptView<'a> {
    /// What the prompt is about: `Stash the working tree`, or the refusal's
    /// own title on a clean tree.
    pub title: SanitizedText,
    /// The message field, always focused — it is the only text here.
    pub message: FieldView<'a>,
    /// `true` when `--staged` is chosen; `false` is `-u`, the default.
    pub staged_only: bool,
    /// Why `Enter` would not save. Drawn under the options row; `None` when
    /// the save may go ahead.
    pub refusal: Option<SanitizedText>,
}

/// What draws instead of, or with, the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StashStatus {
    /// The read is in flight.
    Loading,
    /// The list is there and empty.
    Empty,
    /// The list could not be read; the line is git's own.
    Failed(SanitizedText),
    /// There are rows.
    Ready,
}

/// Everything the overlay draws.
#[derive(Debug)]
pub struct StashView<'a> {
    /// Newest first, `stash@{0}` at the top.
    pub rows: Vec<StashRowView>,
    /// The row under the cursor, an index into `rows`.
    pub selected: usize,
    /// The selected stash's files, once `Space` has asked for them.
    pub preview: Option<&'a FileListView>,
    /// The save prompt, while it is open. Modal within the overlay.
    pub prompt: Option<StashPromptView<'a>>,
    /// What to draw when there are no rows.
    pub status: StashStatus,
    /// What is happening or just happened — a running write, a refusal, a
    /// finished apply. The overlay covers the graph screen's own footer, so
    /// without this line a write started from here would have no
    /// acknowledgement at all (m3-working-tree-undo-and-stash/06).
    pub notice: Option<SanitizedText>,
    /// Is `notice` a failure? Drawn in `Warning` rather than muted.
    pub notice_is_failure: bool,
}

/// The overlay itself. Carries the theme like every other screen; the
/// caller paints the surface underneath.
#[derive(Debug)]
pub struct StashScreen<'a> {
    /// What to draw.
    pub view: &'a StashView<'a>,
    /// Resolved colours.
    pub theme: &'a Theme,
}

/// `3 files` / `1 file`, because "1 files" reads as a bug in the counter.
#[must_use]
pub fn files_label(files: usize) -> String {
    if files == 1 {
        "1 file".to_owned()
    } else {
        format!("{files} files")
    }
}

/// The message column's width: what is left once the name, the age and the
/// file count have had theirs, capped so one long message does not push
/// every age off a wide screen.
fn message_columns(width: u16) -> usize {
    const NAME: usize = 2 + 11 + 1;
    const AGE_AND_FILES: usize = 9 + 1 + 9;
    usize::from(width)
        .saturating_sub(NAME + AGE_AND_FILES)
        .min(48)
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
/// it. Mirrors `journal_screen`'s own guard.
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

/// The status letter a file row carries in the preview, the same one the
/// detail pane draws.
fn status_letter(row: &crate::detail::FileRow) -> Option<char> {
    match &row.kind {
        FileRowKind::File { status, .. } => Some(crate::detail_pane::status_letter(status)),
        _ => None,
    }
}

impl StashScreen<'_> {
    /// `STASHES (2)`.
    fn header(&self) -> String {
        format!("STASHES ({})", self.view.rows.len())
    }

    /// The keys, spelled out. `⏎ apply` first: it is what the list is for.
    fn footer(&self) -> &'static str {
        if self.view.prompt.is_some() {
            return "⏎ save  tab scope  esc back";
        }
        match self.view.status {
            StashStatus::Ready => "⏎ apply  p pop  s save  d drop  space files  esc close",
            StashStatus::Loading | StashStatus::Empty | StashStatus::Failed(_) => {
                "s save  esc close"
            }
        }
    }

    /// One row of the list.
    fn draw_row(&self, buf: &mut Buffer, area: Rect, y: u16, index: usize) {
        let Some(row) = self.view.rows.get(index) else {
            return;
        };
        let on_cursor = index == self.view.selected;
        let text = foreground(self.theme, Token::Text);
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
            "{prefix}{:<11} {} {:<9}{}",
            row.name.as_str(),
            pad(row.message.as_str(), message_columns(area.width)),
            row.age.as_str(),
            files_label(row.files),
        );
        set_text(buf, area.x, y, area, &line, style);
    }

    /// The selected stash's files under the list, once `Space` has asked.
    /// A file list with counts and no diff body: the overlay's job is to
    /// say *what* a stash would bring back, and the diff of a stash is the
    /// detail pane's own view (m3-working-tree-undo-and-stash/06).
    fn draw_preview(&self, buf: &mut Buffer, area: Rect, preview: &FileListView) {
        if area.is_empty() {
            return;
        }
        let muted = foreground(self.theme, Token::TextMuted);
        let text = foreground(self.theme, Token::Text);
        let name = self
            .view
            .rows
            .get(self.view.selected)
            .map_or("", |row| row.name.as_str());
        set_text(buf, area.x, area.y, area, &format!("── {name} ──"), muted);
        let mut y = area.y.saturating_add(1);
        for (_, row) in preview.visible() {
            if y >= area.bottom() {
                return;
            }
            let indent = "  ".repeat(usize::from(row.depth) + 1);
            let line = match &row.kind {
                FileRowKind::File { added, removed, .. } => {
                    let letter = status_letter(row).unwrap_or(' ');
                    // A binary file has no line counts at all, and "+0 −0"
                    // would be a lie rather than an absence.
                    let counts = match (added, removed) {
                        (Some(added), Some(removed)) => format!("  +{added} −{removed}"),
                        _ => "  binary".to_owned(),
                    };
                    format!("{indent}{letter} {}{counts}", row.name.as_str())
                }
                _ => format!("{indent}{}", row.name.as_str()),
            };
            set_text(buf, area.x, y, area, &line, text);
            y = y.saturating_add(1);
        }
    }

    /// The save prompt: title, the message field, the scope options, and
    /// the refusal when there is one.
    fn draw_prompt(&self, buf: &mut Buffer, area: Rect, prompt: &StashPromptView<'_>) {
        if area.is_empty() {
            return;
        }
        let text = foreground(self.theme, Token::Text);
        set_text(
            buf,
            area.x,
            area.y,
            area,
            prompt.title.as_str(),
            text.add_modifier(Modifier::BOLD),
        );
        let field_y = area.y.saturating_add(1);
        if field_y >= area.bottom() {
            return;
        }
        FieldWidget {
            view: &prompt.message,
            theme: self.theme,
            ascii: false,
        }
        .render(
            Rect {
                x: area.x,
                y: field_y,
                width: area.width,
                height: 1,
            },
            buf,
        );
        self.draw_options(buf, area, field_y.saturating_add(1), prompt);
    }

    /// The scope row and the line under it: the refusal when there is one,
    /// else the reassurance that a stash is undoable like every other write.
    fn draw_options(&self, buf: &mut Buffer, area: Rect, y: u16, prompt: &StashPromptView<'_>) {
        let (all, staged) = if prompt.staged_only {
            ("( )", "(•)")
        } else {
            ("(•)", "( )")
        };
        set_text(
            buf,
            area.x,
            y,
            area,
            &format!("  {all} all incl. untracked   {staged} staged only   (tab)"),
            foreground(self.theme, Token::Text),
        );
        let (line, style) = match &prompt.refusal {
            Some(refusal) => (
                format!("  {}", refusal.as_str()),
                foreground(self.theme, Token::Warning),
            ),
            None => (
                "  Graft journals it, so u takes the stash back.".to_owned(),
                foreground(self.theme, Token::TextMuted),
            ),
        };
        set_text(buf, area.x, y.saturating_add(1), area, &line, style);
    }

    /// The list, and the selected stash's files under it once `Space` has
    /// asked for them.
    fn draw_list(&self, buf: &mut Buffer, body: Rect) {
        // The list keeps at most half the body once a preview is open, so
        // the files of the selected stash are never one row of context.
        let listed = match self.view.preview {
            Some(_) => self
                .view
                .rows
                .len()
                .min(usize::from(body.height).saturating_div(2).max(1)),
            None => self.view.rows.len().min(usize::from(body.height)),
        };
        for index in 0..listed {
            let y = body
                .y
                .saturating_add(u16::try_from(index).unwrap_or(u16::MAX));
            self.draw_row(buf, body, y, index);
        }
        if let Some(preview) = self.view.preview {
            let used = u16::try_from(listed).unwrap_or(body.height);
            let rest = Rect {
                x: body.x,
                y: body.y.saturating_add(used),
                width: body.width,
                height: body.height.saturating_sub(used),
            };
            self.draw_preview(buf, rest, preview);
        }
    }
}

impl Widget for StashScreen<'_> {
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
        let mut body = Rect {
            x: area.x,
            y: area.y.saturating_add(1),
            width: area.width,
            height: area.height.saturating_sub(2),
        };
        if let Some(notice) = &self.view.notice {
            let notice_y = footer_y.saturating_sub(1);
            let style = if self.view.notice_is_failure {
                foreground(self.theme, Token::Warning)
            } else {
                muted
            };
            set_text(buf, area.x, notice_y, area, notice.as_str(), style);
            body.height = body.height.saturating_sub(1);
        }
        if let Some(prompt) = &self.view.prompt {
            self.draw_prompt(buf, body, prompt);
            return;
        }
        match &self.view.status {
            StashStatus::Ready => {}
            StashStatus::Loading => return centred(buf, body, "reading the stash list…", muted),
            StashStatus::Empty => return centred(buf, body, EMPTY, muted),
            StashStatus::Failed(why) => {
                return centred(
                    buf,
                    body,
                    &format!("the stash list could not be read: {}", why.as_str()),
                    foreground(self.theme, Token::Warning),
                );
            }
        }
        self.draw_list(buf, body);
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

    fn row(name: &str, message: &str, age: &str, files: usize) -> StashRowView {
        StashRowView {
            name: sanitize(name),
            message: sanitize(message),
            age: sanitize(age),
            files,
        }
    }

    fn view<'a>(rows: Vec<StashRowView>, status: StashStatus) -> StashView<'a> {
        StashView {
            rows,
            selected: 0,
            preview: None,
            prompt: None,
            status,
            notice: None,
            notice_is_failure: false,
        }
    }

    fn grid(view: &StashView<'_>, width: u16, height: u16) -> Vec<String> {
        let theme = theme();
        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("a test backend opens");
        terminal
            .draw(|frame| {
                frame.render_widget(
                    StashScreen {
                        view,
                        theme: &theme,
                    },
                    frame.area(),
                );
            })
            .expect("drawing into a test backend");
        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| buffer[(x, y)].symbol().to_owned())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
            .collect()
    }

    #[test]
    fn the_header_counts_the_entries_and_the_rows_read_as_the_design_says() {
        let view = view(
            vec![
                row("stash@{0}", "\"token refresh\"", "2h ago", 3),
                row("stash@{1}", "\"quick fix\"", "1d ago", 1),
            ],
            StashStatus::Ready,
        );
        let lines = grid(&view, 80, 10);
        assert_eq!(lines[0], "STASHES (2)");
        assert!(lines[1].contains("stash@{0}"), "{:?}", lines[1]);
        assert!(lines[1].contains("\"token refresh\""), "{:?}", lines[1]);
        assert!(lines[1].contains("2h ago"), "{:?}", lines[1]);
        assert!(lines[1].contains("3 files"), "{:?}", lines[1]);
        assert!(lines[2].contains("1 file"), "{:?}", lines[2]);
        assert!(!lines[2].contains("1 files"), "{:?}", lines[2]);
    }

    #[test]
    fn an_empty_list_says_what_makes_one() {
        let lines = grid(&view(Vec::new(), StashStatus::Empty), 80, 10);
        assert!(lines.iter().any(|line| line.contains(EMPTY)), "{lines:?}");
    }

    #[test]
    fn a_failed_read_says_why() {
        let lines = grid(
            &view(Vec::new(), StashStatus::Failed(sanitize("reflog is gone"))),
            80,
            10,
        );
        assert!(
            lines.iter().any(|line| line.contains("reflog is gone")),
            "{lines:?}"
        );
    }

    #[test]
    fn the_prompt_marks_the_chosen_scope_in_text_not_colour() {
        let field = FieldView {
            label: "message",
            text: sanitize("wip"),
            cursor: Some(3),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        };
        let mut screen = view(Vec::new(), StashStatus::Empty);
        screen.prompt = Some(StashPromptView {
            title: sanitize("Stash the working tree"),
            message: field,
            staged_only: true,
            refusal: None,
        });
        let lines = grid(&screen, 80, 10);
        let options = lines
            .iter()
            .find(|line| line.contains("staged only"))
            .expect("the options row draws");
        assert!(options.contains("( ) all incl. untracked"), "{options:?}");
        assert!(options.contains("(•) staged only"), "{options:?}");
    }

    #[test]
    fn a_refused_save_says_so_under_the_options() {
        let field = FieldView {
            label: "message",
            text: sanitize(""),
            cursor: Some(0),
            guide_at: None,
            counter: None,
            focused: true,
            secret: None,
        };
        let mut screen = view(Vec::new(), StashStatus::Empty);
        screen.prompt = Some(StashPromptView {
            title: sanitize("Stash the working tree"),
            message: field,
            staged_only: false,
            refusal: Some(sanitize("nothing to stash")),
        });
        let lines = grid(&screen, 80, 10);
        assert!(
            lines.iter().any(|line| line.contains("nothing to stash")),
            "{lines:?}"
        );
    }

    #[test]
    fn a_notice_draws_above_the_footer() {
        let mut screen = view(
            vec![row("stash@{0}", "\"a\"", "2h ago", 1)],
            StashStatus::Ready,
        );
        screen.notice = Some(sanitize("stash touches files you changed: a.rs"));
        screen.notice_is_failure = true;
        let lines = grid(&screen, 80, 10);
        assert!(
            lines[8].contains("stash touches files you changed"),
            "{lines:?}"
        );
        assert!(lines[9].contains("apply"), "{lines:?}");
    }

    #[test]
    fn it_draws_nothing_outside_a_tiny_area() {
        for height in [0_u16, 1, 2, 3] {
            let view = view(
                vec![row("stash@{0}", "\"a\"", "2h ago", 1)],
                StashStatus::Ready,
            );
            let lines = grid(&view, 40, height.max(1));
            assert_eq!(lines.len(), usize::from(height.max(1)));
        }
    }
}
