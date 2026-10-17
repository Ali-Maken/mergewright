//! The header's tail: the items right of the repository name, and the rule
//! for which give way first when the row is short.

use ratatui::style::Style;
use ratatui::text::Span;

use super::layout::text_width;
use crate::fit::fit;

/// Between two items of the header's tail.
pub(super) const TAIL_SEPARATOR: &str = "  ";

/// Which items of the header's tail give way first when the row is short
/// (m8/15b, ticket 15 A2): the graphics status, then the walk (with its
/// `Esc cancels` and order notice), then the push count (ease-pass/08),
/// then the filters, then the search. The operation banner, when there is
/// one, outlasts them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Drop {
    Gfx,
    Walk,
    Push,
    Filters,
    Search,
}

/// One item of the header's tail.
pub(super) struct TailItem {
    pub(super) text: String,
    pub(super) style: Style,
    pub(super) drop: Drop,
}

/// `items` in `room` cells: whole items dropped in [`Drop`] order until the
/// rest fit. The last group standing, unless it is the search, is cut with
/// `…` rather than dropped — a lone walk failure is shortened, never
/// silently gone while any cell is free. The search is whole or not at all.
pub(super) fn fitted_tail(mut items: Vec<TailItem>, room: usize) -> Vec<Span<'static>> {
    let width = |items: &[TailItem]| -> usize {
        items
            .iter()
            .map(|item| text_width(&item.text))
            .sum::<usize>()
            + TAIL_SEPARATOR.len() * items.len().saturating_sub(1)
    };
    for drop in [
        Drop::Gfx,
        Drop::Walk,
        Drop::Push,
        Drop::Filters,
        Drop::Search,
    ] {
        if width(&items) <= room {
            break;
        }
        let last_group = items.iter().all(|item| item.drop == drop);
        if last_group && drop != Drop::Search {
            break;
        }
        items.retain(|item| item.drop != drop);
    }
    let mut used = 0;
    let mut spans = Vec::new();
    for (index, item) in items.into_iter().enumerate() {
        let text = if index == 0 {
            item.text
        } else {
            format!("{TAIL_SEPARATOR}{}", item.text)
        };
        let left = room.saturating_sub(used);
        if left == 0 {
            break;
        }
        let text = fit(&text, left.min(text_width(&text)));
        used += text_width(&text);
        spans.push(Span::styled(text, item.style));
    }
    spans
}
