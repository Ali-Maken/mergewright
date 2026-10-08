//! The practice tour's card (ADR-0056 §3, design note New UI "Tour card"):
//! three rows at the top of the graph pane, a warning-coloured border.
//!
//! ```text
//! ╭─ practice 3 of 8 · undo that commit ─────────────────── esc leave ╮
//! │ u takes back the last thing you did — nothing is lost   ✓ stage … │
//! ╰───────────────────────────────────────────────────────────────────╯
//! ```
//!
//! Everything it says arrives worded and spelled by the caller from the live
//! keymap (`graft::view`); the widget only lays it out. The ticks go first
//! when the row is short, and always below [`TICKS_FROM`] columns.

use graft_term::SanitizedText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Widget;

use crate::fit::fit;
use crate::frame_box::cells;
use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// Rows the card takes: its two borders and the line between.
pub const TOUR_CARD_ROWS: u16 = 3;

/// Narrower than this, the ticks are not drawn (the note: "< 100 cols drops
/// the ticks").
pub const TICKS_FROM: u16 = 100;

/// What the card says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TourCardView {
    /// The task shown, from 1.
    pub step: usize,
    /// How many tasks there are.
    pub of: usize,
    /// The task in a few words: `undo that commit`.
    pub title: SanitizedText,
    /// The line that teaches it, its key spelled from the live table.
    pub line: SanitizedText,
    /// Every task's short name and whether it is done, in card order.
    pub ticks: Vec<(SanitizedText, bool)>,
    /// The leave key and its verb, spelled from the live table (`esc
    /// leave`); `None` when `tour.leave` is unbound.
    pub leave: Option<SanitizedText>,
}

/// [`TourCardView`] drawn into its rows.
#[derive(Debug)]
pub struct TourCard<'a> {
    /// What to draw.
    pub view: &'a TourCardView,
    /// Colours.
    pub theme: &'a Theme,
}

/// The ticks as one string: `✓ stage ✓ commit · undo stash …` — the done
/// ones marked, a `·` where the done run ends.
#[must_use]
pub fn ticks_text(ticks: &[(SanitizedText, bool)]) -> String {
    let mut out = String::new();
    let mut previous_done = None;
    for (name, done) in ticks {
        if let Some(previous) = previous_done {
            out.push_str(if previous && !done { " \u{b7} " } else { " " });
        }
        if *done {
            out.push_str("\u{2713} ");
        }
        out.push_str(name.as_str());
        previous_done = Some(*done);
    }
    out
}

/// The three rows as text, `width` cells each: what [`TourCard`] draws, and
/// what a test reads.
#[must_use]
pub fn card_lines(view: &TourCardView, width: u16) -> [String; 3] {
    let width = usize::from(width);
    if width < 4 {
        return [" ".repeat(width), " ".repeat(width), " ".repeat(width)];
    }
    let inner = width - 2;
    [
        top_row(view, inner),
        middle_row(view, inner, width),
        format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(inner)),
    ]
}

/// `╭─ practice 3 of 8 · <title> ──── esc leave ╮`: the leave words keep
/// their place; the title is cut first.
fn top_row(view: &TourCardView, inner: usize) -> String {
    let right = view
        .leave
        .as_ref()
        .map(|leave| format!(" {} ", leave.as_str()))
        .filter(|right| cells(right) + 4 <= inner)
        .unwrap_or_default();
    let room = inner.saturating_sub(cells(&right) + 1);
    let label = format!(
        " practice {} of {} \u{b7} {} ",
        view.step,
        view.of,
        view.title.as_str()
    );
    let label = if cells(&label) > room {
        // Cut with `…`, one blank cell kept before the dashes (frame_box's
        // rule: never an ellipsis against a border glyph).
        format!("{} ", fit(&label, room.saturating_sub(1)))
    } else {
        label
    };
    let dashes = inner.saturating_sub(1 + cells(&label) + cells(&right));
    format!(
        "\u{256d}\u{2500}{label}{}{right}\u{256e}",
        "\u{2500}".repeat(dashes)
    )
}

/// `│ <line>   <ticks> │`: the ticks right-aligned, dropped below
/// [`TICKS_FROM`] columns or when they would cut the line.
fn middle_row(view: &TourCardView, inner: usize, width: usize) -> String {
    let text_room = inner.saturating_sub(2);
    let line = view.line.as_str();
    let ticks = ticks_text(&view.ticks);
    let show_ticks = width >= usize::from(TICKS_FROM)
        && !ticks.is_empty()
        && cells(line) + 3 + cells(&ticks) <= text_room;
    let body = if show_ticks {
        let gap = text_room - cells(line) - cells(&ticks);
        format!("{line}{}{ticks}", " ".repeat(gap))
    } else {
        let fitted = fit(line, text_room);
        let pad = text_room.saturating_sub(cells(&fitted));
        format!("{fitted}{}", " ".repeat(pad))
    };
    format!("\u{2502} {body} \u{2502}")
}

impl Widget for &TourCard<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.is_empty() {
            return;
        }
        let border = foreground(self.theme, Token::Warning);
        let lines = card_lines(self.view, area.width);
        for (offset, line) in lines.iter().enumerate() {
            let Ok(offset) = u16::try_from(offset) else {
                break;
            };
            if offset >= area.height {
                break;
            }
            let y = area.y + offset;
            buf.set_stringn(area.x, y, line, usize::from(area.width), border);
            if offset == 1 {
                // The words between the side borders in the text colour.
                let text: Style = foreground(self.theme, Token::Text);
                let inner = line
                    .chars()
                    .skip(1)
                    .take(line.chars().count().saturating_sub(2))
                    .collect::<String>();
                buf.set_stringn(
                    area.x + 1,
                    y,
                    inner,
                    usize::from(area.width.saturating_sub(2)),
                    text,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests;
