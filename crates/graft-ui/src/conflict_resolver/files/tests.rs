use graft_core::capabilities::ColorDepth;
use graft_term::sanitize_line;
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
    for columns in [180, 200] {
        assert!(
            matches!(placement(&two, columns), Placement::Column { width } if (22..=32).contains(&width)),
            "{columns}: {:?}",
            placement(&two, columns)
        );
    }
    for columns in [179, 160, 120, 80] {
        assert!(
            matches!(placement(&two, columns), Placement::Strip { .. }),
            "{columns}"
        );
    }
    assert_eq!(
        placement(&two, 120),
        Placement::Strip { rows: 1 },
        "two files fit one row at 120"
    );
    let buf = draw_list(&two, Placement::Strip { rows: 1 }, 120, 1);
    assert_eq!(
        line(&buf, 0),
        "files   gone.txt · done ✓    ▸ notes.txt · 2 of 2 open"
    );
    let buf = draw_list(&two, Placement::Strip { rows: 1 }, 80, 1);
    assert_eq!(line(&buf, 0), "files   ✓ gone.txt   ▸ notes.txt 2/2 open");

    for columns in [120, 80] {
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
    };
    let placed = placement(&view, 200);
    let Placement::Column { width } = placed else {
        panic!("a column at 200")
    };
    assert_eq!(width, 32, "clamped to the widest");
    let buf = draw_list(&view, placed, width, 6);
    let drawn = line(&buf, 2);
    assert!(drawn.contains("…"), "{drawn:?}");
    assert!(drawn.contains("final_name.rs"), "{drawn:?}");
}

#[test]
fn an_unavailable_list_says_so() {
    let mut view = two_files();
    view.unavailable = true;
    let placed = placement(&view, 120);
    let Placement::Strip { rows } = placed else {
        panic!("a strip at 120")
    };
    let buf = draw_list(&view, placed, 120, rows);
    assert!(
        line(&buf, 0).starts_with("files: git's list unavailable · the unmerged ones below"),
        "{:?}",
        line(&buf, 0)
    );
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
    let laid = layout(&doc, placed.arrangement(), areas.doc.width);
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
    assert!(first.starts_with("FILES · 2 conflicted"), "{first:?}");
    assert!(
        first.contains("│▸ notes.txt · 2 conflicts · 2 open"),
        "{first:?}"
    );
    assert!(line(&buf, 3).contains("▸ notes.txt"), "{:?}", line(&buf, 3));
}

// ------------------------------------------------ state colours (ADR-0044)

/// Every state once, the cursor on the open file.
fn six_states() -> FileListView {
    FileListView {
        rows: vec![
            row("a.txt", FileRowLook::Open { open: 1, of: 2 }),
            row("b.txt", FileRowLook::AllTaken),
            row(
                "c.txt",
                FileRowLook::WholeFile {
                    how: sanitize_line("deleted by them"),
                },
            ),
            row("d.txt", FileRowLook::Done),
            row("e.bin", FileRowLook::Binary),
            row("f.txt", FileRowLook::Unread),
        ],
        cursor: 0,
        unavailable: false,
    }
}

fn draw_list_with(
    view: &FileListView,
    placement: Placement,
    (width, height): (u16, u16),
    ascii: bool,
) -> Buffer {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    let theme = theme();
    FileList {
        view,
        placement,
        theme: &theme,
        ascii,
    }
    .render(area, &mut buf);
    buf
}

/// `(x, y)` of the first cell of `word` anywhere in `buf`.
fn find_cell(buf: &Buffer, word: &str) -> (u16, u16) {
    let n = word.chars().count();
    for y in 0..buf.area.height {
        let cells: Vec<String> = (0..buf.area.width)
            .map(|x| buf[(x, y)].symbol().to_owned())
            .collect();
        if let Some(x) =
            (0..=cells.len().saturating_sub(n)).find(|&x| cells[x..x + n].concat() == word)
        {
            return (u16::try_from(x).expect("small"), y);
        }
    }
    panic!("{word:?} not drawn")
}

#[test]
fn file_list_state_colours() {
    let theme = theme();
    let fg = |t: Token| crate::paint::color(theme.token(t));
    let bold = |c: &ratatui::buffer::Cell| c.modifier.contains(Modifier::BOLD);
    let view = six_states();

    // Full words, in the column at 180+.
    let buf = draw_list_with(&view, Placement::Column { width: 32 }, (32, 10), false);
    let at = |w: &str| &buf[find_cell(&buf, w)];
    for (words, token) in [
        ("1 of 2 open", Token::Open),
        ("⏎ marks done", Token::Open),
        ("keep or delete", Token::Open),
        ("done ✓", Token::TextMuted),
        ("binary", Token::Text),
        ("•", Token::Text),
    ] {
        assert_eq!(at(words).fg, fg(token), "{words:?}");
    }
    // The name keeps today's style; the cursor's BOLD is on both parts.
    assert_eq!(at("a.txt").fg, fg(Token::Text));
    assert!(bold(at("a.txt")) && bold(at("1 of 2 open")), "cursor bold");
    assert!(!bold(at("b.txt")) && !bold(at("⏎ marks done")));
    assert_eq!(at("d.txt").fg, fg(Token::Text), "a done file's name");

    // Compact forms, in the strip below 100 columns.
    let buf = draw_list_with(&view, Placement::Strip { rows: 3 }, (90, 3), false);
    let at = |w: &str| &buf[find_cell(&buf, w)];
    for (words, token) in [
        ("1/2 open", Token::Open),
        ("keep?", Token::Open),
        ("bin", Token::Text),
    ] {
        assert_eq!(at(words).fg, fg(token), "{words:?}");
    }
    // `⏎` and `✓` alone: find them after their file's name.
    let (x, y) = find_cell(&buf, "b.txt ⏎");
    assert_eq!(buf[(x + 6, y)].fg, fg(Token::Open), "compact ⏎");
    let (x, y) = find_cell(&buf, "✓ d.txt");
    assert_eq!(buf[(x, y)].fg, fg(Token::TextMuted), "compact ✓");
    assert_eq!(buf[(x + 2, y)].fg, fg(Token::Text), "its name");
}

#[test]
fn unread_file_is_marked_with_a_bullet() {
    let view = six_states();
    let buf = draw_list_with(&view, Placement::Column { width: 32 }, (32, 10), false);
    assert!(line(&buf, 7).ends_with("f.txt  •"), "{:?}", line(&buf, 7));
    let buf = draw_list_with(&view, Placement::Column { width: 32 }, (32, 10), true);
    assert!(line(&buf, 7).ends_with("f.txt  ~"), "{:?}", line(&buf, 7));

    // `…` keeps its other jobs: the overflow cell and a left-cut path.
    for columns in [80, 120] {
        let buf = draw_list(&twelve_files(3), Placement::Strip { rows: 3 }, columns, 3);
        assert!(more_cell(&buf, 3), "{columns}: `… N more`");
        let drawn = (0..3).map(|y| line(&buf, y)).collect::<Vec<_>>().join("\n");
        assert!(drawn.contains('•'), "{columns}: unread marked\n{drawn}");
    }
    let buf = draw_list(&twelve_files(0), Placement::Column { width: 24 }, 24, 6);
    let drawn = line(&buf, 2);
    assert!(drawn.contains('…'), "a left-cut path: {drawn:?}");
}

/// The text a list draws is today's, byte for byte, except the unread
/// mark: the colour split never moves a character.
#[test]
fn file_list_text_is_unchanged_but_the_mark() {
    let view = six_states();
    let buf = draw_list_with(&view, Placement::Column { width: 32 }, (32, 8), false);
    let drawn: Vec<String> = (0..8).map(|y| line(&buf, y)).collect();
    let wide = draw_list_with(&view, Placement::Strip { rows: 3 }, (170, 3), false);
    let wide: Vec<String> = (0..3).map(|y| line(&wide, y)).collect();
    assert_eq!(wide, GOLDEN_WIDE_STRIP, "{wide:#?}");
    assert_eq!(drawn, GOLDEN_COLUMN, "{drawn:#?}");
    let buf = draw_list_with(&view, Placement::Strip { rows: 3 }, (90, 3), false);
    let drawn: Vec<String> = (0..3).map(|y| line(&buf, y)).collect();
    assert_eq!(drawn, GOLDEN_STRIP, "{drawn:#?}");
}

/// Drawn before the colour split, with `…` where `•` now stands.
const GOLDEN_COLUMN: [&str; 8] = [
    "FILES · 6 conflicted",
    "",
    "▏▸ a.txt  1 of 2 open",
    "   b.txt  ⏎ marks done",
    "   c.txt  keep or delete",
    "   d.txt  done ✓",
    "   e.bin  binary",
    "   f.txt  •",
];

/// Drawn before the colour split, with `…` where `•` now stands.
const GOLDEN_WIDE_STRIP: [&str; 3] = [
    "files   ▸ a.txt · 1 of 2 open    b.txt · ⏎ marks done    c.txt · keep or delete    d.txt · done ✓    e.bin · binary    f.txt · •",
    "",
    "",
];

/// Drawn before the colour split, with `…` where `•` now stands.
const GOLDEN_STRIP: [&str; 3] = [
    "files   ▸ a.txt 1/2 open   b.txt ⏎   c.txt keep?   ✓ d.txt   e.bin bin   f.txt •",
    "",
    "",
];

// ------------------------------------ every file done (m6-conflicts/60)

/// `files` with every row done, the cursor on the last.
fn all_done(files: FileListView) -> FileListView {
    FileListView {
        rows: files
            .rows
            .into_iter()
            .map(|row| FileRow {
                state: FileRowLook::Done,
                ..row
            })
            .collect(),
        ..files
    }
}

/// Once every file is done the list's head says so: the column
/// `FILES · all <n> done ✓`, the strip `files · all done ✓`, one row or a
/// grid alike.
#[test]
fn list_head_when_every_file_is_done() {
    let two = all_done(two_files());
    let buf = draw_list(&two, Placement::Column { width: 28 }, 28, 6);
    assert_eq!(line(&buf, 0), "FILES · all 2 done ✓");

    let strip = placement(&two, 120);
    assert!(matches!(strip, Placement::Strip { rows: 1 }), "{strip:?}");
    let buf = draw_list(&two, strip, 120, 1);
    assert!(
        line(&buf, 0).starts_with("files · all done ✓"),
        "{:?}",
        line(&buf, 0)
    );

    let twelve = all_done(twelve_files(11));
    let grid = placement(&twelve, 80);
    let Placement::Strip { rows } = grid else {
        panic!("80 columns is a strip: {grid:?}")
    };
    assert!(rows > 1, "twelve long names make a grid: {rows}");
    let buf = draw_list(&twelve, grid, 80, rows);
    let drawn: Vec<String> = (0..rows).map(|y| line(&buf, y)).collect();
    assert!(
        drawn.iter().any(|row| row.contains("files · all done ✓")),
        "{drawn:#?}"
    );

    // One file still open: the heads as before.
    let buf = draw_list(&two_files(), Placement::Column { width: 28 }, 28, 6);
    assert_eq!(line(&buf, 0), "FILES · 2 conflicted");
}
