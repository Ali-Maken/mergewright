//! The screen Graft shows outside a repository (mock-ups §1.15, keymap
//! §2.13; m8-polish-and-ship/08).
//!
//! ```text
//!  graft · no repository here
//!  ~/code
//!
//!   o  Open…     i  Init     c  Clone…     t  Practice tour
//!
//!   RECENT
//!   ▸ payments-service    ~/code/payments-service     yesterday
//!     web-dashboard       ~/code/web-dashboard        3 days ago
//! ───────────────────────────────────────────────────────────────
//!  o open… · i init · c clone… · t practice tour · ⏎ open the selected · q quit · ? keys
//! ```
//!
//! Only keys the screen answers are drawn (§1.15's last rule). `t` starts the
//! practice tour (m8/19b); it is the bar's first clause shed when the bar
//! is too narrow — the actions row still names it. A missing folder draws no
//! `i` and no `c` — a typo must not become a repository. RECENT is not
//! drawn when the list is empty, and neither is the `⏎` hint.
//!
//! A prompt (`o`'s path, `i`'s confirm, `c`'s URL) sits in a box under the list, where
//! the clone prompt of the prototype sits, and the bar row offers that
//! prompt's keys only. A `problem` (an open or init that failed) replaces the
//! bar row until the next key.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::frame_box::{border_row, cells};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::{Theme, Token};

mod bar;
mod clone_prompt;
mod draw;

pub use bar::{bar_fitted, bar_keys};
use clone_prompt::window;
pub use clone_prompt::{CloneShown, CloneView};

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
    /// `c`: `clone › <url>`, and under it where it goes and how it went.
    Clone(CloneView),
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
    /// His first launches, before he has opened the palette (m8/18d):
    /// [`first_launch_words`] drawn muted on the row above the rule when
    /// nothing else is there; else not drawn this launch.
    pub first_launch: bool,
    /// git's credential question is open over the screen (m8/18c): the bar
    /// carries the box's keys. The box is the app's to draw over this
    /// widget (`graft::empty_state::render`), as on a repository screen.
    pub credential: bool,
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
    /// His keymap, built from config before the first draw: the bar's keys
    /// are spelled from it.
    pub keys: &'a Keymap,
}

/// The first-launch line fitted to `cells`: `first time? ? keys here ·
/// ⌃P anything`, keys spelled from `keys`. Whole clauses drop from the
/// right, never a fragment (ADR-0039); an unbound key's clause goes (the
/// `first time?` lead rides on the first clause left); `None` when no
/// clause is bound or none fits. Shared by the graph's status row
/// (`graft::view`) and this screen. The tour is not a clause here: the
/// palette's `Practice tour` row is found by typing "tour", and the empty
/// state names `t` on its own actions row and bar (m8/19b).
#[must_use]
pub fn first_launch_words(keys: &Keymap, cells_wide: usize) -> Option<String> {
    let clauses: Vec<String> = [("app.help", "keys here"), ("app.palette", "anything")]
        .iter()
        .filter_map(|(action, verb)| keys.key_verb(&[ActionId(action)], verb))
        .collect();
    let mut line: Option<String> = None;
    for clause in clauses {
        let next = match &line {
            None => format!("first time? {clause}"),
            Some(line) => format!("{line} · {clause}"),
        };
        if cells(&next) > cells_wide {
            break;
        }
        line = Some(next);
    }
    line
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
    /// The last row the head, the list or the prompt draws on.
    last_used: u16,
    rule: u16,
    bar: u16,
}

/// Columns the screen's text starts at: the header and folder at 1, the
/// actions and the list at 2, as §1.15 draws them.
const HEAD_X: u16 = 1;
const BODY_X: u16 = 2;
/// Said after a folder that does not exist.
const MISSING: &str = " · no such folder";
/// The prompt box's height: border, field, a spare row (the clone's `→`
/// line), border.
const PROMPT_HEIGHT: u16 = 4;

/// `prompt`'s box height: an ended clone has its reason on one more row.
fn prompt_height(prompt: &EmptyPrompt) -> u16 {
    match prompt {
        EmptyPrompt::Clone(CloneView {
            shown: CloneShown::Ended { .. },
            ..
        }) => PROMPT_HEIGHT + 1,
        _ => PROMPT_HEIGHT,
    }
}

fn layout(view: &EmptyStateView, area: Rect) -> Rows {
    let bottom = area.y + area.height;
    let bar = bottom.saturating_sub(1);
    let rule = bottom.saturating_sub(2);
    // Header, folder, blank, actions, blank.
    let mut next = area.y + 5;
    // The actions row.
    let mut last_used = area.y + 3;
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
        last_used = (first + shown).saturating_sub(1).max(title);
        (Some(title), first)
    };
    let prompt = view.prompt.as_ref().map(|prompt| {
        let height = prompt_height(prompt);
        let y = next.min(rule.saturating_sub(height));
        last_used = last_used.max(y + height - 1);
        y
    });
    Rows {
        recent_title,
        first_recent,
        prompt,
        last_used,
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
        self.draw_first_launch(area, &rows, buf);
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

    /// The first-launch line, on the row above the rule when free.
    fn draw_first_launch(&self, area: Rect, rows: &Rows, buf: &mut Buffer) {
        let Some(y) = rows.rule.checked_sub(1) else {
            return;
        };
        if !self.view.first_launch || y <= rows.last_used {
            return;
        }
        let room = usize::from(area.width.saturating_sub(HEAD_X + 1));
        if let Some(words) = first_launch_words(self.keys, room) {
            buf.set_string(area.x + HEAD_X, y, words, self.muted());
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

/// A rounded box's border around `at`, `title` in its top edge.
fn draw_box(buf: &mut Buffer, at: Rect, title: &str, border: Style) {
    let w = usize::from(at.width);
    buf.set_string(at.x, at.y, border_row('╭', '╮', title, w), border);
    buf.set_string(
        at.x,
        at.y + at.height - 1,
        border_row('╰', '╯', "", w),
        border,
    );
    for row in 1..at.height - 1 {
        buf.set_string(at.x, at.y + row, "│", border);
        buf.set_string(at.x + at.width - 1, at.y + row, "│", border);
    }
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
