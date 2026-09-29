//! Where the preview box sits on screen, and which choice a click lands on.

use super::{
    AppState, BoxRect, MIN_BOX_ROWS, PREVIEW_MIN, PreviewBox, fits, preview_box, preview_rows,
};

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
    // `graft_ui::preview_box::preview_box_area`'s arithmetic, in plain
    // numbers: `update` never names a drawing type (`update::tests::    // update_cannot_reach_the_world`), so the two are kept in step by
    // `tests::a_click_lands_on_the_button_the_widget_drew`, which renders
    // the real widget and reads the buttons back off the buffer.
    let footer_row = u16::from(height > 1);
    let usable = height.saturating_sub(footer_row);
    let rows = if width < PREVIEW_MIN.0 || height < PREVIEW_MIN.1 {
        usable.min(1)
    } else {
        preview_rows(view, width).max(MIN_BOX_ROWS).min(usable)
    };
    Some(BoxRect {
        x: 0,
        y: usable.saturating_sub(rows),
        width,
        height: rows,
    })
}

/// Which choice a click at `(column, row)` lands on, if any.
///
/// Mirrors `graft_ui`'s `buttons_row` layout — every choice bracketed, the
/// cursor's prefixed `▸` and the rest a space, two cells between them, the
/// whole row inset by the border and a space. `graft_ui` keeps that
/// arithmetic private, so
/// `the_click_targets_match_what_the_widget_draws` renders the real widget
/// and checks the two agree rather than trusting this copy.
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
