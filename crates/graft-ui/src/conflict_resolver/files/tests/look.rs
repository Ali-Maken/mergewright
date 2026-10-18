//! The list's look: each state's colour (ADR-0044), the unread bullet,
//! and the column and strip as the golden rows draw them (ease-pass/11).
//! Split from `files/tests.rs` to keep it under the size ratchet.

use super::*;

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
        tail: ListTail::None,
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

    // Full words, in the column from 100.
    let buf = draw_list_with(&view, Placement::Column { width: 32 }, (32, 10), false);
    let at = |w: &str| &buf[find_cell(&buf, w)];
    for (words, token) in [
        ("1 open", Token::Open),
        ("⏎ marks done", Token::Open),
        ("keep or delete", Token::Open),
        ("binary", Token::Text),
        ("•", Token::Text),
    ] {
        assert_eq!(at(words).fg, fg(token), "{words:?}");
    }
    // A done file's words: two cells after its padded name.
    let (x, y) = find_cell(&buf, "  done");
    assert_eq!(buf[(x + 2, y)].fg, fg(Token::TextMuted), "`done`");
    // The name keeps today's style; the cursor's BOLD is on both parts.
    assert_eq!(at("a.txt").fg, fg(Token::Text));
    assert!(bold(at("a.txt")) && bold(at("1 open")), "cursor bold");
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
    assert_eq!(drawn, GOLDEN_COLUMN, "{drawn:#?}");
    let buf = draw_list_with(&view, Placement::Strip { rows: 3 }, (90, 3), false);
    let drawn: Vec<String> = (0..3).map(|y| line(&buf, y)).collect();
    assert_eq!(drawn, GOLDEN_STRIP, "{drawn:#?}");
}

/// The merge mock's column (ease-pass/11): `?` or `✓`, the name, the words.
const GOLDEN_COLUMN: [&str; 8] = [
    "Clashing files",
    "",
    "▏? a.txt  1 open",
    " ? b.txt  ⏎ marks done",
    " ? c.txt  keep or delete",
    " ✓ d.txt  done",
    " ? e.bin  binary",
    "   f.txt  •",
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

/// Once every file is done the strip's head says so, `files · all done ✓`,
/// one row or a grid alike; the column's stays `Clashing files`.
#[test]
fn list_head_when_every_file_is_done() {
    let two = all_done(two_files());
    // The column's head is the mock's, done or not (ease-pass/11).
    let buf = draw_list(&two, Placement::Column { width: 28 }, 28, 6);
    assert_eq!(line(&buf, 0), "Clashing files");

    let strip = placement(&two, 99);
    assert!(matches!(strip, Placement::Strip { rows: 1 }), "{strip:?}");
    let buf = draw_list(&two, strip, 99, 1);
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

    // One file still open: the column's head the same.
    let buf = draw_list(&two_files(), Placement::Column { width: 28 }, 28, 6);
    assert_eq!(line(&buf, 0), "Clashing files");
}
