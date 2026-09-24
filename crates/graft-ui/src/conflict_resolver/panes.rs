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

use graft_term::SanitizedText;
use ratatui::layout::Rect;

use super::{BlockState, BlockView, DocView, RegionView, ScreenRow, TakeView};

mod band;
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
            RegionView::WholeFile {
                how,
                keys,
                lines,
                note,
                ours_absent,
                theirs_absent,
            } => {
                let file = WholeFileRows {
                    how,
                    keys,
                    lines: lines.len(),
                    note: note.as_ref(),
                    ours_absent: ours_absent.is_some(),
                    theirs_absent: theirs_absent.is_some(),
                };
                whole_file_rows(&file, region, &mut rows);
            }
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

/// What [`whole_file_rows`] needs of a [`RegionView::WholeFile`], taken
/// apart at the call site so no other variant can reach it.
struct WholeFileRows<'a> {
    how: &'a SanitizedText,
    keys: &'a SanitizedText,
    /// How many lines of the file the app read.
    lines: usize,
    note: Option<&'a SanitizedText>,
    /// The file is not on yours / theirs.
    ours_absent: bool,
    theirs_absent: bool,
}

/// A whole-file shape: its card row, then the file in each side it is on
/// (`(deleted on …)` in one it is not), `? keep or delete` in the result,
/// then the card's note.
fn whole_file_rows(file: &WholeFileRows, region: usize, rows: &mut Vec<ScreenRow>) {
    let WholeFileRows {
        how,
        keys,
        lines,
        note,
        ours_absent,
        theirs_absent,
    } = *file;
    rows.push(ScreenRow::WholeFile {
        how: how.clone(),
        keys: keys.clone(),
    });
    let side = |absent: bool, k: usize| match (absent, k) {
        (true, 0) => Cell::Placeholder,
        (true, _) => Cell::Pad,
        (false, k) => side_cell(k, lines, 1, LineRef::Plain { region, i: k }),
    };
    for k in 0..lines.max(1) {
        let result = if k == 0 { Cell::Placeholder } else { Cell::Pad };
        rows.push(aligned(
            None,
            [side(ours_absent, k), result, side(theirs_absent, k)],
        ));
    }
    if let Some(text) = note {
        rows.push(ScreenRow::Note { text: text.clone() });
    }
}

/// Which of the three panes a cell is in.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Pane {
    Yours,
    Result,
    Theirs,
}

/// Digits of each pane's widest number.
#[derive(Clone, Copy, Default)]
pub(super) struct PaneWidths {
    pub(super) yours: usize,
    pub(super) result: usize,
    pub(super) theirs: usize,
}

impl PaneWidths {
    /// `pane`'s width.
    pub(super) fn of(self, pane: Pane) -> usize {
        match pane {
            Pane::Yours => self.yours,
            Pane::Result => self.result,
            Pane::Theirs => self.theirs,
        }
    }

    fn see(&mut self, pane: Pane, n: usize) {
        let slot = match pane {
            Pane::Yours => &mut self.yours,
            Pane::Result => &mut self.result,
            Pane::Theirs => &mut self.theirs,
        };
        *slot = (*slot).max(n);
    }
}

/// Digits of the widest number each pane draws — each pane's number column
/// is as wide as its own widest number.
pub(super) fn pane_number_widths(doc: &DocView) -> PaneWidths {
    let last = |first: usize, len: usize| (first + len).saturating_sub(1);
    let mut widest = PaneWidths::default();
    for region in &doc.regions {
        match region {
            RegionView::Plain { numbers, lines } => {
                widest.see(Pane::Yours, last(numbers.ours, lines.len()));
                widest.see(Pane::Result, last(numbers.result, lines.len()));
                widest.see(Pane::Theirs, last(numbers.theirs, lines.len()));
            }
            RegionView::WholeFile { lines, .. } => {
                widest.see(Pane::Yours, lines.len());
                widest.see(Pane::Theirs, lines.len());
            }
            RegionView::Block(block) => {
                widest.see(Pane::Yours, last(block.numbers.ours, block.ours.len()));
                widest.see(Pane::Theirs, last(block.numbers.theirs, block.theirs.len()));
                if let BlockState::Took(take) = block.state {
                    widest.see(
                        Pane::Result,
                        last(block.numbers.result, taken_len(block, take)),
                    );
                }
            }
        }
    }
    let digits = |n: usize| n.max(1).to_string().len();
    PaneWidths {
        yours: digits(widest.yours),
        result: digits(widest.result),
        theirs: digits(widest.theirs),
    }
}
