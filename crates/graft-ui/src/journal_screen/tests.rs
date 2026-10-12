use graft_core::capabilities::ColorDepth;
use graft_term::sanitize::sanitize;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::test_support::terminal as test_capabilities;
use crate::theme::ThemeKind;

fn theme() -> Theme {
    Theme::new(
        ThemeKind::Dark,
        &test_capabilities(ColorDepth::TrueColor, false),
    )
}

fn row(age: &str, description: &str, mark: RowMark) -> JournalRowView {
    JournalRowView {
        age: sanitize(age),
        description: sanitize(description),
        summary: sanitize("backup saved"),
        mark,
        commands: vec![sanitize("git commit -F -")],
        ..JournalRowView::default()
    }
}

fn outside(age: &str, words: &str, subject: &str) -> JournalRowView {
    JournalRowView {
        age: sanitize(age),
        description: sanitize(words),
        kind: RowKind::Outside,
        git_subject: Some(sanitize(subject)),
        ..JournalRowView::default()
    }
}

fn lost(words: &str, under: usize) -> JournalRowView {
    JournalRowView {
        description: sanitize(words),
        kind: RowKind::Lost,
        under: Some(under),
        ..JournalRowView::default()
    }
}

/// The drawn grid, one string per line, trailing blanks trimmed.
fn draw(view: &JournalView, width: u16, height: u16) -> Vec<String> {
    draw_with(view, width, height, Keymap::shipped())
}

/// [`draw`] under `keys`.
fn draw_with(view: &JournalView, width: u16, height: u16, keys: &Keymap) -> Vec<String> {
    let theme = theme();
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("the test backend starts");
    terminal
        .draw(|frame| {
            frame.render_widget(
                JournalScreen {
                    view,
                    theme: &theme,
                    keys,
                },
                frame.area(),
            );
        })
        .expect("the widget draws");
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_owned()
        })
        .collect()
}

fn view(rows: Vec<JournalRowView>, status: JournalStatus) -> JournalView {
    JournalView {
        location: sanitize("~/code/api"),
        rows,
        selected: 0,
        status,
        title_suffix: None,
        status_row: None,
        enter_words: None,
        branch_here: false,
        box_open: false,
        find: None,
        find_prompt: None,
        esc_cancels: false,
    }
}

/// J1's mock-up rows: a fetch, the shell's reset with its lost commit
/// under it, a shell switch with the lost spike tip, Graft's switch.
fn mock_rows() -> Vec<JournalRowView> {
    vec![
        JournalRowView {
            summary: sanitize("origin/main moved · not undoable"),
            ..row("2m ago", "fetch all remotes", RowMark::None)
        },
        outside(
            "5m ago",
            "reset main 1 commit back",
            "reset: moving to HEAD~1",
        ),
        lost(
            "9a8b7c6 \"Add rate limiter\" · was main's tip · no branch holds it",
            1,
        ),
        outside(
            "9m ago",
            "switch from spike to main",
            "checkout: moving from spike to main",
        ),
        lost(
            "4d5e6f7 \"Try redis cache\" · was spike's tip · spike is gone",
            3,
        ),
        row("40m ago", "switch from main to spike", RowMark::NextUndo),
    ]
}

#[test]
fn the_next_undo_row_says_so_in_text_not_in_colour() {
    let view = view(
        vec![
            row("2m ago", "commit \"Add OAuth\"", RowMark::NextUndo),
            row("1h ago", "stage src/a.rs", RowMark::None),
        ],
        JournalStatus::Ready,
    );
    let grid = draw(&view, 60, 8);
    assert!(grid[0].starts_with("JOURNAL · "), "{grid:?}");
    assert!(grid[1].contains("2m ago"), "{grid:?}");
    assert!(grid[1].contains("← u"), "{grid:?}");
    assert!(
        !grid[2].contains('←'),
        "only the next one is marked: {grid:?}"
    );
}

/// J1: the Command Log is `┆ git …` under `commands Graft ran` — `$`
/// means only "outside Graft" (UX §3.6). Replaces the `$ git` tests.
#[test]
fn the_command_log_uses_the_dotted_rule() {
    let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
    rows[0].expanded = true;
    let grid = draw(&view(rows, JournalStatus::Ready), 60, 10);
    assert!(
        grid[2].trim_start().starts_with(COMMANDS_HEADING),
        "{grid:?}"
    );
    assert!(
        grid[3].trim_start().starts_with("┆ git commit -F -"),
        "{grid:?}"
    );
    assert!(
        !grid.iter().any(|line| line.contains("$ git")),
        "no `$` in the log: {grid:?}"
    );
}

#[test]
fn an_aborted_rows_reason_is_shown_in_full_when_it_is_expanded() {
    let mut rows = vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)];
    rows[0].expanded = true;
    rows[0].failure = Some(sanitize("failed: hook rejected the commit"));
    let grid = draw(&view(rows, JournalStatus::Ready), 60, 10);
    assert!(grid[2].contains("failed: hook rejected"), "{grid:?}");
    assert!(grid[3].contains(COMMANDS_HEADING), "{grid:?}");
    assert!(grid[4].contains("┆ git commit"), "{grid:?}");
}

/// ADR-0062: an expanded fetch row lists the refs its fetch moved, between
/// its failure and its Command Log, under a heading that says `u` leaves
/// them where they are — every line the caller gives (`graft` caps them at
/// twenty and a `+N more`), `┆ `-led like the commands. Collapsed, none.
#[test]
fn an_expanded_fetch_row_lists_its_moves() {
    let fetch = fetch_with_moves();
    let expanded = view(vec![fetch.clone()], JournalStatus::Ready);
    let grid = draw(&expanded, 120, 40);
    let lines: Vec<&str> = grid.iter().map(|line| line.trim_start()).collect();
    let heading = lines
        .iter()
        .position(|line| line.starts_with("refs the fetch moved"))
        .unwrap_or_else(|| panic!("no heading: {grid:#?}"));
    assert_eq!(
        lines[heading],
        "refs the fetch moved · u does not move them back"
    );
    assert!(
        lines[heading - 1].starts_with("failed: no repository"),
        "after the failure: {grid:#?}"
    );
    for n in 0..20 {
        assert_eq!(
            lines[heading + 1 + n],
            format!("┆ origin/b{n:02}  a1b2c3d → e4f5a6b"),
            "{grid:#?}"
        );
    }
    assert_eq!(lines[heading + 21], "┆ +5 more", "{grid:#?}");
    assert_eq!(lines[heading + 22], COMMANDS_HEADING, "{grid:#?}");
    assert_eq!(lines[heading + 23], "┆ git fetch --progress --all");

    // Collapsed: no heading, no moves.
    let collapsed = view(
        vec![JournalRowView {
            expanded: false,
            ..fetch
        }],
        JournalStatus::Ready,
    );
    let grid = draw(&collapsed, 120, 40);
    assert!(
        !grid
            .iter()
            .any(|line| line.contains("refs the fetch moved") || line.contains("┆ origin/")),
        "{grid:#?}"
    );
}

/// The expanded fetch row both tests above and below draw: twenty moves
/// and a `+5 more`, a failure, one command.
fn fetch_with_moves() -> JournalRowView {
    let mut moved: Vec<SanitizedText> = (0..20)
        .map(|n| sanitize(&format!("origin/b{n:02}  a1b2c3d → e4f5a6b")))
        .collect();
    moved.push(sanitize("+5 more"));
    JournalRowView {
        summary: sanitize(
            "failed: no repository at '/x' · origin/b00 moved +24 more before it stopped",
        ),
        failure: Some(sanitize(
            "failed: no repository at '/x', or no access to it",
        )),
        commands: vec![sanitize("git fetch --progress --all")],
        moved,
        expanded: true,
        ..row("2m ago", "fetch all remotes", RowMark::None)
    }
}

/// `app.undo` unbound: the moved list's heading is its first clause alone,
/// naming no key that does nothing.
#[test]
fn the_moved_heading_names_no_unbound_undo_key() {
    let unbound = crate::test_support::unbound(&["app.undo"]);
    assert_eq!(moved_heading(&unbound), "refs the fetch moved");
    let expanded = view(vec![fetch_with_moves()], JournalStatus::Ready);
    let grid = draw_with(&expanded, 120, 40, &unbound);
    assert!(
        grid.iter()
            .any(|line| line.trim_start() == "refs the fetch moved"),
        "{grid:#?}"
    );
}

#[test]
fn an_empty_journal_says_what_will_fill_it() {
    let grid = draw(&view(Vec::new(), JournalStatus::Empty), 70, 8);
    assert!(
        grid.iter()
            .any(|line| line.contains(&empty(Keymap::shipped()))),
        "{grid:?}"
    );
}

#[test]
fn a_failed_read_shows_the_reason_rather_than_an_empty_list() {
    let status = JournalStatus::Failed(sanitize("line 3: unexpected end of input"));
    let grid = draw(&view(Vec::new(), status), 70, 8);
    assert!(
        grid.iter()
            .any(|line| line.contains("line 3: unexpected end of input")),
        "{grid:?}"
    );
}

#[test]
fn a_one_row_terminal_draws_the_header_and_nothing_else() {
    let view = view(
        vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)],
        JournalStatus::Ready,
    );
    for height in [1, 2, 3] {
        let grid = draw(&view, 40, height);
        assert!(grid[0].starts_with("JOURNAL"), "{grid:?}");
    }
}

/// :848: a row too long for its room is cut at a word with `…`, never
/// run into the mark (`the woundone`).
#[test]
fn a_cut_row_cuts_at_a_word() {
    let mut long = row("2m ago", "undo: discard the wound", RowMark::Undone);
    long.summary = sanitize("the index · the working tree · backup saved");
    let grid = draw(&view(vec![long], JournalStatus::Ready), 70, 8);
    let line = &grid[1];
    assert!(line.contains('…'), "{line:?}");
    assert!(line.ends_with("undone"), "{line:?}");
    let before_mark = line.trim_end_matches("undone").trim_end();
    assert!(before_mark.ends_with('…'), "cut before the mark: {line:?}");
    let kept = before_mark.trim_end_matches('…');
    assert!(
        kept.ends_with(|c: char| c.is_alphanumeric()) && !kept.ends_with(' '),
        "{line:?}"
    );
}

#[test]
fn cut_at_word_keeps_whole_words_and_says_where_it_cut() {
    assert_eq!(cut_at_word("put main back", 20), "put main back");
    assert_eq!(cut_at_word("put main back to it", 12), "put main…");
    assert_eq!(cut_at_word("unbreakable", 5), "unbr…");
    assert_eq!(cut_at_word("a   b   c", 6), "a   b…");
}

/// J1's breakpoints: at 180 and wider every column, the `git:` column
/// on `$` rows; the `✗` row indented `└ ✗ lost` under its event.
#[test]
fn at_180_every_column_is_drawn() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.selected = 2;
    let grid = draw(&view, 187, 14);
    assert!(grid[1].contains("●  Mergewright"), "{grid:?}");
    assert!(grid[2].contains("$  outside "), "{grid:?}");
    assert!(
        grid[2].contains("git: \"reset: moving to HEAD~1\""),
        "{grid:?}"
    );
    assert!(grid[3].starts_with("▸    └ ✗ lost"), "{grid:?}");
    assert!(grid[3].contains("9a8b7c6 \"Add rate limiter\""), "{grid:?}");
    assert!(
        grid.iter()
            .any(|line| line.contains(&legend(Keymap::shipped()))),
        "{grid:?}"
    );
    assert!(grid.iter().any(|line| line.contains(NO_TRACE)), "{grid:?}");
}

/// 120–179: no `git:` column; the selected `$` row shows it under it.
#[test]
fn at_140_the_selected_shell_rows_subject_is_under_it() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.selected = 1;
    let grid = draw(&view, 140, 14);
    assert!(grid[2].contains("$  outside "), "{grid:?}");
    assert!(!grid[2].contains("git:"), "{grid:?}");
    assert!(
        grid[3]
            .trim_start()
            .starts_with("git: \"reset: moving to HEAD~1\""),
        "{grid:?}"
    );
    assert!(
        !grid.iter().any(|line| line.contains("checkout: moving")),
        "only the selected row's: {grid:?}"
    );
}

/// Under 120: no source word; the legend carries it.
#[test]
fn under_120_there_is_no_source_word() {
    let view = view(mock_rows(), JournalStatus::Ready);
    let grid = draw(&view, 100, 14);
    assert!(!grid[1].contains("Mergewright"), "{grid:?}");
    assert!(!grid[2].contains("outside"), "{grid:?}");
    assert!(grid[2].contains("$  5m ago"), "{grid:?}");
    assert!(
        grid.iter()
            .any(|line| line.starts_with("● done in Mergewright")),
        "{grid:?}"
    );
}

/// At 24 rows the title, the legend, the status row and the bar stay;
/// the rows scroll so the cursor's row is on screen.
#[test]
fn at_24_rows_the_frame_stays_and_the_rows_scroll() {
    let mut rows = Vec::new();
    for n in 0..30 {
        rows.push(row(
            &format!("{n}m ago"),
            &format!("stage file{n}"),
            RowMark::None,
        ));
    }
    let mut view = view(rows, JournalStatus::Ready);
    view.selected = 29;
    view.status_row = Some(JournalStatusRow {
        words: "nothing to undo".to_owned(),
        problem: true,
    });
    view.enter_words = Some("revert to before this".to_owned());
    let grid = draw(&view, 187, 24);
    assert!(grid[0].starts_with("JOURNAL"), "{grid:?}");
    assert!(grid[23].starts_with("⏎ revert to before this"), "{grid:?}");
    assert!(grid[22].starts_with("● done in Mergewright"), "{grid:?}");
    assert_eq!(grid[21], "nothing to undo", "{grid:?}");
    assert!(
        grid[20].starts_with("▸ ") && grid[20].contains("stage file29"),
        "the cursor's row is on screen: {grid:?}"
    );
}

/// The status rule: no status row, no line for it.
#[test]
fn no_status_row_when_nothing_is_wrong() {
    let view = view(mock_rows(), JournalStatus::Ready);
    let grid = draw(&view, 187, 14);
    assert!(grid[12].starts_with("● done in Mergewright"), "{grid:?}");
    assert!(
        grid[11].is_empty() || grid[11].contains(NO_TRACE),
        "{grid:?}"
    );
}

/// The bar is per row: `⏎`'s words for this row, `b` where the row
/// names a commit, the log where there is one.
#[test]
fn the_bar_follows_the_selected_row() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.selected = 2;
    view.enter_words = Some("put main back to it".to_owned());
    view.branch_here = true;
    let grid = draw(&view, 187, 14);
    assert_eq!(
        grid[13],
        "⏎ put main back to it  b new branch here  / find  J/esc close  ? keys"
    );
    view.selected = 0;
    view.enter_words = None;
    view.branch_here = false;
    let grid = draw(&view, 187, 14);
    assert_eq!(grid[13], "/ find  space command log  J/esc close  ? keys");
}

/// `b new branch here` is drawn from `branch_here` alone (m7/30o Minor 2):
/// a `$` or `✗` row whose step names no commit offers no `b`.
#[test]
fn the_bar_offers_b_only_where_b_names_a_commit() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.selected = 2;
    view.enter_words = Some("put main back to it".to_owned());
    let grid = draw(&view, 187, 14);
    assert_eq!(
        grid[13],
        "⏎ put main back to it  / find  J/esc close  ? keys"
    );
}

/// A box docked over J names the keys; J's bar is blank under it.
#[test]
fn the_bar_is_blank_while_a_box_is_open() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.box_open = true;
    view.enter_words = Some("undo this".to_owned());
    let grid = draw(&view, 187, 14);
    assert!(grid[13].is_empty(), "{grid:?}");
}

#[test]
fn the_title_says_while_the_scan_runs() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.title_suffix = Some("finding lost commits…".to_owned());
    let grid = draw(&view, 187, 14);
    assert_eq!(
        grid[0],
        "JOURNAL · what happened in this repo, newest first · finding lost commits…"
    );
}

/// J's find (m7-power-tools/30o-c) on the title row, as blame's
/// `FindTitle` tail: `find "<q>" <at> of <of>`, or `· no match`.
#[test]
fn a_kept_find_ends_the_title_as_blames_does() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.find = Some(JournalFindView {
        query: sanitize("spike"),
        at: Some(1),
        of: 2,
    });
    let grid = draw(&view, 187, 14);
    assert_eq!(
        grid[0],
        "JOURNAL · what happened in this repo, newest first · find \"spike\" 1 of 2"
    );
    view.find = Some(JournalFindView {
        query: sanitize("zzz"),
        at: None,
        of: 0,
    });
    let grid = draw(&view, 187, 14);
    assert!(
        grid[0].ends_with(" · find \"zzz\" · no match"),
        "{}",
        grid[0]
    );
}

/// The bar as blame's (`blame_place_list`): `/ find` always, `n N match`
/// after it while a find is kept.
#[test]
fn the_bar_offers_n_n_only_with_a_find_kept() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.selected = 2;
    view.enter_words = Some("put main back to it".to_owned());
    view.find = Some(JournalFindView {
        query: sanitize("spike"),
        at: Some(1),
        of: 2,
    });
    let grid = draw(&view, 187, 14);
    assert_eq!(
        grid[13],
        "⏎ put main back to it  / find  n/N match  J/esc close  ? keys"
    );
}

/// While he types, the prompt stands where the bar was, as blame's.
#[test]
fn typing_draws_the_prompt_in_place_of_the_bar() {
    let mut view = view(mock_rows(), JournalStatus::Ready);
    view.find_prompt = Some(sanitize("spi"));
    let grid = draw(&view, 187, 14);
    assert_eq!(grid[13], "/spi");
}

/// ADR-0058 §3: every character on the cursor row's `selection` fill clears
/// 4.5:1 against it in dark B — the muted age and the warning words too.
#[test]
fn text_on_the_cursor_row_is_legible() {
    let theme = theme();
    for selected in 0..mock_rows().len() {
        let mut view = view(mock_rows(), JournalStatus::Ready);
        view.selected = selected;
        let mut terminal = Terminal::new(TestBackend::new(187, 14)).expect("the test backend");
        terminal
            .draw(|frame| {
                frame.render_widget(
                    JournalScreen {
                        view: &view,
                        theme: &theme,
                        keys: Keymap::shipped(),
                    },
                    frame.area(),
                );
            })
            .expect("the widget draws");
        let checked =
            crate::test_support::assert_legible_on_selection(terminal.backend().buffer(), &theme);
        assert!(
            checked > 5,
            "row {selected}: the cursor row is drawn ({checked})"
        );
    }
}

/// Spell sweep (c), ticket m8/10c: under the sweep keymap the footer, the
/// legend and the empty state name his chords, never the defaults.
#[test]
fn the_journal_spells_every_key_from_his_keymap() {
    use crate::keymap::sweep::{assert_spelled, swept};
    let (keys, _) = swept();
    let mut view = view(
        vec![row("2m", "switch", RowMark::None)],
        JournalStatus::Ready,
    );
    view.enter_words = Some("revert this".to_owned());
    view.branch_here = true;
    view.find = Some(JournalFindView {
        query: sanitize("x"),
        at: Some(1),
        of: 1,
    });
    let theme = theme();
    let screen = JournalScreen {
        view: &view,
        theme: &theme,
        keys: &keys,
    };
    assert_spelled(
        &screen.footer(400),
        &keys,
        &[
            &["journal.revert"],
            &["journal.branch"],
            &["app.find"],
            &["app.next_match", "app.prev_match"],
            &["journal.expand"],
            &["journal.close", "app.back"],
        ],
    );
    assert_spelled(&legend(&keys), &keys, &[&["app.undo"]]);
    assert_spelled(&empty(&keys), &keys, &[&["app.undo"]]);
}

/// J's bottom row at `width` under `keys`, trailing blanks trimmed.
fn footer_row(view: &JournalView, width: u16, keys: &Keymap) -> String {
    let theme = theme();
    let mut terminal = Terminal::new(TestBackend::new(width, 14)).expect("the test backend starts");
    terminal
        .draw(|frame| {
            frame.render_widget(
                JournalScreen {
                    view,
                    theme: &theme,
                    keys,
                },
                frame.area(),
            );
        })
        .expect("the widget draws");
    let buffer = terminal.backend().buffer();
    (0..width)
        .map(|x| buffer[(x, 13)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

/// m8/11e F2, E3: J's own footer is the bar he sees, so it carries the
/// doorway — on every row kind, and while the read is not ready
/// (`close  ? keys`).
#[test]
fn footer_ends_with_the_keys_doorway() {
    for (what, view, clauses) in &footer_cases() {
        crate::test_support::assert_place_footer(what, clauses, &[], |width, keys| {
            footer_row(view, width, keys)
        });
    }
}

/// Every row kind J draws a footer for, and not ready, each with the full
/// clauses it draws under the shipped keymap.
fn footer_cases() -> Vec<(&'static str, JournalView, Vec<&'static str>)> {
    let mut cases: Vec<(&str, JournalView, Vec<&str>)> = Vec::new();
    // A Graft row: `⏎` worded, its command log.
    let mut graft = view(
        vec![row("2m ago", "commit \"x\"", RowMark::NextUndo)],
        JournalStatus::Ready,
    );
    graft.enter_words = Some("undo this".to_owned());
    graft.branch_here = true;
    cases.push((
        "graft row",
        graft,
        vec![
            "⏎ undo this",
            "b new branch here",
            "/ find",
            "space command log",
            "J/esc close",
        ],
    ));
    // A row `⏎` cannot name (a fetch, a live step): no `⏎` clause.
    cases.push((
        "no ⏎ words",
        view(mock_rows(), JournalStatus::Ready),
        vec!["/ find", "space command log", "J/esc close"],
    ));
    // A shell step, and a lost commit with a find kept.
    let mut shell = view(mock_rows(), JournalStatus::Ready);
    shell.selected = 1;
    shell.enter_words = Some("put main back to before this".to_owned());
    cases.push((
        "shell row",
        shell,
        vec!["⏎ put main back to before this", "/ find", "J/esc close"],
    ));
    let mut lost_row = view(mock_rows(), JournalStatus::Ready);
    lost_row.selected = 2;
    lost_row.enter_words = Some("put main back to it".to_owned());
    lost_row.branch_here = true;
    lost_row.find = Some(JournalFindView {
        query: sanitize("main"),
        at: Some(1),
        of: 2,
    });
    cases.push((
        "lost row, find kept",
        lost_row,
        vec![
            "⏎ put main back to it",
            "b new branch here",
            "/ find",
            "n/N match",
            "J/esc close",
        ],
    ));
    for status in [
        JournalStatus::Loading,
        JournalStatus::Empty,
        JournalStatus::Failed(sanitize("bad journal")),
    ] {
        cases.push(("not ready", view(Vec::new(), status), vec!["J/esc close"]));
    }
    cases
}

mod replaced;
mod source_column;

/// m8/59 Major A: while a write runs, `Esc` stops it before J sees the key
/// (graft's `update/keys.rs`, design note choice 4). The bar must say so:
/// `J close` and `esc cancels`, never `J/esc close` — on a ready list and
/// while the read is out alike.
#[test]
fn while_a_write_runs_the_bar_says_esc_cancels_it() {
    let keys = Keymap::shipped();
    for status in [JournalStatus::Ready, JournalStatus::Loading] {
        let mut view = view(mock_rows(), status.clone());
        view.esc_cancels = true;
        let bar = footer_row(&view, 120, keys);
        assert!(bar.contains("J close"), "{status:?}: {bar}");
        assert!(bar.contains("esc cancels"), "{status:?}: {bar}");
        assert!(!bar.contains("J/esc"), "{status:?}: {bar}");

        view.esc_cancels = false;
        let bar = footer_row(&view, 120, keys);
        assert!(bar.contains("J/esc close"), "{status:?}: {bar}");
        assert!(!bar.contains("cancels"), "{status:?}: {bar}");
    }
}
