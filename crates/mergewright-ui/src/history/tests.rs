use mergewright_core::capabilities::ColorDepth;
use mergewright_term::sanitize::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::*;
use crate::syntax::SyntaxSpan;
use crate::test_support::terminal;
use crate::theme::ThemeKind;

const NOW: i64 = 2_000_000_000;
const DAY: i64 = 86_400;

pub(crate) fn theme() -> Theme {
    Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false))
}

/// Pass 1b's mock-up: four commits, Lina's selected; Priya's renamed the
/// file from `src/session.rs`.
fn mock_rows() -> Vec<HistoryRow<'static>> {
    vec![
        HistoryRow {
            sha7: "7f6e5d4",
            subject: "Handle expired tokens",
            who: "you",
            when: NOW,
            added: 1,
            removed: 1,
            was: None,
        },
        HistoryRow {
            sha7: "c4d5e6f",
            subject: "Move session TTL to config",
            who: "Priya Raman",
            when: NOW - 120 * DAY,
            added: 1,
            removed: 1,
            was: Some("src/session.rs"),
        },
        HistoryRow {
            sha7: "a1b2c3d",
            subject: "Retry refresh once on 401",
            who: "Lina",
            when: NOW - 210 * DAY,
            added: 3,
            removed: 1,
            was: None,
        },
        HistoryRow {
            sha7: "5e6f7a8",
            subject: "Add token refresh",
            who: "Tom",
            when: NOW - 300 * DAY,
            added: 12,
            removed: 0,
            was: None,
        },
    ]
}

fn line(kind: LineKindView, old_no: Option<u32>, new_no: Option<u32>, text: &str) -> LineView {
    LineView {
        kind,
        old_no,
        new_no,
        text: sanitize_line(text),
        words: Vec::new(),
        syntax: Vec::<SyntaxSpan>::new(),
        pair: None,
    }
}

/// Lina's commit as the whole file: lines 17–24, line 19 added, the old
/// line 22 removed and replaced.
fn lina_file() -> Vec<HunkView> {
    use LineKindView::*;
    let lines = vec![
        line(HunkHeading, None, None, ""),
        line(Context, Some(16), Some(16), "impl Session {"),
        line(
            Context,
            Some(17),
            Some(17),
            "    /// Refresh the token when the server says it expired.",
        ),
        line(
            Context,
            Some(18),
            Some(18),
            "    pub fn refresh_token(&mut self, client: &Client) -> Result<()> {",
        ),
        line(Added, None, Some(19), "        let mut attempt = 0;"),
        line(Context, Some(19), Some(20), "        loop {"),
        line(
            Context,
            Some(20),
            Some(21),
            "            let resp = client.refresh(&self.token)?;",
        ),
        line(
            Removed,
            Some(21),
            None,
            "            if resp.status() == 401 {",
        ),
        line(
            Added,
            None,
            Some(22),
            "            if resp.status() == 401 && attempt == 0 {",
        ),
        line(Added, None, Some(23), "                attempt += 1;"),
        line(Context, Some(22), Some(24), "                continue;"),
        line(Context, Some(23), Some(25), "            }"),
        line(Context, Some(24), Some(26), "            return Ok(());"),
        line(Context, Some(25), Some(27), "        }"),
        line(Context, Some(26), Some(28), "    }"),
        line(Context, Some(27), Some(29), "}"),
        line(Context, Some(28), Some(30), ""),
    ];
    vec![HunkView {
        heading: sanitize_line(""),
        lines,
    }]
}

const TITLE: &str = "src/auth/session.rs \u{b7} fn refresh_token, lines 18\u{2013}29";
const HEADER: &str = "a1b2c3d \u{b7} Lina \u{b7} 7 months ago \u{b7} Retry refresh once on 401 \u{b7} this file +3 \u{2212}1";
const END: &str = "the file starts here (5e6f7a8 added it)";

pub(crate) fn view<'a>(
    rows: &'a [HistoryRow<'a>],
    hunks: &'a [HunkView],
    theme: &'a Theme,
    focus: HistoryFocus,
) -> HistoryView<'a> {
    HistoryView {
        title: TITLE,
        tracing: false,
        list: HistoryList::Rows {
            rows,
            selected: Some(2),
            top: 0,
            end: Some(END),
        },
        file: FilePane::Lines {
            header: HEADER,
            hunks,
            top: 0,
            range: Some(18..=29),
            side_by_side: false,
            word_tint: false,
        },
        focus,
        now: NOW,
        theme,
        whitespace_ignored: false,
    }
}

fn draw(view: &HistoryView, width: u16, height: u16) -> Vec<String> {
    let mut terminal =
        Terminal::new(TestBackend::new(width, height)).expect("the test backend starts");
    terminal
        .draw(|frame| draw_history(frame, frame.area(), view, crate::keymap::Keymap::shipped()))
        .expect("the history place draws");
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

/// The file pane's part of a row: right of the rule.
fn file_side(row: &str) -> String {
    row.split_once('\u{2502}')
        .map_or(row, |(_, file)| file)
        .to_owned()
}

// --- Layout ----------------------------------------------------------------

#[test]
fn the_breakpoints_split_the_width() {
    use HistoryFocus::*;
    assert_eq!(
        columns(187, List),
        Columns {
            list: Some((0, 56)),
            file: Some((57, 130))
        }
    );
    assert_eq!(
        columns(140, File),
        Columns {
            list: Some((0, 40)),
            file: Some((41, 99))
        }
    );
    assert_eq!(
        columns(100, List),
        Columns {
            list: Some((0, 100)),
            file: None
        }
    );
    assert_eq!(
        columns(100, File),
        Columns {
            list: None,
            file: Some((0, 100))
        }
    );
}

#[test]
fn a_was_row_belongs_to_its_commit() {
    let rows = mock_rows();
    assert_eq!(list_index_at(&rows, 0, 0), Some(0));
    assert_eq!(list_index_at(&rows, 0, 1), Some(1));
    assert_eq!(list_index_at(&rows, 0, 2), Some(1), "Priya's ↳ was row");
    assert_eq!(list_index_at(&rows, 0, 3), Some(2));
    assert_eq!(list_index_at(&rows, 0, 5), None, "the end row");
    assert_eq!(list_index_at(&rows, 2, 0), Some(2));
    assert_eq!(rows_through(&rows, 0, 2), 4);
    assert_eq!(rows_through(&rows, 2, 3), 2);
}

#[test]
fn file_rows_skip_the_heading_and_find_lines() {
    let hunks = lina_file();
    assert_eq!(file_rows(&hunks).len(), 16);
    assert_eq!(row_of_new_line(&hunks, 18), Some(2));
    assert_eq!(row_of_new_line(&hunks, 99), Some(15), "clamped to the end");
    assert_eq!(row_of_old_line(&hunks, 21), Some(6), "the removed line");
    let changed = changed_rows(&hunks);
    assert_eq!(
        changed.iter().filter(|&&is| is).count(),
        4,
        "19, the old 22, 22, 23"
    );
}

// --- Rows ------------------------------------------------------------------

#[test]
fn removed_rows_have_no_number_and_the_bracket_sits_on_the_range() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    let frame = draw(&view(&rows, &hunks, &theme, HistoryFocus::List), 187, 20);
    let file: Vec<String> = frame.iter().map(|row| file_side(row)).collect();
    let removed = file
        .iter()
        .find(|row| row.contains("if resp.status() == 401 {"))
        .expect("the removed row");
    assert!(
        removed.contains("\u{2503}     \u{2212}"),
        "┃, no number, − in its column: {removed:?}"
    );
    let added = file
        .iter()
        .find(|row| row.contains("let mut attempt"))
        .expect("line 19");
    assert!(added.contains("\u{2503}  19 +"), "{added:?}");
    let before = file
        .iter()
        .find(|row| row.contains("/// Refresh"))
        .expect("line 17");
    assert!(
        !before.contains(RANGE_MARK) && before.trim_start().starts_with("17"),
        "line 17 is outside the range: {before:?}"
    );
    let signs: Vec<usize> = file
        .iter()
        .filter(|row| row.contains('+') || row.contains('\u{2212}'))
        .filter(|row| !row.contains("this file"))
        .map(|row| {
            row.chars()
                .position(|c| c == '+' || c == '\u{2212}')
                .expect("a sign")
        })
        .collect();
    assert!(
        signs.windows(2).all(|pair| pair[0] == pair[1]),
        "every sign in one column: {signs:?}"
    );
}

#[test]
fn the_list_marks_the_selected_row_and_the_rename() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    let frame = draw(&view(&rows, &hunks, &theme, HistoryFocus::List), 187, 20);
    assert!(frame[0].contains("HISTORY \u{b7} src/auth/session.rs"));
    assert!(frame[0].ends_with("4 changes"), "{}", frame[0]);
    assert!(
        frame[3].contains("\u{21b3} was src/session.rs"),
        "{}",
        frame[3]
    );
    assert!(frame[4].starts_with(" \u{25b8} a1b2c3d"), "{}", frame[4]);
    assert!(frame[4].contains("+3 \u{2212}1"), "{}", frame[4]);
    assert!(frame[6].contains(END), "{}", frame[6]);
}

#[test]
fn an_empty_trace_shows_its_end_row_alone() {
    let theme = theme();
    let end = "git stops tracing here: this clone is shallow; nothing older was fetched";
    let v = HistoryView {
        title: TITLE,
        tracing: false,
        list: HistoryList::Rows {
            rows: &[],
            selected: None,
            top: 0,
            end: Some(end),
        },
        file: FilePane::NoChange,
        focus: HistoryFocus::List,
        now: NOW,
        theme: &theme,
        whitespace_ignored: false,
    };
    let frame = draw(&v, 140, 10);
    let list: String = frame[1..4]
        .iter()
        .map(|row| row.split('\u{2502}').next().unwrap_or("").trim())
        .collect::<Vec<_>>()
        .join(" ");
    assert!(list.starts_with(end), "wrapped in the list: {list}");
    assert!(!frame[0].contains("change"), "no count: {}", frame[0]);
    assert!(file_side(&frame[1]).contains("no change to show"));
    assert!(
        !frame.iter().any(|row| row.contains('\u{25b8}')),
        "nothing selected"
    );
}

#[test]
fn a_failed_trace_is_one_line() {
    let theme = theme();
    let v = HistoryView {
        title: TITLE,
        tracing: false,
        list: HistoryList::Failed("git could not run"),
        file: FilePane::Empty,
        focus: HistoryFocus::List,
        now: NOW,
        theme: &theme,
        whitespace_ignored: false,
    };
    let frame = draw(&v, 140, 10);
    assert!(frame[1].starts_with("⊗ couldn't trace: git could not run"));
    assert!(!frame[0].contains("tracing"));
}

#[test]
fn a_binary_commit_shows_sizes() {
    let theme = theme();
    let rows = mock_rows();
    let v = HistoryView {
        file: FilePane::Binary {
            header: HEADER,
            old: Some(12 * 1024),
            new: Some(14 * 1024),
            sizes_failed: None,
        },
        ..view(&rows, &[], &theme, HistoryFocus::File)
    };
    let frame = draw(&v, 100, 10);
    assert!(
        frame[2].contains("binary \u{b7} 12 KB \u{2192} 14 KB"),
        "{}",
        frame[2]
    );
}

// --- Snapshots -------------------------------------------------------------

/// A snapshot whose header carries its "Right:" line.
macro_rules! snap {
    ($name:literal, $right:literal, $text:expr) => {
        insta::with_settings!({ description => concat!("Right: ", $right) }, {
            insta::assert_snapshot!($name, $text);
        })
    };
}

fn snapshot(v: &HistoryView, width: u16, height: u16) -> String {
    draw(v, width, height).join("\n")
}

#[test]
fn history_187x50() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    snap!(
        "history_187x50",
        "list 56 wide, ▸ on Lina's row, ↳ was under Priya's; the file pane: ┃ on 18–29, 19 +, the old 22 − with no number, signs in one column.",
        snapshot(&view(&rows, &hunks, &theme, HistoryFocus::List), 187, 50)
    );
}

#[test]
fn history_140x40() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    snap!(
        "history_140x40",
        "list 40 wide, authors as initials (PR for Priya Raman); the file pane as at 187.",
        snapshot(&view(&rows, &hunks, &theme, HistoryFocus::List), 140, 40)
    );
}

#[test]
fn history_100x30() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    snap!(
        "history_100x30",
        "one pane, the file (focused): the header, ┃ on 18–29, removed rows unnumbered.",
        snapshot(&view(&rows, &hunks, &theme, HistoryFocus::File), 100, 30)
    );
}

#[test]
fn history_tracing_140x40() {
    let theme = theme();
    let v = HistoryView {
        title: TITLE,
        tracing: true,
        list: HistoryList::Rows {
            rows: &[],
            selected: None,
            top: 0,
            end: None,
        },
        file: FilePane::Empty,
        focus: HistoryFocus::List,
        now: NOW,
        theme: &theme,
        whitespace_ignored: false,
    };
    snap!(
        "history_tracing_140x40",
        "the title ends tracing… · esc stops; both panes blank but the rule.",
        snapshot(&v, 140, 40)
    );
}

#[test]
fn history_side_by_side_187x50() {
    let rows = mock_rows();
    let hunks = lina_file();
    let theme = theme();
    let mut v = view(&rows, &hunks, &theme, HistoryFocus::File);
    if let FilePane::Lines { side_by_side, .. } = &mut v.file {
        *side_by_side = true;
    }
    let text = snapshot(&v, 187, 50);
    assert!(!text.contains(RANGE_MARK), "no ┃ side by side");
    snap!(
        "history_side_by_side_187x50",
        "the detail pane's side-by-side body in the file pane, no ┃; the title still names the range.",
        text
    );
}

#[test]
fn history_binary_140x40() {
    let theme = theme();
    let rows = mock_rows();
    let v = HistoryView {
        file: FilePane::Binary {
            header: HEADER,
            old: Some(700),
            new: Some(1500),
            sizes_failed: None,
        },
        ..view(&rows, &[], &theme, HistoryFocus::File)
    };
    snap!(
        "history_binary_140x40",
        "the file pane reads binary · 700 B → 2 KB under the header; no line rows.",
        snapshot(&v, 140, 40)
    );
}

#[test]
fn history_binary_sizes_failed_140x40() {
    let theme = theme();
    let rows = mock_rows();
    let v = HistoryView {
        file: FilePane::Binary {
            header: HEADER,
            old: None,
            new: None,
            sizes_failed: Some("git was stopped"),
        },
        ..view(&rows, &[], &theme, HistoryFocus::File)
    };
    snap!(
        "history_binary_sizes_failed_140x40",
        "the file pane reads binary · couldn't read sizes: git was stopped (dim, colour unverified); no line rows.",
        snapshot(&v, 140, 40)
    );
}

/// Spell sweep (c), ticket m8/10c: the tracing title names his `app.back`.
#[test]
fn history_spells_its_stop_key_from_his_keymap() {
    use crate::keymap::sweep::{assert_spelled, swept};
    let (keys, _) = swept();
    let theme = theme();
    let rows = mock_rows();
    let hunks = lina_file();
    let mut tracing = view(&rows, &hunks, &theme, HistoryFocus::List);
    tracing.tracing = true;
    assert_spelled(&title(&tracing, &keys), &keys, &[&["app.back"]]);
}

/// m8-polish-and-ship/17 item 8: with `w` on the title ends
/// `· whitespace ignored`; off, it does not.
#[test]
fn history_title_says_whitespace_ignored_while_w_is_on() {
    let theme = theme();
    let rows = mock_rows();
    let hunks = lina_file();
    let mut v = view(&rows, &hunks, &theme, HistoryFocus::List);
    assert!(!draw(&v, 187, 50)[0].contains("whitespace ignored"));
    v.whitespace_ignored = true;
    let title = draw(&v, 187, 50)[0].clone();
    assert!(
        title.trim_end().ends_with("\u{b7} whitespace ignored"),
        "{title:?}"
    );
}
