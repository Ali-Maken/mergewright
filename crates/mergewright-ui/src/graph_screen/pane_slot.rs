//! The branch pane's slot beside the rows (ease-pass 09, G12): split out of
//! `layout` for size. The carve stays `layout`'s; this only takes the
//! slot off the body's right.

use ratatui::layout::Rect;

use super::GraphView;

/// The body less the branch pane's slot, and that slot: the pane's width plus
/// its `│` rule and a space, at the screen's right edge, on the body's rows.
/// The rule stands where the body's right margin was.
pub(super) fn beside_pane(view: &GraphView<'_>, area: Rect, body: Rect) -> (Rect, Rect) {
    let none = Rect { width: 0, ..body };
    let Some(width) = view
        .pane
        .as_ref()
        .and_then(|_| crate::branch_pane::pane_width(area.width))
    else {
        return (
            body,
            Rect {
                x: body.right(),
                ..none
            },
        );
    };
    let slot = width + 2;
    let x = area.right().saturating_sub(slot).max(body.x);
    let pane = Rect {
        x,
        width: area.right() - x,
        ..body
    };
    (
        Rect {
            width: x - body.x,
            ..body
        },
        pane,
    )
}
