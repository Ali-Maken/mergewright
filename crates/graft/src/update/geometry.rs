//! Pure screen geometry `update` needs to answer a mouse event: where the
//! detail pane, its regions and the context menu land, mirrored from the
//! `view` crate's own layout rather than depending on its rendering library
//! (`update/tests.rs` checks this literally by grepping the module's source
//! for that library's name — this file must stay just as clean).

use graft_ui::detail_pane::DetailLayout;

use super::context_menu;
use super::detail;
use super::state::AppState;

/// Screen rows above the graph body: the header line `graph_screen` draws at
/// the top (`CHROME_ROWS` also counts the footer, which a mouse row never
/// needs to skip past — `graph_rows` already excludes it). A harness banner
/// adds its rows; see [`rows_above_graph`].
pub(super) const HEADER_ROWS: u16 = 1;

/// Screen rows above the first graph row: the header and any banner.
pub(super) fn rows_above_graph(state: &AppState) -> u16 {
    HEADER_ROWS.saturating_add(state.harness.banner_rows())
}

/// A screen rectangle in the four plain numbers a `Rect` would carry.
/// `update` mirrors the pane's geometry rather than depending on the view
/// crate's rendering library to compute it: the module must stay pure enough
/// to run with no terminal at all, and `update/tests.rs` checks this
/// literally by grepping the module's source for that library's name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Bounds {
    pub(super) x: u16,
    pub(super) y: u16,
    pub(super) width: u16,
    pub(super) height: u16,
}

impl Bounds {
    pub(super) fn contains(self, column: u16, row: u16) -> bool {
        column >= self.x
            && column < self.x + self.width
            && row >= self.y
            && row < self.y + self.height
    }
}

/// `size`'s graph rectangle and pane rectangle for `layout`, either of which
/// is absent in `Tab` mode: mirrors `graft_ui::detail_pane::split_area`,
/// which `view/frame.rs::split_for_detail` draws from — same split, so a
/// click or wheel notch lands exactly where the pixel is.
pub(super) fn detail_split(
    layout: DetailLayout,
    size: (u16, u16),
) -> (Option<Bounds>, Option<Bounds>) {
    match layout {
        DetailLayout::Right { columns } => {
            let columns = columns.min(size.0);
            let graph_width = size.0 - columns;
            let graph = Bounds {
                x: 0,
                y: 0,
                width: graph_width,
                height: size.1,
            };
            let pane = Bounds {
                x: graph_width,
                y: 0,
                width: columns,
                height: size.1,
            };
            (Some(graph), Some(pane))
        }
        DetailLayout::Below { rows } => {
            let rows = rows.min(size.1);
            let graph_height = size.1 - rows;
            let graph = Bounds {
                x: 0,
                y: 0,
                width: size.0,
                height: graph_height,
            };
            let pane = Bounds {
                x: 0,
                y: graph_height,
                width: size.0,
                height: rows,
            };
            (Some(graph), Some(pane))
        }
        DetailLayout::Tab { shown } => {
            let strip = 1.min(size.1);
            let content = Bounds {
                x: 0,
                y: strip,
                width: size.0,
                height: size.1 - strip,
            };
            if shown {
                (None, Some(content))
            } else {
                (Some(content), None)
            }
        }
    }
}

/// `cells * pct / 100`, saturating rather than overflowing for any `u16`.
/// Mirrors the private `percent` in `graft_ui::detail_pane`.
fn percent_of(cells: u16, pct: u32) -> u16 {
    u16::try_from(u32::from(cells) * pct / 100).unwrap_or(u16::MAX)
}

/// `pane`'s files rectangle and body rectangle for `layout`, `files_count`
/// capping how much of the files region a `Right`/`Tab` pane actually needs:
/// mirrors `graft_ui::detail_pane::detail_areas`.
fn detail_hit_areas(pane: Bounds, layout: DetailLayout, files_count: usize) -> (Bounds, Bounds) {
    let header_rows: u16 = match layout {
        DetailLayout::Right { .. } => 5,
        DetailLayout::Below { .. } => 2,
        DetailLayout::Tab { .. } => 3,
    };
    let header_height = header_rows.min(pane.height);
    let rest = Bounds {
        y: pane.y + header_height,
        height: pane.height - header_height,
        ..pane
    };
    if matches!(layout, DetailLayout::Below { .. }) {
        let files_width = 30.min(rest.width);
        let files = Bounds {
            width: files_width,
            ..rest
        };
        let body = Bounds {
            x: rest.x + files_width,
            width: rest.width - files_width,
            ..rest
        };
        (files, body)
    } else {
        let cap = percent_of(rest.height, 40);
        let files_height = cap
            .min(u16::try_from(files_count).unwrap_or(u16::MAX))
            .min(rest.height);
        let files = Bounds {
            height: files_height,
            ..rest
        };
        let body = Bounds {
            y: rest.y + files_height,
            height: rest.height - files_height,
            ..rest
        };
        (files, body)
    }
}

/// How many rows the diff body has on screen right now, `0` while the pane
/// is hidden or its size is not known yet. The diff region's line cursor
/// needs it to scroll only as far as it must to stay in view
/// (`DetailState::move_body_cursor`), and `0` is exactly the "height
/// unknown" case that call falls back on.
pub(super) fn diff_body_rows(state: &AppState) -> usize {
    if !state.detail.shown {
        return 0;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    let Some(pane) = detail_split(layout, size).1 else {
        return 0;
    };
    let files_count = match &state.detail.files {
        detail::Loading::Ready(files) => files.visible().count(),
        _ => 0,
    };
    let (_, body) = detail_hit_areas(pane, layout, files_count);
    usize::from(body.height)
}

/// Where a screen point lands over the detail pane, when it is shown. `None`
/// when the pane is hidden or the point isn't over it (including the graph
/// side of a `Right` or `Below` layout, and the pane's header).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DetailHit {
    /// The pane's near edge: the first column (`Right`) or row (`Below`) of
    /// the pane. There is no drawn border to grab — nothing paints a glyph
    /// on this cell — so this is a deliberate choice of hit zone rather than
    /// a rendered seam; dragging from here maps straight onto
    /// `set_pane_cells` with no jump (`size - point` reads back the pane's
    /// current cell count right at this column/row). `Tab` has no divider:
    /// nothing to drag.
    Divider,
    /// Inside the file list.
    Files,
    /// Inside the message or diff body.
    Body,
}

pub(super) fn detail_hit(state: &AppState, column: u16, row: u16) -> Option<DetailHit> {
    if !state.detail.shown {
        return None;
    }
    let size = (state.size.columns, state.size.rows);
    let layout = detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
    match layout {
        DetailLayout::Right { columns } => {
            let divider = size.0.saturating_sub(columns.min(size.0));
            if divider < size.0 && column == divider {
                return Some(DetailHit::Divider);
            }
        }
        DetailLayout::Below { rows } => {
            let divider = size.1.saturating_sub(rows.min(size.1));
            if divider < size.1 && row == divider {
                return Some(DetailHit::Divider);
            }
        }
        DetailLayout::Tab { .. } => {}
    }
    let (_, pane) = detail_split(layout, size);
    let pane = pane?;
    if !pane.contains(column, row) {
        return None;
    }
    let files_count = match &state.detail.files {
        detail::Loading::Ready(files) => files.visible().count(),
        _ => 0,
    };
    let (files, body) = detail_hit_areas(pane, layout, files_count);
    if files.contains(column, row) {
        Some(DetailHit::Files)
    } else if body.contains(column, row) {
        Some(DetailHit::Body)
    } else {
        None
    }
}

/// The absolute history row a screen row names, if it names a row at all:
/// `None` over the header, the footer, the uncommitted-changes row (selected
/// through the flag, not a `History` row — see `mouse_event`), or a row
/// below what has laid out so far.
pub(super) fn graph_row_at(state: &AppState, screen_row: u16) -> Option<usize> {
    let history = state.history.as_ref()?;
    let offset = usize::from(screen_row.checked_sub(rows_above_graph(state))?);
    if offset >= state.graph_rows() {
        return None;
    }
    let offset = if crate::view::uncommitted_shown(state) {
        offset.checked_sub(1)?
    } else {
        offset
    };
    let absolute = history.top + offset;
    (absolute < history.rows).then_some(absolute)
}

/// The graph's own rectangle, whatever the detail pane is doing: mirrors
/// `detail_hit`'s split, but the graph side rather than the pane
/// (`context-actions-menu`, m2-details-and-diffs/12).
pub(super) fn context_menu_graph_bounds(state: &AppState) -> Option<Bounds> {
    let size = (state.size.columns, state.size.rows);
    if state.detail.shown {
        let layout = detail::effective_layout(state.detail.focus, &state.detail.pane_size, size);
        detail_split(layout, size).0
    } else {
        Some(Bounds {
            x: 0,
            y: 0,
            width: size.0,
            height: size.1,
        })
    }
}

/// The overview strip's own one-column rectangle, while it is shown
/// (`overview-strip`, m2-details-and-diffs/20): the graph bounds' last
/// column, mirroring `view::frame::render_graph_screen`'s carve-out. `None`
/// while `AppState::strip_shown` is off or the map is shown — the strip
/// draws in neither case.
pub(super) fn strip_bounds(state: &AppState) -> Option<Bounds> {
    if !state.strip_shown || state.map.shown {
        return None;
    }
    let graph = context_menu_graph_bounds(state)?;
    if graph.width == 0 {
        return None;
    }
    Some(Bounds {
        x: graph.x + graph.width - 1,
        y: graph.y,
        width: 1,
        height: graph.height,
    })
}

/// Where a `ContextMenu` opened on `row` draws: anchored bottom-left of the
/// row, or the graph's own bottom-left when there is no room below (design
/// note, "Context actions"). `view::frame` renders from this exact rectangle
/// too, so a click and a pixel always agree.
pub(crate) fn context_menu_area(
    state: &AppState,
    row: usize,
    menu: &graft_ui::context_menu::ContextMenu,
) -> Option<(u16, u16, u16, u16)> {
    context_menu_bounds(state, row, menu)
        .map(|bounds| (bounds.x, bounds.y, bounds.width, bounds.height))
}

/// [`context_menu_area`]'s own `Bounds` shape, kept private so callers stay
/// on the plain tuple.
pub(super) fn context_menu_bounds(
    state: &AppState,
    row: usize,
    menu: &graft_ui::context_menu::ContextMenu,
) -> Option<Bounds> {
    let graph = context_menu_graph_bounds(state)?;
    let width = menu.width().min(graph.width.max(1));
    let height = u16::try_from(menu.items.len())
        .unwrap_or(u16::MAX)
        .min(graph.height.max(1));
    let anchor = if row == context_menu::WORKING_TREE_ROW {
        rows_above_graph(state)
    } else {
        let history = state.history.as_ref()?;
        if row < history.top {
            return None;
        }
        let mut offset = row - history.top;
        if crate::view::uncommitted_shown(state) {
            offset += 1;
        }
        rows_above_graph(state).saturating_add(u16::try_from(offset).unwrap_or(u16::MAX))
    };
    let below = anchor.saturating_add(1);
    let y = if below.saturating_add(height) <= graph.y + graph.height {
        below
    } else {
        (graph.y + graph.height).saturating_sub(height)
    };
    Some(Bounds {
        x: graph.x,
        y,
        width,
        height,
    })
}
