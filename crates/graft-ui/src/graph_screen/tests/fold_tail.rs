//! A folding merge's tail (ease-pass 06, mock G4): `▸ 4 commits from PR
//! 16021` folded, `▾ …` opened, `>` / `v` in the ASCII tier; `▸ commits
//! from …` until the count lands, `▸ ? commits from …` when it failed; no
//! name recovered, no ` from …`. Muted, one space after the subject, never
//! cut.

use graft_term::sanitize_line;
use ratatui::Terminal;
use ratatui::backend::TestBackend;

use super::super::{FoldCount, FoldMark, WalkView};
use super::{draw, linear, text, view};
use graft_core::capabilities::ColorDepth;

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
        lines[1].contains("Commit 2 \u{25b8} 4 commits from PR 16021 "),
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
        opened[1].contains("Commit 2 \u{25be} commits "),
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
        lines[1].contains("\u{2026} \u{25b8} 12 commits from PR 7 "),
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
