//! The finish summary (`c` on a merge, m6-conflicts/19): what will land —
//! the files the user resolved, the files git combined on its own, and the
//! message — before `y` commits it. It is drawn as a preview, inside the
//! same rounded frame as the merge preview: title in the top border, keys in
//! the bottom one (`docs/design-notes/m6-conflicts.md` Amendment 2, "Finish
//! summary", and Amendment 2026-09-24, "The finish summary is a preview").
//!
//! Owns no state and reads no repository. `graft::update::finish` holds the
//! state and `graft::view::finish` turns it into the [`FinishView`] this
//! draws. The layout is pure arithmetic over the view and the area
//! ([`layout`], [`list_lines`]), and the app's keys and mouse ask the same
//! functions where a row is ([`hit`], [`top_for_cursor`]), so what is drawn
//! and what a click lands on cannot disagree.
//!
//! Every fact is in **characters**: the cursor row is `▸`, a problem notice
//! says so in words. A capture records the grid and nothing else
//! (CLAUDE.md rule 5); colour ships unverified.

use std::sync::Arc;

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::detail::{DiffView, LineKindView};
use crate::detail_pane::{DiffFocus, diff_body};
use crate::fit::fit;
use crate::frame_box::border_row_spans;
use crate::graph_screen::{Divider, Hint, HintContext, hints_for, hints_text};
use crate::paint::foreground;
use crate::text_input::{AreaView, AreaWidget, FOCUS, FieldView, FieldWidget};
use crate::theme::{Theme, Token};

/// The smallest area the summary draws in, `(columns, rows)`. Below it the
/// widget draws one line, `finish needs 62x18 · esc back`, and the app
/// answers only `Esc` (and `Ctrl+C`). (60, 14) grown by the frame: its two
/// side columns, its two border rows and the two preview lines.
pub const FINISH_MIN: (u16, u16) = (62, 18);

/// From this width the file lists take the left 60 % and the message the
/// right; below it, one column.
pub const TWO_COLUMN_MIN: u16 = 160;

/// The most rows an expanded diff draws under its row. More than that adds
/// one row, `… N more lines · ⏎ hides`.
pub const DIFF_ROWS_MAX: usize = 12;

/// Cells an expanded diff is indented by, under its row.
const DIFF_INDENT: u16 = 4;

/// Rows the message block needs at the least in one column: `MESSAGE`, the
/// subject, the body's label and one body line.
const MESSAGE_MIN_ROWS: u16 = 4;

/// One file row: its path and what the summary says about it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SummaryRow {
    /// The path.
    pub path: SanitizedText,
    /// `theirs · feature in 2 of 2 conflicts`, `modified  +1 −1`, ….
    pub what: SanitizedText,
}

/// What has the keyboard in the summary.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SummaryFocus {
    /// The file lists.
    Files,
    /// The subject field.
    Subject,
    /// The body area.
    Body,
}

/// The diff drawn under an expanded row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExpandedDiff {
    /// Asked for, not arrived.
    Loading,
    /// The read failed, and why: drawn in place of the diff.
    Failed(SanitizedText),
    /// Arrived.
    Ready(Arc<DiffView>),
}

/// Whether the summary itself has been read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SummaryState {
    /// `reading what this merge changes…`.
    Loading,
    /// The lists are the summary's.
    Ready,
    /// The read failed: `summary unavailable: <why> · y commits with git's
    /// message`.
    Unavailable(SanitizedText),
}

/// Everything the summary draws.
#[derive(Clone, Debug)]
pub struct FinishView<'a> {
    /// `(from, into)`: the branch merged in and the one it lands on. `None`
    /// while the read has not landed, or when the names are not known: the
    /// title then reads `Preview: finish the merge`, never an invented name.
    pub names: Option<(SanitizedText, SanitizedText)>,
    /// The files the merge conflicted on, worded by what the user chose.
    pub by_you: Vec<SummaryRow>,
    /// Every other staged change: the files git combined on its own.
    pub by_git: Vec<SummaryRow>,
    /// Whether it is known which files were conflicts. `false`: one section,
    /// `CHANGED BY THIS MERGE`, and every row is in `by_git`.
    pub recorded: bool,
    /// The cursor row, counted over `by_you` then `by_git`.
    pub cursor: usize,
    /// The row whose diff is shown under it, and the diff.
    pub expanded: Option<(usize, ExpandedDiff)>,
    /// The subject field.
    pub subject: FieldView<'a>,
    /// The body area.
    pub body: AreaView<'a>,
    /// What has the keyboard.
    pub focus: SummaryFocus,
    /// The cursor stands on a row git combined: the footer offers `⏎ diff`
    /// (`HintContext::Finish`'s own meaning).
    pub expandable: bool,
    /// The frame has the keyboard: its border is `accent` and its keys are
    /// drawn. `false` under a live box (a confirm, a prompt), which claims
    /// the next key, so the summary's keys would do nothing (rulings 07b,
    /// Q1).
    pub focused: bool,
    /// A line above the bottom border, and whether it is a problem.
    pub notice: Option<(SanitizedText, bool)>,
    /// The commit is running.
    pub busy: bool,
    /// Whether the summary has been read.
    pub state: SummaryState,
    /// The first list line on screen.
    pub top: usize,
}

impl FinishView<'_> {
    /// Rows in both lists together: what the cursor counts over.
    #[must_use]
    pub fn row_count(&self) -> usize {
        self.by_you.len() + self.by_git.len()
    }

    /// The row the cursor index names.
    fn row(&self, index: usize) -> Option<&SummaryRow> {
        self.by_you
            .get(index)
            .or_else(|| self.by_git.get(index.checked_sub(self.by_you.len())?))
    }
}

/// The top border's words for a frame `width` cells wide. With names: the
/// full title, else the short one, else the short one cut with `…`.
/// Without: `Preview: finish the merge`, cut with `…` if it must be. What
/// comes back always fits the border whole (`width` less the corners and
/// the gutter either side).
#[must_use]
pub fn preview_title(names: Option<(&str, &str)>, width: u16) -> String {
    let room = usize::from(width).saturating_sub(4);
    let ladder = match names {
        Some((from, into)) => vec![
            format!("Preview: finish merging {from} into {into}"),
            format!("Preview: merge {from} → {into}"),
        ],
        None => vec!["Preview: finish the merge".to_owned()],
    };
    for title in &ladder {
        if crate::frame_box::cells(title) <= room {
            return title.clone();
        }
    }
    fit(ladder.last().map_or("", String::as_str), room)
}

/// The first line inside the frame: what lands, and that nothing does
/// until `y`. `on <into>` only when the branch is known.
#[must_use]
pub fn preview_line(into: Option<&str>) -> String {
    match into {
        Some(into) => format!("1 merge commit on {into} · nothing is committed until y"),
        None => "1 merge commit · nothing is committed until y".to_owned(),
    }
}

/// The keys the bottom border draws in a frame `width` cells wide: the
/// `HintContext::Finish` list for the focus, fitted to the border. None
/// while the commit runs or a box has the keyboard, when none of them acts.
#[must_use]
pub fn footer_hints(view: &FinishView<'_>, width: u16) -> Vec<Hint> {
    if view.busy || !view.focused {
        return Vec::new();
    }
    let context = HintContext::Finish {
        focus: view.focus,
        expandable: view.expandable,
    };
    hints_for(&context, Divider::Hidden, width.saturating_sub(4))
}

/// The second line inside the frame.
pub const UNDO_LINE: &str = "undo: u takes this back";

/// [`FinishView`] drawn into the area under the header and above the bar.
#[derive(Debug)]
pub struct FinishScreen<'a> {
    /// What to draw.
    pub view: &'a FinishView<'a>,
    /// Colours.
    pub theme: &'a Theme,
    /// The ASCII renderer tier: `>` for the cursor, `->` in the title.
    pub ascii: bool,
}

impl Widget for &FinishScreen<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        // The summary replaces the graph and the detail pane both, so it
        // owns its background.
        buf.set_style(
            area,
            Style::new().bg(crate::paint::color(self.theme.token(Token::Bg))),
        );
        let Some(placed) = layout(self.view, area) else {
            let (columns, rows) = FINISH_MIN;
            buf.set_string(
                area.x,
                area.y,
                fit(
                    &format!("finish needs {columns}x{rows} · esc back"),
                    usize::from(area.width),
                ),
                foreground(self.theme, Token::Warning),
            );
            return;
        };
        self.draw_frame(buf, placed.frame);
        self.draw_preview(buf, placed.preview);
        self.draw_list(buf, placed.list);
        self.draw_message(buf, &placed);
        if let (Some(rect), Some((words, problem))) = (placed.notice, &self.view.notice) {
            let token = if *problem {
                Token::Warning
            } else {
                Token::TextMuted
            };
            self.text(buf, rect, words.as_str(), token, false);
        }
    }
}

impl FinishScreen<'_> {
    /// `text` on one row of `rect`, fitted to its width.
    fn text(&self, buf: &mut Buffer, rect: Rect, text: &str, token: Token, bold: bool) {
        let mut style = foreground(self.theme, token);
        if bold {
            style = style.add_modifier(Modifier::BOLD);
        }
        buf.set_string(rect.x, rect.y, fit(text, usize::from(rect.width)), style);
    }

    /// The rounded frame: `accent` while it has focus (`border` under a
    /// live box), the title in the
    /// top border in `accent` BOLD, the keys in the bottom border with the
    /// first key `accent` BOLD and the rest muted, as the merge preview's
    /// footer is. While the commit runs no key acts, so none is drawn.
    /// Under `NO_COLOR` every token paints nothing: plain glyphs, and the
    /// title and first key still BOLD.
    fn draw_frame(&self, buf: &mut Buffer, frame: Rect) {
        if frame.width < 2 || frame.height < 2 {
            return;
        }
        let line = if self.view.focused {
            foreground(self.theme, Token::Accent)
        } else {
            foreground(self.theme, Token::Border)
        };
        let strong = line.add_modifier(Modifier::BOLD);
        let muted = foreground(self.theme, Token::TextMuted);
        let names = self
            .view
            .names
            .as_ref()
            .map(|(from, into)| (from.as_str(), into.as_str()));
        // `→` is two cells at the ASCII tier (`->`): measure for it.
        let room = frame.width.saturating_sub(u16::from(self.ascii));
        let title = self.ascii_safe(&preview_title(names, room));
        let top = border_row_spans('╭', '╮', &[(&title, strong)], line, frame.width);
        buf.set_line(frame.x, frame.y, &top, frame.width);
        for y in frame.y + 1..frame.bottom() - 1 {
            buf.set_string(frame.x, y, "│", line);
            buf.set_string(frame.right() - 1, y, "│", line);
        }
        let keys = hints_text(&footer_hints(self.view, frame.width))
            .trim_start()
            .to_owned();
        let first = keys.split(' ').next().unwrap_or_default();
        let rest = &keys[first.len()..];
        let bottom = border_row_spans(
            '╰',
            '╯',
            &[(first, strong), (rest, muted)],
            line,
            frame.width,
        );
        buf.set_line(frame.x, frame.bottom() - 1, &bottom, frame.width);
    }

    /// The two lines the merge preview opens with: what lands, and how to
    /// take it back.
    fn draw_preview(&self, buf: &mut Buffer, rect: Rect) {
        if rect.height < 2 {
            return;
        }
        let into = self.view.names.as_ref().map(|(_, into)| into.as_str());
        self.text(buf, rect, &preview_line(into), Token::Text, false);
        let second = Rect {
            y: rect.y + 1,
            height: 1,
            ..rect
        };
        self.text(buf, second, UNDO_LINE, Token::TextMuted, false);
    }

    /// The title's arrow at the ASCII tier.
    fn ascii_safe(&self, text: &str) -> String {
        if self.ascii {
            text.replace('→', "->")
        } else {
            text.to_owned()
        }
    }

    /// The cursor glyph.
    fn focus_glyph(&self) -> char {
        if self.ascii { '>' } else { FOCUS }
    }

    /// The file list from `view.top`, into `rect`.
    fn draw_list(&self, buf: &mut Buffer, rect: Rect) {
        let lines = list_lines(self.view);
        let path_width = self
            .view
            .by_you
            .iter()
            .chain(&self.view.by_git)
            .map(|row| row.path.as_str().chars().count())
            .max()
            .unwrap_or(0);
        let mut y = rect.y;
        let mut at = self.view.top;
        while y < rect.bottom() {
            let Some(line) = lines.get(at) else {
                break;
            };
            let one = Rect {
                y,
                height: 1,
                ..rect
            };
            match *line {
                ListLine::Status => self.draw_status(buf, one),
                ListLine::Section(section) => {
                    let words = self.section_words(section);
                    self.text(buf, one, &words, Token::TextMuted, true);
                }
                ListLine::Row(index) => self.draw_row(buf, one, index, path_width),
                ListLine::More { hidden, .. } => {
                    let indented = indent(one);
                    let words = format!("… {hidden} more lines · ⏎ hides");
                    self.text(buf, indented, &words, Token::TextMuted, false);
                }
                ListLine::Diff { nth, .. } => {
                    // The whole visible run of this diff in one call, so a
                    // hunk heading is drawn where `diff_body` puts it.
                    let run = lines[at..]
                        .iter()
                        .take_while(|line| matches!(line, ListLine::Diff { .. }))
                        .count();
                    let height = run.min(usize::from(rect.bottom() - y));
                    let height = u16::try_from(height).unwrap_or(u16::MAX);
                    self.draw_diff(
                        buf,
                        Rect {
                            height,
                            ..indent(one)
                        },
                        nth,
                    );
                    at += usize::from(height);
                    y += height;
                    continue;
                }
            }
            at += 1;
            y += 1;
        }
    }

    /// The list's one line while there are no lists.
    fn draw_status(&self, buf: &mut Buffer, rect: Rect) {
        match &self.view.state {
            SummaryState::Loading | SummaryState::Ready => {
                self.text(
                    buf,
                    rect,
                    "reading what this merge changes…",
                    Token::TextMuted,
                    false,
                );
            }
            SummaryState::Unavailable(why) => {
                let words = format!(
                    "summary unavailable: {} · y commits with git's message",
                    why.as_str()
                );
                self.text(buf, rect, &words, Token::Warning, false);
            }
        }
    }

    /// A section heading's words.
    fn section_words(&self, section: Section) -> String {
        match section {
            Section::ByYou => format!("RESOLVED BY YOU · {}", self.view.by_you.len()),
            Section::ByGit if self.view.by_git.is_empty() => {
                "COMBINED BY GIT ON ITS OWN · none".to_owned()
            }
            Section::ByGit => format!(
                "COMBINED BY GIT ON ITS OWN · {} · ⏎ shows the change",
                self.view.by_git.len()
            ),
            Section::Unrecorded => format!(
                "CHANGED BY THIS MERGE · {} · which were conflicts is not recorded",
                self.view.by_git.len()
            ),
        }
    }

    /// `▸ path  what`, the path padded to the widest one.
    fn draw_row(&self, buf: &mut Buffer, rect: Rect, index: usize, path_width: usize) {
        let Some(row) = self.view.row(index) else {
            return;
        };
        let cursor = index == self.view.cursor;
        let marker = if cursor { self.focus_glyph() } else { ' ' };
        let path = row.path.as_str();
        let pad = path_width.saturating_sub(path.chars().count());
        let line = format!("{marker} {path}{}  {}", " ".repeat(pad), row.what.as_str());
        let token = if cursor && self.view.focus == SummaryFocus::Files {
            Token::Text
        } else {
            Token::TextMuted
        };
        let mut style = foreground(self.theme, token);
        if cursor {
            style = style.bg(crate::paint::color(self.theme.token(Token::Selection)));
        }
        buf.set_string(rect.x, rect.y, fit(&line, usize::from(rect.width)), style);
        // The words take the colours that tie them back to the panes and to
        // diffs; the characters already say it all (rule 5).
        let before = crate::frame_box::cells(&format!("{marker} {path}{}  ", " ".repeat(pad)));
        let what = row.what.as_str();
        for (start, end, token) in tints(what) {
            let x = before + crate::frame_box::cells(&what[..start]);
            let width = crate::frame_box::cells(&what[start..end]);
            let x = rect.x.saturating_add(u16::try_from(x).unwrap_or(u16::MAX));
            if x >= rect.right() {
                continue;
            }
            let width = u16::try_from(width)
                .unwrap_or(u16::MAX)
                .min(rect.right() - x);
            buf.set_style(
                Rect {
                    x,
                    y: rect.y,
                    width,
                    height: 1,
                },
                Style::new().fg(crate::paint::color(self.theme.token(token))),
            );
        }
    }

    /// The expanded diff's lines from `nth`, into `rect`.
    fn draw_diff(&self, buf: &mut Buffer, rect: Rect, nth: usize) {
        let Some((_, diff)) = &self.view.expanded else {
            return;
        };
        match diff {
            ExpandedDiff::Loading => {
                self.text(buf, rect, "reading the diff…", Token::TextMuted, false);
            }
            ExpandedDiff::Failed(why) => {
                self.text(buf, rect, why.as_str(), Token::Warning, false);
            }
            ExpandedDiff::Ready(view) => match &view.binary {
                Some(note) => self.text(buf, rect, note.as_str(), Token::TextMuted, false),
                None if view.hunks.is_empty() => {
                    self.text(buf, rect, "no change to show", Token::TextMuted, false);
                }
                // `truncated: None`: the summary draws its own `… N more
                // lines · space hides`, and `L load all` is not one of its
                // keys (ticket 19 S5). The row count `diff_body` returns is
                // for scroll clamping; `list_lines` already counted the rows
                // (`diff_extent`), so it is not needed here.
                None => {
                    let _rows: usize = diff_body(
                        &view.hunks,
                        nth,
                        None,
                        rect,
                        buf,
                        self.theme,
                        DiffFocus {
                            focused: false,
                            word_tint: true,
                            cursor: None,
                        },
                    );
                }
            },
        }
    }

    /// `MESSAGE · ⇥ edit`, the subject and the body.
    fn draw_message(&self, buf: &mut Buffer, placed: &FinishLayout) {
        if placed.message.height == 0 {
            return;
        }
        self.text(
            buf,
            placed.message,
            "MESSAGE · ⇥ edit",
            Token::TextMuted,
            true,
        );
        if placed.subject.height > 0 {
            (&FieldWidget {
                view: &self.view.subject,
                theme: self.theme,
                ascii: self.ascii,
            })
                .render(placed.subject, buf);
        }
        if placed.body.height > 0 {
            (&AreaWidget {
                view: &self.view.body,
                theme: self.theme,
            })
                .render(placed.body, buf);
        }
    }
}

/// Byte ranges of a row's words that take a colour: `yours · <branch>` in
/// `ours`, `theirs · <branch>` in `theirs`, `+n` in `added`, `−n` in
/// `removed`.
fn tints(what: &str) -> Vec<(usize, usize, Token)> {
    let mut found = Vec::new();
    for (lead, token) in [("yours · ", Token::Ours), ("theirs · ", Token::Theirs)] {
        let mut from = 0;
        while let Some(at) = what[from..].find(lead).map(|at| at + from) {
            let name = at + lead.len();
            let end = what[name..]
                .find([' ', ','])
                .map_or(what.len(), |stop| name + stop);
            found.push((at, end, token));
            from = end;
        }
    }
    for (sign, token) in [('+', Token::Added), ('\u{2212}', Token::Removed)] {
        for (at, _) in what.match_indices(sign) {
            let digits = what[at + sign.len_utf8()..]
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(what.len() - at - sign.len_utf8());
            if digits > 0 {
                found.push((at, at + sign.len_utf8() + digits, token));
            }
        }
    }
    found
}

/// `rect` moved right by [`DIFF_INDENT`].
fn indent(rect: Rect) -> Rect {
    let by = DIFF_INDENT.min(rect.width);
    Rect {
        x: rect.x + by,
        width: rect.width - by,
        ..rect
    }
}

mod layout;

pub use layout::{
    FinishHit, FinishLayout, ListLine, Section, fits, hit, layout, list_height, list_lines,
    max_top, top_for_cursor,
};

#[cfg(test)]
mod tests;
