//! The commit screen's one layout (08s M5): a row the hit-test names is the
//! row drawn, at every scroll.

use super::fixture::{draw, view};
use super::*;
use crate::Cells;

fn screen(columns: u16, rows: u16) -> Cells {
    Cells {
        x: 0,
        y: 0,
        width: columns,
        height: rows,
    }
}

/// Row `y` of a drawn frame, as text.
fn drawn_row(drawn: &str, y: u16) -> String {
    drawn
        .lines()
        .nth(usize::from(y))
        .unwrap_or_default()
        .to_owned()
}

#[test]
fn file_at_skips_headings_and_follows_the_scroll() {
    // 120×30: every line fits. `Going in (3)`, three files, `Staying out
    // (1)`, `.env.local`.
    let whole = view("fix toc", CommitFocus::Files);
    let files = commit_layout(screen(120, 30)).files;
    let at = |row| file_at(&whole, files, files.y + row);
    assert_eq!(
        (0..7).map(at).collect::<Vec<_>>(),
        [None, Some(0), Some(1), Some(2), None, Some(3), None]
    );
    assert_eq!(file_at(&whole, files, files.y - 1), None, "the rule");

    // Two list rows, the cursor on the last file: scrolled to `Staying out
    // (1)` and `.env.local`, as drawn.
    let mut scrolled = view("fix toc", CommitFocus::Files);
    scrolled.cursor = 3;
    let rows = 12;
    let files = commit_layout(screen(120, rows)).files;
    assert_eq!(files.height, 2, "{files:?}");
    let drawn = draw(&scrolled, 120, rows);
    assert!(
        drawn_row(&drawn, files.y).contains("Staying out"),
        "{drawn}"
    );
    assert!(
        drawn_row(&drawn, files.y + 1).contains(".env.local"),
        "{drawn}"
    );
    assert_eq!(file_at(&scrolled, files, files.y), None);
    assert_eq!(file_at(&scrolled, files, files.y + 1), Some(3));

    // A group with nothing in it draws a `nothing` line: not a file.
    let mut none_out = view("fix toc", CommitFocus::Files);
    none_out.files.truncate(3);
    let files = commit_layout(screen(120, 30)).files;
    assert_eq!(file_at(&none_out, files, files.y + 5), None);
}

/// Every row `file_at` names holds that file's path where it is drawn, at
/// both widths (files beside the diff, and alone under 100 columns).
#[test]
fn file_at_names_the_file_drawn_on_its_row() {
    let screen_view = view("fix toc", CommitFocus::Files);
    for (columns, rows) in [(120, 30), (90, 30), (80, 24)] {
        let drawn = draw(&screen_view, columns, rows);
        let files = commit_layout(screen(columns, rows)).files;
        for y in files.y..files.y + files.height {
            if let Some(at) = file_at(&screen_view, files, y) {
                let path = screen_view.files[at].path.as_str();
                assert!(
                    drawn_row(&drawn, y).contains(path),
                    "{columns}x{rows} row {y}: {path}\n{drawn}"
                );
            }
        }
    }
}
