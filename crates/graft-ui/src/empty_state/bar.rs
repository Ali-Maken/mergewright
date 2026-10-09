//! The empty state's key bar: which keys it names, how it sheds them when
//! the row is narrow (the tour clause first, m8/19b), and its drawing.
//! Moved out of `empty_state.rs` (m8/18e B1).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;

use super::{CloneShown, EmptyPrompt, EmptyState, EmptyStateView};
use crate::frame_box::cells;
use crate::keymap::{ActionId, Keymap};
use crate::paint::foreground;
use crate::theme::Token;

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
        None => list_keys(view, keys),
    }
}

/// `parts` whose actions `keys` binds, each with its key spelled.
fn spelled(keys: &Keymap, parts: &[(&'static str, &'static str)]) -> Vec<(String, &'static str)> {
    parts
        .iter()
        .filter_map(|(action, label)| {
            keys.spell_keys(&[ActionId(action)])
                .map(|key| (key, *label))
        })
        .collect()
}

/// The bar over the list, no prompt open.
fn list_keys(view: &EmptyStateView, keys: &Keymap) -> Vec<(String, &'static str)> {
    let mut parts = vec![("empty_state.open", "open…")];
    if !view.missing {
        parts.push(("empty_state.init", "init"));
        parts.push(("empty_state.clone", "clone…"));
    }
    parts.push(("empty_state.tour", TOUR_CLAUSE));
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

/// The bar's `t` clause (m8/19b).
pub(super) const TOUR_CLAUSE: &str = "practice tour";

/// [`bar_keys`] as drawn in `width` cells: the tour clause is the first
/// shed when the whole bar does not fit (it is opt-in, and the actions row
/// still names `t`); past that, the bar is cut at the edge as before.
#[must_use]
pub fn bar_fitted(
    view: &EmptyStateView,
    keys: &Keymap,
    width: usize,
) -> Vec<(String, &'static str)> {
    let mut parts = bar_keys(view, keys);
    let wide = |parts: &[(String, &str)]| {
        parts
            .iter()
            .map(|(key, label)| cells(key) + 1 + cells(label))
            .sum::<usize>()
            + 3 * parts.len().saturating_sub(1)
    };
    if wide(&parts) > width {
        parts.retain(|(_, label)| *label != TOUR_CLAUSE);
    }
    parts
}

impl EmptyState<'_> {
    pub(super) fn draw_bar(&self, area: Rect, y: u16, buf: &mut Buffer) {
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
        for (index, (key, label)) in bar_fitted(self.view, self.keys, width)
            .into_iter()
            .enumerate()
        {
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
