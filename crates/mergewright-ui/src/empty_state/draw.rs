//! The empty state's parts as it draws them: the head, the actions row,
//! RECENT, the prompt box and the cursors. Moved out of `empty_state.rs`
//! (m8/21b-5 5k); `keep_end` stays in the parent (`clone_prompt` uses it).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::{
    BODY_X, CloneShown, CloneView, EmptyPrompt, EmptyState, HEAD_X, MISSING, column_widths,
    draw_box, keep_end, prompt_height, window,
};
use crate::fit::fit;
use crate::frame_box::cells;
use crate::graph_screen::APP_NAME;
use crate::paint::{foreground, lift_on_selection};
use crate::theme::Token;

impl EmptyState<'_> {
    pub(super) fn draw_head(&self, area: Rect, buf: &mut Buffer) {
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

    pub(super) fn draw_actions(&self, area: Rect, buf: &mut Buffer) {
        let mut actions = vec![("o", "Open…")];
        if !self.view.missing {
            actions.push(("i", "Init"));
            actions.push(("c", "Clone…"));
        }
        // Not gated on a missing folder: the tour builds its own (m8/19b).
        actions.push(("t", "Practice tour"));
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

    pub(super) fn draw_recent(
        &self,
        area: Rect,
        title: u16,
        first: u16,
        rule: u16,
        buf: &mut Buffer,
    ) {
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

    pub(super) fn draw_prompt(&self, area: Rect, y: u16, prompt: &EmptyPrompt, buf: &mut Buffer) {
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
        self.draw_field_cursor(prompt, cursor_at, (inner_x, y + 1), inner, buf);
    }

    /// The prompt's field cursor on the row at `at`: none on the init box or
    /// a running clone. `cursor_at` is where the clone's scrolled window put
    /// it; otherwise counted from the field's lead.
    pub(super) fn draw_field_cursor(
        &self,
        prompt: &EmptyPrompt,
        cursor_at: Option<usize>,
        at: (u16, u16),
        inner: usize,
        buf: &mut Buffer,
    ) {
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
            self.draw_cursor(at, before, inner, at_end, buf);
        }
    }

    /// The field's cursor `before` cells in: `▏` past the end, the
    /// character under it reversed inside the text.
    pub(super) fn draw_cursor(
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
}
