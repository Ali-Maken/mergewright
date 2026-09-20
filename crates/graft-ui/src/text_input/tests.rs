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
        secret: None,
    };
    let lines = rows(80, 1, |area, buf| {
        (&FieldWidget {
            view: &view,
            theme: &theme,
            ascii: false,
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
        secret: None,
    };
    let lines = rows(40, 1, |area, buf| {
        (&FieldWidget {
            view: &view,
            theme: &theme,
            ascii: false,
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

/// A secret field builder: `text`/`cursor` are irrelevant to a secret
/// field's draw (there is no real text to draw from), so they are left at
/// the caller-visible default a real `credential_view` would build —
/// deliberately empty/`None` — to prove the row does not depend on them.
fn secret_view(pulse: bool) -> FieldView<'static> {
    FieldView {
        label: "",
        text: sanitize_line(""),
        cursor: None,
        guide_at: Some(50),
        counter: Some((3, 50)),
        focused: true,
        secret: Some(pulse),
    }
}

fn draw_secret(view: &FieldView<'_>, ascii: bool) -> String {
    let theme = theme();
    let lines = rows(80, 1, |area, buf| {
        (&FieldWidget {
            view,
            theme: &theme,
            ascii,
        })
            .render(area, buf);
    });
    lines[0].clone()
}

/// ADR-0038 §2: a secret field draws exactly [`SECRET_GLYPHS`] mask glyphs,
/// whatever the real length — never `len()` glyphs, and never a character
/// of the secret itself (there is none to leak: `secret_view` passes no
/// text at all, proving the row cannot depend on it).
#[test]
fn masked_prompt_draws_only_glyphs() {
    let drawn = draw_secret(&secret_view(false), false);
    let bullets = drawn.chars().filter(|c| *c == '•').count();
    let pulses = drawn.chars().filter(|c| *c == '◦').count();
    assert_eq!(bullets + pulses, SECRET_GLYPHS, "{drawn:?}");
    // A guide and a counter both name the typed length; masking hides both.
    assert!(!drawn.contains(GUIDE), "{drawn:?}");
    assert!(!drawn.contains('/'), "a counter leaked: {drawn:?}");
}

/// The ASCII renderer tier draws `*`/`+` instead of `•`/`◦` (design note
/// "New UI").
#[test]
fn masked_field_at_the_ascii_tier_draws_stars() {
    let drawn = draw_secret(&secret_view(false), true);
    let stars = drawn.chars().filter(|c| *c == '*').count();
    let plusses = drawn.chars().filter(|c| *c == '+').count();
    assert_eq!(stars + plusses, SECRET_GLYPHS, "{drawn:?}");
    assert_eq!(drawn.chars().filter(|c| *c == '•').count(), 0);
    assert_eq!(drawn.chars().filter(|c| *c == '◦').count(), 0);
}

/// Every cell that differs between `before` and `after`, as
/// `(column, was, now)` — the shape of a visible change, independent of
/// what the rest of the row says.
fn changed_cells(before: &str, after: &str) -> Vec<(usize, char, char)> {
    before
        .chars()
        .zip(after.chars())
        .enumerate()
        .filter(|(_, (was, now))| was != now)
        .map(|(column, (was, now))| (column, was, now))
        .collect()
}

/// Half one of ADR-0038 §2 and of the conductor's binding condition: the
/// masked row must carry **no** length signal at rest.
///
/// The row does flip as you type (the test below), so it cannot also be a
/// constant. What makes it say nothing about the length is that it does not
/// start at a fixed value: it starts at the seed the app draws afresh for
/// every prompt. So the thing to assert is not "both lengths draw the same
/// row" — they do not, for a given seed — but that **the seed, not the
/// length, decides which row**: over the two seeds a 3-character secret and
/// a 30-character one rest at exactly the same two rows, one each. With the
/// seed uniform and unknown, so is the row, for any password.
#[test]
fn the_resting_row_is_decided_by_the_seed_not_the_length() {
    let rows = |text: &str| -> Vec<String> {
        [false, true]
            .into_iter()
            .map(|seed| {
                let mut field = SecretField::new(seed);
                field.insert_str(text);
                draw_secret(&secret_view(field.pulse()), false)
            })
            .collect()
    };
    let short = rows("abc");
    let long = rows(&"x".repeat(30));

    for drawn in short.iter().chain(long.iter()) {
        assert_eq!(
            drawn
                .chars()
                .filter(|c| *c == '\u{2022}' || *c == '\u{25e6}')
                .count(),
            SECRET_GLYPHS,
            "{drawn:?}"
        );
    }
    assert_ne!(
        short[0], short[1],
        "the seed must be what moves the row, or seeding it changes nothing"
    );
    let (mut short_sorted, mut long_sorted) = (short.clone(), long.clone());
    short_sorted.sort();
    long_sorted.sort();
    assert_eq!(
        short_sorted, long_sorted,
        "3 characters and 30 rest at the same two rows, one per seed — so an \
         observer who sees a resting row learns nothing about the length"
    );
}

/// Half two: a keystroke must visibly register — **every** keystroke, not
/// just the first. The row flips on each insert, on a backspace, on a
/// delete and on `Ctrl+U`, from either seed.
///
/// This is the half the group 01 fix round gave up to close the length
/// leak (`pulse` became `!is_empty()`, so the 2nd..nth character changed
/// nothing on screen). Seeding the toggle gets it back without giving the
/// leak back — see [`SecretField::pulse`] for what an observer watching
/// that one prompt throughout can still count.
#[test]
fn every_keystroke_visibly_changes_the_masked_row() {
    for seed in [false, true] {
        let mut field = SecretField::new(seed);
        let mut previous = draw_secret(&secret_view(field.pulse()), false);
        let one_cell_moved = |drawn: &str, previous: &str, what: &str| {
            let delta = changed_cells(previous, drawn);
            assert_eq!(delta.len(), 1, "{what} changed {delta:?}, seed {seed}");
            let (_, was, now) = delta[0];
            assert!(
                matches!(
                    (was, now),
                    ('\u{2022}', '\u{25e6}') | ('\u{25e6}', '\u{2022}')
                ),
                "{what}: {was:?} -> {now:?}"
            );
        };
        for (index, character) in "hunter2!".chars().enumerate() {
            field.insert(character);
            let drawn = draw_secret(&secret_view(field.pulse()), false);
            one_cell_moved(&drawn, &previous, &format!("keystroke {index}"));
            previous = drawn;
        }
        field.backspace();
        let drawn = draw_secret(&secret_view(field.pulse()), false);
        one_cell_moved(&drawn, &previous, "backspace");
        previous = drawn;

        field.home();
        field.delete();
        let drawn = draw_secret(&secret_view(field.pulse()), false);
        one_cell_moved(&drawn, &previous, "delete");
        previous = drawn;

        field.clear();
        let drawn = draw_secret(&secret_view(field.pulse()), false);
        one_cell_moved(&drawn, &previous, "ctrl+u");
    }
}

/// The buffer never reallocates behind `Zeroizing`'s back: that is the
/// whole of [`SecretField`]'s heap story, and the one half of it a safe
/// test can observe (M5 group 01 review, Critical 1 — the old version grew
/// 0 → 8 → 16 → 32, freeing an unwiped copy of the prefix each time).
///
/// Not asserted, because safe Rust cannot look at memory it no longer owns:
/// that the freed blocks read as zero. That rests on zeroize's contract for
/// `Zeroize for Vec` ("ensures the entire capacity of the `Vec` is
/// zeroed"), which is why this test checks the precondition that contract
/// needs — that the block being dropped is one `Zeroizing` still holds.
#[test]
fn secret_field_never_reallocates_silently() {
    let mut field = SecretField::new(false);
    let reserved = field.capacity();
    assert!(reserved >= 64, "a real password fits without growing");

    // Every keystroke up to the reserved size lands in the same buffer.
    for _ in 0..reserved {
        field.insert('x');
        assert_eq!(field.capacity(), reserved, "the buffer moved under us");
    }

    // Past it, growth happens through the replacing path, not `String`'s.
    field.insert('y');
    let grown = field.capacity();
    assert!(grown >= reserved * 2, "grown to {grown} from {reserved}");

    // Shrinking and clearing never reallocate either, so the wipes land in
    // the buffer that is still held.
    field.backspace();
    field.home();
    field.delete();
    assert_eq!(field.capacity(), grown);
    field.clear();
    assert!(field.is_empty());
    assert_eq!(field.capacity(), grown, "`clear` keeps its one allocation");

    // A clone starts with room too, so its next keystroke does not
    // reallocate the copy.
    let mut typed = SecretField::new(false);
    typed.insert_str("hunter2");
    let cloned = typed.clone();
    assert_eq!(cloned.capacity(), typed.capacity());
}

/// `{:?}` on a secret field must not print the typed text: a log line leaks
/// a password as readily as the screen does (CLAUDE.md choice 2).
#[test]
fn debug_redacts_a_secret_fields_text() {
    // `cursor: Some(7)` on purpose: `credential_view` passes `None` for a
    // secret today, but the type still accepts a cursor here and the
    // cursor index *is* the typed length — the redaction is what covers
    // that, so it stays tested (m5-01 review, Minor 9).
    let view = FieldView {
        label: "Password",
        text: sanitize_line(""),
        cursor: Some(7),
        guide_at: None,
        counter: None,
        focused: true,
        secret: Some(true),
    };
    let printed = format!("{view:?}");
    assert!(!printed.contains("hunter2"), "{printed}");
    assert!(printed.contains("<redacted>"), "{printed}");
    assert!(
        !printed.contains("Some(7)"),
        "the cursor is the typed length: {printed}"
    );

    let unmasked = FieldView {
        text: sanitize_line("hunter2"),
        cursor: Some(7),
        secret: None,
        ..view
    };
    let printed_unmasked = format!("{unmasked:?}");
    assert!(printed_unmasked.contains("hunter2"));
    assert!(printed_unmasked.contains("Some(7)"), "{printed_unmasked}");
}

/// [`SecretField`]'s edits mirror [`TextField`]'s, judged by length (there
/// is no `text()` to compare directly) and by [`SecretField::into_bytes`].
#[test]
fn secret_field_edits_by_length() {
    let mut field = SecretField::new(false);
    field.insert('a');
    field.insert(WIDE);
    field.insert('b');
    assert_eq!(field.len(), 3);
    assert_eq!(field.cursor(), 3);
    field.left();
    field.backspace();
    assert_eq!(field.len(), 2, "the wide character went whole");
    assert_eq!(field.cursor(), 1);
    field.insert_str("XY");
    assert_eq!(field.len(), 4);
    field.home();
    field.delete();
    assert_eq!(field.len(), 3);
    field.end();
    field.insert('\u{7}'); // control character: refused at the door
    assert_eq!(field.len(), 3);
    assert_eq!(
        String::from_utf8(field.into_bytes().to_vec()).unwrap(),
        "XYb",
        "into_bytes round-trips what editing actually left behind"
    );

    let mut cleared = SecretField::new(false);
    cleared.insert_str("hunter2");
    cleared.clear();
    assert!(cleared.is_empty());
    assert_eq!(cleared.cursor(), 0);
    assert_eq!(cleared.into_bytes().len(), 0);
}

/// `Clone` makes its own zeroizing copy — editing one does not touch the
/// other, and both still round-trip through `into_bytes` (`AppState` clones
/// this field once per event while a prompt is open).
#[test]
fn secret_field_clone_is_independent() {
    let mut field = SecretField::new(false);
    field.insert_str("hunter2");
    let mut cloned = field.clone();
    cloned.insert('!');
    assert_eq!(field.len(), 7);
    assert_eq!(cloned.len(), 8);
    assert_eq!(
        String::from_utf8(field.into_bytes().to_vec()).unwrap(),
        "hunter2"
    );
    assert_eq!(
        String::from_utf8(cloned.into_bytes().to_vec()).unwrap(),
        "hunter2!"
    );
}

/// `Debug` prints a constant — not the text and **not the length**, which
/// ADR-0038 §2 puts in the same list as the drawn row (M5 group 01 review,
/// Major 1: it used to print `SecretField(7)`, and `PromptField` derives
/// `Debug`, so `AppState` dumps read the length straight off). `PartialEq`/
/// `Eq` still compare by length, which is never printed, so two different
/// secrets of the same length are `==` (the contract `Command`'s tests need
/// — see `graft::update::credential::Answer`'s own `PartialEq`).
#[test]
fn secret_field_debug_and_eq_never_compare_content() {
    let mut a = SecretField::new(false);
    a.insert_str("hunter2");
    let mut b = SecretField::new(false);
    b.insert_str("orange17");
    assert_ne!(a, b, "different lengths");
    b.backspace();
    assert_eq!(
        a, b,
        "same length, different content — still equal by design"
    );
    let printed = format!("{a:?}");
    assert_eq!(printed, "SecretField(..)");
    assert!(!printed.contains("hunter2"));
    assert!(
        !printed.chars().any(char::is_numeric),
        "a digit in a secret field's Debug is a length: {printed}"
    );
    // The same constant whatever is in it: nothing to difference.
    let mut long = SecretField::new(false);
    long.insert_str(&"x".repeat(30));
    assert_eq!(format!("{long:?}"), printed);
    assert_eq!(format!("{:?}", SecretField::new(false)), printed);
}
