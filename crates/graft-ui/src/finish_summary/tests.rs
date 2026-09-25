//! What the summary's character grid says at each width: one column below
//! 160, two from 160, one line below the minimum, and a long diff cut at 12
//! rows with the summary's own words.

use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

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
        names: Some((sanitize_line("feature"), sanitize_line("main"))),
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
        expandable: true,
        focused: true,
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

/// A row with the frame's left border and gutter taken off.
fn inside(line: &str) -> &str {
    line.strip_prefix("│ ").unwrap_or(line)
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
        grid[0].starts_with("╭ Preview: finish merging feature into main ─"),
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
    assert!(inside(&grid[message]).starts_with("MESSAGE"), "{text}");
}

#[test]
fn two_columns_from_160() {
    for (columns, rows) in [(160, 48), (200, 50)] {
        let grid = draw(&view(), columns, rows);
        let text = grid.join("\n");
        let message = line_of(&grid, "MESSAGE · ⇥ edit");
        assert_eq!(
            message, 3,
            "the message starts under the preview lines:\n{text}"
        );
        let at = grid[message]
            .find("MESSAGE")
            .map(|byte| grid[message][..byte].chars().count())
            .unwrap_or_default();
        let left = usize::from(columns) * 3 / 5;
        assert!(at >= left, "message at {at}, left column {left}:\n{text}");
        // The lists sit to its left on the same rows.
        assert!(
            inside(&grid[message]).starts_with("RESOLVED BY YOU · 2"),
            "{text}"
        );
        let readme = line_of(&grid, "▸ README.md");
        assert!(readme < 10, "{text}");
    }
}

fn buffer(view: &FinishView<'_>, columns: u16, rows: u16) -> Buffer {
    let theme = theme();
    let area = Rect::new(0, 0, columns, rows);
    let mut buf = Buffer::empty(area);
    (&FinishScreen {
        view,
        theme: &theme,
        ascii: false,
    })
        .render(area, &mut buf);
    buf
}

#[test]
fn preview_title_shortens_then_cuts() {
    let names = Some(("feature", "main"));
    assert_eq!(
        preview_title(names, 80),
        "Preview: finish merging feature into main"
    );
    // 41 cells whole, 4 more for the corners and gutters: 44 is one short.
    assert_eq!(preview_title(names, 44), "Preview: merge feature → main");
    let cut = preview_title(names, 20);
    assert!(
        cut.starts_with("Preview: merge") && cut.ends_with('…'),
        "{cut}"
    );
    assert_eq!(crate::frame_box::cells(&cut), 16, "{cut}");
    assert_eq!(preview_title(None, 80), "Preview: finish the merge");
    let unnamed = preview_title(None, 20);
    assert!(unnamed.ends_with('…'), "{unnamed}");
    assert_eq!(crate::frame_box::cells(&unnamed), 16, "{unnamed}");
}

#[test]
fn summary_draws_inside_the_preview_frame() {
    let view = view();
    let grid = draw(&view, 80, 24);
    let text = grid.join("\n");
    assert!(
        grid[0].starts_with("╭ Preview: finish merging feature into main ─")
            && grid[0].ends_with('╮'),
        "{text}"
    );
    assert!(
        grid[1].starts_with("│ 1 merge commit on main · nothing is committed until y ")
            && grid[1].ends_with("  │"),
        "{text}"
    );
    assert!(grid[2].starts_with("│ undo: u takes this back"), "{text}");
    assert!(
        grid[23].starts_with("╰ y commit  ⏎/space diff  j/k move  ⇥ message  esc back ─"),
        "{text}"
    );
    assert!(grid[23].ends_with('╯'), "{text}");
    assert!(
        grid[3..23]
            .iter()
            .all(|line| line.starts_with('│') && line.ends_with('│')),
        "{text}"
    );
}

/// The frame's styles: border and title `accent`, the first key `accent`
/// BOLD, the rest muted, the row words in their panes' tokens. The cells
/// only — colour on a screen stays unverified (rule 5).
#[test]
fn preview_frame_styles() {
    let view = view();
    let grid = draw(&view, 80, 24);
    let buf = buffer(&view, 80, 24);
    let theme = theme();
    let accent = crate::paint::color(theme.token(Token::Accent));
    for (x, y) in [
        (0, 0),
        (79, 0),
        (0, 5),
        (79, 5),
        (0, 23),
        (79, 23),
        (70, 23),
    ] {
        assert_eq!(buf[(x, y)].fg, accent, "border cell ({x}, {y})");
    }
    // `╰ y`: the key at x = 2.
    assert_eq!(buf[(2, 23)].symbol(), "y");
    assert_eq!(buf[(2, 23)].fg, accent);
    assert!(buf[(2, 23)].modifier.contains(Modifier::BOLD));
    let muted = crate::paint::color(theme.token(Token::TextMuted));
    assert_eq!(buf[(4, 23)].fg, muted, "`commit` is muted");
    assert!(buf[(2, 0)].modifier.contains(Modifier::BOLD), "the title");
    assert_eq!(buf[(2, 0)].fg, accent);
    // Words that tie the rows back to the panes (colour unverified by eye).
    let grid_row = line_of(&grid, "gone.txt");
    let row = u16::try_from(grid_row).unwrap_or(u16::MAX);
    let at = grid[grid_row]
        .find("yours")
        .map_or(0, |byte| grid[grid_row][..byte].chars().count());
    let ours = crate::paint::color(theme.token(Token::Ours));
    assert_eq!(buf[(u16::try_from(at).unwrap_or(0), row)].fg, ours);
    let readme = line_of(&grid, "▸ README.md");
    let plus = grid[readme]
        .find('+')
        .map_or(0, |byte| grid[readme][..byte].chars().count());
    let added = crate::paint::color(theme.token(Token::Added));
    let y = u16::try_from(readme).unwrap_or(0);
    assert_eq!(buf[(u16::try_from(plus).unwrap_or(0), y)].fg, added);
}

#[test]
fn unread_summary_has_no_names() {
    let mut unread = view();
    unread.names = None;
    unread.state = SummaryState::Loading;
    unread.by_you.clear();
    unread.by_git.clear();
    let grid = draw(&unread, 80, 24);
    let text = grid.join("\n");
    assert!(
        grid[0].starts_with("╭ Preview: finish the merge ─"),
        "{text}"
    );
    assert!(
        grid[1].starts_with("│ 1 merge commit · nothing is committed until y "),
        "{text}"
    );
    assert!(
        !grid[0].contains("main") && !grid[1].contains("main"),
        "{text}"
    );
}

#[test]
fn unavailable_summary_names_y() {
    let mut failed = view();
    failed.state = SummaryState::Unavailable(sanitize_line("1 still conflicted"));
    let grid = draw(&failed, 120, 40);
    let text = grid.join("\n");
    assert!(
        text.contains("summary unavailable: 1 still conflicted · y commits with git's message"),
        "{text}"
    );
}

#[test]
fn hit_round_trips_inside_the_frame() {
    let mut view = view();
    view.by_git = (0..6)
        .map(|n| row(&format!("file{n}.txt"), "modified  +1 −1"))
        .collect();
    for (columns, rows) in [(80, 24), (120, 40), (187, 50)] {
        let area = Rect::new(0, 0, columns, rows);
        let grid = draw(&view, columns, rows);
        let text = grid.join("\n");
        for index in 0..view.row_count() {
            let path = view
                .row(index)
                .map(|row| row.path.as_str().to_owned())
                .unwrap_or_default();
            let y = line_of(&grid, &format!(" {path}  "));
            let y = u16::try_from(y).unwrap_or(u16::MAX);
            assert_eq!(
                hit(&view, area, 4, y),
                Some(FinishHit::Row(index)),
                "{columns}x{rows} row {index}:\n{text}"
            );
        }
        assert_eq!(hit(&view, area, 4, 0), None, "the top border");
        assert_eq!(hit(&view, area, 4, 1), None, "the preview line");
        assert_eq!(hit(&view, area, 4, rows - 1), None, "the bottom border");
    }
}

#[test]
fn too_small_says_so() {
    let grid = draw(&view(), 61, 20);
    assert_eq!(grid[0], "finish needs 62x18 · esc back");
    assert!(
        grid[1..].iter().all(String::is_empty),
        "{}",
        grid.join("\n")
    );

    let short = draw(&view(), 100, 17);
    assert_eq!(short[0], "finish needs 62x18 · esc back");

    // At the minimum it draws.
    let fits = draw(&view(), 62, 18);
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
        // say `⏎ hides`, never `L load all`.
        truncated: Some((30, 40)),
        untracked: false,
    })
}

#[test]
fn a_long_diff_stops_at_12_rows_and_says_enter_hides() {
    let mut expanded = view();
    expanded.expanded = Some((2, ExpandedDiff::Ready(long_diff(30))));
    let grid = draw(&expanded, 120, 40);
    let text = grid.join("\n");
    let readme = line_of(&grid, "▸ README.md");
    let more = line_of(&grid, "more lines · ⏎ hides");
    assert_eq!(
        more,
        readme + 1 + DIFF_ROWS_MAX,
        "12 diff rows between the row and the footer:\n{text}"
    );
    // 31 rows (heading + 30 lines), 12 drawn: 19 left, plus the 10 the job
    // cut (30 shown of 40).
    assert!(grid[more].contains("… 29 more lines · ⏎ hides"), "{text}");
    // Indented under the row.
    assert!(inside(&grid[more]).starts_with("    …"), "{text}");
    assert!(inside(&grid[readme + 1]).starts_with("    "), "{text}");
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
        grid.iter()
            .any(|line| inside(line).starts_with("▸ file39.txt")),
        "{}",
        grid.join("\n")
    );
    view.cursor = 0;
    assert_eq!(top_for_cursor(&view, area), 0, "back up brings the heading");
}
