//! The acceptance for `text_input`: character boundaries survive editing
//! around wide characters and combining marks, words move the way a person
//! expects, and the multi-line area round-trips its own text.

use graft_term::sanitize_line;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::widgets::Widget;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::{Theme, ThemeKind};

/// A wide (fullwidth) character and a base with a combining mark: the two
/// shapes a byte-indexed cursor would cut in half.
const WIDE: char = '世';
const MARK: char = '\u{0301}'; // combining acute

#[test]
fn a_wide_character_is_inserted_and_deleted_whole() {
    let mut field = TextField::new("ab");
    field.left();
    field.insert(WIDE);
    assert_eq!(field.text(), "a世b");
    assert_eq!(field.cursor(), 2);
    field.backspace();
    assert_eq!(field.text(), "ab");
    assert_eq!(field.cursor(), 1);
}

#[test]
fn a_combining_mark_is_its_own_character_to_the_cursor() {
    let mut field = TextField::new("");
    field.insert('e');
    field.insert(MARK);
    assert_eq!(field.len(), 2, "the mark counts as its own character");
    field.backspace();
    assert_eq!(field.text(), "e", "the mark goes, the base stays");
    // And the text is still valid UTF-8 the way `text()` promises.
    assert!(field.text().is_char_boundary(field.text().len()));
}

#[test]
fn delete_forwards_over_a_wide_character() {
    let mut field = TextField::new("a世b");
    field.home();
    field.right();
    field.delete();
    assert_eq!(field.text(), "ab");
    assert_eq!(field.cursor(), 1);
}

#[test]
fn word_left_walks_back_over_the_gap_then_the_word() {
    let mut field = TextField::new("foo  bar");
    assert_eq!(field.cursor(), 8);
    field.word_left();
    assert_eq!(field.cursor(), 5, "the start of `bar`");
    field.word_left();
    assert_eq!(
        field.cursor(),
        0,
        "past the two spaces, to the start of `foo`"
    );
    field.word_left();
    assert_eq!(field.cursor(), 0, "and stops there");
}

#[test]
fn word_right_lands_on_the_start_of_the_next_word() {
    let mut field = TextField::new("foo  bar");
    field.home();
    field.word_right();
    assert_eq!(field.cursor(), 5);
    field.word_right();
    assert_eq!(field.cursor(), 8);
}

#[test]
fn a_control_character_is_never_typed_into_a_field() {
    let mut field = TextField::new("a\nb\u{1b}c");
    assert_eq!(
        field.text(),
        "abc",
        "newline and escape dropped at the door"
    );
    field.insert('\u{7}');
    field.insert_str("d\ne");
    assert_eq!(field.text(), "abcde");
}

#[test]
fn text_round_trips_through_newlines() {
    for original in ["", "one", "one\ntwo", "one\n\nthree", "trailing\n"] {
        let area = TextArea::new(original);
        assert_eq!(area.text(), original, "round trip of {original:?}");
    }
}

#[test]
fn newline_splits_the_line_at_the_cursor() {
    let mut area = TextArea::new("onetwo");
    area.home();
    area.word_right();
    assert_eq!(area.cursor(), (0, 6));
    area.home();
    for _ in 0..3 {
        area.right();
    }
    area.newline();
    assert_eq!(area.text(), "one\ntwo");
    assert_eq!(area.cursor(), (1, 0));
}

#[test]
fn backspace_at_the_start_of_a_line_joins_it_upwards() {
    let mut area = TextArea::new("one\ntwo");
    area.home();
    assert_eq!(area.cursor(), (1, 0));
    area.backspace();
    assert_eq!(area.text(), "onetwo");
    assert_eq!(area.cursor(), (0, 3));
}

#[test]
fn delete_at_the_end_of_a_line_pulls_the_next_one_up() {
    let mut area = TextArea::new("one\ntwo");
    area.up();
    area.end();
    area.delete();
    assert_eq!(area.text(), "onetwo");
}

#[test]
fn append_line_on_an_empty_area_adds_no_leading_blank() {
    let mut area = TextArea::new("");
    area.append_line("Co-authored-by: ");
    assert_eq!(area.text(), "Co-authored-by: ");
    assert_eq!(area.cursor(), (0, 16), "the cursor lands after the trailer");
}

#[test]
fn append_line_on_a_written_area_starts_a_new_line() {
    let mut area = TextArea::new("why this change");
    area.append_line("Co-authored-by: ");
    assert_eq!(area.text(), "why this change\nCo-authored-by: ");
    assert_eq!(area.cursor(), (1, 16));
}

#[test]
fn a_pasted_run_breaks_lines_the_way_enter_does() {
    let mut area = TextArea::new("");
    area.insert_str("first\r\nsecond\nthird");
    assert_eq!(area.text(), "first\nsecond\nthird");
    assert_eq!(area.cursor(), (2, 5));
}

#[test]
fn up_and_down_clamp_the_column_to_the_shorter_line() {
    let mut area = TextArea::new("longer line\nab");
    assert_eq!(area.cursor(), (1, 2));
    area.up();
    assert_eq!(area.cursor(), (0, 2));
    area.end();
    area.down();
    assert_eq!(area.cursor(), (1, 2), "clamped to the short line's end");
}

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

/// Draw `widget` into a `width × height` buffer and return its rows as
/// plain strings — the character grid a capture would show.
fn rows(width: u16, height: u16, draw: impl FnOnce(Rect, &mut Buffer)) -> Vec<String> {
    let area = Rect::new(0, 0, width, height);
    let mut buf = Buffer::empty(area);
    draw(area, &mut buf);
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

#[test]
fn a_field_draws_its_label_guide_and_counter() {
    let theme = theme();
    let view = FieldView {
        label: "Subject",
        text: sanitize_line("add the commit editor"),
        cursor: Some(21),
        guide_at: Some(50),
        counter: Some((21, 50)),
        focused: true,
    };
    let lines = rows(80, 1, |area, buf| {
        (&FieldWidget {
            view: &view,
            theme: &theme,
        })
            .render(area, buf);
    });
    assert!(
        lines[0].starts_with("▸Subject add the commit editor"),
        "{lines:?}"
    );
    assert!(
        lines[0].contains("21/50"),
        "the counter is drawn: {lines:?}"
    );
    assert!(
        lines[0].contains(GUIDE),
        "the 50-character guide is drawn: {lines:?}"
    );
}

#[test]
fn a_field_scrolls_so_the_cursor_stays_on_screen() {
    let theme = theme();
    let long = "x".repeat(200);
    let view = FieldView {
        label: "Subject",
        text: sanitize_line(&long),
        cursor: Some(200),
        guide_at: None,
        counter: None,
        focused: true,
    };
    let lines = rows(40, 1, |area, buf| {
        (&FieldWidget {
            view: &view,
            theme: &theme,
        })
            .render(area, buf);
    });
    // 40 cells, 9 of them the label, leaving 31: the cursor sits one past
    // the last character, so 30 of them are drawn and the last cell is where
    // the next one would go.
    assert_eq!(
        lines[0].chars().filter(|c| *c == 'x').count(),
        30,
        "{lines:?}"
    );
}

#[test]
fn an_area_scrolls_so_the_cursor_line_stays_on_screen() {
    let theme = theme();
    let body: Vec<_> = (0..10)
        .map(|n| sanitize_line(&format!("line{n}")))
        .collect();
    let view = AreaView {
        label: "Body",
        lines: body,
        cursor: Some((9, 0)),
        guide_at: None,
        focused: true,
    };
    // 1 label row + 3 text rows: lines 7, 8, 9.
    let lines = rows(40, 4, |area, buf| {
        (&AreaWidget {
            view: &view,
            theme: &theme,
        })
            .render(area, buf);
    });
    assert_eq!(lines[0], "▸Body");
    assert!(lines[1].ends_with("line7"), "{lines:?}");
    assert!(lines[3].ends_with("line9"), "{lines:?}");
}
