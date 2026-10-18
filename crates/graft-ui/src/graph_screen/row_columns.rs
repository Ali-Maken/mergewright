//! Where the row columns stand — marker, graph, subject, author, date, SHA
//! — and how a box snaps onto them; split out of `layout` for size
//! (ease-pass/08s M6).

use ratatui::layout::Rect;

use super::{
    AUTHOR_BREAKPOINT, AUTHOR_WIDTH, GUTTER, MARKER_WIDTH, RIGHT_MARGIN, SHA_WIDTH, SUBJECT_GAP,
    WHEN_WIDTH,
};

/// Where each trailing column sits and how wide it is, once the graph and
/// the subject have taken their share.
///
/// Computed right to left from `area.right()`, each column clamped by
/// `min`/`saturating_sub` against what is left: at any width, down to zero,
/// a column simply stops taking space rather than the arithmetic
/// underflowing. The author chip is the one column that can drop entirely,
/// below [`AUTHOR_BREAKPOINT`] (DESIGN.md §1.2/§1.3); the rest only narrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    /// Where the Mistake Radar's marker slot starts.
    pub marker_x: u16,
    /// Its width; always reserved, whether or not a marker draws.
    pub marker_width: u16,
    /// Where the subject column starts — and so where the row's badges do.
    pub subject_x: u16,
    /// Its width, badges included.
    pub subject_width: u16,
    /// Where the author chip starts.
    pub author_x: u16,
    /// Its width; `0` below [`AUTHOR_BREAKPOINT`], the one column that drops.
    pub author_width: u16,
    /// Where the relative-date column starts.
    pub when_x: u16,
    /// Its width.
    pub when_width: u16,
    /// Where the short-SHA column starts.
    pub sha_x: u16,
    /// Its width.
    pub sha_width: u16,
}

impl Columns {
    // The slot is always reserved, `marker_column` or not: it already
    // carries the pinned-compare-base accent (`GraphEntry::compare_marked`,
    // m2-details-and-diffs/11), so collapsing it to zero width pre-finding
    // would move every later column out from under that feature. Stability
    // ("never shifts on scroll", m2-details-and-diffs.md) is met either way
    // — the width never changes at all; `marker_column` instead gates which
    // *glyph* the slot may draw (`GraphScreen::draw_row`).
    pub(crate) fn compute(area: Rect, graph_width: u16) -> Self {
        let marker_x = area.x + graph_width.min(area.width);
        let marker_width = MARKER_WIDTH.min(area.width.saturating_sub(graph_width));
        let content_x = marker_x + marker_width;

        // Right to left: the rightmost column claims space first, so a
        // narrow terminal drops the leftmost of these (the author chip)
        // before it ever eats into the SHA or the date.
        let mut right = area.right();
        let sha_width = SHA_WIDTH.min(right.saturating_sub(content_x));
        right -= sha_width;
        let sha_x = right;

        let when_width = WHEN_WIDTH.min(right.saturating_sub(content_x));
        right -= when_width;
        let when_x = right;

        // The breakpoint is the row's whole width (DESIGN.md §1.2/§1.3);
        // `area` is the body, inside the gutter and the margin.
        let show_author = area.width + GUTTER + RIGHT_MARGIN >= AUTHOR_BREAKPOINT;
        let author_width = if show_author {
            AUTHOR_WIDTH.min(right.saturating_sub(content_x))
        } else {
            0
        };
        right -= author_width;
        let author_x = right;

        let gap_width = if sha_width + when_width + author_width > 0 {
            SUBJECT_GAP.min(right.saturating_sub(content_x))
        } else {
            0
        };
        right -= gap_width;

        Columns {
            marker_x,
            marker_width,
            subject_x: content_x,
            subject_width: right.saturating_sub(content_x),
            author_x,
            author_width,
            when_x,
            when_width,
            sha_x,
            sha_width,
        }
    }

    /// Every x where one column ends and the next begins, ascending and
    /// deduped, with `area`'s own edges at each end.
    ///
    /// The trailing columns are laid out contiguously right to left, so one
    /// column's end is the next one's start and naming the starts is enough;
    /// the subject's end is named separately because [`SUBJECT_GAP`] sits
    /// between it and the author chip, and that gap is a region of its own
    /// that an overlay may legitimately stop inside. A column that took no
    /// width (the author chip below [`AUTHOR_BREAKPOINT`]) contributes a
    /// boundary equal to its neighbour's and the dedup drops it.
    pub(super) fn boundaries(&self, area: Rect) -> Vec<u16> {
        let mut xs = vec![
            area.x,
            self.marker_x,
            self.subject_x,
            self.subject_x.saturating_add(self.subject_width),
            self.author_x,
            self.when_x,
            self.sha_x,
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
