//! What the look's character grid says (ease-pass/12b, mock
//! `mergewright-merge-screen.html:163-177`).

use mergewright_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn draw(view: &LookView, columns: u16, rows: u16) -> Vec<String> {
    let theme = Theme::new(
        ThemeKind::Dark,
        &test_capabilities(mergewright_core::capabilities::ColorDepth::TrueColor, false),
    );
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| {
            frame.render_widget(
                &MergeLook {
                    view,
                    theme: &theme,
                    keys: Keymap::shipped(),
                },
                frame.area(),
            );
        })
        .unwrap_or_else(|why| panic!("a test backend always draws: {why}"));
    let buffer = terminal.backend().buffer();
    (0..rows)
        .map(|y| {
            (0..columns)
                .map(|x| buffer[(x, y)].symbol().to_owned())
                .collect::<String>()
        })
        .collect()
}

fn look(body: LookBody) -> LookView {
    LookView {
        path: sanitize_line("docs/report.md"),
        body,
        top: 0,
    }
}

fn plain(n: u32, text: &str) -> LookLine {
    LookLine::Plain {
        n,
        text: sanitize_line(text),
    }
}

fn from(n: u32, text: &str, origin: Origin) -> LookLine {
    LookLine::From {
        n,
        text: sanitize_line(text),
        origin,
    }
}

fn combined(path: &str, yours: (u32, u32), theirs: (u32, u32)) -> CombinedFile {
    CombinedFile {
        path: sanitize_line(path),
        yours,
        theirs,
    }
}

#[test]
fn a_clashed_file_draws_numbered_and_read_only() {
    let view = look(LookBody::Clashed {
        lines: vec![plain(1, "# Report"), plain(2, ""), plain(3, "Totals")],
    });
    let grid = draw(&view, 80, 12);
    assert!(
        grid[1].contains("docs/report.md · as it will land · read only"),
        "{grid:#?}"
    );
    assert!(grid[3].contains("1  # Report"), "{grid:#?}");
    assert!(grid[5].contains("3  Totals"), "{grid:#?}");
    assert!(
        grid[11].contains("esc back to the review · y commit the merge"),
        "{grid:#?}"
    );
}

#[test]
fn the_combined_rows_leave_out_zero_parts_and_unchanged_sides() {
    let view = look(LookBody::Combined {
        files: vec![
            combined("src/refund.rs", (6, 0), (2, 1)),
            combined("src/locale.rs", (0, 0), (14, 0)),
            combined("src/receipt.rs", (3, 3), (0, 0)),
        ],
        sample: None,
    });
    let grid = draw(&view, 100, 12);
    let text = grid.join("\n");
    assert!(text.contains("3 files git combined on its own"), "{text}");
    assert!(
        grid[3].contains("✓ src/refund.rs    yours +6      theirs +2 −1"),
        "{text}"
    );
    assert!(
        grid[4].contains("✓ src/locale.rs                  theirs +14"),
        "{text}"
    );
    assert!(grid[5].contains("✓ src/receipt.rs   yours +3 −3"), "{text}");
    assert!(
        !grid[4].contains("yours"),
        "an unchanged side is not drawn: {text}"
    );
    assert!(!grid[5].contains("theirs"), "{text}");
    assert!(!text.contains("−0") && !text.contains("+0"), "{text}");
}

#[test]
fn the_sample_marks_where_each_line_came_from() {
    let view = look(LookBody::Combined {
        files: vec![combined("src/refund.rs", (1, 0), (1, 0))],
        sample: Some((
            sanitize_line("src/refund.rs"),
            vec![
                plain(12, "pub fn refund_line(r: &Receipt) -> String {"),
                from(
                    13,
                    "    let d = locale::date(r.refunded_on);",
                    Origin::Theirs,
                ),
                from(14, "    let amt = cents(r.refund);", Origin::Yours),
            ],
        )),
    });
    let text = draw(&view, 100, 14).join("\n");
    assert!(text.contains("src/refund.rs, around line 12:"), "{text}");
    assert!(
        text.contains("13 +    let d = locale::date(r.refunded_on);   from theirs"),
        "{text}"
    );
    assert!(text.contains("from yours"), "{text}");
}

#[test]
fn reading_and_failed_say_so() {
    let reading = draw(&look(LookBody::Reading), 60, 8).join("\n");
    assert!(reading.contains("reading…"), "{reading}");
    let failed = draw(
        &look(LookBody::Failed(sanitize_line("no merge is in progress"))),
        60,
        8,
    )
    .join("\n");
    assert!(
        failed.contains("can't tell: no merge is in progress"),
        "{failed}"
    );
}
