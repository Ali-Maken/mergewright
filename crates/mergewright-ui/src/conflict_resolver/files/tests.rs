use mergewright_core::capabilities::ColorDepth;
use mergewright_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::*;
use crate::conflict_resolver::{
    DocView, HEADER_ROWS, MIN_COLUMNS, MIN_ROWS, ResolverScreen, SideNames, SideRoles, layout,
};
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn row(name: &str, state: FileRowLook) -> FileRow {
    FileRow {
        name: sanitize_line(name),
        state,
    }
}

/// The prototype's two-file merge after `o` on `gone.txt`.
fn two_files() -> FileListView {
    FileListView {
        rows: vec![
            row("gone.txt", FileRowLook::Done),
            row("notes.txt", FileRowLook::Open { open: 2, of: 2 }),
        ],
        cursor: 1,
        unavailable: false,
        tail: ListTail::None,
    }
}

/// Twelve files with long enough names that no width lays them out whole.
fn twelve_files(cursor: usize) -> FileListView {
    FileListView {
        rows: (0..12)
            .map(|i| {
                let state = match i % 4 {
                    0 => FileRowLook::Done,
                    1 => FileRowLook::Open { open: 1, of: 3 },
                    2 => FileRowLook::WholeFile {
                        how: sanitize_line("modified on yours · main"),
                    },
                    _ => FileRowLook::Unread,
                };
                row(&format!("src/module_{i:02}/handler.rs"), state)
            })
            .collect(),
        cursor,
        unavailable: false,
        tail: ListTail::None,
    }
}

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::TrueColor, false),
    )
}

fn draw_list(view: &FileListView, placement: Placement, width: u16, height: u16) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    let theme = theme();
    FileList {
        view,
        placement,
        theme: &theme,
        ascii: false,
    }
    .render(area, &mut buf);
    buf
}

fn line(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_owned())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn more_cell(buf: &Buffer, rows: u16) -> bool {
    (0..rows).any(|y| line(buf, y).contains("… ") && line(buf, y).contains(" more"))
}

#[test]
fn placement_by_width() {
    let two = two_files();
    // ease-pass/11: a column `LIST_COLUMN` wide from `INLINE_FROM`.
    for columns in [100, 120, 179, 180, 200] {
        assert_eq!(
            placement(&two, columns),
            Placement::Column { width: LIST_COLUMN },
            "{columns}"
        );
    }
    for columns in [99, 80] {
        assert!(
            matches!(placement(&two, columns), Placement::Strip { .. }),
            "{columns}"
        );
    }
    assert_eq!(
        placement(&two, 99),
        Placement::Strip { rows: 1 },
        "two files fit one row at 99"
    );
    let buf = draw_list(&two, Placement::Strip { rows: 1 }, 80, 1);
    assert_eq!(line(&buf, 0), "files   ✓ gone.txt   ▸ notes.txt 2/2 open");

    for columns in [99, 80] {
        for cursor in [0, 5, 11] {
            twelve_files_keep_the_cursor_shown(columns, cursor);
        }
    }
}

/// Twelve files at `columns`: two or three strip rows, the last cell
/// `… N more`, and the cursor's file on screen.
fn twelve_files_keep_the_cursor_shown(columns: u16, cursor: usize) {
    let many = twelve_files(cursor);
    let Placement::Strip { rows } = placement(&many, columns) else {
        panic!("{columns}: not a strip")
    };
    assert!(
        (2..=3).contains(&rows),
        "{columns}: {rows} rows for 12 files"
    );
    let buf = draw_list(&many, Placement::Strip { rows }, columns, rows);
    assert!(
        more_cell(&buf, rows),
        "{columns}, cursor {cursor}: no `… N more`:\n{}",
        (0..rows)
            .map(|y| line(&buf, y))
            .collect::<Vec<_>>()
            .join("\n")
    );
    let name = format!("module_{cursor:02}/handler.rs");
    let cursor_row = (0..rows).find(|&y| line(&buf, y).contains("▸ "));
    assert!(
        cursor_row.is_some_and(|y| line(&buf, y).contains(&name)),
        "{columns}, cursor {cursor}: the cursor's file is not on screen:\n{}",
        (0..rows)
            .map(|y| line(&buf, y))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Every cell of the list, read back: each position `file_at` names must
/// have that file's name drawn in the span it claims.
#[test]
fn file_at_matches_what_is_drawn() {
    let cases = [
        (two_files(), 200u16, 20u16),
        (two_files(), 120, 20),
        (two_files(), 80, 20),
        (twelve_files(7), 120, 20),
        (twelve_files(0), 80, 20),
        (twelve_files(11), 200, 8),
    ];
    for (view, columns, height) in cases {
        let placed = placement(&view, columns);
        let area = Rect::new(0, 0, columns, height);
        let areas = resolver_areas(area, placed);
        let buf = draw_list(&view, placed, areas.list.width, areas.list.height);
        let mut seen = 0;
        for dy in 0..areas.list.height {
            let mut dx = 0;
            while dx < areas.list.width {
                let Some(i) = file_at(&view, placed, areas.list.width, areas.list.height, dx, dy)
                else {
                    dx += 1;
                    continue;
                };
                let start = dx;
                while dx < areas.list.width
                    && file_at(&view, placed, areas.list.width, areas.list.height, dx, dy)
                        == Some(i)
                {
                    dx += 1;
                }
                let drawn: String = (start..dx).map(|x| buf[(x, dy)].symbol()).collect();
                let name = view.rows[i].name.as_str();
                let tail = &name[name.len().saturating_sub(10)..];
                assert!(
                    drawn.contains(tail),
                    "{columns}: file {i} ({name}) claimed at row {dy}, cols {start}..{dx}, drawn {drawn:?}"
                );
                seen += 1;
            }
        }
        assert!(seen > 0, "{columns}: no file could be clicked");
        // The rule row under a strip and the `│` beside a column are not
        // the list's: they hit nothing.
        match placed {
            Placement::Strip { rows } => assert!(
                (0..columns).all(|dx| file_at(&view, placed, columns, rows, dx, rows).is_none()),
                "{columns}: the rule row hit a file"
            ),
            Placement::Column { width } => assert!(
                (0..height).all(|dy| file_at(&view, placed, width, height, width, dy).is_none()),
                "{columns}: the `│` hit a file"
            ),
        }
    }
}

#[test]
fn areas_put_the_list_above_or_beside_the_doc() {
    let area = Rect::new(0, 1, 120, 38);
    for rows in [1, 2, 3] {
        let areas = resolver_areas(area, Placement::Strip { rows });
        assert_eq!(areas.list, Rect::new(0, 1, 120, rows));
        assert_eq!(areas.doc.y, areas.list.y + rows + 1);
        assert_eq!(areas.doc.x, area.x);
        assert_eq!(areas.doc.width, area.width);
        assert_eq!(areas.doc.bottom(), area.bottom());
    }
    let area = Rect::new(0, 1, 200, 48);
    let areas = resolver_areas(area, Placement::Column { width: 26 });
    assert_eq!(areas.list, Rect::new(0, 1, 26, 48));
    assert_eq!(areas.doc.x, areas.list.x + 26 + 1);
    assert_eq!(areas.doc.y, area.y);
    assert_eq!(areas.doc.height, area.height);
    assert_eq!(areas.doc.right(), area.right());
}

#[test]
fn the_floor_still_leaves_a_document() {
    let many = twelve_files(0);
    let placed = placement(&many, MIN_COLUMNS);
    assert_eq!(placed, Placement::Strip { rows: 3 });
    let areas = resolver_areas(Rect::new(0, 0, MIN_COLUMNS, MIN_ROWS), placed);
    let document = areas.doc.height - HEADER_ROWS;
    assert_eq!(document, MIN_ROWS - HEADER_ROWS - 4);
    assert!(document > 0);
}

/// A long path is cut from the left: the file name survives.
#[test]
fn a_long_name_keeps_its_file_name() {
    let view = FileListView {
        rows: vec![row(
            "a/very/long/directory/path/that/goes/on/and/on/final_name.rs",
            FileRowLook::Open { open: 1, of: 1 },
        )],
        cursor: 0,
        unavailable: false,
        tail: ListTail::None,
    };
    let placed = placement(&view, 200);
    let Placement::Column { width } = placed else {
        panic!("a column at 200")
    };
    assert_eq!(width, LIST_COLUMN, "one width");
    let buf = draw_list(&view, placed, width, 6);
    let drawn = line(&buf, 2);
    assert!(drawn.contains("…"), "{drawn:?}");
    assert!(drawn.contains("final_name.rs"), "{drawn:?}");
}

#[test]
fn an_unavailable_list_says_so() {
    let mut view = two_files();
    view.unavailable = true;
    let placed = placement(&view, 80);
    let Placement::Strip { rows } = placed else {
        panic!("a strip at 80")
    };
    let buf = draw_list(&view, placed, 80, rows);
    assert!(
        line(&buf, 0).starts_with("files: git's list unavailable · the unmerged ones below"),
        "{:?}",
        line(&buf, 0)
    );
    // The column says it under its heading, whole (ease-pass/11).
    let buf = draw_list(
        &view,
        Placement::Column { width: LIST_COLUMN },
        LIST_COLUMN,
        6,
    );
    assert_eq!(line(&buf, 0), "Clashing files");
    assert_eq!(line(&buf, 1), "git's list unavailable");
}

/// The whole widget at 200: a list column, its `│`, then the card.
#[test]
fn the_column_sits_beside_the_card() {
    let doc = DocView {
        title: sanitize_line("notes.txt · 2 conflicts · 2 open"),
        files: two_files(),
        names: SideNames {
            ours: sanitize_line("yours · main"),
            theirs: sanitize_line("theirs · feature"),
            roles: SideRoles::Merge,
        },
        regions: Vec::new(),
        cursor_block: None,
        base_shown: false,
        notice: None,
        finish: None,
    };
    let placed = placement(&doc.files, 200);
    let area = Rect::new(0, 0, 200, 20);
    let areas = resolver_areas(area, placed);
    let laid = layout(
        &doc,
        crate::conflict_resolver::arrangement(200),
        areas.doc.width,
    );
    let theme = theme();
    let mut buf = Buffer::empty(area);
    ResolverScreen {
        doc: &doc,
        layout: &laid,
        top: 0,
        placement: placed,
        theme: &theme,
        ascii: false,
        keys: crate::keymap::Keymap::shipped(),
    }
    .render(area, &mut buf);
    let first = line(&buf, 0);
    assert!(first.starts_with("Clashing files"), "{first:?}");
    let doc_x = usize::from(LIST_COLUMN);
    let rest: String = first.chars().skip(doc_x).collect();
    assert!(
        rest.starts_with("│▸ notes.txt · 2 conflicts · 2 open"),
        "{first:?}"
    );
    assert!(line(&buf, 3).contains("? notes.txt"), "{:?}", line(&buf, 3));
}

mod look;
