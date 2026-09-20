//! The `.`/right-click popup: a small list of actions for the row that
//! opened it (`context-actions-menu`, m2-details-and-diffs/12).
//!
//! The widget draws only the item labels and the cursor row's emphasis; it
//! carries no theme (the exact signature this ticket ships), so the
//! `surface-raised` background the design note asks for is painted by the
//! caller directly on the buffer before this renders on top — the same split
//! `graph_screen`'s row emphasis uses (background separately from text).

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Widget;

use crate::fit::fit;

/// An action a context-menu item dispatches. `apply_context_action` (in the
/// app crate) carries each one out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextAction {
    /// Open the explain panel for a marked row (only ever offered there).
    Explain,
    /// Copy the row's full SHA — the same command `y` already sends.
    CopySha,
    /// Pin or complete a compare against this row (`x`'s three branches).
    CompareWithThis,
    /// Open the detail pane on this row, or the working tree.
    OpenDetails,
    /// Fetch, the same as `f`.
    Fetch,
    /// Pull, the same as `p`.
    Pull,
    /// Push, the same as `P` (its confirm box included).
    Push,
    /// Undo the last operation, the same as `u`.
    Undo,
}

/// One row of the menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuItem {
    /// What choosing this item does.
    pub action: ContextAction,
    /// Its label.
    pub label: &'static str,
}

/// The popup itself: its items and which one the cursor is on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContextMenu {
    /// The items, in display order (top to bottom).
    pub items: Vec<MenuItem>,
    /// The highlighted item.
    pub cursor: usize,
}

/// The hints the width rule drops first and the user reaches for most: the
/// remote verbs and undo, in the footer's own order. Every menu lists them,
/// since none depends on which row the cursor is on.
const ALWAYS_ITEMS: [MenuItem; 4] = [
    MenuItem {
        action: ContextAction::Fetch,
        label: "Fetch (f)",
    },
    MenuItem {
        action: ContextAction::Pull,
        label: "Pull (p)",
    },
    MenuItem {
        action: ContextAction::Push,
        label: "Push (P)",
    },
    MenuItem {
        action: ContextAction::Undo,
        label: "Undo (u)",
    },
];

impl ContextMenu {
    /// A commit row's menu: `Explain` first when the row is marked (design
    /// note, "Context actions"), then the row's own actions, then the keys
    /// the hint bar drops at 80 and 120 columns.
    ///
    /// `. more` is a promise: at 80 columns the bar shows three keys and
    /// says the rest are behind `.`, so the rest have to *be* behind `.`
    /// (ADR-0037 §4; m5-03 review, Major 1). Each label carries its key, so
    /// choosing an item once teaches the key for next time. `S stash`,
    /// `J journal` and `/ search` open places of their own and are not here
    /// — the footer shows them from 160 columns, and `?` help (M8) is where
    /// the whole keymap finally lands.
    #[must_use]
    pub fn for_row(marked: bool) -> ContextMenu {
        let mut items = Vec::with_capacity(8);
        if marked {
            items.push(MenuItem {
                action: ContextAction::Explain,
                label: "Explain",
            });
        }
        items.push(MenuItem {
            action: ContextAction::CopySha,
            label: "Copy SHA (y)",
        });
        items.push(MenuItem {
            action: ContextAction::CompareWithThis,
            label: "Compare with this (x)",
        });
        items.push(MenuItem {
            action: ContextAction::OpenDetails,
            label: "Open details (⏎)",
        });
        items.extend(ALWAYS_ITEMS);
        ContextMenu { items, cursor: 0 }
    }

    /// The uncommitted-changes row's menu: `Open details` (design note,
    /// "Working tree row selected": `.` "lists `Open details` only"), then
    /// the same dropped keys every menu carries.
    #[must_use]
    pub fn for_working_tree() -> ContextMenu {
        let mut items = vec![MenuItem {
            action: ContextAction::OpenDetails,
            label: "Open details (⏎)",
        }];
        items.extend(ALWAYS_ITEMS);
        ContextMenu { items, cursor: 0 }
    }

    /// Move the cursor by `delta` items, clamped to the ends (the files
    /// list's own convention, `graft_ui::detail::FileListView::move_cursor`
    /// — no wrap).
    pub fn move_cursor(&mut self, delta: isize) {
        let last = self.items.len().saturating_sub(1);
        self.cursor = self.cursor.saturating_add_signed(delta).min(last);
    }

    /// The action the cursor is on. Never panics: both constructors always
    /// list at least one item.
    #[must_use]
    pub fn selected(&self) -> ContextAction {
        self.items[self.cursor].action
    }

    /// The longest label's cell width, plus 4 (design note, "Context
    /// actions").
    #[must_use]
    pub fn width(&self) -> u16 {
        let longest = self
            .items
            .iter()
            .map(|item| item.label.chars().count())
            .max()
            .unwrap_or(0);
        u16::try_from(longest).unwrap_or(u16::MAX).saturating_add(4)
    }
}

impl Widget for &ContextMenu {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        for (offset, item) in (0_u16..).zip(self.items.iter()) {
            if offset >= area.height {
                break;
            }
            let style = if usize::from(offset) == self.cursor {
                // The accent cursor row: reversed so it reads over whatever
                // background the caller painted underneath.
                Style::new().add_modifier(Modifier::REVERSED)
            } else {
                Style::default()
            };
            let text = fit(&format!(" {}", item.label), usize::from(area.width));
            let y = area.y.saturating_add(offset);
            buf.set_stringn(area.x, y, text, usize::from(area.width), style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The row's own actions, then the keys the bar drops
    /// ([`ALWAYS_ITEMS`]): `. more` has to lead somewhere (m5-03 review,
    /// Major 1).
    #[test]
    fn for_row_omits_explain_when_not_marked() {
        let menu = ContextMenu::for_row(false);
        assert!(
            !menu
                .items
                .iter()
                .any(|item| item.action == ContextAction::Explain)
        );
        assert_eq!(
            menu.items
                .iter()
                .map(|item| item.action)
                .collect::<Vec<_>>(),
            vec![
                ContextAction::CopySha,
                ContextAction::CompareWithThis,
                ContextAction::OpenDetails,
                ContextAction::Fetch,
                ContextAction::Pull,
                ContextAction::Push,
                ContextAction::Undo,
            ]
        );
    }

    #[test]
    fn for_row_lists_explain_first_when_marked() {
        let menu = ContextMenu::for_row(true);
        assert_eq!(menu.items[0].action, ContextAction::Explain);
        assert_eq!(menu.items.len(), 8);
    }

    /// Every label but `Explain`'s names the key that does the same thing,
    /// so the menu teaches the keyboard rather than replacing it.
    #[test]
    fn every_label_names_its_key() {
        let menu = ContextMenu::for_row(true);
        let labels: Vec<&str> = menu.items.iter().map(|item| item.label).collect();
        assert_eq!(
            labels,
            vec![
                "Explain",
                "Copy SHA (y)",
                "Compare with this (x)",
                "Open details (⏎)",
                "Fetch (f)",
                "Pull (p)",
                "Push (P)",
                "Undo (u)",
            ]
        );
    }

    #[test]
    fn for_working_tree_lists_open_details_then_the_dropped_keys() {
        let menu = ContextMenu::for_working_tree();
        assert_eq!(menu.items[0].action, ContextAction::OpenDetails);
        assert_eq!(
            menu.items[1..]
                .iter()
                .map(|item| item.action)
                .collect::<Vec<_>>(),
            vec![
                ContextAction::Fetch,
                ContextAction::Pull,
                ContextAction::Push,
                ContextAction::Undo,
            ]
        );
    }

    #[test]
    fn move_cursor_clamps_at_both_ends() {
        let mut menu = ContextMenu::for_row(true);
        menu.move_cursor(-5);
        assert_eq!(menu.cursor, 0);
        menu.move_cursor(100);
        assert_eq!(menu.cursor, menu.items.len() - 1);
    }

    #[test]
    fn selected_follows_cursor() {
        let mut menu = ContextMenu::for_row(true);
        menu.move_cursor(1);
        assert_eq!(menu.selected(), ContextAction::CopySha);
    }

    #[test]
    fn width_is_longest_label_plus_four() {
        let menu = ContextMenu::for_row(true);
        // "Compare with this (x)" is 21 cells, the longest label.
        assert_eq!(menu.width(), 21 + 4);
    }

    #[test]
    fn for_working_tree_width_is_its_own_longest_label_plus_four() {
        let menu = ContextMenu::for_working_tree();
        // "Open details (⏎)" is 16 characters.
        assert_eq!(menu.width(), 16 + 4);
    }
}
