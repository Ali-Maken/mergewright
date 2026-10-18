//! A folding merge's tail (ease-pass 06, mock G4): `▸ 4 commits from PR
//! 16021` folded, `▾ …` opened, `>` / `v` in the ASCII tier; `▸ commits
//! from …` until the count lands, `▸ ? commits from …` when it failed; no
//! name recovered, no ` from …`. Muted, two cells after the subject (the
//! mock's gap, ease-pass 06r m3); short of room it sheds ` from …`, then
//! its count, before it drops, and never cuts a word (06r m2).

use mergewright_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::super::{FoldCount, FoldMark, WalkView};
use super::{draw, linear, text, view};
use mergewright_core::capabilities::ColorDepth;

use crate::paint::color;
use crate::test_support::terminal;
use crate::theme::{Theme, ThemeKind, Token};

fn mark(open: bool, count: FoldCount, from: Option<&str>) -> FoldMark {
    FoldMark {
        open,
        count,
        from: from.map(sanitize_line),
    }
}

/// The cell `needle` starts at on screen row `y`.
fn cell_of(terminal: &Terminal<TestBackend>, y: usize, needle: &str) -> (u16, u16) {
    let line = &text(terminal)[y];
    let at = line
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} in {line:?}"));
    let x = u16::try_from(line[..at].chars().count()).expect("fits");
    (x, u16::try_from(y).expect("fits"))
}

#[test]
fn the_tail_says_how_many_and_from_where() {
    use FoldCount::{Failed, Known, Pending};
    let pr = Some("PR 16021");
    assert_eq!(
        mark(false, Known(4), pr).words(false),
        "\u{25b8} 4 commits from PR 16021"
    );
    assert_eq!(
        mark(false, Known(1), Some("sara/fix-toc")).words(false),
        "\u{25b8} 1 commit from sara/fix-toc"
    );
    assert_eq!(
        mark(false, Pending, pr).words(false),
        "\u{25b8} commits from PR 16021"
    );
    assert_eq!(
        mark(false, Failed, pr).words(false),
        "\u{25b8} ? commits from PR 16021"
    );
    assert_eq!(
        mark(false, Known(4), None).words(false),
        "\u{25b8} 4 commits"
    );
    assert_eq!(
        mark(true, Known(4), pr).words(false),
        "\u{25be} 4 commits from PR 16021"
    );
    assert_eq!(
        mark(false, Known(4), pr).words(true),
        "> 4 commits from PR 16021"
    );
    assert_eq!(mark(true, Pending, None).words(true), "v commits");
}

#[test]
fn a_folding_merge_row_ends_with_its_tail_muted() {
    let rows = linear(2);
    let plain = view(&rows, WalkView::Complete { rows: 2 }, 1);
    let before = text(&draw(&plain, 120, 5));
    let marked = |tail: FoldMark| {
        let mut folded = view(&rows, WalkView::Complete { rows: 2 }, 1);
        folded.rows[0].fold = Some(tail);
        draw(&folded, 120, 5)
    };
    let folded = marked(mark(false, FoldCount::Known(4), Some("PR 16021")));
    let lines = text(&folded);
    assert!(
        lines[1].contains("Commit 2  \u{25b8} 4 commits from PR 16021 "),
        "{lines:#?}"
    );
    assert_eq!(lines[2], before[2], "a row with no tail draws as before");

    let theme = Theme::new(ThemeKind::Dark, &terminal(ColorDepth::TrueColor, false));
    let muted = color(theme.token(Token::TextMuted));
    let buffer = folded.backend().buffer();
    let (x, y) = cell_of(&folded, 1, "\u{25b8}");
    assert_eq!(buffer[(x, y)].fg, muted, "the arrow");
    let (from, _) = cell_of(&folded, 1, "PR 16021");
    assert_eq!(buffer[(from, y)].fg, muted, "the name");
    let (subject, _) = cell_of(&folded, 1, "Commit 2");
    assert_ne!(buffer[(subject, y)].fg, muted, "the subject keeps its ink");

    let opened = text(&marked(mark(true, FoldCount::Pending, None)));
    assert!(
        opened[1].contains("Commit 2  \u{25be} commits "),
        "{opened:#?}"
    );
}

/// At 80 columns a long subject is cut with `…` before the tail, which is
/// never cut; the ASCII tier's tail is ASCII.
#[test]
fn the_tail_survives_a_cut_subject() {
    let mut long = linear(2);
    long[0].commit.message =
        b"A very long subject line that is much wider than the row and needs the ellipsis".to_vec();
    let mut folded = view(&long, WalkView::Complete { rows: 2 }, 1);
    folded.rows[0].fold = Some(mark(false, FoldCount::Known(12), Some("PR 7")));
    let lines = text(&draw(&folded, 80, 5));
    assert!(
        lines[1].contains("\u{2026}  \u{25b8} 12 commits from PR 7 "),
        "{lines:#?}"
    );

    let rows = linear(2);
    let mut ascii = view(&rows, WalkView::Complete { rows: 2 }, 1);
    ascii.ascii = true;
    for open in [false, true] {
        ascii.rows[0].fold = Some(mark(open, FoldCount::Known(3), Some("PR 7")));
        let lines = text(&draw(&ascii, 120, 5));
        assert!(lines[1].contains("Commit 2 "), "{lines:#?}");
        assert!(lines[1].contains(" 3 commits from PR 7"), "{lines:#?}");
        assert!(lines[1].is_ascii(), "{:?}", lines[1]);
    }
}

/// ease-pass 06r m3: two cells between the subject and the tail, as the
/// mock draws them (`prototypes/ease-mocks/mergewright-main-screen.html:188`).
#[test]
fn the_fold_tail_sits_two_cells_after_the_subject() {
    let rows = linear(2);
    let mut folded = view(&rows, WalkView::Complete { rows: 2 }, 1);
    folded.rows[0].fold = Some(mark(false, FoldCount::Known(4), Some("PR 16021")));
    let terminal = draw(&folded, 120, 5);
    let (subject, _) = cell_of(&terminal, 1, "Commit 2");
    let (arrow, _) = cell_of(&terminal, 1, "\u{25b8}");
    assert_eq!(arrow, subject + 8 + 2, "{:?}", text(&terminal)[1]);
}

/// Which of the tail's forms a drawn row shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Form {
    Full,
    Count,
    Arrow,
    Gone,
}

const FULL: &str = "\u{25b8} 6 commits from omar/eligibility-api";
const COUNT: &str = "\u{25b8} 6 commits";

fn tail() -> FoldMark {
    mark(false, FoldCount::Known(6), Some("omar/eligibility-api"))
}

fn width(text: &str) -> usize {
    text.chars().count()
}

/// ease-pass 06r m2: short of room the tail sheds ` from …`, then its
/// count, before it drops, and never cuts a word.
#[test]
fn the_fold_tail_shortens_before_it_drops() {
    let tail = tail();
    let fits = |cells| tail.fitting(false, cells);
    assert_eq!(fits(width(FULL)).as_deref(), Some(FULL));
    assert_eq!(fits(width(FULL) - 1).as_deref(), Some(COUNT));
    assert_eq!(fits(width(COUNT)).as_deref(), Some(COUNT));
    assert_eq!(fits(width(COUNT) - 1).as_deref(), Some("\u{25b8}"));
    assert_eq!(fits(1).as_deref(), Some("\u{25b8}"));
    assert_eq!(fits(0), None);
    assert_eq!(
        mark(false, FoldCount::Known(6), Some("x")).fitting(true, 11),
        Some("> 6 commits".to_owned()),
        "ASCII sheds the same way"
    );
}

/// The form a drawn tail shows, from the cells at its arrow on: panics on
/// a cut one.
fn form_of(rest: &str, columns: u16) -> (Form, &'static str) {
    // The count form ends there: a space after it, and not ` from`'s.
    let ends = |after: &str| !after.starts_with(|c| c != ' ') && !after.starts_with(" f");
    if rest.starts_with(FULL) {
        (Form::Full, FULL)
    } else if rest.strip_prefix(COUNT).is_some_and(ends) {
        (Form::Count, COUNT)
    } else if !rest.starts_with("\u{25b8} 6") {
        (Form::Arrow, "\u{25b8}")
    } else {
        panic!("a cut tail at {columns}: {rest:?}")
    }
}

/// ease-pass 06r m2 on screen: narrowing the terminal, every form shows,
/// each two cells after a subject of at least one cell (the subject yields
/// first), none cut, and none while a longer one had room.
#[test]
fn narrowing_the_terminal_sheds_the_tail_before_dropping_it() {
    let mut long = linear(2);
    long[0].commit.message = b"Qa subject far wider than any row the sweep below draws".to_vec();
    let wide = text(&draw(
        &view(&long, WalkView::Complete { rows: 2 }, 1),
        120,
        5,
    ));
    let start = wide[1]
        .chars()
        .position(|cell| cell == 'Q')
        .expect("the subject");
    let mut seen = Vec::new();
    for columns in (24..=120).rev() {
        let mut folded = view(&long, WalkView::Complete { rows: 2 }, 1);
        folded.rows[0].fold = Some(tail());
        let line: Vec<char> = text(&draw(&folded, columns, 5))[1].chars().collect();
        let Some(arrow) = line.iter().position(|&cell| cell == '\u{25b8}') else {
            seen.push(Form::Gone);
            continue;
        };
        let (form, drawn) = form_of(&line[arrow..].iter().collect::<String>(), columns);
        assert_eq!(line[arrow - 2..arrow], [' ', ' '], "the gap at {columns}");
        assert_ne!(line[arrow - 3], ' ', "one gap, two cells, at {columns}");
        let subject = arrow - 2 - start;
        assert!(subject >= 1, "a subject cell at {columns}");
        let room = subject + 2 + width(drawn);
        let longer = match form {
            Form::Full | Form::Gone => None,
            Form::Count => Some(FULL),
            Form::Arrow => Some(COUNT),
        };
        if let Some(longer) = longer {
            assert!(
                room < 1 + 2 + width(longer),
                "{longer:?} had room at {columns}"
            );
        }
        seen.push(form);
    }
    seen.dedup();
    for form in [Form::Full, Form::Count, Form::Arrow, Form::Gone] {
        assert!(seen.contains(&form), "{form:?} never drawn: {seen:?}");
    }
}
