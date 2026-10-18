//! [`layout`]: a [`DocView`] planned into the [`ScreenRow`]s the widget
//! draws, pure and total, and [`row_at`], the one windowed lookup the app
//! scrolls and hit-tests through.

use mergewright_term::SanitizedText;

use super::{Arrangement, BlockState, BlockView, Cell, DocView, LabelOf, RegionView, SideKind};

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
    /// A block's first row: the top of its outline while it has the
    /// cursor, blank otherwise. Every block keeps it, so no row moves as
    /// the cursor does.
    Header {
        /// The block's index.
        block: usize,
    },
    /// The side-by-side labels row, the open mark in its gutter
    /// (`? yours · wallet-release │ theirs · sara/fix-toc`).
    Labels {
        /// The block's index.
        block: usize,
    },
    /// A row of the two sides, side by side, unnumbered. `None` pads the
    /// shorter side.
    Pair {
        /// The block's index.
        block: usize,
        /// Index into `ours`, if this row has one.
        ours: Option<usize>,
        /// Index into `theirs`, if this row has one.
        theirs: Option<usize>,
    },
    /// A stacked row (below [`INLINE_FROM`](super::INLINE_FROM), or BASE
    /// at any width): a label row (`i` unused, `0`) or the `i`-th line of
    /// `side`.
    Side {
        /// The block's index.
        block: usize,
        /// Which section.
        side: SideKind,
        /// The line's index within that section.
        i: usize,
    },
    /// A taken block's `i`-th kept line ([`BlockView::kept`]), numbered as
    /// a plain line: the first carries the mark, the last the mark's
    /// words. A pick that kept no lines still gets one row, `i` 0.
    Kept {
        /// The block's index.
        block: usize,
        /// Index into `kept`.
        i: usize,
    },
    /// A block's last row: the bottom of its outline while it has the
    /// cursor, blank otherwise.
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
    /// `✓ Every clash in <name> is solved.`, then the way on.
    Solved {
        /// The file's last path segment.
        name: SanitizedText,
        /// The way on, in the app's words.
        next: SanitizedText,
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
    /// inline, of its rule row ([`ScreenRow::Aligned`], all
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
        Arrangement::Inline => inline_rows(doc, true),
        Arrangement::Card => inline_rows(doc, false),
        Arrangement::ThreePane => super::panes::pane_rows(doc),
    };
    Layout {
        rows,
        block_row,
        columns,
    }
}

/// The document's rows, an open clash's sides side by side or stacked, and
/// each block's header-row index.
fn inline_rows(doc: &DocView, side_by_side: bool) -> (Vec<ScreenRow>, Vec<usize>) {
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
                layout_block(doc, block, side_by_side, &mut rows);
                rows.push(ScreenRow::End { block: block.index });
            }
            RegionView::Solved { name, next } => rows.push(ScreenRow::Solved {
                name: name.clone(),
                next: next.clone(),
            }),
        }
    }
    (rows, block_row)
}

/// The rows between a block's header and its end.
fn layout_block(doc: &DocView, block: &BlockView, side_by_side: bool, rows: &mut Vec<ScreenRow>) {
    match block.state {
        // Shrunk to what the pick wrote; one row even when it wrote none,
        // for the mark and its words.
        BlockState::Took(_) => {
            for i in 0..block.kept.len().max(1) {
                rows.push(ScreenRow::Kept {
                    block: block.index,
                    i,
                });
            }
        }
        BlockState::Open if side_by_side => {
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
    for i in 0..len {
        rows.push(ScreenRow::Side { block, side, i });
    }
}

/// The row `top + screen_row` names, or `None` past the end. The app scrolls
/// and hit-tests through this — the same indexing [`ResolverScreen`] draws
/// with, so a click and the pixel it lands on never disagree.
///
/// [`ResolverScreen`]: super::ResolverScreen
#[must_use]
pub fn row_at(layout: &Layout, top: usize, screen_row: u16) -> Option<&ScreenRow> {
    layout.rows.get(top + usize::from(screen_row))
}
