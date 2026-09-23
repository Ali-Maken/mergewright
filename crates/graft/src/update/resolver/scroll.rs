//! The window over the resolver's document: `j`/`k`, the pages, `g`/`G`,
//! the wheel, and a click that puts the cursor on a block.

use graft_core::event::{KeyCode, MouseButton, MouseInput, MouseKind};
use graft_ui::conflict_resolver::{ScreenRow, row_at};

use super::fits;
use super::geometry::{body, body_y, max_top, view_height};
use crate::update::state::AppState;

/// Rows the wheel moves the document per notch — the same three rows the
/// graph's own wheel moves (`update::mouse::WHEEL_LINES`, DESIGN.md §2.15),
/// so the two pagers never scroll by different amounts.
const WHEEL_LINES: isize = 3;

/// The wheel and the click, while the resolver is open.
///
/// `true` always — the resolver is modal for the mouse exactly as it is for
/// the keyboard, so **no click reaches the graph underneath it**. (The Map,
/// the journal and the stash list do not do this: `left_click` runs
/// `graph_row_at` with no overlay check at all, so a click over one of them
/// selects a row of the graph behind it. That is a separate defect,
/// `docs/reviews/m5-10-30-31.md:291-296`, and not this ticket's to fix.)
pub fn resolver_mouse(state: &mut AppState, mouse: MouseInput) -> bool {
    if state.resolver.is_none() {
        return false;
    }
    if !fits(state) {
        return true;
    }
    match mouse.kind {
        MouseKind::ScrollUp => scroll_by(state, -WHEEL_LINES),
        MouseKind::ScrollDown => scroll_by(state, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => click(state, mouse.row),
        _ => {}
    }
    true
}

/// A left click: the block whose row was hit takes the cursor. The title
/// and notice rows, and any row past the end of the document, name no
/// block and so do nothing.
pub(super) fn click(state: &mut AppState, row: u16) {
    let Some((layout, _)) = body(state) else {
        return;
    };
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    let Some(body_row) = row.checked_sub(body_y(state)) else {
        return;
    };
    let Some(block) = row_at(&layout, top, body_row).and_then(block_of) else {
        return;
    };
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    open.cursor_block = Some(block);
    open.notice = None;
}

/// Which block a laid-out row belongs to, if any.
pub(super) fn block_of(row: &ScreenRow) -> Option<usize> {
    match row {
        ScreenRow::Header { block }
        | ScreenRow::Labels { block }
        | ScreenRow::Pair { block, .. }
        | ScreenRow::Side { block, .. }
        | ScreenRow::End { block } => Some(*block),
        ScreenRow::Plain { .. } | ScreenRow::WholeFile { .. } | ScreenRow::Note { .. } => None,
    }
}

/// `j`/`k`, the arrows, `PgUp`/`PgDn`, `g`/`G`: move the document under the
/// window. Anything else is swallowed.
pub(super) fn scroll_key(state: &mut AppState, code: KeyCode) {
    // A page keeps one row of overlap, so the eye has something to land on.
    let page = isize::try_from(usize::from(view_height(state)))
        .unwrap_or(1)
        .saturating_sub(1)
        .max(1);
    match code {
        KeyCode::Char('j') | KeyCode::Down => scroll_by(state, 1),
        KeyCode::Char('k') | KeyCode::Up => scroll_by(state, -1),
        KeyCode::PageDown => scroll_by(state, page),
        KeyCode::PageUp => scroll_by(state, -page),
        KeyCode::Char('g') | KeyCode::Home => {
            if let Some(open) = state.resolver.as_mut() {
                open.top = 0;
            }
        }
        KeyCode::Char('G') | KeyCode::End => {
            let bottom = max_top(state);
            if let Some(open) = state.resolver.as_mut() {
                open.top = bottom;
            }
        }
        _ => {}
    }
}

/// Move the window `lines` rows, within the document.
pub(super) fn scroll_by(state: &mut AppState, lines: isize) {
    let bottom = max_top(state);
    let Some(open) = state.resolver.as_mut() else {
        return;
    };
    let top = isize::try_from(open.top).unwrap_or(isize::MAX);
    let moved = top.saturating_add(lines).max(0);
    open.top = usize::try_from(moved).unwrap_or(0).min(bottom);
}
