//! The window over the resolver's document: `j`/`k`, the pages, `g`/`G`,
//! the wheel, and a click that puts the cursor on a block.

use graft_core::event::{KeyCode, MouseButton, MouseInput, MouseKind};
use graft_core::job::JobId;
use graft_ui::conflict_resolver::{HEADER_ROWS, ScreenRow, file_at, finish_row_at, row_at};

use super::files::select_file;
use super::fits;
use super::geometry::{areas, body, max_top, view_height};
use crate::update::events::Command;
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
pub fn resolver_mouse(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    mouse: MouseInput,
    next_id: &mut impl FnMut() -> JobId,
) -> bool {
    if state.resolver.is_none() {
        return false;
    }
    super::seek::he_acted(state);
    // A box up over the resolver (`d`'s "Start … over?") owns the input,
    // the mouse as much as the keys: `keys::key_event` hands every key to
    // `confirm_key` first, so the resolver sees none of them. A click or a
    // wheel under it does nothing either — before, a click on another
    // file's row closed the resolver and left the box standing over the
    // graph (review m6-amend-13, Major 1).
    if !fits(state) || super::box_open(state) {
        return true;
    }
    match mouse.kind {
        MouseKind::ScrollUp => scroll_by(state, -WHEEL_LINES),
        MouseKind::ScrollDown => scroll_by(state, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => {
            click(state, commands, mouse.column, mouse.row, next_id);
        }
        _ => {}
    }
    true
}

/// A left click: a file in the list opens it; the block whose row was hit
/// takes the cursor. The list's rule row or `│`, the title and notice rows,
/// and any row past the end of the document name nothing and so do nothing.
pub(super) fn click(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    column: u16,
    row: u16,
    next_id: &mut impl FnMut() -> JobId,
) {
    let (files, placed, split) = areas(state);
    let inside = |area: &graft_ui::conflict_resolver::ResolverAreas, list: bool| {
        let rect = if list { area.list } else { area.doc };
        column >= rect.x && column < rect.right() && row >= rect.y && row < rect.bottom()
    };
    if inside(&split, true) {
        if let Some(index) = file_at(
            &files,
            placed,
            split.list.width,
            split.list.height,
            column - split.list.x,
            row - split.list.y,
        ) {
            let answered = select_file(state, commands, index, next_id);
            debug_assert!(answered, "`file_at` named a listed file");
        }
        return;
    }
    if !inside(&split, false) {
        return;
    }
    // The finish row does what `c` does (m6-conflicts/60).
    let area = crate::view::resolver::resolver_area(state, super::banner_rows(state));
    if crate::view::resolver::resolver_view(state, split.doc.width)
        .is_some_and(|doc| finish_row_at(&doc, placed, area, column, row))
    {
        super::leave::finish_pressed(state, commands, next_id);
        return;
    }
    let Some((layout, _)) = body(state) else {
        return;
    };
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    let Some(body_row) = row.checked_sub(split.doc.y.saturating_add(HEADER_ROWS)) else {
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
        ScreenRow::Aligned { block, .. } => *block,
        ScreenRow::Plain { .. }
        | ScreenRow::WholeFile { .. }
        | ScreenRow::Note { .. }
        | ScreenRow::PaneTitles => None,
    }
}

/// `PgUp`/`PgDn`, `g`/`G`, `Home`/`End`: move the document under the
/// window. `j`/`k` and the arrows step between files instead
/// (m6-conflicts/17 D5). Anything else is swallowed.
pub(super) fn scroll_key(state: &mut AppState, code: KeyCode) {
    // A page keeps one row of overlap, so the eye has something to land on.
    let page = isize::try_from(usize::from(view_height(state)))
        .unwrap_or(1)
        .saturating_sub(1)
        .max(1);
    match code {
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
