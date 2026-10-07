//! The palette's box (§1.16): groups in order, the key column, the config
//! id by width, placement, scrolling and the empty result.

use graft_term::sanitize_line;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn action(key: &str, label: &str, id: &'static str) -> PaletteRow {
    PaletteRow {
        label: sanitize_line(label),
        key: Some(KeyLabel(key.to_owned())),
        config: Some(ActionId(id)),
        kind: RowKind::Action,
    }
}

fn group(name: &str, rows: Vec<PaletteRow>) -> (SanitizedText, Vec<PaletteRow>) {
    (sanitize_line(name), rows)
}

/// §1.16's drawing, as a view.
fn help(wide: bool) -> PaletteView {
    PaletteView {
        mode: PaletteMode::Here(ContextId::Graph),
        query: sanitize_line(""),
        query_cursor: 0,
        groups: vec![
            group(
                "IN PROGRESS",
                vec![
                    action("c", "continue the rebase", "operation.continue"),
                    action("s", "skip this commit", "operation.skip"),
                ],
            ),
            group(
                "HERE",
                vec![
                    action("m", "merge the selected branch into yours", "graph.merge"),
                    action("—", "rebase yours onto the selected commit", "graph.rebase"),
                ],
            ),
            group(
                "EVERYWHERE",
                vec![
                    action("⌃P", "find anything (palette)", "app.palette"),
                    action("u", "undo the last thing you did", "app.undo"),
                ],
            ),
        ],
        cursor: 0,
        wide,
        placeholder: None,
    }
}

/// The box drawn into a buffer, for a cell's style.
fn buffer(view: &PaletteView) -> Buffer {
    let frame = Rect::new(0, 0, 120, 36);
    let mut buf = Buffer::empty(frame);
    Palette {
        view,
        theme: &theme(),
    }
    .render(palette_area(Rect::new(0, 1, 120, 34), frame), &mut buf);
    buf
}

/// Where `needle` starts in `buf`, and that cell's foreground.
fn style_of(buf: &Buffer, needle: &str) -> ratatui::style::Color {
    let area = buf.area;
    for y in 0..area.height {
        let line: String = (0..area.width)
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect();
        if let Some(at) = line.find(needle) {
            let column = line[..at].chars().count();
            return buf[(u16::try_from(column).expect("a column"), y)].fg;
        }
    }
    panic!("`{needle}` not drawn");
}

fn muted() -> ratatui::style::Color {
    foreground(&theme(), Token::TextMuted)
        .fg
        .expect("a muted colour")
}

/// P-find-1: the empty `⌃P` field says what it finds, muted; typed, the
/// words are gone.
#[test]
fn the_empty_field_draws_its_placeholder_muted() {
    let words = "type to find actions, branches, tags, files, commits";
    let view = PaletteView {
        mode: PaletteMode::Everything,
        placeholder: Some(words),
        ..help(true)
    };
    let buf = buffer(&view);
    assert_eq!(style_of(&buf, words), muted());
    let (_, lines) = drawn(&view, 120, 36);
    assert!(line_with(&lines, words).contains("› "), "on the query row");
    let typed = PaletteView {
        query: sanitize_line("f"),
        query_cursor: 1,
        ..view
    };
    let (_, lines) = drawn(&typed, 120, 36);
    assert!(!lines.join("\n").contains(words), "gone once typed");
}

/// `+N more`, `reading…`, `searching…`: muted, no key; a branch row is
/// drawn as text, its label in the label column.
#[test]
fn more_rows_draw_muted_and_find_rows_as_text() {
    let row = |label: &str, kind| PaletteRow {
        label: sanitize_line(label),
        key: None,
        config: None,
        kind,
    };
    let view = PaletteView {
        mode: PaletteMode::Everything,
        groups: vec![group(
            "BRANCHES",
            vec![
                row("feature", RowKind::Branch),
                row("+3 more", RowKind::More),
            ],
        )],
        ..help(true)
    };
    let buf = buffer(&view);
    assert_eq!(style_of(&buf, "+3 more"), muted());
    assert_ne!(style_of(&buf, "feature"), muted());
}

/// 12b: a FILES row mutes its directory and draws its base name as text;
/// a COMMITS row mutes its sha and draws its subject as text.
#[test]
fn file_rows_mute_the_directory_and_commit_rows_the_sha() {
    let row = |label: &str, kind| PaletteRow {
        label: sanitize_line(label),
        key: None,
        config: None,
        kind,
    };
    let view = PaletteView {
        mode: PaletteMode::Everything,
        groups: vec![
            group("FILES", vec![row("src/update/rebase.rs", RowKind::File)]),
            group(
                "COMMITS",
                vec![row("a1b2c3d  Tidy the rebase", RowKind::Commit)],
            ),
        ],
        cursor: 5,
        ..help(true)
    };
    let buf = buffer(&view);
    assert_eq!(style_of(&buf, "src/update/"), muted());
    assert_ne!(style_of(&buf, "rebase.rs"), muted());
    assert_eq!(style_of(&buf, "a1b2c3d"), muted());
    assert_ne!(style_of(&buf, "Tidy the rebase"), muted());
}

/// The whole frame, `width`×`height`, with the box drawn over a pane that
/// is the frame less its top and bottom rows.
fn drawn(view: &PaletteView, width: u16, height: u16) -> (Rect, Vec<String>) {
    let frame = Rect::new(0, 0, width, height);
    let over = Rect::new(0, 1, width, height - 2);
    let area = palette_area(over, frame);
    let mut buf = Buffer::empty(frame);
    Palette {
        view,
        theme: &theme(),
    }
    .render(area, &mut buf);
    let lines = (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect();
    (area, lines)
}

fn line_with<'a>(lines: &'a [String], needle: &str) -> &'a str {
    lines
        .iter()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line with {needle:?}:\n{}", lines.join("\n")))
}

#[test]
fn groups_and_rows_in_the_order_of_the_drawing() {
    for (width, height) in [(80, 24), (120, 36), (200, 60)] {
        let (_, lines) = drawn(&help(width >= WIDE_FROM), width, height);
        let text = lines.join("\n");
        let order: Vec<usize> = [
            "keys here",
            "IN PROGRESS",
            "continue the rebase",
            "skip this commit",
            "HERE",
            "merge the selected",
            "rebase yours",
            "EVERYWHERE",
            "find anything",
            "undo the last",
        ]
        .iter()
        .map(|needle| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{width}x{height}: no {needle:?}:\n{text}"))
        })
        .collect();
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "{width}x{height} out of order:\n{text}"
        );
        assert!(
            line_with(&lines, "keys here").contains("esc close"),
            "{text}"
        );
        assert!(
            line_with(&lines, "keys here").trim_start().starts_with('╭'),
            "{text}"
        );
        assert!(line_with(&lines, "› ").contains('│'), "{text}");
    }
}

#[test]
fn the_key_column_is_six_cells_and_the_cursor_marks_its_row() {
    let (area, lines) = drawn(&help(true), 120, 36);
    let continue_row = line_with(&lines, "continue the rebase");
    let skip_row = line_with(&lines, "skip this commit");
    let x = usize::from(area.x);
    let cells_of = |line: &str| line.chars().skip(x).collect::<String>();
    assert!(
        cells_of(continue_row).starts_with("│▸c     continue"),
        "{continue_row}"
    );
    assert!(cells_of(skip_row).starts_with("│ s     skip"), "{skip_row}");
    // An unbound row reads `—`.
    assert!(cells_of(line_with(&lines, "rebase yours")).starts_with("│ —     rebase"));
    // A two-cell key still leaves the label in the same column.
    assert!(cells_of(line_with(&lines, "find anything (")).starts_with("│ ⌃P    find"));
}

#[test]
fn the_config_id_is_drawn_from_100_columns_and_dropped_below() {
    for (width, height, wide) in [(120, 36, true), (200, 60, true), (80, 24, false)] {
        let (_, lines) = drawn(&help(wide), width, height);
        let merge = line_with(&lines, "merge the selected");
        assert_eq!(
            merge.contains("graph.merge"),
            wide,
            "{width}x{height}:\n{}",
            lines.join("\n")
        );
        if wide {
            assert!(
                merge.ends_with("graph.merge  │"),
                "right-aligned: {merge:?}"
            );
        }
    }
}

#[test]
fn the_box_is_at_most_96_wide_and_centred_from_100_columns_full_width_below() {
    let pane = Rect::new(0, 1, 120, 34);
    let frame = Rect::new(0, 0, 120, 36);
    let area = palette_area(pane, frame);
    assert_eq!(area.width, 96);
    assert_eq!(area.x, 12, "centred over the pane");
    let right_pane = Rect::new(80, 1, 120, 58);
    let area = palette_area(right_pane, Rect::new(0, 0, 200, 60));
    assert_eq!(
        (area.x, area.width),
        (80 + 12, 96),
        "centred over that pane"
    );
    let narrow = palette_area(Rect::new(40, 1, 40, 22), Rect::new(0, 0, 80, 24));
    assert_eq!((narrow.x, narrow.width), (0, 80), "the frame's full width");
    // Drawn: the corners where the area says.
    let (_, lines) = drawn(&help(true), 200, 60);
    let top = line_with(&lines, "keys here");
    assert_eq!(top.chars().position(|c| c == '╭'), Some(52));
    assert_eq!(top.chars().count(), 52 + 96);
}

#[test]
fn the_box_is_as_tall_as_its_lines() {
    let (_, lines) = drawn(&help(true), 120, 36);
    // Borders, query, 3 headers, 6 rows.
    let boxed = lines
        .iter()
        .filter(|line| line.contains('│') || line.contains('╭') || line.contains('╰'))
        .count();
    assert_eq!(boxed, 2 + 1 + 3 + 6, "{}", lines.join("\n"));
}

#[test]
fn the_cursor_row_is_scrolled_into_view_with_40_rows() {
    let rows: Vec<PaletteRow> = (0..40)
        .map(|n| action("x", &format!("row number {n}"), "graph.merge"))
        .collect();
    let mut view = PaletteView {
        mode: PaletteMode::Everything,
        groups: vec![group("ACTIONS", rows)],
        cursor: 39,
        ..help(true)
    };
    let (_, lines) = drawn(&view, 120, 24);
    let text = lines.join("\n");
    assert!(line_with(&lines, "row number 39").contains('▸'), "{text}");
    assert!(
        !text.contains("row number 0 "),
        "scrolled past the top:\n{text}"
    );
    assert!(text.contains("find anything"), "{text}");
    // Within view from the top: nothing scrolls.
    view.cursor = 3;
    let (_, lines) = drawn(&view, 120, 24);
    assert!(line_with(&lines, "row number 3").contains('▸'));
    assert!(lines.join("\n").contains("ACTIONS"));
    // The box never grows past the pane less two rows.
    assert!(lines[0].trim().is_empty() && lines[23].trim().is_empty());
}

#[test]
fn an_empty_result_draws_one_line_that_says_how_out() {
    let view = PaletteView {
        query: sanitize_line("zzz"),
        query_cursor: 3,
        groups: Vec::new(),
        ..help(true)
    };
    let (_, lines) = drawn(&view, 120, 36);
    assert!(line_with(&lines, "› zzz").contains('│'));
    line_with(&lines, "nothing matches · esc close");
    assert_eq!(
        palette_row_at(
            palette_area(Rect::new(0, 1, 120, 34), Rect::new(0, 0, 120, 36)),
            &view,
            50,
            4
        ),
        None
    );
}

#[test]
fn a_click_finds_the_row_it_landed_on() {
    let view = help(true);
    let area = palette_area(Rect::new(0, 1, 120, 34), Rect::new(0, 0, 120, 36));
    // Box top at area.y; query +1; IN PROGRESS header +2; continue +3.
    let y = area.y;
    assert_eq!(palette_row_at(area, &view, area.x + 10, y + 3), Some(0));
    assert_eq!(
        palette_row_at(area, &view, area.x + 10, y + 2),
        None,
        "a header"
    );
    assert_eq!(palette_row_at(area, &view, area.x + 10, y + 6), Some(2));
    assert_eq!(
        palette_row_at(area, &view, area.x, y + 3),
        None,
        "the border"
    );
    assert_eq!(
        palette_row_at(area, &view, area.x + 10, y + 1),
        None,
        "the query"
    );
}
