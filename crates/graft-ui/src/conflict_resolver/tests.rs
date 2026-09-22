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

/// The widget draws only `layout.rows[top..top + height]`: a row far past
/// the window is never even reached (a panicking `SanitizedText`-shaped
/// probe would be overkill here; instead the buffer's last row, which the
/// small window cannot reach, stays blank).
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

    let top = 1000;
    let buf = draw(&doc, &laid, top, 120, 10 + HEADER_ROWS, false);
    // Row 0 of the body is `layout.rows[top]` = "line 1001".
    let body_row0: String = (0..120)
        .map(|x| buf[(x, HEADER_ROWS)].symbol().chars().next().unwrap_or(' '))
        .collect();
    assert!(body_row0.contains("1001"), "{body_row0:?}");
    // The buffer never named a line past what the window could hold —
    // `layout.rows[top + height..]` never reaches `draw_row` (no line
    // number from beyond the window appears anywhere in the buffer).
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
