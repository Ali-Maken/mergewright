//! The message column's give-way order (ease-pass 09 ruling W, the main
//! mock's one-cell order): labels drop first, right-most first and whole;
//! then the fold tail sheds ` from <branch>`, then its count, then drops;
//! the subject is cut with `…` last. Checked at 120 columns beside the
//! branch pane, where the column is narrowest for its width.

use mergewright_term::sanitize_line;
use ratatui::layout::Rect;

use super::super::Icons;
use super::super::layout::row_layout;
use super::super::{FoldCount, FoldMark, GraphView, Tracking, WalkView};
use super::{badged, branch, draw, linear, text, view};
use crate::branch_pane::BranchPaneView;

const FULL: &str = "\u{25b8} 6 commits from PR 7";
const COUNT: &str = "\u{25b8} 6 commits";
const LABEL: &str = "x is here";

/// The newest commit (screen row 1) says `subject`.
fn row(rows: &mut [mergewright_graph::row::Row], subject: &str) {
    rows[0].commit.message = subject.as_bytes().to_vec();
}

/// The pane open, a folded tail on screen row 1 and, when `label`, one more
/// branch's label there.
fn screen(rows: &[mergewright_graph::row::Row], label: bool) -> GraphView<'_> {
    let mut view = view(rows, WalkView::Complete { rows: 2 }, 1);
    view.pane = Some(BranchPaneView::NotOnBranch {
        sha7: sanitize_line("abc1234"),
    });
    view.rows[0].fold = Some(FoldMark {
        open: false,
        count: FoldCount::Known(6),
        from: Some(sanitize_line("PR 7")),
    });
    if label {
        view.rows[0].badges = vec![branch(
            "x",
            false,
            Tracking::InSync {
                remote: sanitize_line("origin"),
            },
        )];
    }
    view
}

/// What the message column of row 1 draws, at 120 with the pane: its cells
/// as one string, trailing blanks kept.
fn message(subject: &str, label: bool) -> String {
    let mut rows = linear(2);
    row(&mut rows, subject);
    let view = screen(&rows, label);
    let columns = row_layout(&view, Rect::new(0, 0, 120, 8)).columns;
    let line: Vec<char> = text(&draw(&view, 120, 8))[1].chars().collect();
    let x = usize::from(columns.subject_x);
    line[x..x + usize::from(columns.subject_width)]
        .iter()
        .collect()
}

/// The message column's width at 120 with the pane.
fn column() -> usize {
    let rows = linear(2);
    let view = screen(&rows, false);
    usize::from(
        row_layout(&view, Rect::new(0, 0, 120, 8))
            .columns
            .subject_width,
    )
}

fn subject(cells: usize) -> String {
    "Q".repeat(cells)
}

#[test]
fn the_subject_gives_way_last_in_the_message_column() {
    let width = column();
    let gap = 2;
    let fits = |s: &str, after: &str| {
        let drawn = message(s, false);
        assert!(drawn.starts_with(&format!("{s}  {after}")), "{drawn:?}");
        assert!(!drawn.contains('\u{2026}'), "no cut: {drawn:?}");
    };

    // A subject that fits keeps its whole tail; one cell more and the tail
    // sheds ` from …` first, then its count, then drops — the subject whole.
    let s = subject(width - gap - FULL.chars().count());
    fits(&s, FULL);
    let s = subject(width - gap - FULL.chars().count() + 1);
    fits(&s, COUNT);
    let drawn = message(&s, false);
    assert!(!drawn.contains(" from"), "{drawn:?}");
    let s = subject(width - gap - COUNT.chars().count() + 1);
    fits(&s, "\u{25b8}");
    let s = subject(width - gap);
    let drawn = message(&s, false);
    assert_eq!(drawn.trim_end(), s, "the tail dropped, the subject whole");

    // Labels drop before the tail, whole, with no `…` inside one.
    let s = subject(width - gap - FULL.chars().count() - gap - LABEL.len());
    let drawn = message(&s, true);
    assert_eq!(drawn.trim_end(), format!("{s}  {FULL}  {LABEL}"));
    let s = subject(width - gap - FULL.chars().count() - gap - LABEL.len() + 1);
    let drawn = message(&s, true);
    assert_eq!(
        drawn.trim_end(),
        format!("{s}  {FULL}"),
        "the label went first"
    );

    // A subject wider than the column by itself is cut, nothing after it.
    let s = subject(width + 5);
    let drawn = message(&s, true);
    assert_eq!(
        drawn,
        format!("{}\u{2026}", subject(width - 1)),
        "cut at the column's end, no tail, no label"
    );
}

#[test]
fn badges_drop_whole_before_the_subject_gives_way_at_80_columns() {
    let rows = linear(2);
    let screen = text(&draw(&badged(&rows, Icons::Plain), 80, 5));

    // Labels follow the subject (ease-pass G7). The ones that do not fit
    // drop whole, right-most first, never cut (ease-pass 09 ruling W).
    assert!(screen[1].contains("Commit 2  main is here"), "{screen:#?}");
    assert!(!screen[1].contains('…'), "no label is cut: {screen:#?}");
    assert!(
        screen[2].contains("Commit 1  topic is here  not pushed yet"),
        "{screen:#?}"
    );
}
