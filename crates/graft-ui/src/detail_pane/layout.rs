//! The pane's own layout math: where it sits (breakpoint), and how its area
//! splits into header/files/body. Pure geometry, no drawing — shared by the
//! frame that reserves space for the pane and by mouse hit-testing
//! (`update/mod.rs`), which is why it stays independent of the drawing
//! modules in this directory.

use ratatui::layout::Rect;

use crate::detail::DetailFocus;

/// Where the detail pane sits, per DESIGN.md §1.1-1.3.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailLayout {
    /// 160+ columns: a right column.
    Right {
        /// Column width, 42% of the screen width, min 48.
        columns: u16,
    },
    /// 100..160 columns: a strip below the graph.
    Below {
        /// Strip height, 40% of the screen height, min 8.
        rows: u16,
    },
    /// Under 100 columns: a `[Graph] Detail` tab strip.
    Tab {
        /// Follows `focus != DetailFocus::Graph` (no separate "which tab"
        /// state).
        shown: bool,
    },
}

impl DetailLayout {
    const RIGHT_BREAKPOINT: u16 = 160;
    const BELOW_BREAKPOINT: u16 = 100;
    const RIGHT_MIN: u16 = 48;
    const BELOW_MIN: u16 = 8;

    /// The layout for a screen of `width` by `height` cells. `override_cells`
    /// (a divider drag or `<`/`>`, `detail-keys-and-focus`) replaces the
    /// percentage figure, still clamped to the breakpoint's minimum.
    #[must_use]
    pub fn compute(
        width: u16,
        height: u16,
        focus: DetailFocus,
        override_cells: Option<u16>,
    ) -> DetailLayout {
        if width >= Self::RIGHT_BREAKPOINT {
            let pct = percent(width, 42);
            DetailLayout::Right {
                columns: override_cells.unwrap_or(pct).max(Self::RIGHT_MIN),
            }
        } else if width >= Self::BELOW_BREAKPOINT {
            let pct = percent(height, 40);
            DetailLayout::Below {
                rows: override_cells.unwrap_or(pct).max(Self::BELOW_MIN),
            }
        } else {
            DetailLayout::Tab {
                shown: focus != DetailFocus::Graph,
            }
        }
    }
}

/// `cells * pct / 100`, saturating rather than overflowing for any `u16`.
pub(crate) fn percent(cells: u16, pct: u32) -> u16 {
    u16::try_from(u32::from(cells) * pct / 100).unwrap_or(u16::MAX)
}

/// The blank column between two side-by-side regions: one, unless one of
/// them has no cells at all, in which case there is no boundary to mark.
///
/// **Always taken out of the left region's width, never the right one's x.**
/// Two regions flush against each other run their text together: at 160
/// columns the graph's short-SHA column ended where the changes pane began,
/// so `a82960b` and `CONFLICTS (2)` read as `a82960bCONFLICTS` — one token,
/// in a capture the user caught by eye (m5/19). Taking the column off the
/// left keeps every boundary *position* where it was, which matters because
/// `graft::update::geometry` mirrors this arithmetic for mouse hit-testing
/// in plain `u16` and cannot call in here: the only cell that changes hands
/// is the gutter itself, which routes to the region it came out of rather
/// than to nothing.
fn gutter(left: u16, right: u16) -> u16 {
    u16::from(left > 0 && right > 0)
}

/// `area` split into the graph rectangle and the pane rectangle for
/// `layout`, either of which is `None` in `Tab` mode (only the shown side
/// gets space). Pure geometry — no drawing — so both the frame that draws
/// from it (`view/frame.rs::split_for_detail`, which also renders the `Tab`
/// strip this doesn't know about) and mouse hit-testing (`update/mod.rs`,
/// divider drag and wheel routing, `detail-resize-and-persistence`) split
/// the pane the same way.
#[must_use]
pub fn split_area(layout: DetailLayout, area: Rect) -> (Option<Rect>, Option<Rect>) {
    match layout {
        DetailLayout::Right { columns } => {
            let columns = columns.min(area.width);
            let left = area.width - columns;
            let graph_area = Rect {
                width: left.saturating_sub(gutter(left, columns)),
                ..area
            };
            let pane_area = Rect {
                x: area.x + left,
                width: columns,
                ..area
            };
            (Some(graph_area), Some(pane_area))
        }
        DetailLayout::Below { rows } => {
            let rows = rows.min(area.height);
            let graph_area = Rect {
                height: area.height - rows,
                ..area
            };
            let pane_area = Rect {
                y: area.y + graph_area.height,
                height: rows,
                ..area
            };
            (Some(graph_area), Some(pane_area))
        }
        DetailLayout::Tab { shown } => {
            let strip = 1.min(area.height);
            let content = Rect {
                y: area.y + strip,
                height: area.height - strip,
                ..area
            };
            if shown {
                (None, Some(content))
            } else {
                (Some(content), None)
            }
        }
    }
}

/// The pane's three regions: header (fixed rows), files (a list) and body
/// (message or diff).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailAreas {
    /// Fixed-height rows at the top: subject, author, parents, contained,
    /// radar (as many as the breakpoint keeps).
    pub header: Rect,
    /// The file list.
    pub files: Rect,
    /// The message or diff.
    pub body: Rect,
}

/// Split `pane_area` into [`DetailAreas`] for `layout`. `files_count` caps
/// how much of the files region the file list actually needs (`Right`/`Tab`
/// only): it never grows past that even when there is room, and never goes
/// negative when there are no files.
#[must_use]
pub fn detail_areas(pane_area: Rect, layout: DetailLayout, files_count: usize) -> DetailAreas {
    let header_rows = match layout {
        DetailLayout::Right { .. } => 5,
        DetailLayout::Below { .. } => 2,
        DetailLayout::Tab { .. } => 3,
    };
    let header = Rect {
        height: header_rows.min(pane_area.height),
        ..pane_area
    };
    let rest = Rect {
        y: pane_area.y.saturating_add(header.height),
        height: pane_area.height.saturating_sub(header.height),
        ..pane_area
    };

    if matches!(layout, DetailLayout::Below { .. }) {
        let files_width = 30.min(rest.width);
        let body_width = rest.width.saturating_sub(files_width);
        let files = Rect {
            width: files_width.saturating_sub(gutter(files_width, body_width)),
            ..rest
        };
        let body = Rect {
            x: rest.x.saturating_add(files_width),
            width: body_width,
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    } else {
        let cap = percent(rest.height, 40);
        let files_height = cap
            .min(u16::try_from(files_count).unwrap_or(u16::MAX))
            .min(rest.height);
        let files = Rect {
            height: files_height,
            ..rest
        };
        let body = Rect {
            y: rest.y.saturating_add(files_height),
            height: rest.height.saturating_sub(files_height),
            ..rest
        };
        DetailAreas {
            header,
            files,
            body,
        }
    }
}
