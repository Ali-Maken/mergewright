//! The incoming list (ease-pass 10): header, rows newest first as given,
//! the cursor, scrolling, and the footer.

use graft_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::{IncomingList, IncomingRow, IncomingView};
use crate::keymap::Keymap;
use crate::test_support::terminal as test_capabilities;
use crate::theme::{Theme, ThemeKind};

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn row(subject: &str, who: &str, age: &str, is_merge: bool) -> IncomingRow {
    IncomingRow {
        subject: sanitize_line(subject),
        who: sanitize_line(who),
        age: sanitize_line(age),
        is_merge,
    }
}

fn view() -> IncomingView {
    IncomingView {
        from: sanitize_line("sara/fix-toc"),
        rows: vec![
            row("Fix the contents links", "Sara", "2h", false),
            row("Merge the numbering", "Omar", "5h", true),
            row("Move the summary up", "Sara", "1d", false),
        ],
        capped: false,
        withheld: false,
        cursor: 0,
        ascii: false,
    }
}

fn drawn(view: &IncomingView, width: u16, height: u16) -> Vec<String> {
    let frame = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(frame);
    (&IncomingList {
        view,
        theme: &theme(),
        keys: Keymap::shipped(),
    })
        .render(frame, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

#[test]
fn the_list_names_the_commits_that_come_in_with_who_and_age() {
    let view = view();
    let rows = drawn(&view, 80, view.rows_wanted());
    assert_eq!(rows.len(), 7);
    assert!(rows[1].contains("│ The 3 commits that come in from sara/fix-toc"));
    assert!(
        rows[3].starts_with("│ ▸●  Fix the contents links"),
        "{}",
        rows[3]
    );
    assert!(rows[3].trim_end().ends_with("Sara  2h │"), "{}", rows[3]);
    assert!(
        rows[4].starts_with("│  ◉  Merge the numbering"),
        "{}",
        rows[4]
    );
    assert!(rows[5].contains("Sara  1d"), "{}", rows[5]);
    assert!(
        rows[6].contains("╰ esc back to the preview  y merge ─"),
        "{}",
        rows[6]
    );
}

#[test]
fn j_and_k_move_the_cursor_clamped() {
    let mut view = view();
    view.up();
    assert_eq!(view.cursor, 0);
    view.down();
    assert_eq!(view.cursor, 1);
    view.down();
    view.down();
    assert_eq!(view.cursor, 2);
    let rows = drawn(&view, 80, view.rows_wanted());
    assert!(
        rows[5].starts_with("│ ▸●  Move the summary up"),
        "{}",
        rows[5]
    );
}

#[test]
fn a_short_rect_scrolls_to_keep_the_cursor_on_screen() {
    let mut view = view();
    view.cursor = 2;
    let rows = drawn(&view, 80, 5);
    assert!(rows[3].contains("▸●  Move the summary up"), "{rows:#?}");
}

#[test]
fn withheld_drops_y_and_capped_adds_a_last_row() {
    let mut view = view();
    view.withheld = true;
    view.capped = true;
    view.ascii = true;
    let rows = drawn(&view, 80, view.rows_wanted());
    assert!(rows[1].contains("The 3+ commits that come in"));
    assert!(rows[3].starts_with("│ ▸*  Fix"), "{}", rows[3]);
    assert!(rows[6].starts_with("│ …"), "{}", rows[6]);
    let footer = rows.last().expect("a footer");
    assert!(footer.contains("╰ esc back to the preview ─"), "{footer}");
    assert!(!footer.contains("y merge"));
}
