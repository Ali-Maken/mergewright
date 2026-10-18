//! What the merge review's character grid says (ticket ease-pass/12, mock
//! `mergewright-merge-screen.html:155-162`): the mock's lines in its order,
//! the bar in the bottom border, one line below the minimum, and a long
//! diff cut at 12 rows with the review's own words.

use mergewright_term::sanitize_line;
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
        &test_capabilities(mergewright_core::capabilities::ColorDepth::TrueColor, false),
    )
}

fn picks(yours: u32, theirs: u32, both: u32) -> HowSolved {
    HowSolved::Picks(PickCounts {
        yours,
        theirs,
        both,
    })
}

fn file(path: &str, how: HowSolved) -> ReviewFile {
    ReviewFile {
        path: sanitize_line(path),
        how,
    }
}

/// The ticket's fixture: `sara/fix-toc` into `wallet-release`, `report.md`
/// (2 clashes, `o` then `t`), `notes.md` (1, `o`), 2 files combined.
fn view<'a>() -> ReviewView<'a> {
    ReviewView {
        keys: crate::keymap::Keymap::shipped(),
        from: sanitize_line("sara/fix-toc"),
        into: sanitize_line("wallet-release"),
        clashes: 3,
        incoming: Some(4),
        clashed: vec![
            file("docs/report.md", picks(1, 1, 0)),
            file("notes.md", picks(1, 0, 0)),
        ],
        combined: 2,
        message: sanitize_line("Merge branch 'sara/fix-toc' into wallet-release"),
        cursor: 0,
        expanded: None,
        focused: true,
        notice: None,
        busy: false,
        state: SummaryState::Ready,
        top: 0,
    }
}

fn draw(view: &ReviewView<'_>, columns: u16, rows: u16) -> Vec<String> {
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

/// A row with the frame's left border and gutter taken off, and its right
/// border.
fn inside(line: &str) -> &str {
    line.strip_prefix("│ ")
        .unwrap_or(line)
        .trim_end_matches('│')
        .trim_end()
}

fn line_of(grid: &[String], needle: &str) -> usize {
    grid.iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}:\n{}", grid.join("\n")))
}

fn buffer(view: &ReviewView<'_>, columns: u16, rows: u16) -> Buffer {
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

/// The mock's lines (`:155-161`), in its order, names swapped, and the bar
/// in the bottom border. No undo line: the mock has none (Minor 7).
#[test]
fn the_review_reads_as_the_mock_at_120() {
    let grid = draw(&view(), 120, 30);
    let text = grid.join("\n");
    let want = [
        "All 3 clashes solved. Nothing committed yet.",
        "This merge will put into wallet-release:",
        "  4 commits from sara/fix-toc",
        "▸ ✓ report.md  you chose: ◀ yours (1) · ▶ theirs (1)",
        "  ✓ notes.md   you chose: ◀ yours",
        "  ✓ 2 files git combined on its own",
        "Message  Merge branch 'sara/fix-toc' into wallet-release",
    ];
    let mut at = 0;
    for line in want {
        let found = grid[at..]
            .iter()
            .position(|drawn| inside(drawn) == line)
            .unwrap_or_else(|| panic!("{line:?} not after line {at}:\n{text}"));
        at += found + 1;
    }
    assert!(grid[0].starts_with('╭'), "{text}");
    assert!(
        grid[29].starts_with("╰ y commit the merge"),
        "the bar is the bottom border:\n{text}"
    );
    assert!(!text.contains("takes this back"), "{text}");
    assert!(!text.contains("undo:"), "{text}");
}

/// At 90 columns every line of the mock still reads whole.
#[test]
fn the_review_reads_whole_at_90() {
    let grid = draw(&view(), 90, 24);
    let text = grid.join("\n");
    for needle in [
        "All 3 clashes solved. Nothing committed yet.",
        "✓ report.md  you chose: ◀ yours (1) · ▶ theirs (1)",
        "✓ 2 files git combined on its own",
        "Message  Merge branch 'sara/fix-toc' into wallet-release",
    ] {
        line_of(&grid, needle);
    }
    assert!(text.contains("╰ y commit the merge"), "{text}");
}

/// Decision 4's words: one kind, several kinds in `◀ ▶ ◆` order with the
/// zero kinds left out, a whole-file card's word, and a file solved with no
/// picks recorded.
#[test]
fn how_solved_words() {
    let cases = [
        (picks(1, 0, 0), "you chose: ◀ yours"),
        (picks(0, 2, 0), "you chose: ▶ theirs"),
        (picks(0, 0, 1), "you chose: ◆ both"),
        (picks(1, 1, 0), "you chose: ◀ yours (1) · ▶ theirs (1)"),
        (picks(2, 0, 1), "you chose: ◀ yours (2) · ◆ both (1)"),
        (
            HowSolved::Whole(sanitize_line("keep theirs")),
            "you chose: keep theirs",
        ),
        (HowSolved::ByHand, "solved in the file"),
    ];
    for (how, want) in cases {
        assert_eq!(how.words(), want, "{how:?}");
    }
}

/// `N clashes` counts each pick, and one per whole-file or by-hand file.
#[test]
fn clashes_count_picks_and_one_per_other_file() {
    assert_eq!(picks(2, 1, 1).clashes(), 4);
    assert_eq!(HowSolved::ByHand.clashes(), 1);
    assert_eq!(HowSolved::Whole(sanitize_line("delete")).clashes(), 1);
}

/// The title and the count line, one and none, and a count that failed.
#[test]
fn title_and_count_words() {
    assert_eq!(
        review_title(3),
        "All 3 clashes solved. Nothing committed yet."
    );
    assert_eq!(review_title(1), "1 clash solved. Nothing committed yet.");
    assert_eq!(review_title(0), "Nothing committed yet.");
    let from = sanitize_line("sara/fix-toc");
    assert_eq!(count_line(Some(4), &from), "4 commits from sara/fix-toc");
    assert_eq!(count_line(Some(1), &from), "1 commit from sara/fix-toc");
    assert_eq!(count_line(None, &from), "? commits from sara/fix-toc");
}

/// Two clashed files with one name are drawn by their whole paths.
#[test]
fn files_with_one_name_are_drawn_whole() {
    let mut twins = view();
    twins.clashed = vec![
        file("src/mod.rs", picks(1, 0, 0)),
        file("tests/mod.rs", picks(0, 1, 0)),
    ];
    let grid = draw(&twins, 120, 30);
    line_of(&grid, "✓ src/mod.rs");
    line_of(&grid, "✓ tests/mod.rs");
}

/// Nothing combined by git: no combined row (status rule).
#[test]
fn no_combined_row_when_git_combined_nothing() {
    let mut none = view();
    none.combined = 0;
    let text = draw(&none, 120, 30).join("\n");
    assert!(!text.contains("combined on its own"), "{text}");
    assert_eq!(none.row_count(), 2);
    assert_eq!(none.row(2), None);
    let one = ReviewView {
        combined: 1,
        ..view()
    };
    line_of(&draw(&one, 120, 30), "✓ 1 file git combined on its own");
    assert_eq!(one.row(2), Some(ReviewRow::Combined));
    assert_eq!(one.row(1), Some(ReviewRow::Clashed(1)));
}

#[test]
fn unavailable_review_names_y() {
    let mut failed = view();
    failed.state = SummaryState::Unavailable(sanitize_line("git said no"));
    let grid = draw(&failed, 120, 30);
    let text = grid.join("\n");
    line_of(
        &grid,
        "summary unavailable: git said no · y commits with git's message",
    );
    assert!(!text.contains("Message"), "{text}");
    assert!(!text.contains("you chose"), "{text}");
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
    let fits = draw(&view(), 62, 18);
    assert!(fits.iter().any(|line| line.contains("Message")));
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
        path: sanitize_line("notes.md"),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: vec![HunkView {
            heading: sanitize_line("@@ -1,30 +1,30 @@"),
            lines: hunk,
        }],
        binary: None,
        // What the job reports when it cut the diff: the review must still
        // say `⏎ hides`, never `L load all`.
        truncated: Some((30, 40)),
        untracked: false,
    })
}

/// Until 12b's look, `⏎` on a clashed row is today's inline diff.
#[test]
fn a_long_diff_stops_at_12_rows_and_says_enter_hides() {
    let mut expanded = view();
    expanded.cursor = 1;
    expanded.expanded = Some((1, ExpandedDiff::Ready(long_diff(30))));
    let grid = draw(&expanded, 120, 40);
    let text = grid.join("\n");
    let notes = line_of(&grid, "▸ ✓ notes.md");
    let more = line_of(&grid, "more lines · ⏎ hides");
    assert_eq!(more, notes + 1 + DIFF_ROWS_MAX, "{text}");
    assert!(grid[more].contains("… 29 more lines · ⏎ hides"), "{text}");
    assert!(inside(&grid[more]).starts_with("    …"), "{text}");
    assert!(!text.contains("load all"), "{text}");
    assert_eq!(
        line_of(&grid, "✓ 2 files git combined"),
        more + 1,
        "the rows after it move down:\n{text}"
    );
}

#[test]
fn a_click_lands_on_what_is_drawn() {
    let view = view();
    let area = Rect::new(0, 0, 120, 30);
    let grid = draw(&view, 120, 30);
    let at = |needle| u16::try_from(line_of(&grid, needle)).unwrap_or(u16::MAX);
    assert_eq!(hit(&view, area, 5, at("notes.md")), Some(1));
    assert_eq!(hit(&view, area, 5, at("git combined")), Some(2));
    assert_eq!(hit(&view, area, 5, at("Message")), None);
    assert_eq!(hit(&view, area, 5, 0), None, "the border is not a row");
}

#[test]
fn the_cursor_row_is_kept_on_screen() {
    let mut view = view();
    view.clashed = (0..40)
        .map(|n| file(&format!("file{n}.txt"), picks(1, 0, 0)))
        .collect();
    let area = Rect::new(0, 0, 120, 20);
    view.cursor = 40;
    view.top = top_for_cursor(&view, area);
    let grid = draw(&view, 120, 20);
    assert!(
        grid.iter()
            .any(|line| inside(line).starts_with("▸ ✓ 2 files git combined")),
        "{}",
        grid.join("\n")
    );
    view.cursor = 0;
    assert_eq!(top_for_cursor(&view, area), 0);
}

/// Review m6-look-finish-wave2 m9: the footer's spans come from the hints,
/// not from splitting their rendered text. The first hint's key alone is
/// strong, every other character muted, and the words read as the bar's.
/// Colour unverified (rule 5): the styles are compared as values.
#[test]
fn footer_spans_take_the_first_key_from_the_hint() {
    use crate::graph_screen::{Hint, HintKey, HintState, Rung, hints_text};
    use crate::keymap::Keymap;
    let keys = Keymap::shipped();
    let strong = Style::new().add_modifier(Modifier::BOLD);
    let muted = Style::new().add_modifier(Modifier::DIM);
    let hints = footer_hints(&view(), 120);
    let spans = footer_spans(&hints, keys, strong, muted);
    assert_eq!(spans.len(), 2, "{spans:?}");
    assert_eq!(spans[0], ("y".to_owned(), strong));
    assert_eq!(spans[1].1, muted);
    let joined: String = spans.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(joined, hints_text(&hints, keys).trim_start());
    // A key with a space in it stays whole: splitting at the first space
    // would have painted `⌃` alone.
    let spaced = [
        Hint {
            keys: &[HintKey::Word("⌃ ⏎")],
            verb: "commit".into(),
            named_short: None,
            state: HintState::Live,
            rung: Rung::Full,
            keeps_words: false,
            words_only: false,
        },
        Hint {
            keys: &[HintKey::Key(crate::keymap::ActionId("app.back"))],
            verb: "back".into(),
            named_short: None,
            state: HintState::Live,
            rung: Rung::Key,
            keeps_words: false,
            words_only: false,
        },
    ];
    assert_eq!(
        footer_spans(&spaced, keys, strong, muted),
        vec![
            ("⌃ ⏎".to_owned(), strong),
            (" commit  esc".to_owned(), muted)
        ]
    );
    assert!(footer_spans(&[], keys, strong, muted).is_empty());
}

/// No keys are drawn while the commit runs or a box has the keyboard.
#[test]
fn no_keys_while_busy_or_under_a_box() {
    let busy = ReviewView {
        busy: true,
        ..view()
    };
    assert!(footer_hints(&busy, 120).is_empty());
    let boxed = ReviewView {
        focused: false,
        ..view()
    };
    assert!(footer_hints(&boxed, 120).is_empty());
}

/// ADR-0058 §3: the cursor row's text clears 4.5:1 on `selection` in dark.
#[test]
fn text_on_the_cursor_row_is_legible() {
    for cursor in 0..3 {
        let mut shown = view();
        shown.cursor = cursor;
        let checked =
            crate::test_support::assert_legible_on_selection(&buffer(&shown, 120, 30), &theme());
        assert!(checked > 5, "cursor {cursor}: drawn ({checked})");
    }
}

/// Spell sweep (c), ticket m8/10c: the too-small line and the unavailable
/// line name his chords.
#[test]
fn the_review_spells_every_key_from_his_keymap() {
    use crate::keymap::sweep::{assert_spelled, swept};
    let (keys, _) = swept();
    let mut swept_view = view();
    swept_view.keys = &keys;
    assert_spelled(&draw(&swept_view, 61, 20)[0], &keys, &[&["app.back"]]);
    swept_view.state = SummaryState::Unavailable(sanitize_line("1 still conflicted"));
    let grid = draw(&swept_view, 120, 40);
    let unavailable = grid
        .iter()
        .find(|line| line.contains("summary unavailable"))
        .unwrap_or_else(|| panic!("{}", grid.join("\n")));
    assert_spelled(unavailable, &keys, &[&["confirm.yes"]]);
}
