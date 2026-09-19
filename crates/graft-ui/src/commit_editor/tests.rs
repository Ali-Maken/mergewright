//! What the editor's character grid says: which field has focus, which
//! options are on, and why the button will not fire.

use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn view<'a>(focus: Field, refusal: Option<&str>) -> CommitEditorView<'a> {
    CommitEditorView {
        title: sanitize_line("COMMIT ── ⎇ main ── 2 files staged"),
        subject: FieldView {
            label: "Subject",
            text: sanitize_line("teach the editor to commit"),
            cursor: (focus == Field::Subject).then_some(26),
            guide_at: Some(u16::try_from(SUBJECT_LIMIT).expect("50 fits in a u16")),
            counter: Some((26, SUBJECT_LIMIT)),
            focused: focus == Field::Subject,
        },
        body: AreaView {
            label: "Body",
            lines: vec![sanitize_line("why this change")],
            cursor: (focus == Field::Body).then_some((0, 15)),
            guide_at: Some(BODY_LIMIT),
            focused: focus == Field::Body,
        },
        options: [
            (Field::Amend, false),
            (Field::SignOff, true),
            (Field::NoVerify, false),
            (Field::AllowEmpty, false),
        ],
        focus,
        refusal: refusal.map(sanitize_line),
        busy: false,
    }
}

fn draw(view: &CommitEditorView<'_>, columns: u16, rows: u16) -> String {
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| {
            frame.render_widget(
                &CommitEditorScreen {
                    view,
                    theme: &theme,
                },
                frame.area(),
            );
        })
        .unwrap_or_else(|why| panic!("a test backend always draws: {why}"));
    let buffer = terminal.backend().buffer().clone();
    (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_grid_says_which_field_has_focus() {
    let subject = draw(&view(Field::Subject, None), 100, 20);
    assert!(subject.contains("▸Subject"), "{subject}");
    assert!(subject.contains(" Body"), "{subject}");

    let body = draw(&view(Field::Body, None), 100, 20);
    assert!(body.contains("▸Body"), "{body}");
    assert!(body.contains(" Subject"), "{body}");

    let button = draw(&view(Field::CommitButton, None), 100, 20);
    assert!(button.contains("▸ [ Commit ]"), "{button}");
}

#[test]
fn the_grid_says_which_options_are_on() {
    let drawn = draw(&view(Field::SignOff, None), 100, 20);
    assert!(drawn.contains("[ ] amend"), "{drawn}");
    assert!(drawn.contains("▸ [x] sign-off"), "{drawn}");
    assert!(drawn.contains("[ ] no-verify"), "{drawn}");
    assert!(drawn.contains("[ ] allow empty"), "{drawn}");
}

#[test]
fn a_refusal_is_drawn_beside_the_button() {
    let drawn = draw(&view(Field::CommitButton, Some("nothing staged")), 100, 20);
    assert!(drawn.contains("[ Commit ]  nothing staged"), "{drawn}");
}

#[test]
fn the_guide_and_the_counter_are_in_the_grid() {
    let drawn = draw(&view(Field::Subject, None), 100, 20);
    assert!(drawn.contains('┊'), "the length guides are drawn: {drawn}");
    assert!(drawn.contains("26/50"), "{drawn}");
}

#[test]
fn a_tiny_area_still_draws_the_title_and_never_panics() {
    for rows in 1..=8 {
        for columns in [1_u16, 8, 40, 100] {
            let drawn = draw(&view(Field::Subject, None), columns, rows);
            assert!(!drawn.is_empty() || columns == 0, "{columns}×{rows}");
        }
    }
    assert!(!fits(Rect::new(0, 0, 80, 6)));
    assert!(fits(Rect::new(0, 0, 80, 7)));
}

#[test]
fn tab_walks_the_four_regions_and_comes_back() {
    let mut focus = Field::Subject;
    focus = focus.next();
    assert_eq!(focus, Field::Body);
    focus = focus.next();
    assert_eq!(focus, Field::Amend, "the options region's first toggle");
    focus = focus.next();
    assert_eq!(focus, Field::CommitButton);
    focus = focus.next();
    assert_eq!(focus, Field::Subject);
}

#[test]
fn shift_tab_walks_the_regions_backwards() {
    assert_eq!(Field::Subject.previous(), Field::CommitButton);
    assert_eq!(Field::CommitButton.previous(), Field::Amend);
    assert_eq!(Field::NoVerify.previous(), Field::Body);
    assert_eq!(Field::Body.previous(), Field::Subject);
}

#[test]
fn arrows_move_between_the_options_and_stop_at_the_ends() {
    assert_eq!(Field::Amend.sibling_option(false), Some(Field::Amend));
    assert_eq!(Field::Amend.sibling_option(true), Some(Field::SignOff));
    assert_eq!(
        Field::AllowEmpty.sibling_option(true),
        Some(Field::AllowEmpty)
    );
    assert_eq!(Field::Subject.sibling_option(true), None);
}
