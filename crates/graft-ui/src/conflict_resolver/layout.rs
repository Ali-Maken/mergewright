//! [`layout`]: a [`DocView`] planned into the [`ScreenRow`]s the widget
//! draws, pure and total, and [`row_at`], the one windowed lookup the app
//! scrolls and hit-tests through.

use graft_term::SanitizedText;

use super::{
    Arrangement, BlockState, BlockView, Cell, DocView, LabelOf, RegionView, SIDE_BY_SIDE_MIN,
    SideKind, TakeView,
};

/// One row of the document, as [`layout`] plans it. Carries indices into
/// `doc`, not text: the widget (and any other reader, e.g. a hit test)
/// looks the text up from the same `doc` it was laid out from, so the two
/// can never disagree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenRow {
    /// A plain line: `{line_no} {text}`, the number right-aligned.
    Plain {
        /// The file's own line number.
        line_no: usize,
        /// The line itself.
        text: SanitizedText,
    },
    /// A block's header/take-keys rule.
    Header {
        /// The block's index.
        block: usize,
    },
    /// The wide-layout labels row (`yours · main │ theirs · feature`).
    Labels {
        /// The block's index.
        block: usize,
    },
    /// A wide-layout row of the two sides, side by side. `None` pads the
    /// shorter side.
    Pair {
        /// The block's index.
        block: usize,
        /// Index into `ours`, if this row has one.
        ours: Option<usize>,
        /// Index into `theirs`, if this row has one.
        theirs: Option<usize>,
    },
    /// A stacked-layout row (narrow, or BASE at any width): a label row
    /// (`i` unused, `0`) or the `i`-th line of `side`.
    Side {
        /// The block's index.
        block: usize,
        /// Which section.
        side: SideKind,
        /// The line's index within that section.
        i: usize,
    },
    /// A block's closing rule.
    End {
        /// The block's index.
        block: usize,
    },
    /// The whole-file card's header row.
    WholeFile {
        /// The shape, worded.
        how: SanitizedText,
        /// The keys that apply.
        keys: SanitizedText,
    },
    /// A whole-file card's note under its lines: not a line of the file.
    Note {
        /// The note.
        text: SanitizedText,
    },
    /// Three panes: the titles row, `yours · main │ result │ theirs ·
    /// feature`. A body row: it scrolls with the document.
    PaneTitles,
    /// Three panes: one row, level across them. `block` is the block it
    /// belongs to (its rule row included), `None` for a plain line.
    Aligned {
        /// The block this row belongs to, if any.
        block: Option<usize>,
        /// The yours pane's cell.
        ours: Cell,
        /// The result pane's cell.
        result: Cell,
        /// The theirs pane's cell.
        theirs: Cell,
    },
}

/// `doc` laid out at `columns`: every row the widget can draw, and where
/// each block's header row is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Every row, in draw order.
    pub rows: Vec<ScreenRow>,
    /// `block_row[i]` is the row index of block `i`'s [`ScreenRow::Header`]
    /// in the card, of its rule row ([`ScreenRow::Aligned`], all
    /// [`Cell::Rule`]) in the three panes.
    pub block_row: Vec<usize>,
    /// The width this layout was built for.
    pub columns: u16,
}

/// `doc` laid out in `arrangement` at `columns` (the document's own width).
/// Pure and total: returns rows for any `columns >= 1`, including below
/// [`MIN_COLUMNS`](super::MIN_COLUMNS) (the widget, not the layout, is what
/// refuses to draw there).
#[must_use]
pub fn layout(doc: &DocView, arrangement: Arrangement, columns: u16) -> Layout {
    let (rows, block_row) = match arrangement {
        Arrangement::Card => card_rows(doc, columns),
        Arrangement::ThreePane => super::panes::pane_rows(doc),
    };
    Layout {
        rows,
        block_row,
        columns,
    }
}

/// The card's rows, and each block's header-row index.
fn card_rows(doc: &DocView, columns: u16) -> (Vec<ScreenRow>, Vec<usize>) {
    let mut rows = Vec::new();
    let mut block_row = Vec::new();
    for region in &doc.regions {
        match region {
            RegionView::Plain { numbers, lines } => {
                for (offset, text) in lines.iter().enumerate() {
                    rows.push(ScreenRow::Plain {
                        line_no: numbers.result + offset,
                        text: text.clone(),
                    });
                }
            }
            RegionView::WholeFile {
                how,
                keys,
                lines,
                note,
                ..
            } => {
                rows.push(ScreenRow::WholeFile {
                    how: how.clone(),
                    keys: keys.clone(),
                });
                for (offset, text) in lines.iter().enumerate() {
                    rows.push(ScreenRow::Plain {
                        line_no: offset + 1,
                        text: text.clone(),
                    });
                }
                if let Some(text) = note {
                    rows.push(ScreenRow::Note { text: text.clone() });
                }
            }
            RegionView::Block(block) => {
                block_row.push(rows.len());
                rows.push(ScreenRow::Header { block: block.index });
                layout_block(doc, block, columns, &mut rows);
                rows.push(ScreenRow::End { block: block.index });
            }
        }
    }
    (rows, block_row)
}

/// The rows between a block's header and its closing rule.
fn layout_block(doc: &DocView, block: &BlockView, columns: u16, rows: &mut Vec<ScreenRow>) {
    match block.state {
        BlockState::Took(take) => layout_taken(block, take, rows),
        BlockState::Open if columns >= SIDE_BY_SIDE_MIN => {
            rows.push(ScreenRow::Labels { block: block.index });
            let paired = block.ours.len().max(block.theirs.len());
            for i in 0..paired {
                rows.push(ScreenRow::Pair {
                    block: block.index,
                    ours: (i < block.ours.len()).then_some(i),
                    theirs: (i < block.theirs.len()).then_some(i),
                });
            }
            if let Some((_, base_lines)) = base_if_shown(doc, block) {
                push_labeled_side(
                    rows,
                    block.index,
                    LabelOf::Base,
                    SideKind::Base,
                    base_lines.len(),
                );
            }
        }
        BlockState::Open => {
            push_labeled_side(
                rows,
                block.index,
                LabelOf::Ours,
                SideKind::Ours,
                block.ours.len(),
            );
            if let Some((_, base_lines)) = base_if_shown(doc, block) {
                push_labeled_side(
                    rows,
                    block.index,
                    LabelOf::Base,
                    SideKind::Base,
                    base_lines.len(),
                );
            }
            push_labeled_side(
                rows,
                block.index,
                LabelOf::Theirs,
                SideKind::Theirs,
                block.theirs.len(),
            );
        }
    }
}

/// A taken block's rows: the taken side's lines, full width, no labels (the
/// header already says which side).
fn layout_taken(block: &BlockView, take: TakeView, rows: &mut Vec<ScreenRow>) {
    match take {
        TakeView::Ours => push_side_lines(rows, block.index, SideKind::Ours, block.ours.len()),
        TakeView::Theirs => {
            push_side_lines(rows, block.index, SideKind::Theirs, block.theirs.len());
        }
        TakeView::Both => {
            push_side_lines(rows, block.index, SideKind::Ours, block.ours.len());
            push_side_lines(rows, block.index, SideKind::Theirs, block.theirs.len());
        }
    }
}

/// `block`'s base, only when `doc.base_shown` and it is attached.
fn base_if_shown<'a>(
    doc: &DocView,
    block: &'a BlockView,
) -> Option<&'a (SanitizedText, Vec<SanitizedText>)> {
    if doc.base_shown {
        block.base.as_ref()
    } else {
        None
    }
}

/// A stacked section: its label row, then `len` content rows.
fn push_labeled_side(
    rows: &mut Vec<ScreenRow>,
    block: usize,
    label: LabelOf,
    side: SideKind,
    len: usize,
) {
    rows.push(ScreenRow::Side {
        block,
        side: SideKind::Label(label),
        i: 0,
    });
    push_side_lines(rows, block, side, len);
}

/// `len` content rows of `side`, no label.
fn push_side_lines(rows: &mut Vec<ScreenRow>, block: usize, side: SideKind, len: usize) {
    for i in 0..len {
        rows.push(ScreenRow::Side { block, side, i });
    }
}

/// The row `top + screen_row` names, or `None` past the end. The app scrolls
/// and hit-tests through this — the same indexing [`ResolverScreen`] draws
/// with, so a click and the pixel it lands on never disagree.
#[must_use]
pub fn row_at(layout: &Layout, top: usize, screen_row: u16) -> Option<&ScreenRow> {
    layout.rows.get(top + usize::from(screen_row))
}
