use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn text(s: &str) -> SanitizedText {
    sanitize_line(s)
}

fn lines(items: &[&str]) -> Vec<SanitizedText> {
    items.iter().map(|s| text(s)).collect()
}

fn open_block(
    index: usize,
    of: usize,
    first_line: usize,
    ours: &[&str],
    theirs: &[&str],
) -> BlockView {
    BlockView {
        index,
        of,
        state: BlockState::Open,
        ours_label: text("HEAD"),
        theirs_label: text("3f2a1b (Add OAuth provider)"),
        ours: lines(ours),
        base: None,
        theirs: lines(theirs),
        first_line,
    }
}

/// A three-block doc: block 0 open (two ours lines, one theirs), block 1
/// taken (theirs), block 2 open with a base attached.
fn three_block_doc(base_shown: bool) -> DocView {
    let mut block2 = open_block(
        2,
        3,
        57,
        &["let tok = self.store.get();"],
        &["let tok = store();"],
    );
    block2.base = Some((text("merge base"), lines(&["let tok = old();"])));

    DocView {
        title: text("src/auth/token.rs · file 1 of 2 · 2 of 3 conflicts open"),
        explainer: Some(text(
            "ours = main, what you are rebasing onto · theirs = the commit being replayed",
        )),
        regions: vec![
            RegionView::Plain {
                first_line: 38,
                lines: lines(&["fn refresh(&mut self) {"]),
            },
            RegionView::Block(open_block(
                0,
                3,
                39,
                &["if expired() {", "self.renew();"],
                &["if is_expired() {", "self.do_refresh();"],
            )),
            RegionView::Plain {
                first_line: 42,
                lines: lines(&["}"]),
            },
            RegionView::Block(BlockView {
                index: 1,
                of: 3,
                state: BlockState::Took(TakeView::Theirs),
                ours_label: text("HEAD"),
                theirs_label: text("3f2a1b"),
                ours: lines(&["old_way();"]),
                base: None,
                theirs: lines(&["new_way();"]),
                first_line: 50,
            }),
            RegionView::Block(block2),
        ],
        cursor_block: Some(0),
        base_shown,
        notice: None,
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
    }
}

/// `layout` at 120 columns: `Plain…`, `Header 0`, `Labels 0`, `max(ours,
/// theirs)` `Pair` rows, `End 0`, `Plain…`, `Header 1`, `Side{Theirs}` rows
/// only, `End 1`, `Header 2`, `Labels`, `Pair…`, `Side{Base}` rows, `End 2`
/// — the acceptance list's order, block by block.
#[test]
fn layout_wide_rows_in_order() {
    let doc = three_block_doc(true);
    let laid = layout(&doc, 120);
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
    let laid = layout(&doc, 80);
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
        let laid = layout(&doc, columns);
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
    let laid = layout(&doc, 120);
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
        let laid = layout(&doc, columns);
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
                    theme: &theme,
                    ascii,
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
    let laid = layout(&doc, 50);
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
    doc.explainer = Some(text("ours = main - theirs = the replayed commit"));
    for region in &mut doc.regions {
        if let RegionView::Block(block) = region {
            block.ours_label = text("HEAD");
            block.theirs_label = text("3f2a1b");
            if let Some((label, _)) = &mut block.base {
                *label = text("merge base");
            }
        }
    }
    for columns in [80u16, 120] {
        let laid = layout(&doc, columns);
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
        first_line: 1,
        lines: (1..=4000).map(|n| text(&format!("line {n}"))).collect(),
    }];
    let doc = DocView {
        title: text("big.txt · file 1 of 1 · 0 of 0 conflicts open"),
        explainer: None,
        regions: std::mem::take(&mut regions),
        cursor_block: None,
        base_shown: false,
        notice: None,
    };
    let laid = layout(&doc, 120);
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
        theme: &theme,
        ascii: false,
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

/// **Fixes a filed disagreement with `docs/design/01-view-mockups.md`
/// §1.5** (continuation session, 2026-09-22; ADR-0042 made §1.5 the
/// resolver's spec): a block's `Header` row now leads with its own
/// `first_line`, the same number the mockup shows (`39`, `57`) — narrower
/// than `numbered`'s 5-wide margin (a single space either side), since this
/// is a hint at the block's position in the file, not a column to read code
/// against line by line.
#[test]
fn header_row_carries_its_block_line_number() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);
    // Block 0: open, first_line 39, the cursor block (gutter mark first).
    let header0 = row_text(
        &buf,
        HEADER_ROWS + u16::try_from(laid.block_row[0]).unwrap(),
        120,
    );
    assert!(
        header0.contains(" 39 ── conflict 1 of 3 · open ──"),
        "{header0:?}"
    );
    // Block 1: took theirs, first_line 50, not the cursor block.
    let header1 = row_text(
        &buf,
        HEADER_ROWS + u16::try_from(laid.block_row[1]).unwrap(),
        120,
    );
    assert!(
        header1.contains(" 50 ── conflict 2 of 3 · took theirs ──"),
        "{header1:?}"
    );
}

/// **Fixes the second half of the same disagreement, corrected again in the
/// m6-02-03 review round (Major 1): `graft-git::conflict::BlockAt::first_line`'s
/// own doc says a resolved block's `first_line` is already "its first taken
/// line once resolved" — not the old `<<<<<<<` marker line.** Once a block
/// is `Took(Theirs)` (or `Both`), its THEIRS lines are on disk at
/// `first_line + i`, not `first_line + 1 + i`: block 1 here has
/// `first_line: 50`, so its one taken line reads `50`, not `51`. Open,
/// THEIRS still draws the blank margin the module doc reserves for it (not
/// yet a line of the working file) — checked here too, so the fix does not
/// leak into the case it must not touch.
#[test]
fn taken_theirs_carries_its_real_line_number_open_theirs_stays_blank() {
    let doc = three_block_doc(false);

    // Block 1 (took theirs, first_line 50): its lone content row is
    // `Header, Side, End` — the content row is one past the header.
    let laid = layout(&doc, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);
    let content_y = HEADER_ROWS + u16::try_from(laid.block_row[1]).unwrap() + 1;
    let taken_row = row_text(&buf, content_y, 120);
    assert!(
        taken_row.trim_start().starts_with("50 new_way();"),
        "{taken_row:?}"
    );

    // Block 0 (open, first_line 39) stacked at 80 columns: its THEIRS
    // section still has no number.
    let narrow = layout(&doc, 80);
    let buf80 = draw(&doc, &narrow, 0, 80, 24, false);
    let theirs_rows: Vec<usize> = narrow
        .rows
        .iter()
        .enumerate()
        .filter(|(_, row)| {
            matches!(
                row,
                ScreenRow::Side {
                    block: 0,
                    side: SideKind::Theirs,
                    ..
                }
            )
        })
        .map(|(i, _)| i)
        .collect();
    assert!(!theirs_rows.is_empty(), "expected block 0's THEIRS rows");
    for row_index in theirs_rows {
        let y = HEADER_ROWS + u16::try_from(row_index).unwrap();
        let row = row_text(&buf80, y, 80);
        assert!(
            !row.trim_start()
                .chars()
                .next()
                .unwrap_or(' ')
                .is_ascii_digit(),
            "open THEIRS drew a line number: {row:?}"
        );
    }
}

/// The number column sits in the same cells whether a row is a bare `Plain`
/// line or a block's own content: both draw through `content_area`, so the
/// digit that starts a number lands at the same `x` (Major 2, m6-02-03
/// review — a `Plain` row used to draw two cells left of a block row's
/// number).
#[test]
fn plain_and_block_numbers_share_one_column() {
    let doc = three_block_doc(false);
    let laid = layout(&doc, 120);
    let buf = draw(&doc, &laid, 0, 120, 24, false);

    // Row 0 (`Plain`, "38 fn refresh…") and the first `Pair` row of block 0
    // ("40 if expired() {") should both start their digit at the same x.
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
    let laid = layout(&doc, 120);
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
    let laid = layout(&doc, 120);
    let mut stale_doc = doc.clone();
    stale_doc.regions.remove(1); // RegionView::Block(0)

    // Must not panic.
    let _buf = draw(&stale_doc, &laid, 0, 120, 24, false);
}
