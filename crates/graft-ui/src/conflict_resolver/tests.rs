use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn text(s: &str) -> SanitizedText {
    sanitize_line(s)
}

fn lines(items: &[&str]) -> Vec<SanitizedText> {
    items.iter().map(|s| text(s)).collect()
}

fn names() -> SideNames {
    SideNames {
        ours: text("yours · main"),
        theirs: text("theirs · feature"),
        roles: SideRoles::Merge,
    }
}

/// An open block whose ours lines start at `ours_at` and theirs lines at
/// `theirs_at`; its result line is `ours_at` (where the placeholder sits).
fn open_block(
    index: usize,
    of: usize,
    (ours_at, theirs_at): (usize, usize),
    ours: &[&str],
    theirs: &[&str],
) -> BlockView {
    BlockView {
        index,
        of,
        state: BlockState::Open,
        ours: lines(ours),
        base: None,
        theirs: lines(theirs),
        first_line: ours_at,
        numbers: BlockNumbers {
            ours: ours_at,
            theirs: theirs_at,
            result: ours_at,
        },
    }
}

/// A three-block doc: block 0 open (two ours lines, one theirs), block 1
/// taken (theirs), block 2 open with a base attached.
fn three_block_doc(base_shown: bool) -> DocView {
    let mut block2 = open_block(
        2,
        3,
        (43, 130),
        &["let tok = self.store.get();"],
        &["let tok = store();"],
    );
    block2.base = Some((text("merge base"), lines(&["let tok = old();"])));

    DocView {
        title: text("src/auth/token.rs · file 1 of 2 · 2 of 3 conflicts open"),
        files: FileListView::default(),
        names: names(),
        regions: vec![
            RegionView::Plain {
                numbers: BlockNumbers {
                    ours: 38,
                    theirs: 38,
                    result: 38,
                },
                lines: lines(&["fn refresh(&mut self) {"]),
            },
            RegionView::Block(open_block(
                0,
                3,
                (39, 125),
                &["if expired() {", "self.renew();"],
                &["if is_expired() {", "self.do_refresh();"],
            )),
            RegionView::Plain {
                numbers: BlockNumbers {
                    ours: 41,
                    theirs: 41,
                    result: 41,
                },
                lines: lines(&["}"]),
            },
            RegionView::Block(BlockView {
                index: 1,
                of: 3,
                state: BlockState::Took(TakeView::Theirs),
                ours: lines(&["old_way();"]),
                base: None,
                theirs: lines(&["new_way();"]),
                first_line: 42,
                numbers: BlockNumbers {
                    ours: 42,
                    theirs: 128,
                    result: 42,
                },
            }),
            RegionView::Block(block2),
        ],
        cursor_block: Some(0),
        base_shown,
        notice: None,
        finish: None,
    }
}

fn block_kind(row: &ScreenRow) -> &'static str {
    match row {
        ScreenRow::Plain { .. } => "Plain",
        ScreenRow::Header { .. } => "Header",
        ScreenRow::Labels { .. } => "Labels",
        ScreenRow::Pair { .. } => "Pair",
        ScreenRow::Side { .. } => "Side",
        ScreenRow::End { .. } => "End",
        ScreenRow::WholeFile { .. } => "WholeFile",
        ScreenRow::Note { .. } => "Note",
        ScreenRow::PaneTitles => "PaneTitles",
        ScreenRow::Aligned { .. } => "Aligned",
    }
}

/// `layout` at 120 columns: `Plain…`, `Header 0`, `Labels 0`, `max(ours,
/// theirs)` `Pair` rows, `End 0`, `Plain…`, `Header 1`, `Side{Theirs}` rows
/// only, `End 1`, `Header 2`, `Labels`, `Pair…`, `Side{Base}` rows, `End 2`
/// — the acceptance list's order, block by block.
#[test]
fn layout_wide_rows_in_order() {
    let doc = three_block_doc(true);
    let laid = layout(&doc, Arrangement::Card, 120);
    let shape: Vec<&'static str> = laid.rows.iter().map(block_kind).collect();

    // Region 0: one Plain line.
    assert_eq!(shape[0], "Plain");
    // Block 0 (open, no base): Header, Labels, 2 Pair (max(2,2)), End.
    assert_eq!(&shape[1..6], ["Header", "Labels", "Pair", "Pair", "End"]);
    // Region: one more Plain line.
    assert_eq!(shape[6], "Plain");
    // Block 1 (took theirs): Header, 1 Side (theirs has 1 line), End.
    assert_eq!(&shape[7..10], ["Header", "Side", "End"]);
    // Block 2 (open, base shown, base has 1 line): Header, Labels, 1 Pair
    // (max(1,1)), 2 Side (base label + 1 base line), End.
    assert_eq!(
        &shape[10..],
        ["Header", "Labels", "Pair", "Side", "Side", "End"]
    );

    // The base rows really are `Side { side: Base | Label(Base), .. }`.
    let ScreenRow::Side {
        side: base_label, ..
    } = &laid.rows[13]
    else {
        panic!("expected the base's label row");
    };
    assert_eq!(*base_label, SideKind::Label(LabelOf::Base));
    let ScreenRow::Side {
        side: base_line, ..
    } = &laid.rows[14]
    else {
        panic!("expected the base's content row");
    };
    assert_eq!(*base_line, SideKind::Base);

    // Block 1's lone row is `Side { side: Theirs, .. }`, no label — a taken
    // block shows only the taken side, full width, no labels row.
    let ScreenRow::Side { side, .. } = &laid.rows[8] else {
        panic!("expected block 1's taken line");
    };
    assert_eq!(*side, SideKind::Theirs);
}

/// The same doc at 80 columns: no `Pair` rows anywhere; a block's own
/// section is `Side { Ours }` then `Side { Theirs }`, `Side { Base }`
/// between them when shown.
#[test]
fn layout_narrow_stacks() {
    let doc = three_block_doc(true);
    let laid = layout(&doc, Arrangement::Card, 80);
    assert!(
        !laid
            .rows
            .iter()
            .any(|row| matches!(row, ScreenRow::Pair { .. })),
        "no Pair rows below SIDE_BY_SIDE_MIN: {:?}",
        laid.rows
    );

    // Block 2 (index 2): label(ours), 1 ours line, label(base), 1 base
    // line, label(theirs), 1 theirs line.
    let block2: Vec<&ScreenRow> = laid
        .rows
        .iter()
        .filter(|row| matches!(row, ScreenRow::Side { block: 2, .. }))
        .collect();
    let kinds: Vec<SideKind> = block2
        .iter()
        .map(|row| match row {
            ScreenRow::Side { side, .. } => *side,
            _ => unreachable!(),
        })
        .collect();
    assert_eq!(
        kinds,
        vec![
            SideKind::Label(LabelOf::Ours),
            SideKind::Ours,
            SideKind::Label(LabelOf::Base),
            SideKind::Base,
            SideKind::Label(LabelOf::Theirs),
            SideKind::Theirs,
        ],
        "Ours then Base (shown) then Theirs, stacked"
    );
}

/// `block_row[i]` is the row index of block `i`'s `Header`.
#[test]
fn block_row_points_at_headers() {
    let doc = three_block_doc(false);
    for columns in [80u16, 120] {
        let laid = layout(&doc, Arrangement::Card, columns);
        for (index, &row_index) in laid.block_row.iter().enumerate() {
            assert_eq!(
                laid.rows[row_index],
                ScreenRow::Header { block: index },
                "block {index} at {columns} columns"
            );
        }
    }
}

/// `row_at(layout, top, r)` is `rows.get(top + r)`, `None` past the end.
#[test]
fn row_at_is_offset() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, Arrangement::Card, 120);
    for top in [0usize, 3, laid.rows.len()] {
        for r in 0u16..3 {
            assert_eq!(row_at(&laid, top, r), laid.rows.get(top + usize::from(r)));
        }
    }
    assert_eq!(row_at(&laid, laid.rows.len(), 0), None);
}

/// BASE is never drawn for a taken block, even with `base_shown` and a base
/// attached — resolved cards show only the taken side (design note, "a
/// taken block shows only the taken side's lines").
#[test]
fn a_taken_block_never_draws_base() {
    let mut doc = three_block_doc(true);
    if let RegionView::Block(block) = &mut doc.regions[3] {
        block.base = Some((text("base"), lines(&["old_way_base();"])));
    }
    for columns in [80u16, 120] {
        let laid = layout(&doc, Arrangement::Card, columns);
        let block1_rows: Vec<&ScreenRow> = laid
            .rows
            .iter()
            .filter(|row| matches!(row, ScreenRow::Side { block: 1, .. }))
            .collect();
        assert!(
            block1_rows.iter().all(|row| !matches!(
                row,
                ScreenRow::Side {
                    side: SideKind::Base | SideKind::Label(LabelOf::Base),
                    ..
                }
            )),
            "block 1 (taken) drew a base row at {columns} columns: {block1_rows:?}"
        );
    }
}

fn draw(
    view: &DocView,
    laid: &Layout,
    top: usize,
    columns: u16,
    rows: u16,
    ascii: bool,
) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    let capabilities = test_capabilities(ColorDepth::TrueColor, false);
    let theme = Theme::new(ThemeKind::Dark, &capabilities);
    terminal
        .draw(|frame| {
            frame.render_widget(
                ResolverScreen {
                    doc: view,
                    layout: laid,
                    top,
                    placement: Placement::Strip { rows: 0 },
                    theme: &theme,
                    ascii,
                    keys: crate::keymap::Keymap::shipped(),
                },
                frame.area(),
            );
        })
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    terminal.backend().buffer().clone()
}

/// Below 60x10 the widget draws the one "too small" line and nothing else
/// — `layout` itself still returns full rows (checked separately); this is
/// the widget refusing to draw them.
#[test]
fn resolver_tiny_frame_says_so() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, Arrangement::Card, 50);
    let buf = draw(&doc, &laid, 0, 50, 8, false);
    let first_row: String = (0..50)
        .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(
        first_row.starts_with("resolver needs 60x10 · esc closes"),
        "{first_row:?}"
    );
    // `layout` at the same width still returns real rows — the app decides
    // no key acts, the widget only refuses to draw.
    assert!(!laid.rows.is_empty());
}

/// With `ascii: true` nothing above U+007F reaches the buffer, at a width
/// where every glyph the widget itself chooses (the divider/join, the
/// cursor gutter, the label separator, `…` truncation, the wide-layout
/// pairing) is exercised.
///
/// App-supplied text (`doc.title`, the marker labels) is drawn verbatim —
/// sanitizing *that* to the ASCII tier is `term::sanitize`'s job, not this
/// widget's — so this fixture keeps it plain-ASCII on purpose, to isolate
/// what the widget controls.
#[test]
fn resolver_ascii_tier_is_ascii() {
    let mut doc = three_block_doc(true);
    doc.title = text("src/auth/token.rs - file 1 of 2 - 2 of 3 conflicts open");
    doc.names.ours = text("yours - main");
    doc.names.theirs = text("theirs - feature");
    for region in &mut doc.regions {
        if let RegionView::Block(block) = region
            && let Some((label, _)) = &mut block.base
        {
            *label = text("merge base");
        }
    }
    for columns in [80u16, 120] {
        let laid = layout(&doc, Arrangement::Card, columns);
        let buf = draw(&doc, &laid, 0, columns, 24, true);
        for cell in buf.content() {
            for ch in cell.symbol().chars() {
                assert!(
                    ch.is_ascii(),
                    "ascii tier at {columns} columns drew {ch:?} (U+{:04X})",
                    ch as u32
                );
            }
        }
    }
}

/// The widget draws only `layout.rows[top..top + height]` and **nothing
/// outside its `Rect`** — rendered here into a sub-`Rect` of a larger buffer
/// pre-filled with a sentinel glyph, so both halves of the claim are
/// actually checked (Critical 1, m6-02-03 review: the old version rendered
/// into `frame.area()`, the whole `TestBackend` buffer, so there was never a
/// cell outside `area` to assert anything about — the box's "asserts cells
/// outside `area` are untouched" was a false tick).
#[test]
fn resolver_draws_only_the_window() {
    let mut regions = vec![RegionView::Plain {
        numbers: BlockNumbers {
            ours: 1,
            theirs: 1,
            result: 1,
        },
        lines: (1..=4000).map(|n| text(&format!("line {n}"))).collect(),
    }];
    let doc = DocView {
        title: text("big.txt · file 1 of 1 · 0 of 0 conflicts open"),
        files: FileListView::default(),
        names: names(),
        regions: std::mem::take(&mut regions),
        cursor_block: None,
        base_shown: false,
        notice: None,
        finish: None,
    };
    let laid = layout(&doc, Arrangement::Card, 120);
    assert_eq!(laid.rows.len(), 4000);

    let full = Rect::new(0, 0, 140, 30);
    let sub = Rect::new(10, 5, 100, 10 + HEADER_ROWS);
    let sentinel = "#";
    let mut buf = sentinel_buffer(full, sentinel);

    let capabilities = test_capabilities(ColorDepth::TrueColor, false);
    let theme = Theme::new(ThemeKind::Dark, &capabilities);
    let top = 1000;
    ResolverScreen {
        doc: &doc,
        layout: &laid,
        top,
        placement: Placement::Strip { rows: 0 },
        theme: &theme,
        ascii: false,
        keys: crate::keymap::Keymap::shipped(),
    }
    .render(sub, &mut buf);

    // Inside the window: row 0 of the body is `layout.rows[top]` = "line 1001".
    let body_row0 = row_text(&buf, sub.y + HEADER_ROWS, sub.right()).split_off(sub.x.into());
    assert!(body_row0.contains("1001"), "{body_row0:?}");

    assert_untouched_outside(&buf, full, sub, sentinel);

    // Rows past `top + height` are never read: no line number from beyond
    // the window appears anywhere in the buffer.
    let whole: String = buf
        .content()
        .iter()
        .map(|cell| cell.symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(
        !whole.contains("1011 "),
        "a row past the window was drawn: {whole:?}"
    );
}

/// A buffer over `area`, every cell set to `sentinel`.
fn sentinel_buffer(area: Rect, sentinel: &str) -> ratatui::buffer::Buffer {
    let mut buf = ratatui::buffer::Buffer::empty(area);
    for cell in &mut buf.content {
        cell.set_symbol(sentinel);
    }
    buf
}

/// Every cell of `buf` outside `sub` still holds `sentinel` — the widget
/// touched nothing past its `Rect`.
fn assert_untouched_outside(buf: &ratatui::buffer::Buffer, full: Rect, sub: Rect, sentinel: &str) {
    for y in full.y..full.bottom() {
        for x in full.x..full.right() {
            if x >= sub.x && x < sub.right() && y >= sub.y && y < sub.bottom() {
                continue;
            }
            assert_eq!(
                buf[(x, y)].symbol(),
                sentinel,
                "cell ({x},{y}) outside {sub:?} was touched"
            );
        }
    }
}

/// A rendered row as a plain string, its own width wide, first-char-per-cell
/// (every glyph this widget draws is one cell).
fn row_text(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    (0..width)
        .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The widths the numbering tests sweep: both sides of 100 (stacked vs
/// paired), the 60-column floor, and wide.
const WIDTHS: [u16; 6] = [60, 80, 99, 100, 120, 179];

/// Row `y` of `buf` from past the cursor gutter to `width`.
fn content_text(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> String {
    row_text(buf, y, width).chars().skip(2).collect()
}

/// A doc with a whole-file card and its note, for the `Note` row.
fn whole_file_doc() -> DocView {
    DocView {
        title: text("big.bin"),
        files: FileListView::default(),
        names: names(),
        regions: vec![RegionView::WholeFile {
            how: text("deleted by them"),
            keys: text("o keeps the file  t deletes it"),
            lines: lines(&["one", "two"]),
            note: Some(text("and 3 more lines unread")),
            ours_absent: None,
            theirs_absent: Some(text("(deleted on theirs · feature)")),
        }],
        cursor_block: None,
        base_shown: false,
        notice: None,
        finish: None,
    }
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

/// ADR-0043 §5: an open card numbers yours from `numbers.ours` and theirs
/// from `numbers.theirs`, paired and stacked alike; BASE carries none; a
/// taken block's lines carry the result's numbers.
/// `drawn` (a row's text past the gutter) starts with the number `n`.
fn starts_with_number(drawn: &str, n: usize) -> bool {
    drawn.trim_start().starts_with(&format!("{n} "))
}

/// Block `index` of `doc`.
fn block_of(doc: &DocView, index: usize) -> &BlockView {
    doc.regions
        .iter()
        .find_map(|region| match region {
            RegionView::Block(block) if block.index == index => Some(block),
            _ => None,
        })
        .unwrap()
}

/// The number a content row of `side` should lead with, `None` for BASE.
fn expected_number(view: &BlockView, side: SideKind, i: usize) -> Option<usize> {
    match (view.state, side) {
        (BlockState::Open, SideKind::Ours) => Some(view.numbers.ours + i),
        (BlockState::Open, SideKind::Theirs) => Some(view.numbers.theirs + i),
        (BlockState::Took(TakeView::Both), SideKind::Theirs) => {
            Some(view.numbers.result + view.ours.len() + i)
        }
        (BlockState::Took(_), SideKind::Ours | SideKind::Theirs) => Some(view.numbers.result + i),
        _ => None,
    }
}

/// Checks one drawn row's numbers; `true` when it numbered an open theirs
/// line.
fn check_row_numbers(doc: &DocView, row: &ScreenRow, drawn: &str, divider: char, at: &str) -> bool {
    match *row {
        ScreenRow::Pair {
            block,
            ours,
            theirs,
            ..
        } => {
            let numbers = block_of(doc, block).numbers;
            let (left, right) = drawn.split_once(divider).expect(at);
            if let Some(i) = ours {
                assert!(starts_with_number(left, numbers.ours + i), "{at}");
            }
            if let Some(i) = theirs {
                assert!(starts_with_number(right, numbers.theirs + i), "{at}");
            }
            theirs.is_some()
        }
        ScreenRow::Side { block, side, i } => {
            let view = block_of(doc, block);
            match expected_number(view, side, i) {
                Some(n) => assert!(starts_with_number(drawn, n), "{at}"),
                None => assert!(
                    !drawn.trim_start().starts_with(|c: char| c.is_ascii_digit()),
                    "{at}"
                ),
            }
            view.state == BlockState::Open && side == SideKind::Theirs
        }
        ScreenRow::Plain { line_no, .. } => {
            assert!(starts_with_number(drawn, line_no), "{at}");
            false
        }
        _ => false,
    }
}

/// ADR-0043 §5: an open card numbers yours from `numbers.ours` and theirs
/// from `numbers.theirs`, paired and stacked alike; BASE carries none; a
/// taken block's lines carry the result's numbers.
#[test]
fn both_sides_are_numbered_at_every_width() {
    let doc = three_block_doc(true);
    for columns in WIDTHS {
        for ascii in [false, true] {
            let laid = layout(&doc, Arrangement::Card, columns);
            let rows = u16::try_from(laid.rows.len()).unwrap() + HEADER_ROWS;
            let buf = draw(&doc, &laid, 0, columns, rows, ascii);
            let divider = if ascii { '|' } else { '│' };
            let mut theirs_numbered = 0;
            for (index, row) in laid.rows.iter().enumerate() {
                let y = HEADER_ROWS + u16::try_from(index).unwrap();
                let drawn = content_text(&buf, y, columns);
                let at = format!("{columns} cols ascii={ascii} {row:?}: {drawn:?}");
                if check_row_numbers(&doc, row, &drawn, divider, &at) {
                    theirs_numbered += 1;
                }
            }
            assert!(theirs_numbered > 0, "{columns}: no open theirs row");
        }
    }
    // The taken block (took theirs, result 42) reads 42, not its theirs 128.
    let laid = layout(&doc, Arrangement::Card, 120);
    let buf = draw(&doc, &laid, 0, 120, 40, false);
    let y = HEADER_ROWS + u16::try_from(laid.block_row[1]).unwrap() + 1;
    assert!(starts_with_number(&content_text(&buf, y, 120), 42));
}

/// The number column sits in the same cells whether a row is a bare `Plain`
/// line or a block's own content: both draw through `content_area`, so the
/// digit that starts a number lands at the same `x` (Major 2, m6-02-03
/// review — a `Plain` row used to draw two cells left of a block row's
/// number).
#[test]
fn plain_and_block_numbers_share_one_column() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, Arrangement::Card, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);

    // Row 0 (`Plain`, "38 fn refresh…") and the first `Pair` row of block 0
    // ("39 if expired() {") should both start their digit at the same x.
    let plain_x = first_digit_x(&buf, HEADER_ROWS, 120);
    let block_row = HEADER_ROWS + u16::try_from(laid.block_row[0]).unwrap() + 2; // Header, Labels, then the first Pair row.
    let block_x = first_digit_x(&buf, block_row, 120);
    assert_eq!(
        plain_x, block_x,
        "plain row's number starts at {plain_x:?}, block row's at {block_x:?}"
    );
}

/// The x of the first ASCII digit on row `y`, or `None` if there is none.
fn first_digit_x(buf: &ratatui::buffer::Buffer, y: u16, width: u16) -> Option<u16> {
    (0..width).find(|&x| {
        buf[(x, y)]
            .symbol()
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_digit())
    })
}

/// Only the cursor block's header reverses; every other row of that block —
/// labels, pair, side, end — draws in the plain (non-reversed) style, the
/// `▏` gutter alone carrying the cursor (Major 3, m6-02-03 review: the whole
/// card used to reverse, a worse version of the m5/44 problem the gutter was
/// built to fix).
#[test]
fn only_the_cursor_headers_reverses() {
    let doc = three_block_doc(false); // cursor_block: Some(0)
    let laid = layout(&doc, Arrangement::Card, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);

    let is_reversed = |y: u16| {
        (0..120u16).any(|x| {
            buf[(x, y)]
                .style()
                .add_modifier
                .contains(Modifier::REVERSED)
        })
    };

    let header_row = HEADER_ROWS + u16::try_from(laid.block_row[0]).unwrap();
    assert!(
        is_reversed(header_row),
        "cursor block's header must reverse"
    );

    // Every other row of block 0's card (Labels, Pair x2, End) must not.
    for offset in 1..=4 {
        let y = header_row + offset;
        assert!(
            !is_reversed(y),
            "row {y} (block 0, offset {offset} past its header) reversed"
        );
    }
}

/// A `Layout` built against a stale `DocView` — one whose `blocks()` shrank
/// after a re-read (`e`/`M`, `retake_replaces` shows 3 → 2 blocks) — draws a
/// blank row instead of panicking (Major 5, m6-02-03 review: `block()` used
/// to `unwrap_or_else(|| panic!(...))` in the render path, and side content
/// was indexed unchecked).
#[test]
fn a_layout_naming_a_block_the_doc_no_longer_has_does_not_panic() {
    let doc = three_block_doc(false);
    // Lay out the full three-block doc, then drop block 0 from the doc
    // without re-laying-out — exactly the shape a stale `Layout` takes.
    let laid = layout(&doc, Arrangement::Card, 120);
    let mut stale_doc = doc.clone();
    stale_doc.regions.remove(1); // RegionView::Block(0)

    // Must not panic.
    let _buf = draw(&stale_doc, &laid, 0, 120, 24, false);
}

/// ADR-0043 §1: three panes from 180 columns, the card below.
#[test]
fn arrangement_turns_at_180() {
    assert_eq!(arrangement(179), Arrangement::Card);
    assert_eq!(arrangement(180), Arrangement::ThreePane);
    assert_eq!(arrangement(THREE_PANE_MIN), Arrangement::ThreePane);
}

mod card_colours;
mod finish_row;
mod three_panes;
