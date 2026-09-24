//! The three-pane arrangement (m6-conflicts/14): rows level by git's own
//! regions, each pane numbered by its own projection, the chosen side in
//! the middle, the split rule.
//!
//! The documents come from real conflict bytes through `graft_git`'s own
//! parser, `take` and `first_lines`, so the numbers the panes are checked
//! against are the projections', not numbers typed into a fixture.

use graft_git::conflict::{ConflictFile, Projection, Region, Take, parse};

use super::*;

/// The files every test here sweeps: two one-line blocks (the capture's
/// `notes.txt`), theirs longer than ours, ours longer than theirs, an
/// empty ours, a block with a base, blocks first and last, and two blocks
/// back to back.
const CORPUS: [&[u8]; 7] = [
    b"one\n<<<<<<< HEAD\ntwo from main\n=======\nTWO from feature\n>>>>>>> feature\nthree\nfour\n<<<<<<< HEAD\nfive from main\n=======\nFIVE from feature\n>>>>>>> feature\nsix\n",
    b"a\n<<<<<<< HEAD\nb\n=======\nB1\nB2\nB3\n>>>>>>> t\nc\n<<<<<<< HEAD\nd\n=======\nD\n>>>>>>> t\ne\n",
    b"a\n<<<<<<< HEAD\nb1\nb2\nb3\n=======\nB\n>>>>>>> t\nc\n<<<<<<< HEAD\nd\n=======\nD\n>>>>>>> t\n",
    b"a\n<<<<<<< HEAD\n=======\nadded\n>>>>>>> t\nb\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> t\n",
    b"a\n<<<<<<< HEAD\nours\n||||||| base\nold one\nold two\nold three\n=======\ntheirs\n>>>>>>> t\nb\n<<<<<<< HEAD\nc\n||||||| base\nC\n=======\nd\n>>>>>>> t\n",
    b"<<<<<<< HEAD\nfirst\n=======\nFIRST\nFIRST TOO\n>>>>>>> t\nmiddle\n<<<<<<< HEAD\nlast\n=======\nLAST\n>>>>>>> t\n",
    b"<<<<<<< HEAD\na\n=======\nA\n>>>>>>> t\n<<<<<<< HEAD\nb\nb\n=======\nB\n>>>>>>> t\nend\n",
];

/// One line of a parsed file, as the app hands it to the widget.
fn text_of(line: &graft_git::conflict::Line) -> SanitizedText {
    text(String::from_utf8_lossy(&line.0).trim_end_matches('\n'))
}

/// `file` as the app's `view::resolver::regions` builds it.
fn doc_of(file: &ConflictFile, base_shown: bool) -> DocView {
    let [ours, theirs, result] =
        [Projection::Ours, Projection::Theirs, Projection::Result].map(|p| file.first_lines(p));
    let of = file.blocks().len();
    let mut index = 0;
    let regions = file
        .regions
        .iter()
        .enumerate()
        .map(|(r, region)| {
            let numbers = BlockNumbers {
                ours: ours[r],
                theirs: theirs[r],
                result: result[r],
            };
            let (block, state) = match region {
                Region::Plain(lines) => {
                    return RegionView::Plain {
                        numbers,
                        lines: lines.iter().map(text_of).collect(),
                    };
                }
                Region::Conflict(block) => (block, BlockState::Open),
                Region::Resolved { block, take } => (
                    block,
                    BlockState::Took(match take {
                        Take::Ours => TakeView::Ours,
                        Take::Theirs => TakeView::Theirs,
                        Take::Both => TakeView::Both,
                    }),
                ),
            };
            index += 1;
            RegionView::Block(BlockView {
                index: index - 1,
                of,
                state,
                ours: block.ours.iter().map(text_of).collect(),
                base: block
                    .shown_base()
                    .map(|(base, _)| (text("base"), base.lines.iter().map(text_of).collect())),
                theirs: block.theirs.iter().map(text_of).collect(),
                first_line: numbers.result,
                numbers,
            })
        })
        .collect();
    DocView {
        title: text("notes.txt · 2 conflicts · 2 open"),
        files: FileListView::default(),
        names: names(),
        regions,
        cursor_block: Some(0),
        base_shown,
        notice: None,
    }
}

/// The states each file is swept through.
#[derive(Clone, Copy, Debug)]
enum Sweep {
    NothingTaken,
    Block0Ours,
    AllTheirs,
    Block1Both,
    Block0OpenWithBase,
}

const SWEEPS: [Sweep; 5] = [
    Sweep::NothingTaken,
    Sweep::Block0Ours,
    Sweep::AllTheirs,
    Sweep::Block1Both,
    Sweep::Block0OpenWithBase,
];

/// `bytes` parsed and taken as `sweep` says: the file, and its doc.
fn swept(bytes: &[u8], sweep: Sweep) -> (ConflictFile, DocView) {
    let mut file = parse(bytes).expect("a corpus file parses");
    let blocks = file.blocks().len();
    let take = |file: &mut ConflictFile, block: usize, take: Take| {
        file.take(block, take).expect("the corpus block exists");
    };
    match sweep {
        Sweep::NothingTaken | Sweep::Block0OpenWithBase => {}
        Sweep::Block0Ours => take(&mut file, 0, Take::Ours),
        Sweep::AllTheirs => (0..blocks).for_each(|b| take(&mut file, b, Take::Theirs)),
        Sweep::Block1Both => take(&mut file, 1, Take::Both),
    }
    let doc = doc_of(&file, matches!(sweep, Sweep::Block0OpenWithBase));
    (file, doc)
}

fn three(doc: &DocView) -> Layout {
    layout(doc, Arrangement::ThreePane, 200)
}

/// The three cells of an aligned row.
fn cells(row: &ScreenRow) -> Option<(Option<usize>, [Cell; 3])> {
    match row {
        ScreenRow::Aligned {
            block,
            ours,
            result,
            theirs,
        } => Some((*block, [*ours, *result, *theirs])),
        _ => None,
    }
}

/// How many rows a region takes in each pane, before padding.
fn pane_lengths(doc: &DocView, region: &RegionView) -> [usize; 3] {
    match region {
        RegionView::Plain { lines, .. } => [lines.len(); 3],
        RegionView::Block(block) => {
            let result = match block.state {
                BlockState::Open => match (&block.base, doc.base_shown) {
                    (Some((_, base)), true) => base.len() + 1,
                    _ => 1,
                },
                BlockState::Took(take) => super::super::panes::taken_len(block, take),
            };
            [block.ours.len(), result, block.theirs.len()]
        }
        RegionView::WholeFile { .. } => unreachable!("the corpus has no whole-file card"),
    }
}

#[test]
fn three_panes_stay_level() {
    for (f, bytes) in CORPUS.iter().enumerate() {
        for sweep in SWEEPS {
            let (_, doc) = swept(bytes, sweep);
            let laid = three(&doc);
            let at = format!("file {f}, {sweep:?}");
            assert_eq!(laid.rows[0], ScreenRow::PaneTitles, "{at}");
            let mut row = 1;
            for region in &doc.regions {
                let lengths = pane_lengths(&doc, region);
                let span = match region {
                    RegionView::Block(block) => {
                        // The rule row: level across all three panes.
                        assert_eq!(
                            cells(&laid.rows[row]),
                            Some((Some(block.index), [Cell::Rule; 3])),
                            "{at}, row {row}"
                        );
                        row += 1;
                        let tall = lengths.iter().copied().max().unwrap_or(0);
                        for k in 0..tall {
                            let (owner, _) = cells(&laid.rows[row + k]).expect("aligned");
                            assert_eq!(owner, Some(block.index), "{at}, row {}", row + k);
                        }
                        tall
                    }
                    _ => lengths[0],
                };
                // Every pane's first row of this region is this region's
                // first row: each pane's content starts at `row` and runs
                // unbroken, pads only after it.
                for k in 0..span {
                    let (_, panes) = cells(&laid.rows[row + k]).expect("aligned");
                    for (pane, cell) in panes.iter().enumerate() {
                        let has = !matches!(cell, Cell::Pad);
                        assert_eq!(has, k < lengths[pane], "{at}, pane {pane}, row {}", row + k);
                    }
                }
                row += span;
            }
            assert_eq!(row, laid.rows.len(), "{at}: rows left over");
        }
    }
}

/// The numbers down pane `pane`, in row order.
fn numbers_down(laid: &Layout, pane: usize) -> Vec<(usize, Cell, Option<usize>)> {
    laid.rows
        .iter()
        .filter_map(cells)
        .map(|(block, cells)| (pane, cells[pane], block))
        .collect()
}

#[test]
fn each_pane_is_numbered_without_gaps() {
    for (f, bytes) in CORPUS.iter().enumerate() {
        for sweep in SWEEPS {
            let (file, doc) = swept(bytes, sweep);
            let laid = three(&doc);
            let at = format!("file {f}, {sweep:?}");
            let count = |p: Projection| -> usize {
                file.regions
                    .iter()
                    .map(|region| match (region, p) {
                        (Region::Plain(lines), _) => lines.len(),
                        (
                            Region::Conflict(b) | Region::Resolved { block: b, .. },
                            Projection::Ours,
                        ) => b.ours.len(),
                        (Region::Conflict(b) | Region::Resolved { block: b, .. }, _) => {
                            b.theirs.len()
                        }
                    })
                    .sum()
            };
            for (pane, p) in [(0, Projection::Ours), (2, Projection::Theirs)] {
                let drawn: Vec<usize> = numbers_down(&laid, pane)
                    .into_iter()
                    .filter_map(|(_, cell, _)| match cell {
                        Cell::Line { line_no, .. } => line_no,
                        _ => None,
                    })
                    .collect();
                let want: Vec<usize> = (1..=count(p)).collect();
                assert_eq!(drawn, want, "{at}, pane {pane}");
            }
            // The result: each number one past the last, plus an open
            // block's ours lines, which its placeholder stands for.
            let mut last = 0;
            for (_, cell, block) in numbers_down(&laid, 1) {
                match cell {
                    Cell::Line {
                        line_no: Some(n), ..
                    } => {
                        assert_eq!(n, last + 1, "{at}: result {n} after {last}");
                        last = n;
                    }
                    Cell::Placeholder => {
                        let view = block_of(&doc, block.expect("a block's placeholder"));
                        last += view.ours.len();
                    }
                    _ => {}
                }
            }
            if file.open() == 0 {
                let lines = String::from_utf8_lossy(&file.render()).lines().count();
                assert_eq!(last, lines, "{at}: the result ends on render()'s last line");
            }
        }
    }
}

#[test]
fn plain_regions_number_each_pane_by_its_projection() {
    // Block 0's theirs is three lines to ours' one.
    let mut file = parse(CORPUS[1]).expect("parses");
    file.take(0, Take::Theirs).expect("block 0");
    let doc = doc_of(&file, false);
    let laid = three(&doc);
    let region = 2; // `c`, the plain region after block 0.
    let firsts = [Projection::Ours, Projection::Result, Projection::Theirs]
        .map(|p| file.first_lines(p)[region]);
    let row = laid
        .rows
        .iter()
        .filter_map(cells)
        .find(|(_, cells)| {
            matches!(cells[0], Cell::Line { text: LineRef::Plain { region: r, i: 0 }, .. } if r == region)
        })
        .expect("the plain region's first row");
    let drawn = row.1.map(|cell| match cell {
        Cell::Line { line_no, .. } => line_no,
        _ => None,
    });
    assert_eq!(drawn, firsts.map(Some));
    assert_ne!(
        firsts[1], firsts[0],
        "the result's number differs from yours'"
    );
}

/// The body rows of the three panes, as drawn at 200 columns.
fn render_panes(doc: &DocView) -> Vec<[String; 3]> {
    let laid = three(doc);
    let height = u16::try_from(laid.rows.len()).expect("a small document") + HEADER_ROWS;
    let area = Rect::new(0, 0, 200, height.max(MIN_ROWS));
    let placed = Placement::Column { width: 22 };
    let buf = paint_three(doc, &laid, placed, area);
    let areas = resolver_areas(area, placed);
    let panes = three_panes(areas.doc);
    (areas.doc.y + HEADER_ROWS..area.bottom())
        .map(|y| {
            panes.map(|pane| {
                (pane.x..pane.right())
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
                    .trim_end()
                    .to_owned()
            })
        })
        .collect()
}

fn paint_three(
    doc: &DocView,
    laid: &Layout,
    placement: Placement,
    area: Rect,
) -> ratatui::buffer::Buffer {
    let theme = Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::TrueColor, false),
    );
    let mut buf = ratatui::buffer::Buffer::empty(area);
    ResolverScreen {
        doc,
        layout: laid,
        top: 0,
        placement,
        theme: &theme,
        ascii: false,
    }
    .render(area, &mut buf);
    buf
}

#[test]
fn the_chosen_side_lands_in_the_middle() {
    let mut file = parse(CORPUS[1]).expect("parses");
    file.take(0, Take::Theirs).expect("block 0");
    let doc = doc_of(&file, false);
    let laid = three(&doc);
    let rule = laid.block_row[0];
    let theirs = &block_of(&doc, 0).theirs;
    for (i, want) in theirs.iter().enumerate() {
        let (_, [_, result, _]) = cells(&laid.rows[rule + 1 + i]).expect("aligned");
        assert_eq!(
            result,
            Cell::Line {
                line_no: Some(block_of(&doc, 0).numbers.result + i),
                text: LineRef::Taken { block: 0, i },
            }
        );
        let drawn = &render_panes(&doc)[rule + 1 + i];
        assert!(drawn[1].ends_with(want.as_str()), "{drawn:?}");
        assert!(drawn[2].ends_with(want.as_str()), "{drawn:?}");
    }
}

#[test]
fn the_rule_row_is_the_block_row() {
    for bytes in CORPUS {
        for sweep in SWEEPS {
            let (_, doc) = swept(bytes, sweep);
            let laid = three(&doc);
            for (i, &row) in laid.block_row.iter().enumerate() {
                assert_eq!(
                    cells(&laid.rows[row]),
                    Some((Some(i), [Cell::Rule; 3])),
                    "{sweep:?}"
                );
            }
        }
    }
}

#[test]
fn panes_split_by_the_rule() {
    for width in (147..=157).chain(167..=177) {
        let doc = Rect::new(33, 3, width, 10);
        let [yours, result, theirs] = three_panes(doc);
        assert_eq!(yours.x, doc.x, "{width}");
        assert_eq!(result.x, yours.right() + 1, "{width}: one `│` after yours");
        assert_eq!(
            theirs.x,
            result.right() + 1,
            "{width}: one `│` after the result"
        );
        assert_eq!(theirs.right(), doc.right(), "{width}");
        assert_eq!(yours.width, theirs.width, "{width}: equal sides");
        assert_eq!(
            result.width - yours.width,
            (width - 2) % 3,
            "{width}: remainder"
        );
        assert_eq!(
            yours.width + result.width + theirs.width + 2,
            width,
            "{width}"
        );
    }
    for column in 22..=32 {
        let areas = resolver_areas(
            Rect::new(0, 0, 180, 50),
            Placement::Column { width: column },
        );
        let narrowest = three_panes(areas.doc).iter().map(|p| p.width).min();
        assert!(narrowest >= Some(48), "column {column}: {narrowest:?}");
    }
}

/// A whole-file shape: the side the file is on shows it, numbered; the
/// side it is not on says so; the result asks keep or delete.
#[test]
fn a_whole_file_shape_shows_each_side() {
    let doc = whole_file_doc();
    let drawn = render_panes(&doc);
    // Row 0 titles, row 1 the card's own row, then the file.
    assert!(drawn[1][0].contains("deleted by them"), "{drawn:?}");
    assert_eq!(drawn[2][0], " 1 one");
    assert_eq!(drawn[3][0], " 2 two");
    assert_eq!(drawn[2][1], "   ? keep or delete · o t");
    assert_eq!(drawn[2][2], "   (deleted on theirs · feature)");
    assert_eq!(drawn[3][2], "");
    assert!(drawn[4][0].contains("and 3 more lines unread"), "{drawn:?}");
}

/// What the titles and rule rows say, drawn.
#[test]
fn titles_and_rules_read_as_designed() {
    let mut file = parse(CORPUS[0]).expect("parses");
    file.take(0, Take::Theirs).expect("block 0");
    let mut doc = doc_of(&file, false);
    doc.cursor_block = Some(1);
    let drawn = render_panes(&doc);
    assert_eq!(
        drawn[0],
        ["yours · main", "result", "theirs · feature"].map(|t| format!(" {t}"))
    );
    let laid = three(&doc);
    assert_eq!(
        drawn[laid.block_row[0]],
        [
            " ── not taken",
            " ── 1 of 2 · took theirs · feature",
            " ── taken ✓"
        ]
        .map(str::to_owned)
    );
    let open = &drawn[laid.block_row[1]];
    assert_eq!(open[0], "▏── o take");
    assert_eq!(open[1], " ── 2 of 2 · open");
    assert_eq!(open[2], " ── t take");
    assert_eq!(drawn[laid.block_row[1] + 1][1], "   ? open · o t b");
}
