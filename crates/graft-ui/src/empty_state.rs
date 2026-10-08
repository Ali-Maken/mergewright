//! The screen Graft shows outside a repository (mock-ups §1.15, keymap
//! §2.13; m8-polish-and-ship/08).
//!
//! ```text
//!  graft · no repository here
//!  ~/code
//!
//!   o  Open…     i  Init     c  Clone…
//!
//!   RECENT
//!   ▸ payments-service    ~/code/payments-service     yesterday
//!     web-dashboard       ~/code/web-dashboard        3 days ago
//! ───────────────────────────────────────────────────────────────
//!  o open… · i init · c clone… · ⏎ open the selected · q quit · ? keys
//! ```
//!
//! Only keys the screen answers are drawn (§1.15's last rule): `t` tour
//! belongs to a later ticket and is not here yet. A missing folder draws no
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

use crate::fit::fit;
use crate::frame_box::{border_row, cells};
use crate::graph_screen::APP_NAME;
use crate::keymap::{ActionId, Keymap};
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
    /// `c`: `clone › <url>`, and under it where it goes and how it went.
    Clone(CloneView),
}

/// The clone prompt as drawn (m8/18c, rulings R6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CloneView {
    /// The URL typed (kept after a failure or a stop, for a retry).
    pub url: SanitizedText,
    /// The cursor's character index into `url`.
    pub cursor: usize,
    /// The folder it goes into, `~`-abbreviated: `~/code/x`, or `~/code/…`
    /// while no name can be read from the URL.
    pub into: SanitizedText,
    /// Typing, running, or ended.
    pub shown: CloneShown,
}

/// Where the clone prompt's clone is, as drawn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CloneShown {
    /// Nothing has run yet.
    Typing,
    /// The clone runs and the field is read-only.
    Running {
        /// git's last progress line, shortened; `None` before its first
        /// (drawn `connecting…`).
        progress: Option<SanitizedText>,
    },
    /// It failed, was stopped, or could not start.
    Ended {
        /// What happened, in R6's words: Problem ink, on the row under the
        /// `→` line.
        reason: SanitizedText,
    },
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

/// The bar row's hints for `view`, as `(key, label)` pairs — the keys the
/// screen answers in its current form, and only those. The one list the
/// widget draws and the app's "every drawn key is answered" test reads.
///
/// Keys are spelled from `keys`, his keymap (ticket m8/10): the empty state
/// translates every key but the open prompt's (`graft::empty_state::
/// outside_edge`), and an unbound one is left out. The open prompt is a
/// text field — it takes every press as typed — so its keys spell from the
/// shipped table.
#[must_use]
pub fn bar_keys(view: &EmptyStateView, keys: &Keymap) -> Vec<(String, &'static str)> {
    let spelled = |keys: &Keymap, parts: &[(&'static str, &'static str)]| {
        parts
            .iter()
            .filter_map(|(action, label)| {
                keys.spell_keys(&[ActionId(action)])
                    .map(|key| (key, *label))
            })
            .collect()
    };
    if view.credential {
        // The question keeps the shipped keys whatever his `[keys]` say,
        // as on a repository screen (`graft::empty_state::outside_edge`).
        return spelled(
            Keymap::shipped(),
            &[("credential.send", "send"), ("app.back", "cancel")],
        );
    }
    match &view.prompt {
        Some(EmptyPrompt::Open { .. }) => spelled(
            Keymap::shipped(),
            &[
                ("app.open", "open"),
                ("app.next_pane", "complete"),
                ("app.back", "cancel"),
            ],
        ),
        Some(EmptyPrompt::ConfirmInit) => {
            spelled(keys, &[("confirm.yes", "init"), ("app.back", "keep it")])
        }
        // A text field, as the open prompt: its keys spell from the shipped
        // table (rulings R6's three bars).
        Some(EmptyPrompt::Clone(clone)) => spelled(
            Keymap::shipped(),
            match clone.shown {
                CloneShown::Typing => &[("app.open", "clone"), ("app.back", "cancel")],
                CloneShown::Running { .. } => &[("app.back", "cancels")],
                CloneShown::Ended { .. } => &[("app.open", "retry"), ("app.back", "cancel")],
            },
        ),
        None => {
            let mut parts = vec![("empty_state.open", "open…")];
            if !view.missing {
                parts.push(("empty_state.init", "init"));
                parts.push(("empty_state.clone", "clone…"));
            }
            if !view.recent.is_empty() {
                parts.push(("app.open", "open the selected"));
            }
            parts.push(("app.quit", "quit"));
            // The doorway, last (m8/11b decision 1): `?` opens the palette
            // on the list. `⌃P` when `?` is unbound; neither, no doorway.
            let mut drawn = spelled(keys, &parts);
            drawn.extend(
                spelled(keys, &[("app.help", "keys")])
                    .into_iter()
                    .next()
                    .or_else(|| spelled(keys, &[("app.palette", "keys")]).into_iter().next()),
            );
            drawn
        }
    }
}

/// The first-launch line fitted to `cells`: `first time? ? keys here ·
/// ⌃P anything`, keys spelled from `keys`. Whole clauses drop from the
/// right, never a fragment (ADR-0039); an unbound key's clause goes (the
/// `first time?` lead rides on the first clause left); `None` when no
/// clause is bound or none fits. Shared by the graph's status row
/// (`graft::view`) and this screen. The tour clause joins when the palette
/// has a tour row (G-tour).
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
            actions.push(("c", "Clone…"));
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
        let height = prompt_height(prompt);
        let (title, line) = match prompt {
            EmptyPrompt::Open { text, .. } => ("open", format!("open › {}", text.as_str())),
            EmptyPrompt::Clone(clone) => ("clone", format!("clone › {}", clone.url.as_str())),
            EmptyPrompt::ConfirmInit => {
                // A long folder loses its start, never the question `y`
                // answers; its end (the folder's own name) stays.
                let words = cells("Make ") + cells(" a Git repository?");
                let folder = keep_end(self.view.folder.as_str(), inner.saturating_sub(words));
                ("init", format!("Make {folder} a Git repository?"))
            }
        };
        draw_box(buf, Rect::new(x, y, width, height), title, border);
        let inner_x = x + 3;
        // A URL longer than the row scrolls so the cursor stays in view —
        // its end is what he just typed or pasted. The open prompt keeps
        // its own drawing (its start is the folder he checks).
        let (shown, cursor_at) = match prompt {
            EmptyPrompt::Clone(clone) => {
                const LEAD: &str = "clone › ";
                let (text, before) = window(
                    clone.url.as_str(),
                    clone.cursor,
                    inner.saturating_sub(cells(LEAD)),
                );
                (format!("{LEAD}{text}"), Some(cells(LEAD) + before))
            }
            _ => (fit(&line, inner), None),
        };
        buf.set_string(inner_x, y + 1, &shown, foreground(self.theme, Token::Text));
        if let EmptyPrompt::Clone(clone) = prompt {
            self.draw_clone_rows(inner_x, y, inner, clone, buf);
        }
        let field = match prompt {
            EmptyPrompt::Open { text, cursor } => Some(("open › ", text, cursor)),
            // Read-only while it runs: no cursor.
            EmptyPrompt::Clone(CloneView {
                url,
                cursor,
                shown: CloneShown::Typing | CloneShown::Ended { .. },
                ..
            }) => Some(("clone › ", url, cursor)),
            EmptyPrompt::ConfirmInit | EmptyPrompt::Clone(_) => None,
        };
        if let Some((lead, text, cursor)) = field {
            let before = cursor_at.unwrap_or_else(|| {
                cells(lead)
                    + text
                        .as_str()
                        .chars()
                        .take(*cursor)
                        .map(|c| cells(&c.to_string()))
                        .sum::<usize>()
            });
            let at_end = *cursor >= text.as_str().chars().count();
            self.draw_cursor((inner_x, y + 1), before, inner, at_end, buf);
        }
    }

    /// The field's cursor `before` cells in: `▏` past the end, the
    /// character under it reversed inside the text.
    fn draw_cursor(
        &self,
        (x, y): (u16, u16),
        before: usize,
        inner: usize,
        at_end: bool,
        buf: &mut Buffer,
    ) {
        if before >= inner {
            return;
        }
        let cx = x + u16::try_from(before).unwrap_or(u16::MAX);
        if at_end {
            buf.set_string(cx, y, "▏", self.key());
        } else {
            buf[(cx, y)].modifier.insert(Modifier::REVERSED);
        }
    }

    /// The clone prompt's `→ <into>[ · <progress>]` row, and an ended
    /// clone's reason on the row under it, in Problem ink.
    fn draw_clone_rows(&self, x: u16, y: u16, inner: usize, clone: &CloneView, buf: &mut Buffer) {
        let progress = match &clone.shown {
            CloneShown::Running { progress } => Some(
                progress
                    .as_ref()
                    .map_or("connecting…", SanitizedText::as_str),
            ),
            CloneShown::Typing | CloneShown::Ended { .. } => None,
        };
        let line = into_line(clone.into.as_str(), progress, inner);
        buf.set_stringn(x, y + 2, &line, inner, self.muted());
        if let CloneShown::Ended { reason } = &clone.shown {
            buf.set_stringn(
                x,
                y + 3,
                fit(reason.as_str(), inner),
                inner,
                foreground(self.theme, Token::Warning),
            );
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
        for (index, (key, label)) in bar_keys(self.view, self.keys).into_iter().enumerate() {
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

/// `text` in `room` cells with its cursor (character index `cursor`) in
/// view and one cell spare for the `▏` past the end: whole when it fits;
/// else cut at the start behind `…` so the cursor is the last thing shown,
/// and cut at the end when what follows the cursor overruns. Returns the
/// shown text and the cells before the cursor in it.
fn window(text: &str, cursor: usize, room: usize) -> (String, usize) {
    let chars: Vec<char> = text.chars().collect();
    let cursor = cursor.min(chars.len());
    let wide = |part: &[char]| part.iter().map(|c| cells(&c.to_string())).sum::<usize>();
    if wide(&chars) < room {
        return (text.to_owned(), wide(&chars[..cursor]));
    }
    // Drop from the start until `…`, the text up to the cursor and the
    // cursor's own cell fit.
    let mut start = 0;
    while start < cursor && cells("…") + wide(&chars[start..cursor]) + 1 > room {
        start += 1;
    }
    let (lead, before) = if start == 0 {
        (String::new(), wide(&chars[..cursor]))
    } else {
        ("…".to_owned(), cells("…") + wide(&chars[start..cursor]))
    };
    let rest: String = chars[start..].iter().collect();
    let shown = fit(&format!("{lead}{rest}"), room);
    (shown, before)
}

/// `→ <into>[ · <progress>]` in `width` cells. The path is the part he
/// checks, its tail most of all: it keeps at least half the row and is cut
/// at the start (`…/code/x`); the progress takes what is left, cut from the
/// right.
fn into_line(into: &str, progress: Option<&str>, width: usize) -> String {
    const ARROW: &str = "→ ";
    let room = width.saturating_sub(cells(ARROW));
    let tail = progress.map_or(0, |p| cells(" · ") + cells(p));
    let path_room = room.saturating_sub(tail).max(room / 2);
    let mut path = keep_end(into, path_room);
    // A cut path starts at a folder boundary when one is in what is kept:
    // `…/code/x`, never `…de/x`.
    if let Some(rest) = path.strip_prefix('…')
        && !rest.starts_with('/')
        && let Some(at) = rest.find('/')
    {
        path = format!("…{}", &rest[at..]);
    }
    let line = match progress {
        Some(progress) => format!("{ARROW}{path} · {progress}"),
        None => format!("{ARROW}{path}"),
    };
    fit(&line, width)
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
