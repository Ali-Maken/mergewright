//! What the editor's character grid says at R1's three breakpoints, and that
//! the hit test lands on the rows drawn. Colour is not asserted anywhere
//! (CLAUDE.md rule 5): every distinction below is a character.

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

fn row(mark: RowMark, short: &str, subject: &str, note: Option<&str>) -> EditorRow {
    EditorRow {
        mark,
        short: short.to_owned(),
        subject: sanitize_line(subject),
        note: note.map(sanitize_line),
        moved: false,
    }
}

fn after(branch: Option<&str>, mark: AfterMark, subject: &str, note: Option<&str>) -> AfterRow {
    AfterRow {
        branch: branch.map(sanitize_line),
        mark,
        subject: sanitize_line(subject),
        note: note.map(sanitize_line),
    }
}

const MELDS: &str = "melds into the row below";

/// R1's mock, plus a reword, an edit and a drop so every glyph is on
/// screen, and a moved row.
#[allow(clippy::too_many_lines)] // one hand-built view fixture, not logic
fn view() -> RewriteEditorView {
    let mut fixup = row(
        RowMark::Fixup,
        "4c3b2a1",
        "fixup! Add OAuth callback",
        Some("melds into the row below \u{b7} marked by its name"),
    );
    fixup.moved = true;
    RewriteEditorView {
        title: sanitize_line(
            "REWRITE feature-a \u{b7} 8 commits since it left main \u{b7} newest first \u{b7} nothing written until y",
        ),
        rows: vec![
            row(
                RowMark::Squash,
                "7f6e5d4",
                "Handle expired tokens",
                Some(MELDS),
            ),
            row(RowMark::Squash, "6e5d4c3", "wip", Some(MELDS)),
            row(RowMark::Pick, "5d4c3b2", "wip: tests", None),
            fixup,
            row(RowMark::Pick, "3b2a1f0", "Add OAuth callback", None),
            row(
                RowMark::Reword,
                "3a1f0e9",
                "Add the login form",
                Some("new message"),
            ),
            row(
                RowMark::Edit,
                "2b1f0e9",
                "Add the session store",
                Some("stops here"),
            ),
            row(RowMark::Drop, "2a2f0e9", "Debug logging", Some("left out")),
            row(
                RowMark::Base {
                    label: sanitize_line("main"),
                },
                "2a1f0e9",
                "Bump version to 2.4",
                Some("stays"),
            ),
        ],
        cursor: 2,
        field: None,
        message: Some(MessageBlock {
            label: sanitize_line(
                "MESSAGE of the squashed commit \u{b7} r on \"wip: tests\" changes it",
            ),
            lines: ["wip: tests", "", "Handle expired tokens", "", "wip"]
                .into_iter()
                .map(sanitize_line)
                .collect(),
        }),
        right: RightSide::After(AfterView::Ready {
            rows: vec![
                after(
                    Some("feature-b"),
                    AfterMark::New,
                    "Show token expiry in settings",
                    Some("built on yours \u{b7} follows"),
                ),
                after(None, AfterMark::New, "Settings page skeleton", None),
                after(
                    Some("feature-a"),
                    AfterMark::NewTip,
                    "wip: tests",
                    Some("3 in one"),
                ),
                after(
                    None,
                    AfterMark::New,
                    "Add OAuth callback",
                    Some("fixup melded in"),
                ),
                after(
                    Some("main"),
                    AfterMark::Unchanged {
                        short: "2a1f0e9".to_owned(),
                    },
                    "Bump version to 2.4",
                    Some("unchanged"),
                ),
            ],
            facts: vec![
                AfterFact::plain(sanitize_line(
                    "built on yours: feature-b (Sara, 2 commits) follows",
                )),
                AfterFact {
                    text: sanitize_line(
                        "! feature-c stays on 86eea6e \u{b7} would conflict in one.txt",
                    ),
                    tone: FactTone::Warning,
                },
                AfterFact {
                    text: sanitize_line("feature-d stays on 86eea6e \u{b7} the default branch"),
                    tone: FactTone::Muted,
                },
                AfterFact::plain(sanitize_line(
                    "on origin: feature-a \u{b7} P will need force after",
                )),
                AfterFact::plain(sanitize_line("no conflicts expected")),
            ],
        }),
        narrow_shows_right: false,
        notice: None,
        ascii: false,
    }
}

fn draw(view: &RewriteEditorView, columns: u16, rows: u16) -> Vec<String> {
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    let theme = theme();
    terminal
        .draw(|frame| draw_rewrite_editor(frame, frame.area(), view, &theme))
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    let buffer = terminal.backend().buffer();
    (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

fn screen(lines: &[String]) -> String {
    lines.join("\n")
}

/// The line containing `needle`, and its index.
fn line_with<'a>(lines: &'a [String], needle: &str) -> (usize, &'a str) {
    lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.contains(needle))
        .map_or_else(
            || panic!("no line holds {needle:?} in\n{}", lines.join("\n")),
            |(at, line)| (at, line.as_str()),
        )
}

/// The non-colour carriers every width shows on the rows side.
fn assert_row_carriers(lines: &[String]) {
    let text = screen(lines);
    for carrier in [
        "squash \u{2193}",
        "fixup \u{2193}",
        "reword \u{270e}",
        "edit \u{23f8}",
        "drop \u{2717}",
        "\u{2195} moved",
        "\u{2500} main",
        "\u{25b8} pick",
        "\u{250a} wip: tests",
        "MESSAGE of the squashed commit",
    ] {
        assert!(text.contains(carrier), "{carrier:?} missing from\n{text}");
    }
}

/// The carriers the AFTER side shows.
fn assert_after_carriers(lines: &[String]) {
    let text = screen(lines);
    for carrier in [
        "AFTER y \u{b7} not written yet",
        "\u{25cb} new",
        "\u{25cf} new",
        "built on yours \u{b7} follows",
        "3 in one",
        "no conflicts expected",
    ] {
        assert!(text.contains(carrier), "{carrier:?} missing from\n{text}");
    }
}

#[test]
fn at_187_the_after_side_stands_right_of_the_rule() {
    let lines = draw(&view(), 187, 50);
    assert_row_carriers(&lines);
    assert_after_carriers(&lines);
    assert!(lines[0].starts_with("REWRITE feature-a \u{b7} 8 commits since it left main"));
    // The rule is one column, the same one on every body row.
    let rule = usize::from(LEFT_WIDTH);
    for line in &lines[..lines.len() - 1] {
        assert_eq!(
            line.chars().nth(rule),
            Some('\u{2502}'),
            "no rule at {rule} in {line:?}"
        );
    }
    // The AFTER header shares the title's row, right of the rule.
    assert!(lines[0].contains("\u{2502} AFTER y \u{b7} not written yet"));
    // Two `squash ↓` rows over the kept `pick`.
    let (squash, _) = line_with(&lines, "7f6e5d4");
    let (kept, _) = line_with(&lines, "5d4c3b2");
    assert_eq!(kept, squash + 2);
    // The message is under the rows, left of the rule.
    let (base, _) = line_with(&lines, "2a1f0e9  Bump");
    let (label, label_line) = line_with(&lines, "MESSAGE of the squashed commit");
    assert!(label > base);
    assert!(label_line.find("MESSAGE") < Some(rule));
}

#[test]
fn from_120_to_179_the_after_side_is_below_the_rows_and_labelled() {
    let lines = draw(&view(), 140, 40);
    assert_row_carriers(&lines);
    assert_after_carriers(&lines);
    let text = screen(&lines);
    assert!(!text.contains('\u{2502}'), "no rule below 180:\n{text}");
    let (base, _) = line_with(&lines, "\u{2500} main");
    let (header, _) = line_with(&lines, "AFTER y \u{b7} not written yet");
    let (label, _) = line_with(&lines, "MESSAGE of the squashed commit");
    assert!(
        base < header && header < label,
        "rows, AFTER, message:\n{text}"
    );
}

#[test]
fn below_120_the_rows_stand_alone_and_tab_swaps_to_after() {
    let lines = draw(&view(), 100, 30);
    assert_row_carriers(&lines);
    let text = screen(&lines);
    assert!(!text.contains("AFTER y"), "rows only below 120:\n{text}");
    assert!(!text.contains('\u{2502}'));

    let swapped = RewriteEditorView {
        narrow_shows_right: true,
        ..view()
    };
    let lines = draw(&swapped, 100, 30);
    assert_after_carriers(&lines);
    assert!(
        !screen(&lines).contains("7f6e5d4"),
        "AFTER only once swapped"
    );
}

#[test]
fn a_short_screen_shows_four_message_lines_and_an_ellipsis() {
    let lines = draw(&view(), 100, 24);
    let text = screen(&lines);
    assert!(text.contains("\u{250a} Handle expired tokens"), "{text}");
    assert!(
        !text.contains("\u{250a} wip\n"),
        "the fifth line is cut:\n{text}"
    );
    assert!(text.contains("\u{250a} \u{2026}"), "{text}");
}

#[test]
fn the_notice_is_on_the_last_row() {
    let noticed = RewriteEditorView {
        notice: Some(sanitize_line(
            "the base stays \u{b7} it isn't yours to rewrite",
        )),
        ..view()
    };
    let lines = draw(&noticed, 140, 40);
    assert_eq!(
        lines.last().map(String::as_str),
        Some(" the base stays \u{b7} it isn't yours to rewrite")
    );
}

#[test]
fn the_field_is_the_line_under_its_row() {
    let with_field = RewriteEditorView {
        field: Some(FieldView {
            row: 2,
            text: sanitize_line("Tests for the token flow"),
            caret: 5,
        }),
        ..view()
    };
    let lines = draw(&with_field, 140, 40);
    let (kept, _) = line_with(&lines, "5d4c3b2");
    assert!(
        lines[kept + 1].contains("Tests for the token flow"),
        "{:?}",
        lines[kept + 1]
    );
    // The row below the field is the next row.
    assert!(lines[kept + 2].contains("4c3b2a1"));
}

#[test]
fn checking_and_refusals_say_so_in_the_after_header_and_body() {
    let checking = RewriteEditorView {
        right: RightSide::After(AfterView::Checking),
        ..view()
    };
    assert!(screen(&draw(&checking, 140, 40)).contains("AFTER y \u{b7} checking\u{2026}"));
    let refused = RewriteEditorView {
        right: RightSide::After(AfterView::Refused(sanitize_line(
            "every commit dropped \u{b7} the branch would be its base again",
        ))),
        ..view()
    };
    assert!(screen(&draw(&refused, 140, 40)).contains("every commit dropped"));
}

#[test]
fn the_changes_side_lists_files_with_counts() {
    let changes = RewriteEditorView {
        right: RightSide::Changes {
            short: "5d4c3b2".to_owned(),
            files: ChangesView::Files(vec![
                (sanitize_line("src/auth.rs"), Some((12, 3))),
                (sanitize_line("logo.png"), None),
            ]),
        },
        ..view()
    };
    let text = screen(&draw(&changes, 187, 50));
    assert!(text.contains("CHANGES of 5d4c3b2"), "{text}");
    assert!(text.contains("src/auth.rs  +12 \u{2212}3"), "{text}");
    assert!(text.contains("logo.png     binary"), "no counts: {text}");
}

#[test]
fn ascii_has_a_stand_in_for_every_glyph() {
    let ascii = RewriteEditorView {
        ascii: true,
        ..view()
    };
    let text = screen(&draw(&ascii, 187, 50));
    // The view's own words (`·`) are the app's; every glyph the widget adds
    // has a stand-in.
    for glyph in ['↓', '✎', '⏸', '✗', '↕', '│', '┊', '▸', '─', '○', '●'] {
        assert!(
            !text.contains(glyph),
            "{glyph:?} drawn at the ASCII tier:\n{text}"
        );
    }
    for carrier in [
        "squash v",
        "drop x",
        "<> moved",
        "| AFTER",
        ": wip: tests",
        "o new",
    ] {
        assert!(text.contains(carrier), "{carrier:?} missing:\n{text}");
    }
}

#[test]
fn rewrite_row_at_matches_the_drawn_rows() {
    for (columns, rows) in [(187, 50), (140, 40), (100, 30), (100, 12)] {
        let view = view();
        let area = Rect::new(0, 0, columns, rows);
        let lines = draw(&view, columns, rows);
        for (index, row) in view.rows.iter().enumerate() {
            // The rows' own column: the AFTER side shows a sha too.
            let hit = lines
                .iter()
                .position(|line| {
                    line.chars()
                        .take(24)
                        .collect::<String>()
                        .contains(&row.short)
                })
                .and_then(|y| u16::try_from(y).ok());
            match hit {
                Some(y) => assert_eq!(
                    rewrite_row_at(area, &view, 5, y),
                    Some(index),
                    "{columns}x{rows}: row {index} drawn at {y}"
                ),
                // Scrolled off a short screen: nothing says it is there.
                None => assert!(
                    (0..rows).all(|y| rewrite_row_at(area, &view, 5, y) != Some(index)),
                    "{columns}x{rows}: row {index} undrawn yet hit"
                ),
            }
        }
        // The title row and the notice row are not rows.
        assert_eq!(rewrite_row_at(area, &view, 5, 0), None);
        assert_eq!(rewrite_row_at(area, &view, 5, rows - 1), None);
    }
}

#[test]
fn a_field_line_is_not_a_row_to_click() {
    let with_field = RewriteEditorView {
        field: Some(FieldView {
            row: 2,
            text: sanitize_line("x"),
            caret: 1,
        }),
        ..view()
    };
    let area = Rect::new(0, 0, 140, 40);
    let lines = draw(&with_field, 140, 40);
    let (kept, _) = line_with(&lines, "5d4c3b2");
    let y = u16::try_from(kept).unwrap_or(0);
    assert_eq!(rewrite_row_at(area, &with_field, 5, y), Some(2));
    assert_eq!(rewrite_row_at(area, &with_field, 5, y + 1), None);
    assert_eq!(rewrite_row_at(area, &with_field, 5, y + 2), Some(3));
}

#[test]
fn a_long_list_scrolls_to_keep_the_cursor_on_screen() {
    let long = RewriteEditorView {
        cursor: 8,
        ..view()
    };
    let lines = draw(&long, 100, 10);
    let text = screen(&lines);
    assert!(
        text.contains("\u{2500} main"),
        "the cursor's (base) row is drawn:\n{text}"
    );
}

/// ADR-0058 §3: on the cursor row's `selection` band every character — the
/// accent `▸`, the action word, the muted sha and note, the subject — clears
/// 4.5:1 in dark B, whichever row the cursor is on.
#[test]
fn text_on_the_cursor_row_is_legible() {
    let theme = theme();
    for cursor in 0..view().rows.len() {
        let shown = RewriteEditorView { cursor, ..view() };
        let mut terminal = Terminal::new(TestBackend::new(187, 40)).expect("test backend");
        terminal
            .draw(|frame| draw_rewrite_editor(frame, frame.area(), &shown, &theme))
            .expect("draws");
        let checked =
            crate::test_support::assert_legible_on_selection(terminal.backend().buffer(), &theme);
        assert!(
            checked > 10,
            "cursor {cursor}: the row is drawn ({checked})"
        );
    }
}

/// Each fact in its tone (m8/14c, ruling R4): a warning in `Warning`, a
/// muted one in `TextMuted`, the rest in `Text`. Colour unverified in
/// captures; this reads the buffer's style.
#[test]
fn each_fact_draws_in_its_tone() {
    let view = view();
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(187, 30))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| draw_rewrite_editor(frame, frame.area(), &view, &theme))
        .unwrap_or_else(|why| panic!("drawing into a test backend cannot fail: {why}"));
    let buffer = terminal.backend().buffer();
    let style_of = |needle: &str| {
        (0..buffer.area.height)
            .find_map(|y| {
                let line: String = (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect();
                let at = line.find(needle)?;
                let x = u16::try_from(line[..at].chars().count()).ok()?;
                Some(buffer[(x, y)].style().fg)
            })
            .unwrap_or_else(|| panic!("{needle:?} drawn"))
    };
    let fg = |token| crate::paint::foreground(&theme, token).fg;
    assert_eq!(style_of("! feature-c"), fg(Token::Warning));
    assert_eq!(style_of("feature-d stays"), fg(Token::TextMuted));
    assert_eq!(style_of("no conflicts expected"), fg(Token::Text));
}
