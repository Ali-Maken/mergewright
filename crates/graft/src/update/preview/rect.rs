//! Where the preview box sits on screen, and which choice a click lands on.

use super::{AppState, PreviewBox, fits, preview_box, preview_rows};

/// Where the preview box goes on screen, in plain screen coordinates —
/// `update` draws nothing and names no drawing type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoxRect {
    /// Leftmost column.
    pub x: u16,
    /// Topmost row.
    pub y: u16,
    /// How wide.
    pub width: u16,
    /// How tall.
    pub height: u16,
}

impl BoxRect {
    /// One past the last row.
    #[must_use]
    pub fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }
}

/// Rows the graph gives up to the box this frame, footer excluded.
///
/// The box takes rows the same way the uncommitted-changes row and the
/// footer do — out of [`AppState::graph_rows`] — so the rows it covers are
/// rows the graph never drew, rather than rows drawn and then painted over.
#[must_use]
pub fn preview_rows_taken(state: &AppState) -> u16 {
    let Some(view) = preview_box(state) else {
        return 0;
    };
    if fits(state) {
        preview_rows(&view, state.size.columns)
    } else {
        // Below `PREVIEW_MIN` the widget draws one line of text, not a box.
        1
    }
}

/// Where the box is drawn: docked to the bottom of the graph area, above
/// the footer row. `None` when the area cannot hold it at all.
#[must_use]
pub fn preview_box_rect(state: &AppState, view: &PreviewBox) -> Option<BoxRect> {
    let (width, height) = (state.size.columns, state.size.rows);
    if width == 0 || height == 0 {
        return None;
    }
    // The guard above is the caller's, not arithmetic: an empty frame has
    // no box. The dock itself is `graft_ui`'s own, in plain numbers — the
    // function `preview_box_area` draws from — since `update` never names a
    // drawing type (`update::tests::purity::update_cannot_reach_the_world`).
    let wanted = preview_rows(view, width);
    Some(
        graft_ui::preview_box::dock_cells(
            graft_ui::Cells {
                x: 0,
                y: 0,
                width,
                height,
            },
            wanted,
        )
        .into(),
    )
}

impl From<graft_ui::Cells> for BoxRect {
    fn from(cells: graft_ui::Cells) -> Self {
        let graft_ui::Cells {
            x,
            y,
            width,
            height,
        } = cells;
        Self {
            x,
            y,
            width,
            height,
        }
    }
}

/// Which choice a click at `(column, row)` lands on, if any.
///
/// Mirrors `graft_ui`'s `buttons_row` layout — every choice bracketed, the
/// cursor's prefixed `▸` and the rest a space, two cells between them, the
/// whole row inset by the border and a space. `graft_ui` keeps that
/// arithmetic private, so `a_click_lands_on_the_button_the_frame_drew`
/// draws the whole frame, reads the buttons off it and clicks them through
/// `update`, rather than trusting this copy (or this file's dock).
pub(super) fn choice_at(view: &PreviewBox, area: BoxRect, column: u16, row: u16) -> Option<usize> {
    if area.height < 2 || row.saturating_add(2) != area.bottom() {
        return None;
    }
    let mut at = area.x.saturating_add(2);
    for (index, label) in view.choices().iter().enumerate() {
        if index > 0 {
            at = at.saturating_add(2);
        }
        // `▸[ ` + label + ` ]`
        let width =
            u16::try_from(label.as_str().chars().count().saturating_add(5)).unwrap_or(u16::MAX);
        if column >= at && column < at.saturating_add(width) {
            return Some(index);
        }
        at = at.saturating_add(width);
    }
    None
}
