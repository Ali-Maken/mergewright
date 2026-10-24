//! The footer's problem line: it takes the footer's place, and the mark is the
//! renderer's (ease-pass 26).

use super::*;

#[test]
fn a_problem_takes_the_footers_place() {
    let rows = linear(2);
    let mut view = view(&rows, WalkView::Complete { rows: 2 }, 0);
    view.problem = Some(ProblemLine {
        kind: ProblemKind::Failure,
        reason: sanitize_line("job 3 failed: not a git repository"),
        keys: None,
    });
    let screen = text(&draw(&view, 60, 5));

    assert!(screen[4].contains("not a git repository"), "{screen:#?}");
    assert!(!screen[4].contains("⏎ open"), "{screen:#?}");
}

/// ease-pass 26, A3: the variant decides the mark. A `Failure` draws `⊗ `
/// once before its words; a `Notice` draws its words bare; narrow, the
/// words are cut and the mark stays.
#[test]
fn only_a_failure_problem_line_is_marked() {
    let rows = linear(2);
    let bar = |kind, reason: &str, columns: u16| {
        let mut shown = view(&rows, WalkView::Complete { rows: 2 }, 0);
        shown.problem = Some(ProblemLine {
            kind,
            reason: sanitize_line(reason),
            keys: None,
        });
        text(&draw(&shown, columns, 5)).pop().expect("a bar row")
    };
    let failure = bar(ProblemKind::Failure, "push refused", 60);
    assert!(failure.starts_with(" ⊗ push refused"), "{failure}");
    assert_eq!(failure.matches('⊗').count(), 1, "{failure}");
    let notice = bar(ProblemKind::Notice, "push refused", 60);
    assert!(notice.starts_with(" push refused"), "{notice}");
    assert!(!notice.contains('⊗') && !notice.contains("(x)"), "{notice}");
    let narrow = bar(ProblemKind::Failure, "push refused: the remote hung up", 14);
    assert!(narrow.starts_with(" ⊗ push"), "{narrow}");
    assert!(narrow.contains('…'), "the words are cut: {narrow}");
    let mut ascii = view(&rows, WalkView::Complete { rows: 2 }, 0);
    ascii.ascii = true;
    ascii.problem = Some(ProblemLine {
        kind: ProblemKind::Failure,
        reason: sanitize_line("push refused"),
        keys: None,
    });
    let row = text(&draw(&ascii, 60, 5)).pop().expect("a bar row");
    assert!(
        row.starts_with(" (x) push refused") && !row.contains('⊗'),
        "{row}"
    );
}
