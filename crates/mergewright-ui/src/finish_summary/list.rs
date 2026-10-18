//! The review's rows (mock `mergewright-merge-screen.html:158-160`): one
//! per clashed file and how it was solved, one for what git combined on its
//! own, and the diff expanded under a row. Where each line stands is
//! `layout`'s arithmetic; this only draws it.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{
    DIFF_INDENT, ExpandedDiff, FinishScreen, ListLine, ReviewRow, ReviewView, SummaryState,
    list_lines,
};
use crate::detail_pane::{DiffFocus, diff_body};
use crate::fit::fit;
use crate::keymap::ActionId;
use crate::paint::foreground;
use crate::text_input::FOCUS;
use crate::theme::Token;

/// The `✓` every solved row leads with (mock `:158`).
const SOLVED: &str = "✓";

impl FinishScreen<'_> {
    /// The cursor glyph.
    fn focus_glyph(&self) -> char {
        if self.ascii { '>' } else { FOCUS }
    }

    /// The list from `view.top`, into `rect`.
    pub(super) fn draw_list(&self, buf: &mut Buffer, rect: Rect) {
        let lines = list_lines(self.view);
        let names = names(self.view);
        let name_width = names
            .iter()
            .map(|name| name.chars().count())
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
                ListLine::Row(index) => self.draw_row(buf, one, index, &names, name_width),
                ListLine::More { hidden, .. } => {
                    let more = format!("\u{2026} {hidden} more lines");
                    let words =
                        self.view
                            .keys
                            .after(&more, ActionId("finish.toggle_diff"), "hides");
                    self.text(buf, indent(one), &words, Token::TextMuted, false);
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

    /// The list's one line while there are no rows.
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
                let unavailable = format!("summary unavailable: {}", why.as_str());
                let words = match self
                    .view
                    .keys
                    .key_verb(&[ActionId("confirm.yes")], "commits with git's message")
                {
                    Some(commit) => format!("{unavailable} · {commit}"),
                    None => unavailable,
                };
                self.text(buf, rect, &words, Token::Warning, false);
            }
        }
    }

    /// `▸ ✓ name  you chose: …`, the name padded to the widest, or
    /// `▸ ✓ N files git combined on its own`.
    fn draw_row(
        &self,
        buf: &mut Buffer,
        rect: Rect,
        index: usize,
        names: &[String],
        name_width: usize,
    ) {
        let Some(row) = self.view.row(index) else {
            return;
        };
        let cursor = index == self.view.cursor;
        let marker = if cursor { self.focus_glyph() } else { ' ' };
        let lead = format!("{marker} {SOLVED} ");
        let (what, how) = match row {
            ReviewRow::Clashed(at) => {
                let name = names.get(at).map_or("", String::as_str);
                let pad = name_width.saturating_sub(name.chars().count());
                let how = self.view.clashed.get(at).map(|file| file.how.words());
                (
                    format!("{name}{}  ", " ".repeat(pad)),
                    how.unwrap_or_default(),
                )
            }
            ReviewRow::Combined => {
                let n = self.view.combined;
                let noun = if n == 1 { "file" } else { "files" };
                (format!("{n} {noun} git combined on its own"), String::new())
            }
        };
        let line = format!("{lead}{what}{how}");
        let mut style = foreground(
            self.theme,
            if cursor {
                Token::Text
            } else {
                Token::TextMuted
            },
        );
        if cursor {
            style = crate::paint::on_selection(self.theme, style)
                .bg(crate::paint::color(self.theme.token(Token::Selection)));
        }
        buf.set_string(rect.x, rect.y, fit(&line, usize::from(rect.width)), style);
        // The tints tie the words back to the clash screen's sides; the
        // characters already say it all (rule 5).
        let mut tints = vec![(2, SOLVED.len(), Token::Added)];
        let how_at = lead.len() + what.len();
        tints.extend(
            side_tints(&how)
                .into_iter()
                .map(|(start, end, token)| (how_at + start, end - start, token)),
        );
        for (start, len, token) in tints {
            self.tint(buf, rect, &line, (start, start + len), token, cursor);
        }
    }

    /// `token` over the byte range `span` of `line`, drawn at `rect`.
    fn tint(
        &self,
        buf: &mut Buffer,
        rect: Rect,
        line: &str,
        (start, end): (usize, usize),
        token: Token,
        cursor: bool,
    ) {
        let (Some(before), Some(inner)) = (line.get(..start), line.get(start..end)) else {
            return;
        };
        let x = crate::frame_box::cells(before);
        let x = rect.x.saturating_add(u16::try_from(x).unwrap_or(u16::MAX));
        if x >= rect.right() {
            return;
        }
        let width = u16::try_from(crate::frame_box::cells(inner))
            .unwrap_or(u16::MAX)
            .min(rect.right() - x);
        let style = foreground(self.theme, token);
        buf.set_style(
            Rect {
                x,
                y: rect.y,
                width,
                height: 1,
            },
            if cursor {
                crate::paint::on_selection(self.theme, style)
            } else {
                style
            },
        );
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
                // `truncated: None`: the review draws its own `… N more
                // lines · ⏎ hides`, and `L load all` is not one of its keys
                // (ticket 19 S5). The row count `diff_body` returns is for
                // scroll clamping; `list_lines` already counted the rows
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
}

/// Each clashed file's drawn name: its last path segment, as the mock and
/// the clash screen name files — or every whole path, when two clashed
/// files share a last segment and the names alone would not tell them apart.
fn names(view: &ReviewView<'_>) -> Vec<String> {
    let last = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    let short: Vec<String> = view
        .clashed
        .iter()
        .map(|file| last(file.path.as_str()))
        .collect();
    let unique = short
        .iter()
        .enumerate()
        .all(|(at, name)| !short[..at].contains(name));
    if unique {
        short
    } else {
        view.clashed
            .iter()
            .map(|file| file.path.as_str().to_owned())
            .collect()
    }
}

/// Byte ranges of the how-solved words that take a side's colour: `◀
/// yours` in `ours`, `▶ theirs` in `theirs`, `◆ both` in `added`.
fn side_tints(how: &str) -> Vec<(usize, usize, Token)> {
    [
        ("◀ yours", Token::Ours),
        ("▶ theirs", Token::Theirs),
        ("◆ both", Token::Added),
    ]
    .into_iter()
    .filter_map(|(kind, token)| how.find(kind).map(|at| (at, at + kind.len(), token)))
    .collect()
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
