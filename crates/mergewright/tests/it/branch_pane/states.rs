//! The pane before its read lands and when it fails, and the route reaching
//! the lanes' colours (ease-pass 09c checks 1 and 2).

use super::{
    MORE_DEVELOP, README, TOC, TOC_TWO, cursor_on, drawn, opened, pane, read, rest_with,
    with_moved_integration, with_sibling,
};

const ANSWERS: usize = 4;

/// The pane's lines between its heading and `built on`, blanks dropped.
fn answers(lines: &[String]) -> Vec<&String> {
    lines
        .iter()
        .skip(1)
        .take_while(|line| !line.starts_with("built on"))
        .filter(|line| !line.is_empty())
        .collect()
}

#[test]
fn before_the_read_lands_every_line_reads_reading() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = rest_with(&app, state, None);
    let lines = pane(&drawn(&state, 120), 120);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("sara/fix-toc"),
        "{lines:#?}"
    );
    let answers = answers(&lines);
    assert_eq!(answers.len(), ANSWERS, "{lines:#?}");
    assert!(answers.iter().all(|line| *line == "reading…"), "{lines:#?}");
    app.close();
}

/// One `branch_status` call answers all four lines, so a failure fails all
/// four, with one reason (09c Ruled: a lone `can't tell` would leave three
/// `reading…` with no read running).
#[test]
fn a_failed_read_says_cant_tell_on_all_four_lines_with_one_reason() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = rest_with(&app, state, Some("the disk went away"));
    let lines = pane(&drawn(&state, 120), 120);
    assert_eq!(
        lines.first().map(String::as_str),
        Some("sara/fix-toc"),
        "{lines:#?}"
    );
    let answers = answers(&lines);
    assert_eq!(answers.len(), ANSWERS, "{lines:#?}");
    assert!(
        answers
            .iter()
            .all(|line| *line == "⊗ can't tell: the disk went away"),
        "{lines:#?}"
    );
    assert!(
        lines.iter().all(|line| !line.starts_with(['!', '✓', '·'])),
        "no line carries a known-line mark: {lines:#?}"
    );
    app.close();
}

/// The column of the lane `subject`'s row starts: its node glyph, the
/// first of `◆●◇○` on that line.
fn node_column(frame: &[String], subject: &str) -> usize {
    let line = frame
        .iter()
        .find(|line| line.contains(subject))
        .unwrap_or_else(|| panic!("{subject} on screen: {frame:#?}"));
    line.chars()
        .position(|cell| "◆●◇○".contains(cell))
        .unwrap_or_else(|| panic!("a node on {line:?}"))
}

fn cell(frame: &[String], subject: &str, column: usize) -> char {
    frame
        .iter()
        .find(|line| line.contains(subject))
        .and_then(|line| line.chars().nth(column))
        .unwrap_or_else(|| panic!("{subject}'s row reaches column {column}"))
}

#[test]
fn the_route_reaches_the_colours_of_a_moved_integration_base() {
    let repo = with_moved_integration();
    let (app, state) = opened(repo.path(), 120);

    let on_sara = cursor_on(&app, state, TOC);
    let on_sara = read(&app, on_sara, 120);
    let frame = drawn(&on_sara, 120);
    // State: `look.route` is crate-private; the pane's `built on` is drawn
    // from the same read (`update::branch_pane::read` sets both).
    let lines = pane(&frame, 120);
    let route: Vec<&str> = lines
        .iter()
        .skip_while(|line| !line.starts_with("built on"))
        .skip(1)
        .take_while(|line| !line.is_empty())
        .map(|line| line.trim_start_matches(['─', '◆', ' ']))
        .collect();
    assert_eq!(
        route,
        ["main", "develop", "wallet-release", "sara/fix-toc"],
        "{lines:#?}"
    );
    let develop = node_column(&frame, MORE_DEVELOP);
    assert_eq!(cell(&frame, TOC_TWO, develop), '│', "{frame:#?}");

    let on_main = cursor_on(&app, on_sara, README);
    let on_main = read(&app, on_main, 120);
    let frame = drawn(&on_main, 120);
    assert_ne!(
        cell(&frame, TOC_TWO, develop),
        '│',
        "develop's lane is coloured by sara's route only while she is focused: {frame:#?}"
    );
    app.close();
}
