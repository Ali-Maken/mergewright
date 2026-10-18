//! The header's tail: the items right of the repository name, and the rule
//! for which give way first when the row is short.

use mergewright_graph::fold::Fold;
use ratatui::style::Style;
use ratatui::text::Span;

use super::GraphEntry;
use super::walk_text::text_width;
use crate::fit::fit;

/// Between two items of the header's tail.
pub(super) const TAIL_SEPARATOR: &str = "  ";

/// Between the two lane phrases: one account, `lanes drawn: 5 of 8 · 3
/// merged branches folded` (ease-pass 06b, Q14 a).
const LANE_SEPARATOR: &str = " \u{b7} ";

/// What the top line says about lanes and folds, counted over the rows in
/// view this frame (ease-pass 06b): facts of the frame, so the screen counts
/// them as it draws, from the fold it drew through.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LaneTally {
    /// N: real columns drawn ([`Fold::kept`]'s length; the rail is not a lane).
    pub drawn: usize,
    /// M: columns open in the window ([`Fold::open_columns`]).
    pub open: usize,
    /// K: window rows whose fold mark is closed.
    pub folded_in_view: usize,
}

impl LaneTally {
    /// The tally of `fold` drawn over `window`: the [`GraphEntry`]s drawn
    /// this frame, in order.
    #[must_use]
    pub fn of(fold: &Fold, window: &[GraphEntry<'_>]) -> Self {
        LaneTally {
            drawn: fold.kept().len(),
            open: fold.open_columns(),
            folded_in_view: window
                .iter()
                .filter(|entry| entry.fold.as_ref().is_some_and(|mark| !mark.open))
                .count(),
        }
    }
}

/// The top line's lane words, left to right; each only while it has
/// something to say: `[]`, `["lanes drawn: 5 of 8"]`, `["3 merged branches
/// folded"]`, or both.
pub(crate) fn lane_phrases(tally: LaneTally) -> Vec<String> {
    let mut phrases = Vec::new();
    if tally.drawn < tally.open {
        phrases.push(format!("lanes drawn: {} of {}", tally.drawn, tally.open));
    }
    let folded = tally.folded_in_view;
    if folded > 0 {
        let branches = if folded == 1 { "branch" } else { "branches" };
        phrases.push(format!("{folded} merged {branches} folded"));
    }
    phrases
}

/// Which items of the header's tail give way first when the row is short
/// (m8/15b, ticket 15 A2): the lane phrases (ease-pass 06b), then the
/// graphics status, then the walk (with its `Esc cancels` and order notice),
/// then the push count (ease-pass/08), then the filters, then the search.
/// The operation banner, when there is one, outlasts them all.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Drop {
    Lanes,
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
/// rest fit. The lane phrases go first, one at a time from the right, and
/// are never cut; they stay only while the whole tail fits `lanes_room`,
/// the room left once the branch name has its cells. The last group
/// standing after them, unless it is the search, is cut with `…` rather
/// than dropped — a lone walk failure is shortened, never silently gone
/// while any cell is free. The search is whole or not at all.
pub(super) fn fitted_tail(
    mut items: Vec<TailItem>,
    room: usize,
    lanes_room: usize,
) -> Vec<Span<'static>> {
    let width = |items: &[TailItem]| -> usize {
        let mut before = None;
        items
            .iter()
            .map(|item| {
                let cells = text_width(separator(before, item.drop)) + text_width(&item.text);
                before = Some(item.drop);
                cells
            })
            .sum::<usize>()
    };
    while width(&items) > lanes_room.min(room) {
        let Some(last) = items.iter().rposition(|item| item.drop == Drop::Lanes) else {
            break;
        };
        items.remove(last);
    }
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
    let mut before = None;
    for item in items {
        let text = format!("{}{}", separator(before, item.drop), item.text);
        before = Some(item.drop);
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

/// What goes before an item of group `drop`, after one of group `before`:
/// nothing first, ` · ` between two lane phrases, [`TAIL_SEPARATOR`]
/// otherwise.
fn separator(before: Option<Drop>, drop: Drop) -> &'static str {
    match before {
        None => "",
        Some(Drop::Lanes) if drop == Drop::Lanes => LANE_SEPARATOR,
        Some(_) => TAIL_SEPARATOR,
    }
}
