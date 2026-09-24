//! How much room the resolver's document has: the banner line above it,
//! the file list above or beside it, the widget's own title rows, and the
//! bar below it.

use graft_ui::conflict_resolver::{
    FileListView, HEADER_ROWS, Layout, MIN_COLUMNS, MIN_ROWS, Placement, ResolverAreas,
};

use crate::update::state::AppState;

/// `top` moved as little as it must for `block`'s header (card) or rule
/// (three panes) row to be on screen — and, when it must move at all, with
/// that row **first**, since everything the block is about is below it.
#[must_use]
pub fn scroll_block_into_view(layout: &Layout, block: usize, top: usize, height: u16) -> usize {
    let height = usize::from(height).max(1);
    let Some(&row) = layout.block_row.get(block) else {
        return top;
    };
    if row >= top && row < top.saturating_add(height) {
        return top;
    }
    row.min(layout.rows.len().saturating_sub(height))
}

/// Rows the operation banner takes above the resolver: one while an
/// operation is live, none otherwise.
///
/// **The resolver keeps the banner word.** Every other overlay replaces the
/// graph *and* the banner riding on its header, which is fine for the Map
/// or the stash list and wrong here: the one screen a user opens mid-rebase
/// is the one where `REBASING step 3 of 7` matters most. So the resolver is
/// an ordinary `GraphOverlay` — the frame's match, the row counts and
/// `graph_screen_on_frame` all stay honest — and it draws the banner line
/// itself, on the first row of its own area (`view::frame::draw_resolver`).
#[must_use]
pub fn banner_rows(state: &AppState) -> u16 {
    u16::from(crate::update::banner::banner_view(state).is_some())
}

/// The file list as drawn, where it goes, and the widget's area split
/// between it and the document — built by the view, through `graft_ui`'s
/// own `resolver_areas`, so a click and the draw agree on every origin.
pub(super) fn areas(state: &AppState) -> (FileListView, Placement, ResolverAreas) {
    crate::view::resolver::resolver_split(state, banner_rows(state))
}

/// Rows of document on screen.
pub(super) fn view_height(state: &AppState) -> u16 {
    areas(state).2.doc.height.saturating_sub(HEADER_ROWS)
}

/// Is there room to draw the resolver at all?
///
/// Measured over the rows the resolver actually gets, not over the
/// terminal: the banner above it and the bar below it are rows the document
/// does not have (ADR-0039's amendment — ask how much room this thing has).
#[must_use]
pub fn fits(state: &AppState) -> bool {
    state.size.columns >= MIN_COLUMNS
        && state
            .size
            .rows
            .saturating_sub(banner_rows(state))
            .saturating_sub(1)
            >= MIN_ROWS
}

/// The document laid out at this frame's width, in the arrangement the
/// draw uses (both from `resolver_split`), with the rows on screen.
pub(super) fn body(state: &AppState) -> Option<(Layout, u16)> {
    let (_, placed, split) = areas(state);
    let layout = crate::view::resolver::resolver_layout(state, placed, split.doc.width)?;
    Some((layout, split.doc.height.saturating_sub(HEADER_ROWS)))
}

/// After a resize: the cursor block back in view, and the window no
/// further down than the document goes. Crossing
/// [`THREE_PANE_MIN`](graft_ui::conflict_resolver::THREE_PANE_MIN) swaps the
/// card's rows for the panes' (or back), so the old `top` can land
/// anywhere.
pub fn resized(state: &mut AppState) {
    let Some(block) = state.resolver.as_ref().and_then(|open| open.cursor_block) else {
        return;
    };
    let top = scrolled_to(state, block).min(max_top(state));
    if let Some(open) = state.resolver.as_mut() {
        open.top = top;
    }
}

/// The largest `top` that still shows a full window of document.
pub(super) fn max_top(state: &AppState) -> usize {
    match body(state) {
        Some((layout, height)) => layout.rows.len().saturating_sub(usize::from(height)),
        None => 0,
    }
}

/// `top`, moved as little as it must for `block` to be on screen.
pub(super) fn scrolled_to(state: &AppState, block: usize) -> usize {
    let top = state.resolver.as_ref().map_or(0, |open| open.top);
    match body(state) {
        Some((layout, height)) => scroll_block_into_view(&layout, block, top, height),
        None => top,
    }
}
