//! The overview strip (`O`, DESIGN.md §5.3): one column, the whole history
//! bucketed onto the screen's height, so a merge or a finding far off
//! screen is still a glyph away.
//!
//! Pure like every other widget here: [`StripMarks`] is built by
//! `crate::graft::view::strip::strip_marks` (a later crate) from what the
//! app already has on hand — no marks for rows off screen and not yet
//! walked (DESIGN §5.3 "Cost": honestly incomplete while streaming).

use graft_core::mistake_radar::Severity;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use crate::paint::foreground;
use crate::theme::{Theme, Token};

/// One thing worth marking about a row: assembled by `strip_marks`, resolved
/// to a glyph by [`cell_for`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripMark {
    /// A merge commit: `row.commit.parents.len() >= 2`.
    Merge,
    /// A Mistake Radar finding at this row, at its severity.
    Radar(Severity),
    /// A search hit at this row.
    SearchHit,
    /// `HEAD`'s row.
    Head,
}

/// What [`strip_marks`] (the `graft` crate) hands the widget: every mark it
/// found on rows currently on hand, plus how the app understands its own
/// size.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StripMarks {
    /// Rows laid out as far as the app has heard (`History::rows`):
    /// honestly incomplete for a repo still streaming.
    pub rows_total: usize,
    /// The rows on screen right now.
    pub viewport: std::ops::Range<usize>,
    /// `(row, mark)`, unsorted allowed — [`cell_for`] resolves ties itself.
    pub marks: Vec<(usize, StripMark)>,
}

/// What one strip cell draws, already resolved to a priority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StripCell {
    /// Past `rows_total`'s last bucket: nothing to draw.
    Blank,
    /// A bucket with rows but no mark and no viewport overlap: the rail.
    Rail,
    /// Rows on screen, nothing higher-priority in the bucket.
    Viewport,
    /// A merge commit in the bucket.
    Merge,
    /// `HEAD`'s row is in the bucket.
    Head,
    /// A search hit is in the bucket.
    SearchHit,
    /// A Mistake Radar finding at this severity is in the bucket.
    Radar(Severity),
}

/// The representative bucket size for `height` cells over `rows_total`
/// rows — `ceil(rows_total / height)`, at least one, per the design note.
/// The actual per-cell boundaries ([`bucket_range`]) spread `rows_total`
/// evenly instead of stepping by this fixed size, so a division that
/// doesn't come out even doesn't leave the tail of the history uncovered
/// (the earlier cells absorb one extra row each rather than the strip
/// running out before `rows_total`).
#[cfg(test)]
fn bucket_size(rows_total: usize, height: u16) -> usize {
    let height = usize::from(height).max(1);
    rows_total.div_ceil(height).max(1)
}

/// `cell`'s row boundary, `height` cells over `rows_total` rows: the rows
/// split as evenly as `rows_total` allows (`ceil(cell * rows_total /
/// height)`), so cells fill left to right and the last cell always ends
/// exactly at `rows_total`.
fn bucket_start(rows_total: usize, height: u16, cell: u16) -> usize {
    let height = usize::from(height).max(1);
    (usize::from(cell) * rows_total).div_ceil(height)
}

/// The row range `cell` covers, `height` cells over `marks.rows_total` rows.
fn bucket_range(marks: &StripMarks, height: u16, cell: u16) -> std::ops::Range<usize> {
    let start = bucket_start(marks.rows_total, height, cell);
    let end = bucket_start(marks.rows_total, height, cell + 1);
    start..end.max(start)
}

/// The highest-priority mark in `cell`'s bucket, `height` cells over
/// `marks.rows_total` rows: `⚠` > `ⓘ` > `◆` search hit > `●` HEAD > `◉`
/// merge > `█` viewport-only > `┆` rail. Blank once `cell`'s bucket starts
/// past `rows_total`.
#[must_use]
pub fn cell_for(marks: &StripMarks, height: u16, cell: u16) -> StripCell {
    let range = bucket_range(marks, height, cell);
    if marks.rows_total == 0 || range.start >= marks.rows_total {
        return StripCell::Blank;
    }

    let mut warning = false;
    let mut info = false;
    let mut search_hit = false;
    let mut head = false;
    let mut merge = false;

    for (row, mark) in &marks.marks {
        if !range.contains(row) {
            continue;
        }
        match mark {
            StripMark::Radar(Severity::Warning) => warning = true,
            StripMark::Radar(Severity::Info) => info = true,
            StripMark::SearchHit => search_hit = true,
            StripMark::Head => head = true,
            StripMark::Merge => merge = true,
        }
    }

    if warning {
        return StripCell::Radar(Severity::Warning);
    }
    if info {
        return StripCell::Radar(Severity::Info);
    }
    if search_hit {
        return StripCell::SearchHit;
    }
    if head {
        return StripCell::Head;
    }
    if merge {
        return StripCell::Merge;
    }
    let viewport_overlap = range.start < marks.viewport.end && marks.viewport.start < range.end;
    if viewport_overlap {
        return StripCell::Viewport;
    }
    StripCell::Rail
}

/// The first row of `cell`'s bucket, `height` cells over `marks.rows_total`
/// rows — where a click on the strip sends the selection.
#[must_use]
pub fn row_for_cell(marks: &StripMarks, height: u16, cell: u16) -> usize {
    bucket_range(marks, height, cell).start
}

/// The one-column widget: `area.height` cells, each [`cell_for`]'s glyph.
pub struct OverviewStrip<'a> {
    /// The marks to draw.
    pub marks: &'a StripMarks,
    /// Where the cell colours come from.
    pub theme: &'a Theme,
    /// The ASCII tier's glyphs in place of Unicode's.
    pub ascii: bool,
}

impl StripCell {
    /// This cell's glyph and colour token, Unicode or ASCII.
    fn glyph(self, ascii: bool) -> (char, Token) {
        match self {
            StripCell::Blank => (' ', Token::TextMuted),
            StripCell::Rail => (if ascii { ':' } else { '┆' }, Token::TextMuted),
            StripCell::Viewport => (if ascii { '#' } else { '█' }, Token::Accent),
            StripCell::Merge => (if ascii { 'o' } else { '◉' }, Token::TextMuted),
            StripCell::Head => (if ascii { '*' } else { '●' }, Token::Accent),
            StripCell::SearchHit => (if ascii { '+' } else { '◆' }, Token::Accent),
            StripCell::Radar(Severity::Info) => (if ascii { 'i' } else { 'ⓘ' }, Token::TextMuted),
            StripCell::Radar(Severity::Warning) => (if ascii { '!' } else { '⚠' }, Token::Warning),
        }
    }
}

impl Widget for OverviewStrip<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for row in 0..area.height {
            let cell = cell_for(self.marks, area.height, row);
            let (glyph, token) = cell.glyph(self.ascii);
            let style = foreground(self.theme, token);
            buf.set_string(area.x, area.y + row, glyph.to_string(), style);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn marks(
        rows_total: usize,
        viewport: std::ops::Range<usize>,
        marks: Vec<(usize, StripMark)>,
    ) -> StripMarks {
        StripMarks {
            rows_total,
            viewport,
            marks,
        }
    }

    #[test]
    fn a_bucket_with_a_merge_a_search_hit_and_a_warning_draws_the_warning() {
        let m = marks(
            24,
            0..24,
            vec![
                (0, StripMark::Merge),
                (0, StripMark::SearchHit),
                (0, StripMark::Radar(Severity::Warning)),
            ],
        );
        assert_eq!(cell_for(&m, 24, 0), StripCell::Radar(Severity::Warning));
    }

    #[test]
    fn a_bucket_with_only_viewport_overlap_draws_viewport() {
        let m = marks(100, 0..24, Vec::new());
        assert_eq!(cell_for(&m, 24, 0), StripCell::Viewport);
    }

    #[test]
    fn a_bucket_with_no_marks_and_no_viewport_draws_rail() {
        let m = marks(100, 0..24, Vec::new());
        assert_eq!(cell_for(&m, 24, 23), StripCell::Rail);
    }

    #[test]
    fn past_rows_total_is_blank() {
        let m = marks(10, 0..10, Vec::new());
        assert_eq!(cell_for(&m, 24, 23), StripCell::Blank);
    }

    #[test]
    fn bucket_math_is_five_rows_per_cell_at_100_over_24() {
        let m = marks(100, 0..24, Vec::new());
        assert_eq!(bucket_size(100, 24), 5);
        assert_eq!(row_for_cell(&m, 24, 23), 96);
        assert_eq!(bucket_range(&m, 24, 23), 96..100);
    }

    #[test]
    fn head_outranks_merge_but_not_search_hit() {
        let m = marks(24, 0..24, vec![(0, StripMark::Head), (0, StripMark::Merge)]);
        assert_eq!(cell_for(&m, 24, 0), StripCell::Head);

        let m = marks(
            24,
            0..24,
            vec![(0, StripMark::Head), (0, StripMark::SearchHit)],
        );
        assert_eq!(cell_for(&m, 24, 0), StripCell::SearchHit);
    }

    #[test]
    fn info_outranks_search_hit_and_head_and_merge() {
        let m = marks(
            24,
            0..24,
            vec![
                (0, StripMark::Radar(Severity::Info)),
                (0, StripMark::SearchHit),
                (0, StripMark::Head),
                (0, StripMark::Merge),
            ],
        );
        assert_eq!(cell_for(&m, 24, 0), StripCell::Radar(Severity::Info));
    }
}
