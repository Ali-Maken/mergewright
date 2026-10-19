//! Where the row columns stand — marker, branch, message, who, age — and
//! how a box snaps onto them; split out of `layout` for size
//! (ease-pass/08s M6). The columns are the main mock's (ease-pass G5, G8):
//! `graph | branch | message | who | age`, no SHA and no author chip.

use ratatui::layout::Rect;

use super::{
    AGE_WIDTH, BRANCH_MIDDLE, BRANCH_NARROW, BRANCH_WIDE, COLUMN_GAP, MARKER_WIDTH, WHO_WIDTH,
};

/// Where each column sits and how wide it is, once the graph has taken its
/// share.
///
/// Computed outward from the marker slot and inward from `area.right()`,
/// each column clamped against what is left: at any width, down to zero, a
/// column simply stops taking space rather than the arithmetic underflowing.
/// The message is the elastic one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    /// Where the Mistake Radar's marker slot starts.
    pub marker_x: u16,
    /// Its width; always reserved, whether or not a marker draws.
    pub marker_width: u16,
    /// Where the branch column starts.
    pub branch_x: u16,
    /// Its width: [`branch_width_for`] the screen's width, never more than
    /// half of what the graph and the trailing columns left.
    pub branch_width: u16,
    /// Where the message column starts; the fold tail, the row tag and the
    /// labels follow the message inside it.
    pub subject_x: u16,
    /// Its width, fold tail, tag and labels included.
    pub subject_width: u16,
    /// Where the who column starts.
    pub who_x: u16,
    /// Its width.
    pub who_width: u16,
    /// Where the age column starts.
    pub when_x: u16,
    /// Its width.
    pub when_width: u16,
}

/// The branch column's width for a row `full` cells wide, gutter and margin
/// included: 20 from 120, 14 from 100, 12 below (ease-pass New UI).
#[must_use]
pub fn branch_width_for(full: u16) -> u16 {
    if full >= 120 {
        BRANCH_WIDE
    } else if full >= 100 {
        BRANCH_MIDDLE
    } else {
        BRANCH_NARROW
    }
}

impl Columns {
    // The slot is always reserved, `marker_column` or not: it already
    // carries the pinned-compare-base accent (`GraphEntry::compare_marked`,
    // m2-details-and-diffs/11), so collapsing it to zero width pre-finding
    // would move every later column out from under that feature. Stability
    // ("never shifts on scroll", m2-details-and-diffs.md) is met either way
    // — the width never changes at all; `marker_column` instead gates which
    // *glyph* the slot may draw (`GraphScreen::draw_row`).
    ///
    /// `screen` is the whole screen's width, which picks the branch column's
    /// (ease-pass New UI: 20 from 120 *with* the branch pane beside the rows,
    /// though the rows themselves are then 82 wide).
    pub(crate) fn compute(area: Rect, graph_width: u16, screen: u16) -> Self {
        let marker_x = area.x + graph_width.min(area.width);
        let marker_width = MARKER_WIDTH.min(area.width.saturating_sub(graph_width));
        let content_x = marker_x + marker_width;

        // Right to left: age, a gap, who, a gap — the message ends there.
        let mut right = area.right();
        let when_width = AGE_WIDTH.min(right.saturating_sub(content_x));
        right -= when_width;
        let when_x = right;
        right -= COLUMN_GAP.min(right.saturating_sub(content_x));
        let who_width = WHO_WIDTH.min(right.saturating_sub(content_x));
        right -= who_width;
        let who_x = right;
        right -= COLUMN_GAP.min(right.saturating_sub(content_x));

        // The branch column takes its width from the left, never more than
        // half of what is left: the message says what the row is.
        let left = right.saturating_sub(content_x);
        let branch_width = branch_width_for(screen).min(left / 2);
        let branch_gap = if branch_width > 0 { COLUMN_GAP } else { 0 };
        let subject_x = content_x + branch_width + branch_gap;

        Columns {
            marker_x,
            marker_width,
            branch_x: content_x,
            branch_width,
            subject_x,
            subject_width: right.saturating_sub(subject_x),
            who_x,
            who_width,
            when_x,
            when_width,
        }
    }

    /// Every x where one column ends and the next begins, ascending and
    /// deduped, with `area`'s own edges at each end.
    ///
    /// Each column's **start** is named, never its end: every start has a
    /// blank before it (the gap, or the marker slot's spare cell), which
    /// [`snap_to_columns`] needs on both sides of a box, while a column's
    /// last cell may hold text — who is right-aligned, labels run to the
    /// message column's edge. A column that took no width contributes a
    /// boundary equal to its neighbour's and the dedup drops it.
    pub(super) fn boundaries(&self, area: Rect) -> Vec<u16> {
        let mut xs = vec![
            area.x,
            self.marker_x,
            self.branch_x,
            self.subject_x,
            self.who_x,
            self.when_x,
            area.right(),
        ];
        xs.retain(|x| *x >= area.x && *x <= area.right());
        xs.sort_unstable();
        xs.dedup();
        xs
    }
}

/// `rect` grown outward until both of its vertical edges sit on a boundary
/// in `boundaries`, so every column it touches is covered whole and none of
/// them is split (ADR-0040).
///
/// Outward, never inward: reaching a boundary by shrinking would cut the
/// box's own content, which trades a legible fragment beside the box for an
/// illegible one inside it. `None` when no such rectangle fits inside
/// `bounds` — the caller then has a box that cannot cover, and ADR-0040
/// says such a box clears the band it stands on instead of splitting it.
///
/// Pure geometry: it knows nothing about the graph and nothing about the
/// box. Both halves arrive as numbers, which is what keeps the rule at the
/// layout site rather than inside either widget.
///
/// **And it stops one cell short of the column it stops at.** The right
/// edge rests on the *gutter* that precedes the next column's text, not on
/// the first cell of that text, so there is a blank column between the
/// border and whatever is beside it (ADR-0040's 2026-09-21 amendment). A
/// box flush against the column it ends at reads as one token — at 80 the
/// confirm box drew `╮60af0eb`, which is the `8443─────` fault of the same
/// ADR turned through ninety degrees. The gutter is not needed at
/// `bounds.right()`, where there is nothing beside the box at all.
///
/// The left edge needs no such step: every row column's own text is laid
/// out after a leading blank, so the cell before a boundary is already a
/// gutter belonging to the column ahead of it.
///
/// Stepping back costs a cell, so a boundary whose gutter no longer covers
/// `rect` is not a snap position at all and the search carries on outward —
/// which is how a box can run out of positions and clear where it used to
/// snap. Shrinking the box to fit one is still forbidden (clause 2).
///
/// An empty `boundaries` returns `rect` unchanged — a screen with no row
/// columns has nothing to split.
#[must_use]
pub fn snap_to_columns(rect: Rect, boundaries: &[u16], bounds: Rect) -> Option<Rect> {
    if rect.is_empty() || boundaries.is_empty() {
        return Some(rect);
    }
    let right = rect.x.saturating_add(rect.width);
    let left = boundaries.iter().copied().filter(|x| *x <= rect.x).max()?;
    let snapped = boundaries
        .iter()
        .copied()
        .filter_map(|x| {
            if x >= bounds.right() {
                Some(x)
            } else {
                x.checked_sub(1)
            }
        })
        .filter(|x| *x >= right)
        .min()?;
    if left < bounds.x || snapped > bounds.right() {
        return None;
    }
    Some(Rect {
        x: left,
        width: snapped.saturating_sub(left),
        ..rect
    })
}
