//! G9–G14: the cursor row's second line, the header's counts, the
//! pane, the bar and `#`, on the mock-shaped repository; one test per id.
//! Shapes and words only; colour is the buffer's attribute at most (rule 5).

use mergewright_core::event::KeyCode;

use super::fixture::{self, mock};
use super::screen::{
    bar, buffer, coloured_cells, cursor_on, drawn, key_at_rest, line_after, node_column, open,
    pane, sized, top,
};

/// G9: the cursor row's second line `<who> · <N hours> ago · made on
/// <branch> … · <sha7>`, lanes continuing through it; a row inside an
/// opened merge gets it too.
#[test]
fn g9_cursor_row_second_line() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let state = cursor_on(&app, state, fixture::SYNC_MERGE);
    let frame = drawn(&state);
    let second = line_after(&frame, fixture::SYNC_MERGE);
    let words = second.trim_end().trim_end_matches('│').trim_end();
    assert!(
        words.contains("Omar · 28 hours ago · made on ops/ci · "),
        "{second:?}"
    );
    let sha = words.rsplit(" · ").next().unwrap_or_default();
    assert!(
        sha.len() == 7 && sha.chars().all(|c| c.is_ascii_hexdigit()),
        "{second:?}"
    );
    let lane = node_column(&frame, fixture::SYNC_MERGE);
    assert_eq!(
        second.chars().nth(lane),
        Some('┃'),
        "the lane runs on: {second:?}"
    );

    let state = cursor_on(&app, state, fixture::NAMED_MERGE);
    let state = key_at_rest(&app, &state, ' ');
    let state = key_at_rest(&app, &state, 'j');
    let frame = drawn(&state);
    assert!(
        line_after(&frame, fixture::RECEIPTS)
            .contains("Dana · 27 hours ago · made on dana/receipts · ")
    );
    app.close();
}

/// G10 and G11: with fewer rows than lanes need, the top line ends
/// `lanes drawn: N of M` and lanes with no commit in view are not drawn;
/// with everything in view it reads `2 merged branches folded`.
#[test]
fn g10_g11_header_counts_lanes_and_folds() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let frame = drawn(&state);
    assert!(
        top(&frame)
            .trim_end()
            .ends_with("14 commits  2 merged branches folded"),
        "{:?}",
        top(&frame)
    );
    assert!(
        !top(&frame).contains("lanes drawn"),
        "nothing dropped: {:?}",
        top(&frame)
    );
    let short = sized(&app, &state, 120, 10);
    let short = key_at_rest(&app, &short, 'G');
    let frame = drawn(&short);
    assert!(
        top(&frame).trim_end().ends_with("lanes drawn: 5 of 7"),
        "{:?}",
        top(&frame)
    );
    assert!(
        !frame
            .iter()
            .any(|line| line.contains("Start the wallet release"))
    );
    app.close();
}

/// G12 on the mock: the pane's heading, four marked lines and the route
/// trunk first; `(you're here)` on `HEAD`'s branch after his commit.
#[test]
fn g12_pane_heading_marks_and_route() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    let lines = pane(&drawn(&state), 120);
    assert_eq!(lines[0], "integration");
    let marked: Vec<&String> = lines.iter().skip(1).take(4).collect();
    assert!(
        marked.iter().all(|line| line.starts_with(['!', '✓', '·'])),
        "{lines:#?}"
    );
    let built = lines
        .iter()
        .position(|line| line == "built on")
        .expect("built on");
    assert_eq!(lines[built + 1], "main");
    assert_eq!(lines[built + 2], "─◆ integration");
    app.close();
}

/// G13: the bar per row kind — a teammate's row, a folded merge, an
/// opened merge, a live branch's row inside it.
#[test]
fn g13_bar_per_row_kind() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 200);
    let state = cursor_on(&app, state, fixture::INTEGRATION);
    assert!(bar(&drawn(&state)).trim_start().starts_with(
        "m merge integration into yours · ⏎ open · o take · C switch to integration · x compare"
    ));
    let state = cursor_on(&app, state, fixture::NAMED_MERGE);
    assert!(
        bar(&drawn(&state))
            .trim_start()
            .starts_with("space show the 1 commit inside · ⏎ open · x compare")
    );
    let state = key_at_rest(&app, &state, ' ');
    assert!(
        bar(&drawn(&state))
            .trim_start()
            .starts_with("space hide the commits inside · ⏎ open · x compare")
    );
    let state = key_at_rest(&app, &state, 'j');
    let inside = bar(&drawn(&state)).to_owned();
    assert!(
        inside
            .trim_start()
            .starts_with("m merge receipts into yours"),
        "{inside:?}"
    );
    assert!(!inside.contains("space"), "{inside:?}");
    app.close();
}

/// G14: `#` takes every colour off; the frame's characters stay as they
/// were.
#[test]
fn g14_hash_turns_colour_off_and_keeps_shapes() {
    let scratch = mock();
    let (app, state) = open(scratch.path(), 120);
    let before = drawn(&state);
    assert!(coloured_cells(&buffer(&state)) > 0);
    let off = app.key(&state, KeyCode::Char('#'));
    assert_eq!(coloured_cells(&buffer(&off)), 0);
    assert_eq!(drawn(&off), before);
    app.close();
}
