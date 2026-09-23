//! How much room the resolver's document has: the banner line above it,
//! the widget's own title rows, and the bar below it.

use graft_ui::conflict_resolver::{HEADER_ROWS, Layout, MIN_COLUMNS, MIN_ROWS};

use crate::update::state::AppState;

/// `top` moved as little as it must for `block`'s header row to be on
/// screen — and, when it must move at all, with the header on the **first**
/// row, since everything the block is about is below it.
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

/// The first screen row of the resolver's document body: past the banner
/// line and past the widget's own title and notice rows.
pub(super) fn body_y(state: &AppState) -> u16 {
    banner_rows(state).saturating_add(HEADER_ROWS)
}

/// Rows of document on screen.
pub(super) fn view_height(state: &AppState) -> u16 {
    state
        .size
        .rows
        .saturating_sub(body_y(state))
        // The hint bar / status line the resolver draws under itself.
        .saturating_sub(1)
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

/// The document laid out at this frame's width, with the rows on screen.
pub(super) fn body(state: &AppState) -> Option<(Layout, u16)> {
    let layout = crate::view::resolver::resolver_layout(state, state.size.columns)?;
    Some((layout, view_height(state)))
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
