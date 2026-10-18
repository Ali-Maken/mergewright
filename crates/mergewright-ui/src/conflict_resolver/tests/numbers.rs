//! Line numbers and the outline (ease-pass/11): an open clash's sides are
//! unnumbered, kept lines and plain lines share one numbered column, and
//! only the cursor's clash is outlined. Split from `tests.rs` to keep it
//! under the size ratchet.

use super::*;

/// The widths the numbering tests sweep: both sides of 100 (stacked vs
/// paired), the 60-column floor, and wide.
const WIDTHS: [u16; 6] = [60, 80, 99, 100, 120, 179];

/// Row `y` of `buf` from past the cursor gutter to `width`.
fn content_text(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    row_text(buf, y, width)
        .chars()
        .skip(usize::from(GUTTER))
        .collect()
}

/// ADR-0043 §5: a marker row is no line of any side, so no row but a
/// content row starts with a number — header, labels (paired or stacked),
/// end and note, at every width and in both glyph tiers.
#[test]
fn no_marker_row_draws_a_number() {
    for doc in [three_block_doc(true), whole_file_doc()] {
        for columns in WIDTHS {
            for ascii in [false, true] {
                let laid = layout(&doc, Arrangement::Card, columns);
                let rows = u16::try_from(laid.rows.len()).unwrap() + HEADER_ROWS;
                let buf = draw(&doc, &laid, 0, columns, rows, ascii);
                for (index, row) in laid.rows.iter().enumerate() {
                    let marker = matches!(
                        row,
                        ScreenRow::Header { .. }
                            | ScreenRow::Labels { .. }
                            | ScreenRow::End { .. }
                            | ScreenRow::Note { .. }
                            | ScreenRow::Side {
                                side: SideKind::Label(_),
                                ..
                            }
                    );
                    if !marker {
                        continue;
                    }
                    let y = HEADER_ROWS + u16::try_from(index).unwrap();
                    let drawn = content_text(&buf, y, columns);
                    assert!(
                        !drawn.trim_start().starts_with(|c: char| c.is_ascii_digit()),
                        "{columns} cols ascii={ascii}: {row:?} drew a number: {drawn:?}"
                    );
                }
            }
        }
    }
}

/// `drawn` (a row's text past the gutter) starts with the number `n`.
fn starts_with_number(drawn: &str, n: usize) -> bool {
    drawn.trim_start().starts_with(&format!("{n} "))
}

/// `drawn` leads with no number.
fn unnumbered(drawn: &str) -> bool {
    !drawn.trim_start().starts_with(|c: char| c.is_ascii_digit())
}

/// Block `index` of `doc`.
pub(super) fn block_of(doc: &DocView, index: usize) -> &BlockView {
    doc.regions
        .iter()
        .find_map(|region| match region {
            RegionView::Block(block) if block.index == index => Some(block),
            _ => None,
        })
        .unwrap()
}

/// Checks one drawn row's numbers; `true` when it drew an open theirs line.
fn check_row_numbers(doc: &DocView, row: &ScreenRow, drawn: &str, divider: char, at: &str) -> bool {
    match *row {
        ScreenRow::Pair { theirs, .. } => {
            let (left, right) = drawn.split_once(divider).expect(at);
            assert!(unnumbered(left) && unnumbered(right), "{at}");
            theirs.is_some()
        }
        ScreenRow::Side { side, .. } => {
            assert!(unnumbered(drawn), "{at}");
            side == SideKind::Theirs
        }
        ScreenRow::Kept { block, i } => {
            let first = block_of(doc, block).numbers.result;
            assert!(starts_with_number(drawn, first + i), "{at}");
            false
        }
        ScreenRow::Plain { line_no, .. } => {
            assert!(starts_with_number(drawn, line_no), "{at}");
            false
        }
        _ => false,
    }
}

/// ease-pass/11 (mock M7, M8): an open clash's sides carry no numbers, paired
/// or stacked; a taken clash's kept lines carry the result's, as the plain
/// lines do.
#[test]
fn open_sides_are_unnumbered_and_kept_lines_numbered_at_every_width() {
    let doc = three_block_doc(true);
    for columns in WIDTHS {
        for ascii in [false, true] {
            let laid = layout(&doc, arrangement(columns), columns);
            let rows = u16::try_from(laid.rows.len()).unwrap() + HEADER_ROWS;
            let buf = draw(&doc, &laid, 0, columns, rows, ascii);
            let divider = if ascii { '|' } else { '│' };
            let mut theirs_drawn = 0;
            for (index, row) in laid.rows.iter().enumerate() {
                let y = HEADER_ROWS + u16::try_from(index).unwrap();
                let drawn = content_text(&buf, y, columns);
                let at = format!("{columns} cols ascii={ascii} {row:?}: {drawn:?}");
                if check_row_numbers(&doc, row, &drawn, divider, &at) {
                    theirs_drawn += 1;
                }
            }
            assert!(theirs_drawn > 0, "{columns}: no open theirs row");
        }
    }
    // The taken block (took theirs, result 42) reads 42, not its theirs 128.
    let laid = layout(&doc, Arrangement::Inline, 120);
    let buf = draw(&doc, &laid, 0, 120, 40, false);
    let y = HEADER_ROWS + u16::try_from(laid.block_row[1]).unwrap() + 1;
    assert!(starts_with_number(&content_text(&buf, y, 120), 42));
}

/// The number column sits in the same cells whether a row is a bare `Plain`
/// line or a taken clash's kept line: both draw through `content_area`
/// (Major 2, m6-02-03 review).
#[test]
fn plain_and_block_numbers_share_one_column() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, Arrangement::Inline, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);
    // Row 0 (`Plain`, "38 fn refresh…") and block 1's kept line ("42
    // new_way();") start their numbers at the same x.
    let plain_x = first_number_x(&buf, HEADER_ROWS, 120);
    let kept_row = HEADER_ROWS + u16::try_from(laid.block_row[1]).unwrap() + 1;
    let kept_x = first_number_x(&buf, kept_row, 120);
    assert_eq!(
        plain_x, kept_x,
        "plain row's number starts at {plain_x:?}, kept row's at {kept_x:?}"
    );
}

/// The x of the first ASCII number cell on row `y`, or `None`.
fn first_number_x(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> Option<u16> {
    (0..width).find(|&x| {
        buf[(x, y)]
            .symbol()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    })
}

/// Mock M7: only the cursor's clash is outlined, box drawing top and bottom
/// (ASCII `+-`); another clash's outline rows stay blank, and nothing is
/// reversed.
#[test]
fn only_the_cursor_clash_is_outlined() {
    let doc = three_block_doc(false); // cursor_block: Some(0)
    let laid = layout(&doc, Arrangement::Inline, 120);
    for (ascii, corner, bottom) in [(false, "┌", "└"), (true, "+", "+")] {
        let buf = draw(&doc, &laid, 0, 120, 24, ascii);
        let at = |block: usize, offset: usize| {
            let y = HEADER_ROWS + u16::try_from(laid.block_row[block] + offset).unwrap();
            row_text(&buf, y, 120)
        };
        assert!(at(0, 0).contains(corner), "{:?}", at(0, 0));
        assert!(at(0, 4).contains(bottom), "{:?}", at(0, 4));
        assert_eq!(at(2, 0).trim(), "", "block 2 has no outline");
        let reversed = (0..24u16).any(|y| {
            (0..120u16).any(|x| {
                buf[(x, y)]
                    .style()
                    .add_modifier
                    .contains(Modifier::REVERSED)
            })
        });
        assert!(!reversed, "ascii={ascii}: a reversed cell");
    }
}
