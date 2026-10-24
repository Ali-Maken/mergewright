//! The branch picker's box over the graph (`C`, m7-power-tools/10).

#[allow(
    clippy::wildcard_imports,
    reason = "one module split in files: its parent is its prelude"
)]
use super::*;

/// The branch picker's box (`C`, m7-power-tools/10) over the lower part of
/// the graph's own rows: the header above and the footer's notice and hint
/// rows below are left alone (`update::picker::picker_box_in`).
pub(super) fn draw_picker(
    state: &AppState,
    frame: &mut Frame,
    area: Rect,
    theme: &Theme,
    ascii: bool,
) -> Option<Rect> {
    let mut view = crate::view::picker::picker_view(state)?;
    let pane = super::layout::primary_pane_area(state, area)?;
    let at = mergewright_ui::cells_of(pane);
    let rect = mergewright_ui::rect_of(crate::update::picker::picker_box_in(state, at));
    if rect.is_empty() {
        return None;
    }
    view.ascii = ascii;
    super::boxes::clear_and_tint(frame, rect, rect, theme);
    mergewright_ui::picker::draw_picker(frame, rect, &view, theme);
    Some(rect)
}
