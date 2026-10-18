//! The screens: widgets, layout, theme and the view models they render from.
//! `ui` draws and nothing else — it holds no repository handle and never
//! imports `git` or `ops`. Every screen renders a plain view model that `app`
//! assembles, and repo-derived strings reach it only as `term::SanitizedText`,
//! so escape-sequence injection is a compile error rather than a review catch.
//! Depends on `graph`, `term` and `core`.
#![forbid(unsafe_code)]

pub mod blame;
pub mod commit_editor;
pub mod confirm;
pub mod conflict_resolver;
pub mod context_menu;
pub mod credential_prompt;
pub mod detail;
pub mod detail_pane;
pub mod diff_words;
pub mod empty_state;
pub mod finish_summary;
pub mod fit;
mod frame_box;
pub mod graph_column;
pub mod graph_screen;
pub mod history;
pub mod incoming_list;
pub mod journal_screen;
pub mod keymap;
pub mod map_screen;
pub mod merge_look;
pub mod name_prompt;
pub mod overview_strip;
pub mod paint;
pub mod palette;
pub mod picker;
pub mod preview_box;
pub mod rewrite_editor;
pub mod stash_screen;
pub mod status_screen;
pub mod still_to_apply;
pub mod syntax;
pub mod text_input;
pub mod theme;
pub mod tour_card;
pub mod whitespace_tail;

#[cfg(test)]
mod test_support;

pub use still_to_apply::{PendingRowView, PendingView, draw_still_to_apply};

/// A screen rectangle in plain numbers, by name: column, row, width and
/// height in cells.
///
/// The shape geometry is handed out in to a caller that must not name the
/// rendering library: `mergewright::update` hit-tests clicks against the very
/// carve the widgets draw from ([`graph_screen::row_carve`],
/// [`preview_box::dock_cells`], [`picker::picker_box`]) rather than a copy
/// of it. Named fields, not a tuple, so a height is never read as `.3` and
/// a width and a height cannot be swapped unseen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cells {
    /// Leftmost column.
    pub x: u16,
    /// Top row.
    pub y: u16,
    /// Columns across.
    pub width: u16,
    /// Rows down.
    pub height: u16,
}

/// `rect` as [`Cells`].
pub const fn cells_of(rect: ratatui::layout::Rect) -> Cells {
    Cells {
        x: rect.x,
        y: rect.y,
        width: rect.width,
        height: rect.height,
    }
}

/// `cells` as a `Rect`.
pub const fn rect_of(cells: Cells) -> ratatui::layout::Rect {
    let Cells {
        x,
        y,
        width,
        height,
    } = cells;
    ratatui::layout::Rect {
        x,
        y,
        width,
        height,
    }
}
