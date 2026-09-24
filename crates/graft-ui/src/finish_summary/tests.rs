//! What the summary's character grid says at each width: one column below
//! 160, two from 160, one line below the minimum, and a long diff cut at 12
//! rows with the summary's own words.

use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::detail::{FileStatusView, HunkView, LineView};
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(graft_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn row(path: &str, what: &str) -> SummaryRow {
    SummaryRow {
        path: sanitize_line(path),
        what: sanitize_line(what),
    }
}

/// The ticket's fixture, built by hand: two files resolved, one combined.
fn view<'a>() -> FinishView<'a> {
    FinishView {
        title: sanitize_line("Finish · merging feature → main · nothing is written until ⏎"),
        by_you: vec![
            row("gone.txt", "kept · yours · main"),
            row("notes.txt", "theirs · feature in 2 of 2 conflicts"),
        ],
        by_git: vec![row("README.md", "modified  +1 −1")],
        recorded: true,
        cursor: 2,
        expanded: None,
        subject: FieldView {
            label: "Subject",
            text: sanitize_line("Merge branch 'feature'"),
            cursor: None,
            guide_at: None,
            counter: None,
            focused: false,
            secret: None,
        },
        body: AreaView {
            label: "Body",
            lines: vec![sanitize_line("")],
            cursor: None,
            guide_at: None,
            focused: false,
        },
        focus: SummaryFocus::Files,
        notice: None,
        busy: false,
        state: SummaryState::Ready,
        top: 0,
    }
}

fn draw(view: &FinishView<'_>, columns: u16, rows: u16) -> Vec<String> {
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(columns, rows))
        .unwrap_or_else(|why| panic!("a test backend always opens: {why}"));
    terminal
        .draw(|frame| {
            frame.render_widget(
                &FinishScreen {
                    view,
                    theme: &theme,
                    ascii: false,
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
        .collect()
}

fn line_of(grid: &[String], needle: &str) -> usize {
    grid.iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}:\n{}", grid.join("\n")))
}

#[test]
fn one_column_below_160() {
    let grid = draw(&view(), 120, 40);
    let text = grid.join("\n");
    assert!(
        grid[0].starts_with("Finish · merging feature → main · nothing is written until ⏎"),
        "{text}"
    );
    let by_you = line_of(&grid, "RESOLVED BY YOU · 2");
    let gone = line_of(&grid, "gone.txt");
    let notes = line_of(&grid, "notes.txt");
    let by_git = line_of(&grid, "COMBINED BY GIT ON ITS OWN · 1");
    let readme = line_of(&grid, "▸ README.md");
    let message = line_of(&grid, "MESSAGE · ⇥ edit");
    let subject = line_of(&grid, "Merge branch 'feature'");
    assert!(by_you < gone && gone < notes && notes < by_git && by_git < readme);
    assert_eq!(message, readme + 1, "the message follows the list:\n{text}");
    assert!(subject > message, "{text}");
    // Paths padded to the widest one, then the words.
    assert!(
        grid[gone].contains("gone.txt   kept · yours · main"),
        "{text}"
    );
    assert!(
        grid[notes].contains("notes.txt  theirs · feature in 2 of 2 conflicts"),
        "{text}"
    );
    assert!(
        grid[readme].contains("README.md  modified  +1 −1"),
        "{text}"
    );
    // One column: the message starts at the left edge like the lists.
    assert!(grid[message].starts_with("MESSAGE"), "{text}");
}

#[test]
fn two_columns_from_160() {
    for (columns, rows) in [(160, 48), (200, 50)] {
        let grid = draw(&view(), columns, rows);
        let text = grid.join("\n");
        let message = line_of(&grid, "MESSAGE · ⇥ edit");
        assert_eq!(message, 1, "the message starts under the title:\n{text}");
        let at = grid[message]
            .find("MESSAGE")
            .map(|byte| grid[message][..byte].chars().count())
            .unwrap_or_default();
        let left = usize::from(columns) * 3 / 5;
        assert!(at >= left, "message at {at}, left column {left}:\n{text}");
        // The lists sit to its left on the same rows.
        assert!(grid[message].starts_with("RESOLVED BY YOU · 2"), "{text}");
        let readme = line_of(&grid, "▸ README.md");
        assert!(readme < 8, "{text}");
    }
}

#[test]
fn too_small_says_so() {
    let grid = draw(&view(), 59, 20);
    assert_eq!(grid[0], "finish needs 60x14 · esc back");
    assert!(
        grid[1..].iter().all(String::is_empty),
        "{}",
        grid.join("\n")
    );

    let short = draw(&view(), 100, 13);
    assert_eq!(short[0], "finish needs 60x14 · esc back");

    // At the minimum it draws.
    let fits = draw(&view(), 60, 14);
    assert!(fits.iter().any(|line| line.contains("MESSAGE")));
}

fn long_diff(lines: usize) -> Arc<DiffView> {
    let mut hunk = vec![LineView {
        kind: LineKindView::HunkHeading,
        old_no: None,
        new_no: None,
        text: sanitize_line("@@ -1,30 +1,30 @@"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }];
    hunk.extend((0..lines).map(|n| LineView {
        kind: LineKindView::Added,
        old_no: None,
        new_no: u32::try_from(n + 1).ok(),
        text: sanitize_line(&format!("added line {n}")),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }));
    Arc::new(DiffView {
        path: sanitize_line("README.md"),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: vec![HunkView {
            heading: sanitize_line("@@ -1,30 +1,30 @@"),
            lines: hunk,
        }],
        binary: None,
        // What the job reports when it cut the diff: the summary must still
        // say `space hides`, never `L load all`.
        truncated: Some((30, 40)),
        untracked: false,
    })
}

#[test]
fn a_long_diff_stops_at_12_rows_and_says_space_hides() {
    let mut expanded = view();
    expanded.expanded = Some((2, ExpandedDiff::Ready(long_diff(30))));
    let grid = draw(&expanded, 120, 40);
    let text = grid.join("\n");
    let readme = line_of(&grid, "▸ README.md");
    let more = line_of(&grid, "more lines · space hides");
    assert_eq!(
        more,
        readme + 1 + DIFF_ROWS_MAX,
        "12 diff rows between the row and the footer:\n{text}"
    );
    // 31 rows (heading + 30 lines), 12 drawn: 19 left, plus the 10 the job
    // cut (30 shown of 40).
    assert!(
        grid[more].contains("… 29 more lines · space hides"),
        "{text}"
    );
    // Indented under the row.
    assert!(grid[more].starts_with("    …"), "{text}");
    assert!(grid[readme + 1].starts_with("    "), "{text}");
    assert!(!text.contains("load all"), "{text}");
    // The rows after it move down: the message follows the footer.
    assert_eq!(line_of(&grid, "MESSAGE · ⇥ edit"), more + 1, "{text}");
}

#[test]
fn a_click_lands_on_what_is_drawn() {
    let view = view();
    let area = Rect::new(0, 0, 120, 40);
    let grid = draw(&view, 120, 40);
    let notes = u16::try_from(line_of(&grid, "notes.txt")).unwrap_or(u16::MAX);
    assert_eq!(hit(&view, area, 5, notes), Some(FinishHit::Row(1)));
    let subject = u16::try_from(line_of(&grid, "Merge branch")).unwrap_or(u16::MAX);
    assert_eq!(hit(&view, area, 20, subject), Some(FinishHit::Subject));
    assert_eq!(hit(&view, area, 5, 0), None, "the title is not a row");
}

#[test]
fn the_cursor_row_is_kept_on_screen() {
    let mut view = view();
    view.by_git = (0..40)
        .map(|n| row(&format!("file{n}.txt"), "modified  +1 −1"))
        .collect();
    let area = Rect::new(0, 0, 120, 20);
    view.cursor = 41;
    let top = top_for_cursor(&view, area);
    view.top = top;
    let grid = draw(&view, 120, 20);
    assert!(
        grid.iter().any(|line| line.starts_with("▸ file39.txt")),
        "{}",
        grid.join("\n")
    );
    view.cursor = 0;
    assert_eq!(top_for_cursor(&view, area), 0, "back up brings the heading");
}
