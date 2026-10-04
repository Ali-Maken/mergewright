//! The finish summary's frame: the rounded border with the title in its
//! top row and the keys in its bottom one, and the two preview lines just
//! inside it — the same frame the merge preview draws.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::{FinishScreen, FinishView};
use crate::fit::fit;
use crate::frame_box::border_row_spans;
use crate::graph_screen::{Divider, Hint, HintContext, hints_for};
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::Token;

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
/// until `y` (`finish.commit`'s key, spelled from `keys`). `on <into>` only
/// when the branch is known.
#[must_use]
pub fn preview_line(into: Option<&str>, keys: &Keymap) -> String {
    let until = keys
        .spell_keys(&[ActionId("finish.commit")])
        .map_or_else(|| "yet".to_owned(), |key| format!("until {key}"));
    match into {
        Some(into) => format!("1 merge commit on {into} · nothing is committed {until}"),
        None => format!("1 merge commit · nothing is committed {until}"),
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
    hints_for(
        &context,
        Divider::Hidden,
        width.saturating_sub(4),
        view.keys,
    )
}

/// The bottom border's words as `(text, style)` spans, built from the
/// hints themselves: the first hint's key in `strong`, everything after it
/// in `muted`. Spaced as `hints_text` spaces the bar, with no lead space
/// (the border puts its own gutter there).
/// Keys are spelled from `keys`; a hint naming no live key is skipped.
#[must_use]
pub fn footer_spans(
    hints: &[Hint],
    keys: &Keymap,
    strong: Style,
    muted: Style,
) -> Vec<(String, Style)> {
    let mut spelled = hints
        .iter()
        .filter_map(|hint| hint.key_text(keys).map(|key| (hint, key)));
    let Some((first, first_key)) = spelled.next() else {
        return Vec::new();
    };
    let mut rest = first
        .drawn_verb()
        .map_or_else(String::new, |verb| format!(" {verb}"));
    for text in spelled.filter_map(|(hint, _)| hint.text(keys)) {
        rest.push_str("  ");
        rest.push_str(&text);
    }
    vec![(first_key, strong), (rest, muted)]
}

/// The second line inside the frame: `undo: u takes this back`, `app.undo`
/// spelled from `keys`.
#[must_use]
pub fn undo_line(keys: &Keymap) -> String {
    match keys.spell_keys(&[ActionId("app.undo")]) {
        Some(undo) => format!("undo: {undo} takes this back"),
        None => "undo: this can be undone".to_owned(),
    }
}

impl FinishScreen<'_> {
    /// The rounded frame: `accent` while it has focus (`border` under a
    /// live box), the title in the
    /// top border in `accent` BOLD, the keys in the bottom border with the
    /// first key `accent` BOLD and the rest muted, as the merge preview's
    /// footer is. While the commit runs no key acts, so none is drawn.
    /// Under `NO_COLOR` every token paints nothing: plain glyphs, and the
    /// title and first key still BOLD.
    pub(super) fn draw_frame(&self, buf: &mut Buffer, frame: Rect) {
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
        let spans = footer_spans(
            &footer_hints(self.view, frame.width),
            self.view.keys,
            strong,
            muted,
        );
        let spans: Vec<(&str, Style)> = spans
            .iter()
            .map(|(text, style)| (text.as_str(), *style))
            .collect();
        let bottom = border_row_spans('╰', '╯', &spans, line, frame.width);
        buf.set_line(frame.x, frame.bottom() - 1, &bottom, frame.width);
    }

    /// The two lines the merge preview opens with: what lands, and how to
    /// take it back.
    pub(super) fn draw_preview(&self, buf: &mut Buffer, rect: Rect) {
        if rect.height < 2 {
            return;
        }
        let into = self.view.names.as_ref().map(|(_, into)| into.as_str());
        self.text(buf, rect, &preview_line(into, self.view.keys), Token::Text, false);
        let second = Rect {
            y: rect.y + 1,
            height: 1,
            ..rect
        };
        self.text(buf, second, &undo_line(self.view.keys), Token::TextMuted, false);
    }

    /// The title's arrow at the ASCII tier.
    fn ascii_safe(&self, text: &str) -> String {
        if self.ascii {
            text.replace('→', "->")
        } else {
            text.to_owned()
        }
    }
}
