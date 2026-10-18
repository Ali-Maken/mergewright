//! The branch picker's box: placement, both column sets, the states that
//! stand in for rows, scrolling and the bottom line.

use mergewright_term::sanitize_line;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(mergewright_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn row(kind: PickerKind, name: &str, relation: &str) -> PickerRow {
    PickerRow {
        kind,
        name: sanitize_line(name),
        who: sanitize_line("Sara"),
        age: "1h".to_owned(),
        subject: sanitize_line("Login page: remember me box"),
        relation: relation.to_owned(),
    }
}

fn listed() -> PickerView {
    PickerView {
        query: sanitize_line(""),
        body: PickerBody::Rows {
            rows: vec![
                row(PickerKind::Before, "feature-a", ""),
                row(PickerKind::Local, "main", "tracks origin/main"),
                row(PickerKind::Local, "feature-b", "built on feature-a"),
                row(
                    PickerKind::Remote,
                    "origin/sara/login-page",
                    "no copy of yours yet",
                ),
                row(PickerKind::Here, "login-page", ""),
            ],
            selected: 0,
        },
        footer: "5 branches".to_owned(),
        note: None,
        wide: true,
    }
}

fn drawn(view: &PickerView, width: u16, height: u16) -> Vec<String> {
    let frame = Rect::new(0, 0, width, height);
    let area = picker_area(frame);
    let mut buf = Buffer::empty(frame);
    render_picker(&mut buf, area, view, &theme());
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
fn the_box_is_the_lower_sixty_percent_and_at_most_150_wide() {
    let area = picker_area(Rect::new(0, 0, 187, 50));
    assert_eq!(area.width, 150);
    assert_eq!(area.height, 30);
    assert_eq!(area.bottom(), 50);
    assert_eq!(area.x, (187 - 150) / 2);
    let narrow = picker_area(Rect::new(0, 3, 100, 30));
    assert_eq!(narrow.width, 96);
    assert_eq!(narrow.height, 18);
    assert_eq!(narrow.bottom(), 33, "the lower part of the graph");
}

#[test]
fn wide_rows_carry_kind_name_who_age_subject_and_relation() {
    let lines = drawn(&listed(), 187, 50);
    let text = lines.join("\n");
    assert!(text.contains("SWITCH BRANCH"), "{text}");
    assert!(text.contains("> "), "the prompt row:\n{text}");
    let before = lines
        .iter()
        .find(|line| line.contains("feature-a") && line.contains("before"))
        .unwrap_or_else(|| panic!("no before row:\n{text}"));
    assert!(before.contains('\u{25b8}'), "the cursor on the first row");
    assert!(before.contains("\u{21a9} before"), "{before}");
    let main = lines
        .iter()
        .find(|line| line.contains("\u{25cf} local") && line.contains("main"))
        .unwrap_or_else(|| panic!("no main row:\n{text}"));
    for part in ["Sara", "1h", "Login page", "tracks origin/main"] {
        assert!(main.contains(part), "{part} missing: {main}");
    }
    assert!(text.contains("\u{25cc} remote"), "{text}");
    assert!(text.contains("no copy of yours yet"), "{text}");
    assert!(text.contains("built on feature-a"), "{text}");
    let here = lines
        .iter()
        .find(|line| line.contains("\u{b7} here"))
        .unwrap_or_else(|| panic!("no here row:\n{text}"));
    assert!(here.contains("you're on it"), "{here}");
    assert!(text.contains("5 branches"), "{text}");
}

#[test]
fn the_kind_column_lines_up() {
    let lines = drawn(&listed(), 187, 50);
    let text = lines.join("\n");
    let names: Vec<usize> = ["before", "local", "remote", "here"]
        .iter()
        .flat_map(|kind| {
            lines
                .iter()
                .filter(move |line| line.contains(&format!(" {kind} ")))
                .map(move |line| {
                    let chars: Vec<char> = line.chars().collect();
                    // The name starts after the kind word and its gap.
                    let word = line[..line.find(&format!(" {kind} ")).unwrap_or(0)]
                        .chars()
                        .count()
                        + 1;
                    let after = word + kind.chars().count();
                    after
                        + chars[after..]
                            .iter()
                            .take_while(|character| **character == ' ')
                            .count()
                })
        })
        .collect();
    assert_eq!(names.len(), 5, "{text}");
    assert!(
        names.windows(2).all(|pair| pair[0] == pair[1]),
        "names start in one column: {names:?}"
    );
}

#[test]
fn narrow_rows_carry_glyph_name_and_age_only() {
    let mut view = listed();
    view.wide = false;
    let lines = drawn(&view, 100, 30);
    let text = lines.join("\n");
    let main = lines
        .iter()
        .find(|line| line.contains(" main"))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(main.contains('\u{25cf}') && main.contains("1h"), "{main}");
    assert!(!text.contains("local"), "no kind word:\n{text}");
    assert!(!text.contains("Sara"), "no who:\n{text}");
    assert!(!text.contains("tracks"), "no relation:\n{text}");
    assert!(!text.contains("Login page"), "no subject:\n{text}");
}

#[test]
fn the_selected_row_scrolls_into_view() {
    let rows: Vec<PickerRow> = (0..40)
        .map(|at| row(PickerKind::Local, &format!("branch-{at:02}"), ""))
        .collect();
    let view = PickerView {
        body: PickerBody::Rows { rows, selected: 30 },
        wide: false,
        ..listed()
    };
    let text = drawn(&view, 100, 24).join("\n");
    assert!(text.contains("\u{25b8} \u{25cf} branch-30"), "{text}");
    assert!(!text.contains("branch-00"), "scrolled:\n{text}");
}

#[test]
fn reading_failing_empty_and_no_match_each_say_so() {
    for (body, words) in [
        (PickerBody::Reading, "reading branches\u{2026}"),
        (
            PickerBody::Failed(sanitize_line("git exploded")),
            "can't list branches \u{b7} git exploded",
        ),
        (PickerBody::NoBranches, "no branches yet \u{b7} esc closes"),
        (
            PickerBody::NoMatch(sanitize_line("xyz")),
            "no branch matches \"xyz\" \u{b7} esc closes",
        ),
    ] {
        let view = PickerView { body, ..listed() };
        let text = drawn(&view, 120, 40).join("\n");
        assert!(text.contains(words), "{words}:\n{text}");
    }
}

#[test]
fn a_note_takes_the_bottom_line() {
    let view = PickerView {
        note: Some(sanitize_line("wait for the fetch to finish")),
        ..listed()
    };
    let text = drawn(&view, 120, 40).join("\n");
    assert!(text.contains("wait for the fetch to finish"), "{text}");
    assert!(
        !text.contains("5 branches"),
        "the note replaces it:\n{text}"
    );
}

#[test]
fn the_query_is_drawn_after_the_prompt() {
    let view = PickerView {
        query: sanitize_line("log"),
        ..listed()
    };
    let text = drawn(&view, 120, 40).join("\n");
    assert!(text.contains("> log\u{258f}"), "{text}");
}

/// A click finds the row drawn on its line, scrolled or not, and nothing on
/// the chrome: the border, the query, the bottom line.
#[test]
fn a_click_finds_the_row_drawn_on_its_line() {
    // 10 rows tall: 6 list rows, lines 2..=7.
    assert_eq!(row_at(10, 20, 0, 0), None, "the top border");
    assert_eq!(row_at(10, 20, 0, 1), None, "the query");
    assert_eq!(row_at(10, 20, 0, 2), Some(0));
    assert_eq!(row_at(10, 20, 0, 7), Some(5));
    assert_eq!(row_at(10, 20, 0, 8), None, "the bottom line");
    assert_eq!(row_at(10, 3, 0, 5), None, "below the last row");
    // Cursor on row 9: rows 4..=9 are drawn.
    assert_eq!(first_drawn(10, 9), 4);
    assert_eq!(row_at(10, 20, 9, 2), Some(4));
    assert_eq!(row_at(10, 20, 9, 7), Some(9));
}

/// Spell sweep (c), ticket m8/10c: the picker is a text field, so its
/// `closes` line spells the shipped `app.back`.
#[test]
fn the_closing_words_spell_the_shipped_back_key() {
    use crate::keymap::sweep::assert_shipped;
    assert_shipped(&closes("no branches yet"), &[&["app.back"]]);
}
