//! The half of `core_event` that answers the mouse (DESIGN.md §2.15): clicks,
//! the wheel, the divider drag and the context menu's own click routing.
//! Geometry itself lives in [`super::geometry`]; this file is what a mouse
//! event *does* once it knows where it landed.

use graft_core::event::{MouseButton, MouseInput, MouseKind};

use super::context_menu;
use super::detail;
use super::events::Command;
use super::geometry::{self, DetailHit};
use super::state::AppState;

/// Rows the wheel moves the viewport per notch (DESIGN.md §2.15). The
/// keyboard's `j`/`k` move the selection; the wheel moves the viewport
/// instead and leaves it alone, the way every other terminal pager's wheel
/// does.
pub(super) const WHEEL_LINES: isize = 3;

/// The half of [`super::core_event`] that answers the mouse (DESIGN.md §2.15).
pub(super) fn mouse_event(next: &mut AppState, commands: &mut Vec<Command>, mouse: MouseInput) {
    // A menu open (`.`/right-click, m2-details-and-diffs/12) is modal: any
    // button down either dispatches (inside it) or closes it (outside), and
    // nothing else this click would have done happens in the same press.
    if let Some(open) = next.context_menu.clone() {
        if matches!(
            mouse.kind,
            MouseKind::Down(MouseButton::Left | MouseButton::Right)
        ) {
            context_menu_click(next, commands, &open, mouse.column, mouse.row);
        }
        return;
    }
    match mouse.kind {
        MouseKind::ScrollUp => wheel(next, commands, mouse.column, mouse.row, -WHEEL_LINES),
        MouseKind::ScrollDown => wheel(next, commands, mouse.column, mouse.row, WHEEL_LINES),
        MouseKind::Down(MouseButton::Left) => {
            if let Some(bounds) = geometry::strip_bounds(next)
                && bounds.contains(mouse.column, mouse.row)
            {
                let marks = crate::view::strip::strip_marks(next);
                let row = graft_ui::overview_strip::row_for_cell(
                    &marks,
                    bounds.height,
                    mouse.row - bounds.y,
                );
                select_row(next, commands, row);
                return;
            }
            if matches!(
                geometry::detail_hit(next, mouse.column, mouse.row),
                Some(DetailHit::Divider)
            ) {
                next.detail.dragging = true;
                return;
            }
            if let Some(row) = geometry::graph_row_at(next, mouse.row) {
                if mouse.modifiers.shift || mouse.modifiers.control {
                    // Same as `x` on that row (`compare-two-commits`,
                    // m2-details-and-diffs/11): no separate click-only path,
                    // and no change to the selection itself.
                    toggle_compare_at(next, commands, row);
                } else {
                    next.on_working_tree = false;
                    select_row(next, commands, row);
                }
            } else if next.uncommitted_row_shown() && mouse.row == geometry::rows_above_graph(next)
            {
                // The first body row while the working-tree row is shown:
                // `graph_row_at` returns `None` for it (offset 0, displaced
                // by the virtual row), so this is the one case its offset
                // math does not already cover.
                next.on_working_tree = true;
            }
        }
        MouseKind::Down(MouseButton::Right) => {
            // Reuses the graph's existing click-to-row mapping, not a new
            // one (`context-actions-menu`, m2-details-and-diffs/12).
            if let Some(row) = geometry::graph_row_at(next, mouse.row) {
                context_menu::open_for_history_row(next, row);
            } else if next.uncommitted_row_shown() && mouse.row == geometry::rows_above_graph(next)
            {
                context_menu::open_for_working_tree(next);
            }
        }
        MouseKind::Drag(MouseButton::Left) if next.detail.dragging => {
            drag_divider(next, commands, mouse.column, mouse.row);
        }
        MouseKind::Up(_) => next.detail.dragging = false,
        // Any-motion reporting (`terminal.rs`) is what makes this arrive with
        // no button held — hover, not a drag.
        MouseKind::Moved => next.hover = geometry::graph_row_at(next, mouse.row),
        _ => {}
    }
}

/// The wheel: over the files or diff region of a shown detail pane, it
/// scrolls that region instead (`lines` already carries the direction);
/// everywhere else, it scrolls the graph viewport as before.
fn wheel(next: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16, lines: isize) {
    match geometry::detail_hit(next, column, row) {
        Some(DetailHit::Files) => {
            let AppState {
                detail, local_jobs, ..
            } = next;
            commands.extend(detail.move_files_cursor(lines, &mut || local_jobs.next_id()));
        }
        Some(DetailHit::Body) => next.detail.scroll_body_by(lines),
        Some(DetailHit::Divider) | None => scroll(next, commands, lines),
    }
}

/// A `Drag(Left)` while [`AppState::detail`]'s divider is held: set the
/// pane's cells straight from the pointer's column (`Right`) or row
/// (`Below`). No-op past a bound or in `Tab`, same as the keyboard resize.
fn drag_divider(next: &mut AppState, commands: &mut Vec<Command>, column: u16, row: u16) {
    let size = (next.size.columns, next.size.rows);
    let layout = detail::effective_layout(next.detail.focus, &next.detail.pane_size, size);
    let cells = match layout {
        graft_ui::detail_pane::DetailLayout::Right { .. } => size.0.saturating_sub(column),
        graft_ui::detail_pane::DetailLayout::Below { .. } => size.1.saturating_sub(row),
        graft_ui::detail_pane::DetailLayout::Tab { .. } => return,
    };
    if next.detail.set_pane_cells(&layout, cells, size) {
        commands.push(Command::SaveLayout(next.detail.pane_size));
    }
}

/// Scrolls the viewport by `lines` wheel notches; the selection stays put.
fn scroll(next: &mut AppState, commands: &mut Vec<Command>, lines: isize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.scroll(lines, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// Selects the row a click landed on.
fn select_row(next: &mut AppState, commands: &mut Vec<Command>, row: usize) {
    let height = next.graph_rows();
    if let Some(history) = next.history.as_mut()
        && let Some(request) = history.select_at(row, height)
        && let Some(id) = next.awaiting
    {
        commands.push(Command::ShowRows(id, request));
    }
}

/// `Shift`/`Ctrl`+click on a row: `detail::toggle_compare`'s three branches,
/// same as `x` on it (`compare-two-commits`, m2-details-and-diffs/11). `row`
/// need not be laid out under the cursor — it is whatever `graph_row_at`
/// named, independent of the selection.
fn toggle_compare_at(next: &mut AppState, commands: &mut Vec<Command>, row: usize) {
    let Some(oid) = next
        .history
        .as_ref()
        .and_then(|history| history.row_at(row))
        .map(|selected| selected.commit.id)
    else {
        return;
    };
    let mut local_jobs = next.local_jobs;
    commands.extend(detail::toggle_compare(next, row, oid, &mut || {
        local_jobs.next_id()
    }));
    next.local_jobs = local_jobs;
}

/// A button down while a menu is open: dispatch the item under it, or close
/// it without acting when the click landed outside — either way the menu is
/// gone after this press.
fn context_menu_click(
    next: &mut AppState,
    commands: &mut Vec<Command>,
    open: &context_menu::OpenMenu,
    column: u16,
    screen_row: u16,
) {
    next.context_menu = None;
    let Some(area) = geometry::context_menu_bounds(next, open.row, &open.menu) else {
        return;
    };
    if !area.contains(column, screen_row) {
        return;
    }
    let index = usize::from(screen_row - area.y);
    let Some(item) = open.menu.items.get(index) else {
        return;
    };
    commands.extend(context_menu::dispatch(
        next,
        open.row,
        &open.verbs,
        item.action,
    ));
}
