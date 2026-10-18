//! The review's frame: the rounded border, with the review's bar in its
//! bottom row — where the merge preview's frame draws its keys.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use super::{FinishScreen, ReviewView};
use crate::frame_box::border_row_spans;
use crate::graph_screen::hints::ReviewBar;
use crate::graph_screen::{Divider, Hint, HintContext, hints_for};
use crate::keymap::Keymap;
use crate::paint::foreground;
use crate::theme::Token;

/// The keys the bottom border draws in a frame `width` cells wide: the
/// `HintContext::Review(ReviewBar::List)` bar, fitted to the border. None
/// while the commit runs or a box has the keyboard, when none of them acts.
#[must_use]
pub fn footer_hints(view: &ReviewView<'_>, width: u16) -> Vec<Hint> {
    if view.busy || !view.focused {
        return Vec::new();
    }
    hints_for(
        &HintContext::Review(ReviewBar::List),
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
        rest.push_str(&hints_separator(hint_words_only(hints)));
        rest.push_str(&text);
    }
    vec![(first_key, strong), (rest, muted)]
}

/// Whether the bar's hints were fitted words-only (they carry the flag
/// `hints_text` spaces them by).
fn hint_words_only(hints: &[Hint]) -> bool {
    hints.first().is_some_and(|hint| hint.words_only)
}

/// What `hints_text` puts between two hints: ` · ` for a words-only bar,
/// two spaces otherwise.
fn hints_separator(words_only: bool) -> String {
    if words_only { " · " } else { "  " }.to_owned()
}

impl FinishScreen<'_> {
    /// The rounded frame: `accent` while it has focus (`border` under a
    /// live box), the keys in the bottom border with the first key `accent`
    /// BOLD and the rest muted, as the merge preview's footer is. While the
    /// commit runs no key acts, so none is drawn. Under `NO_COLOR` every
    /// token paints nothing: plain glyphs, the first key still BOLD.
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
        let top = border_row_spans('╭', '╮', &[], line, frame.width);
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
}
