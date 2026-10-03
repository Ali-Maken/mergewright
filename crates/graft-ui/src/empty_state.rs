//! The screen Graft shows outside a repository (mock-ups §1.15, keymap
//! §2.13; m8-polish-and-ship/08).
//!
//! ```text
//!  graft · no repository here
//!  ~/code
//!
//!   o  Open…     i  Init
//!
//!   RECENT
//!   ▸ payments-service    ~/code/payments-service     yesterday
//!     web-dashboard       ~/code/web-dashboard        3 days ago
//! ───────────────────────────────────────────────────────────────
//!  o open… · i init · ⏎ open the selected · q quit
//! ```
//!
//! Only keys the screen answers are drawn (§1.15's last rule): `c` clone and
//! `t` tour belong to later tickets and are not here yet. A missing folder
//! draws no `i` either — a typo must not become a repository. RECENT is not
//! drawn when the list is empty, and neither is the `⏎` hint.
//!
//! A prompt (`o`'s path, `i`'s confirm) sits in a box under the list, where
//! the clone prompt of the prototype sits, and the bar row offers that
//! prompt's keys only. A `problem` (an open or init that failed) replaces the
//! bar row until the next key.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::frame_box::{border_row, cells};
use crate::graph_screen::APP_NAME;
use crate::paint::{foreground, lift_on_selection};
use crate::theme::{Theme, Token};

/// One remembered repository as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecentRow {
    /// The folder's name, bold.
    pub name: SanitizedText,
    /// Its path, `~`-abbreviated, muted.
    pub path: SanitizedText,
    /// How long ago it was opened ([`recent_age`]), muted.
    pub age: SanitizedText,
}

/// The prompt open over the list, if any.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EmptyPrompt {
    /// `o`: `open › <text>▏`. `cursor` is the character index.
    Open {
        /// What has been typed (prefilled with the shown folder).
        text: SanitizedText,
        /// The cursor's character index into `text`.
        cursor: usize,
    },
    /// `i`: `Make <folder> a Git repository?`.
    ConfirmInit,
}

/// Everything the empty state draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmptyStateView {
    /// The folder Graft was started in, `~`-abbreviated.
    pub folder: SanitizedText,
    /// The folder does not exist: ` · no such folder`, and no `i`.
    pub missing: bool,
    /// Newest first. Empty: the RECENT block is not drawn.
    pub recent: Vec<RecentRow>,
    /// Index into `recent` of the selected row.
    pub cursor: usize,
    /// The open prompt, if any.
    pub prompt: Option<EmptyPrompt>,
    /// Read in the bar row instead of the keys, until the next key.
    pub problem: Option<SanitizedText>,
}

/// [`EmptyStateView`] drawn over a whole frame.
#[derive(Debug)]
pub struct EmptyState<'a> {
    /// What to draw.
    pub view: &'a EmptyStateView,
    /// Colours.
    pub theme: &'a Theme,
    /// The ASCII tier: `>` for the cursor's `▸`, `-` for the rule.
    pub ascii: bool,
}

/// The bar row's hints for `view`, as `(key, label)` pairs — the keys the
/// screen answers in its current form, and only those. The one list the
/// widget draws and the app's "every drawn key is answered" test reads.
#[must_use]
pub fn bar_keys(view: &EmptyStateView) -> Vec<(&'static str, &'static str)> {
    match view.prompt {
        Some(EmptyPrompt::Open { .. }) => {
            vec![("⏎", "open"), ("⇥", "complete"), ("esc", "cancel")]
        }
        Some(EmptyPrompt::ConfirmInit) => vec![("y", "init"), ("esc", "keep it")],
        None => {
            let mut keys = vec![("o", "open…")];
            if !view.missing {
                keys.push(("i", "init"));
            }
            if !view.recent.is_empty() {
                keys.push(("⏎", "open the selected"));
            }
            keys.push(("q", "quit"));
            keys
        }
    }
}

/// How long ago, in the words the mock-up uses: `today`, `yesterday`,
/// `3 days ago`, `last week`, `2 weeks ago`, `last month`, `5 months ago`,
/// `last year`, `3 years ago`. A time ahead of `now` (a clock moved back)
/// reads `today`. Day-granular on purpose: the list answers "what was I in
/// lately", not "when exactly".
#[must_use]
pub fn recent_age(opened_secs: i64, now_secs: i64) -> String {
    const DAY: i64 = 86_400;
    let days = (now_secs - opened_secs).max(0) / DAY;
    match days {
        0 => "today".to_owned(),
        1 => "yesterday".to_owned(),
        2..=6 => format!("{days} days ago"),
        7..=13 => "last week".to_owned(),
        14..=29 => format!("{} weeks ago", days / 7),
        30..=59 => "last month".to_owned(),
        60..=364 => format!("{} months ago", days / 30),
        365..=729 => "last year".to_owned(),
        _ => format!("{} years ago", days / 365),
    }
}

/// Where each block lands, top to bottom.
struct Rows {
    recent_title: Option<u16>,
    first_recent: u16,
    prompt: Option<u16>,
    rule: u16,
    bar: u16,
}

/// Columns the screen's text starts at: the header and folder at 1, the
/// actions and the list at 2, as §1.15 draws them.
const HEAD_X: u16 = 1;
const BODY_X: u16 = 2;
/// Said after a folder that does not exist.
const MISSING: &str = " · no such folder";
/// The prompt box's height: border, field, a spare row, border.
const PROMPT_HEIGHT: u16 = 4;

fn layout(view: &EmptyStateView, area: Rect) -> Rows {
    let bottom = area.y + area.height;
    let bar = bottom.saturating_sub(1);
    let rule = bottom.saturating_sub(2);
    // Header, folder, blank, actions, blank.
    let mut next = area.y + 5;
    let (recent_title, first_recent) = if view.recent.is_empty() {
        (None, next)
    } else {
        let title = next;
        next += 1;
        let first = next;
        let room = rule.saturating_sub(first);
        let shown = u16::try_from(view.recent.len())
            .unwrap_or(u16::MAX)
            .min(room);
        next += shown + 1;
        (Some(title), first)
    };
    let prompt = view
        .prompt
        .as_ref()
        .map(|_| next.min(rule.saturating_sub(PROMPT_HEIGHT)));
    Rows {
        recent_title,
        first_recent,
        prompt,
        rule,
        bar,
    }
}

impl Widget for &EmptyState<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width < 4 || area.height < 4 {
            return;
        }
        let rows = layout(self.view, area);
        self.draw_head(area, buf);
        self.draw_actions(area, buf);
        if let Some(title) = rows.recent_title {
            self.draw_recent(area, title, rows.first_recent, rows.rule, buf);
        }
        if let (Some(y), Some(prompt)) = (rows.prompt, &self.view.prompt) {
            self.draw_prompt(area, y, prompt, buf);
        }
        let rule = if self.ascii { "-" } else { "─" };
        buf.set_string(
            area.x,
            rows.rule,
            rule.repeat(usize::from(area.width)),
            foreground(self.theme, Token::Border),
        );
        self.draw_bar(area, rows.bar, buf);
    }
}

impl EmptyState<'_> {
    fn muted(&self) -> Style {
        foreground(self.theme, Token::TextMuted)
    }

    fn key(&self) -> Style {
        foreground(self.theme, Token::Accent).add_modifier(Modifier::BOLD)
    }

    fn draw_head(&self, area: Rect, buf: &mut Buffer) {
        let x = area.x + HEAD_X;
        let width = usize::from(area.width.saturating_sub(HEAD_X + 1));
        let (end, _) = buf.set_stringn(
            x,
            area.y,
            APP_NAME,
            width,
            foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
        );
        let left = width.saturating_sub(usize::from(end - x));
        buf.set_stringn(end, area.y, " · no repository here", left, self.muted());

        // A long folder loses its start; its end (the folder's own name) and
        // the reason stay.
        let reason = if self.view.missing { cells(MISSING) } else { 0 };
        let folder = keep_end(self.view.folder.as_str(), width.saturating_sub(reason));
        let (end, _) = buf.set_stringn(
            x,
            area.y + 1,
            &folder,
            width,
            foreground(self.theme, Token::Text),
        );
        if self.view.missing {
            let left = width.saturating_sub(usize::from(end - x));
            buf.set_stringn(
                end,
                area.y + 1,
                MISSING,
                left,
                foreground(self.theme, Token::Warning),
            );
        }
    }

    fn draw_actions(&self, area: Rect, buf: &mut Buffer) {
        let mut actions = vec![("o", "Open…")];
        if !self.view.missing {
            actions.push(("i", "Init"));
        }
        let y = area.y + 3;
        let mut x = area.x + BODY_X;
        let right = area.x + area.width.saturating_sub(1);
        for (key, label) in actions {
            if x >= right {
                break;
            }
            let (end, _) = buf.set_stringn(x, y, key, usize::from(right - x), self.key());
            let (end, _) = buf.set_stringn(
                end + 2,
                y,
                label,
                usize::from(right.saturating_sub(end + 2)),
                foreground(self.theme, Token::Text),
            );
            x = end + 5;
        }
    }

    fn draw_recent(&self, area: Rect, title: u16, first: u16, rule: u16, buf: &mut Buffer) {
        buf.set_string(
            area.x + BODY_X,
            title,
            "RECENT",
            self.muted().add_modifier(Modifier::BOLD),
        );
        let recent = &self.view.recent;
        let (name_width, path_width) = column_widths(recent);
        // Two cells for the cursor mark and its gap, then the columns.
        let text_x = area.x + BODY_X + 2;
        let right = area.x + area.width.saturating_sub(1);
        let room = usize::from(right.saturating_sub(text_x));
        let path_at = name_width + 4;
        for (index, row) in recent.iter().enumerate() {
            let Some(y) = u16::try_from(index).ok().map(|i| first + i) else {
                break;
            };
            if y >= rule {
                break;
            }
            let selected = index == self.view.cursor;
            if selected {
                let mark = if self.ascii { ">" } else { "\u{25b8}" };
                buf.set_string(area.x + BODY_X, y, mark, self.key());
            }
            let line = format!(
                "{name:<name_w$}    {path:<path_w$}     {age}",
                name = row.name.as_str(),
                path = row.path.as_str(),
                age = row.age.as_str(),
                name_w = name_width,
                path_w = path_width,
            );
            let line = fit(&line, room);
            // Name bold, path and age muted: three runs over one fitted line.
            let name_end = line.chars().take(path_at).collect::<String>();
            let rest: String = line.chars().skip(path_at).collect();
            let (end, _) = buf.set_stringn(
                text_x,
                y,
                &name_end,
                room,
                foreground(self.theme, Token::Text).add_modifier(Modifier::BOLD),
            );
            buf.set_string(end, y, &rest, self.muted());
            if selected {
                let fill = Rect::new(text_x, y, right.saturating_sub(text_x), 1);
                buf.set_style(
                    fill,
                    Style::new().bg(crate::paint::color(self.theme.token(Token::Selection))),
                );
                lift_on_selection(buf, fill, self.theme);
            }
        }
    }

    fn draw_prompt(&self, area: Rect, y: u16, prompt: &EmptyPrompt, buf: &mut Buffer) {
        let x = area.x + BODY_X;
        let width = area.width.saturating_sub(BODY_X * 2).min(90);
        if width < 8 {
            return;
        }
        let border = foreground(self.theme, Token::Accent);
        let inner = usize::from(width.saturating_sub(5));
        let (title, line) = match prompt {
            EmptyPrompt::Open { text, .. } => ("open", format!("open › {}", text.as_str())),
            EmptyPrompt::ConfirmInit => {
                // A long folder loses its start, never the question `y`
                // answers; its end (the folder's own name) stays.
                let words = cells("Make ") + cells(" a Git repository?");
                let folder = keep_end(self.view.folder.as_str(), inner.saturating_sub(words));
                ("init", format!("Make {folder} a Git repository?"))
            }
        };
        let w = usize::from(width);
        buf.set_string(x, y, border_row('╭', '╮', title, w), border);
        buf.set_string(
            x,
            y + PROMPT_HEIGHT - 1,
            border_row('╰', '╯', "", w),
            border,
        );
        for row in 1..PROMPT_HEIGHT - 1 {
            buf.set_string(x, y + row, "│", border);
            buf.set_string(x + width - 1, y + row, "│", border);
        }
        let inner_x = x + 3;
        let shown = fit(&line, inner);
        buf.set_string(inner_x, y + 1, &shown, foreground(self.theme, Token::Text));
        if let EmptyPrompt::Open { text, cursor } = prompt {
            // The cursor: `▏` past the end, the character under it reversed
            // inside the text.
            let before = cells("open › ")
                + text
                    .as_str()
                    .chars()
                    .take(*cursor)
                    .map(|c| cells(&c.to_string()))
                    .sum::<usize>();
            if before < inner {
                let cx = inner_x + u16::try_from(before).unwrap_or(u16::MAX);
                if *cursor >= text.as_str().chars().count() {
                    buf.set_string(cx, y + 1, "▏", self.key());
                } else {
                    buf[(cx, y + 1)].modifier.insert(Modifier::REVERSED);
                }
            }
        }
    }

    fn draw_bar(&self, area: Rect, y: u16, buf: &mut Buffer) {
        let x = area.x + 1;
        let width = usize::from(area.width.saturating_sub(2));
        if let Some(problem) = &self.view.problem {
            buf.set_stringn(
                x,
                y,
                problem.as_str(),
                width,
                foreground(self.theme, Token::Warning),
            );
            return;
        }
        let mut at = x;
        let end_x = x + u16::try_from(width).unwrap_or(u16::MAX);
        for (index, (key, label)) in bar_keys(self.view).into_iter().enumerate() {
            if index > 0 {
                at = buf
                    .set_stringn(
                        at,
                        y,
                        " · ",
                        usize::from(end_x.saturating_sub(at)),
                        self.muted(),
                    )
                    .0;
            }
            at = buf
                .set_stringn(
                    at,
                    y,
                    key,
                    usize::from(end_x.saturating_sub(at)),
                    self.key(),
                )
                .0;
            at = buf
                .set_stringn(
                    at,
                    y,
                    format!(" {label}"),
                    usize::from(end_x.saturating_sub(at)),
                    self.muted(),
                )
                .0;
        }
    }
}

/// The widest name and the widest path: where RECENT's columns start.
fn column_widths(recent: &[RecentRow]) -> (usize, usize) {
    let widest = |of: fn(&RecentRow) -> &SanitizedText| {
        recent
            .iter()
            .map(|row| cells(of(row).as_str()))
            .max()
            .unwrap_or(0)
    };
    (widest(|row| &row.name), widest(|row| &row.path))
}

/// `text` in at most `width` cells, cut at the start with `…` when longer.
/// The text is already sanitized; zero-width marks ride with their base.
fn keep_end(text: &str, width: usize) -> String {
    if cells(text) <= width {
        return text.to_owned();
    }
    let room = width.saturating_sub(1);
    let mut kept: Vec<char> = Vec::new();
    let mut used = 0;
    for character in text.chars().rev() {
        let wide = cells(&character.to_string());
        if used + wide > room {
            break;
        }
        kept.push(character);
        used += wide;
    }
    kept.push('…');
    kept.into_iter().rev().collect()
}

#[cfg(test)]
mod tests;
