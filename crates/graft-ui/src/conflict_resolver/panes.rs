//! The three-pane arrangement (ADR-0043 §2): yours | result | theirs, at
//! [`THREE_PANE_MIN`](super::THREE_PANE_MIN) columns and up.
//!
//! **Rows line up by git's own regions**, so no 3-way diff is needed. A
//! plain region is drawn on the same rows in all three panes. A block is a
//! rule row across all three, then as many rows as its longest pane needs;
//! shorter panes are padded with unnumbered [`Cell::Pad`]s. In the result
//! pane an open block is a [`Cell::Placeholder`] (`? open · o t b`, or its
//! BASE lines when `B` is on); a take fills it in with the chosen side.
//!
//! Each pane is numbered by its own projection: yours by the ours
//! projection, theirs by the theirs projection, the result by the result
//! projection (ADR-0043 §5).

use ratatui::layout::Rect;

use super::{BlockState, BlockView, DocView, RegionView, ScreenRow, TakeView};

mod draw;

/// Which line a [`Cell::Line`] draws. Indices, not text: the drawer looks
/// the text up in the same `DocView` the rows were laid out from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LineRef {
    /// Line `i` of `doc.regions[region]` — a plain region, or a whole-file
    /// region's file.
    Plain {
        /// Index into `DocView::regions`.
        region: usize,
        /// Index into that region's lines.
        i: usize,
    },
    /// Line `i` of block `block`'s ours side.
    Ours {
        /// The block's index.
        block: usize,
        /// Index into `BlockView::ours`.
        i: usize,
    },
    /// Line `i` of block `block`'s theirs side.
    Theirs {
        /// The block's index.
        block: usize,
        /// Index into `BlockView::theirs`.
        i: usize,
    },
    /// Line `i` of what a taken block kept: ours, theirs, or ours then
    /// theirs for `both`.
    Taken {
        /// The block's index.
        block: usize,
        /// Index into the kept lines.
        i: usize,
    },
    /// Line `i` of block `block`'s base.
    Base {
        /// The block's index.
        block: usize,
        /// Index into the base's lines.
        i: usize,
    },
}

/// One pane's share of a [`ScreenRow::Aligned`] row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cell {
    /// A line, with its number in this pane's projection (`None`: BASE,
    /// which no projection numbers).
    Line {
        /// The number drawn in this pane's gutter.
        line_no: Option<usize>,
        /// The line.
        text: LineRef,
    },
    /// A blank, unnumbered row: this pane has fewer lines here than another.
    Pad,
    /// Words standing in for lines: `? open · o t b` in the result of an
    /// open block (`base` over its BASE lines when `B` is on), `? keep or
    /// delete · o t` in a whole-file result, `(deleted on …)` in a side
    /// the file is not on.
    Placeholder,
    /// A block's rule, worded per pane (`── o take`, `── 1 of 2 · open`).
    Rule,
}

/// `doc`'s area split into its three panes, `[yours, result, theirs]`,
/// with a one-column `│` between each two. The sides are equal; what does
/// not divide by three goes to the result. Each pane's width includes its
/// own gutter (ADR-0043 §2, amended).
#[must_use]
pub fn three_panes(doc: Rect) -> [Rect; 3] {
    let room = doc.width.saturating_sub(2);
    let side = room / 3;
    let middle = room - 2 * side;
    let at = |x: u16| x.min(doc.right());
    let yours = Rect { width: side, ..doc };
    let result = Rect {
        x: at(yours.right().saturating_add(1)),
        width: middle,
        ..doc
    };
    let theirs = Rect {
        x: at(result.right().saturating_add(1)),
        width: side,
        ..doc
    };
    [yours, result, theirs]
}

/// The three-pane rows for `doc`, and each block's rule-row index.
pub(super) fn pane_rows(doc: &DocView) -> (Vec<ScreenRow>, Vec<usize>) {
    let mut rows = vec![ScreenRow::PaneTitles];
    let mut block_row = Vec::new();
    for (region, view) in doc.regions.iter().enumerate() {
        match view {
            RegionView::Plain { numbers, lines } => {
                for i in 0..lines.len() {
                    let line = |first: usize| Cell::Line {
                        line_no: Some(first + i),
                        text: LineRef::Plain { region, i },
                    };
                    rows.push(aligned(
                        None,
                        [
                            line(numbers.ours),
                            line(numbers.result),
                            line(numbers.theirs),
                        ],
                    ));
                }
            }
            RegionView::WholeFile { .. } => whole_file_rows(view, region, &mut rows),
            RegionView::Block(block) => {
                block_row.push(rows.len());
                rows.push(aligned(Some(block.index), [Cell::Rule; 3]));
                block_rows(doc, block, &mut rows);
            }
        }
    }
    (rows, block_row)
}

fn aligned(block: Option<usize>, [ours, result, theirs]: [Cell; 3]) -> ScreenRow {
    ScreenRow::Aligned {
        block,
        ours,
        result,
        theirs,
    }
}

/// Row `k` of a side with `len` lines numbered from `first`: its line, or
/// a pad past its end.
fn side_cell(k: usize, len: usize, first: usize, text: LineRef) -> Cell {
    if k < len {
        Cell::Line {
            line_no: Some(first + k),
            text,
        }
    } else {
        Cell::Pad
    }
}

/// A block's rows under its rule: `max` of its panes' lines.
fn block_rows(doc: &DocView, block: &BlockView, rows: &mut Vec<ScreenRow>) {
    let b = block.index;
    let (ours, theirs) = (block.ours.len(), block.theirs.len());
    let base = match (&block.state, &block.base) {
        (BlockState::Open, Some((_, lines))) if doc.base_shown => Some(lines.len()),
        _ => None,
    };
    let result_len = match block.state {
        BlockState::Open => base.map_or(1, |lines| lines + 1),
        BlockState::Took(take) => taken_len(block, take),
    };
    for k in 0..ours.max(theirs).max(result_len) {
        let result = match block.state {
            BlockState::Open if k == 0 => Cell::Placeholder,
            BlockState::Open if k < result_len => Cell::Line {
                line_no: None,
                text: LineRef::Base { block: b, i: k - 1 },
            },
            BlockState::Open => Cell::Pad,
            BlockState::Took(_) => side_cell(
                k,
                result_len,
                block.numbers.result,
                LineRef::Taken { block: b, i: k },
            ),
        };
        rows.push(aligned(
            Some(b),
            [
                side_cell(
                    k,
                    ours,
                    block.numbers.ours,
                    LineRef::Ours { block: b, i: k },
                ),
                result,
                side_cell(
                    k,
                    theirs,
                    block.numbers.theirs,
                    LineRef::Theirs { block: b, i: k },
                ),
            ],
        ));
    }
}

/// Lines a taken block keeps in the result.
pub(super) fn taken_len(block: &BlockView, take: TakeView) -> usize {
    match take {
        TakeView::Ours => block.ours.len(),
        TakeView::Theirs => block.theirs.len(),
        TakeView::Both => block.ours.len() + block.theirs.len(),
    }
}

/// A whole-file shape: its card row, then the file in each side it is on
/// (`(deleted on …)` in one it is not), `? keep or delete` in the result,
/// then the card's note.
fn whole_file_rows(view: &RegionView, region: usize, rows: &mut Vec<ScreenRow>) {
    let RegionView::WholeFile {
        how,
        keys,
        lines,
        note,
        ours_absent,
        theirs_absent,
    } = view
    else {
        return;
    };
    rows.push(ScreenRow::WholeFile {
        how: how.clone(),
        keys: keys.clone(),
    });
    let side = |absent: bool, k: usize| match (absent, k) {
        (true, 0) => Cell::Placeholder,
        (true, _) => Cell::Pad,
        (false, k) => side_cell(k, lines.len(), 1, LineRef::Plain { region, i: k }),
    };
    for k in 0..lines.len().max(1) {
        let result = if k == 0 { Cell::Placeholder } else { Cell::Pad };
        rows.push(aligned(
            None,
            [
                side(ours_absent.is_some(), k),
                result,
                side(theirs_absent.is_some(), k),
            ],
        ));
    }
    if let Some(text) = note {
        rows.push(ScreenRow::Note { text: text.clone() });
    }
}

/// Digits of the widest number each pane draws, `[yours, result, theirs]`
/// — each pane's number column is as wide as its own widest number.
pub(super) fn pane_number_widths(doc: &DocView) -> [usize; 3] {
    let last = |first: usize, len: usize| (first + len).saturating_sub(1);
    let mut widest = [0usize; 3];
    let mut see = |pane: usize, n: usize| widest[pane] = widest[pane].max(n);
    for region in &doc.regions {
        match region {
            RegionView::Plain { numbers, lines } => {
                see(0, last(numbers.ours, lines.len()));
                see(1, last(numbers.result, lines.len()));
                see(2, last(numbers.theirs, lines.len()));
            }
            RegionView::WholeFile { lines, .. } => {
                see(0, lines.len());
                see(2, lines.len());
            }
            RegionView::Block(block) => {
                see(0, last(block.numbers.ours, block.ours.len()));
                see(2, last(block.numbers.theirs, block.theirs.len()));
                if let BlockState::Took(take) = block.state {
                    see(1, last(block.numbers.result, taken_len(block, take)));
                }
            }
        }
    }
    widest.map(|n| n.max(1).to_string().len())
}
