//! The cursor's second line next to the main mock (07b, G9), on the
//! main screen's repository: his own row, a teammate's inside the opened
//! merge, the opened merge's own.

use mergewright_core::event::MouseButton;

use super::{
    HEAD_NOTE, ROWS, drawn, label_words, main_screen_repo, open_the_named_merge, opened, shot,
};
use crate::captures::at_rest;

/// The cursor's second line next to the main mock (07b, G9): on his own
/// row, on a teammate's row inside the opened merge, and on the opened
/// merge row.
#[test]
fn capture_the_cursor_rows_second_line() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let mut shots = mergewright_fixtures::Captures::new();
    let own = drawn(&state, 120);
    let merge = open_the_named_merge(&app, &state);
    let frame = drawn(&merge, 120);
    let y = frame
        .iter()
        .position(|line| line.contains("toc: fix the anchors"))
        .expect("the inner row is drawn");
    let teammate = app.mouse_down(
        &merge,
        MouseButton::Left,
        40,
        u16::try_from(y).expect("short"),
    );
    let teammate = at_rest(&app, teammate);
    names_what_it_merged(&merge);
    for (name, frame, right) in [
        (
            "main-cursor-own-120.txt",
            own,
            "the cursor on HEAD's row `Draw the receipts list`; \
          under it `you · N hours ago · made on feature/receipts ·…`, cut at the branch \
          pane's rule (ease-pass 09)",
        ),
        (
            "main-cursor-teammate-120.txt",
            drawn(&teammate, 120),
            "the cursor on Sara's row \
          inside the opened merge; under it `Sara · N hours ago · made on sara/fix-toc · \
          <sha7>`, the sha cut at the branch pane's rule (ease-pass 09)",
        ),
        (
            "main-opened-merge-120.txt",
            frame,
            "the cursor on the opened merge row, `wallet-release`'s tip; under \
          it `you · N hours ago · made on wallet-release · …` (the lane's branch, as the \
          branch column names it: the lane took the name at this tip, ADR-0064), cut at \
          the branch pane's rule before `merged 1 from fix-toc` (ease-pass 09; asserted \
          at 200)",
        ),
    ] {
        shots.check(
            shot(name),
            format!(
                "{HEAD_NOTE}\n120x{ROWS}. What right looks like: {right}; muted, no bold, \
                 the lanes straight down through it (no node, no connector); only the \
                 cursor row has one; never `reading…`.\n\n{}\n",
                frame.join("\n")
            ),
        );
    }
    drop(teammate);
    app.close();
    shots.finish();
}

/// A known merge names what it merged by its last segment (G9), read at
/// [`super::WIDE`]: at 120 beside the pane the second line is cut before
/// that (ease-pass 09).
fn names_what_it_merged(merge: &mergewright::update::AppState) {
    assert!(
        label_words(merge)
            .iter()
            .any(|line| line.contains(" \u{b7} merged 1 from fix-toc \u{b7} ")),
        "{}",
        label_words(merge).join("\n")
    );
}

/// ADR-0064: on the release's first commit the second line, the pane's
/// heading and the bar's push state all name `wallet-release`, though the
/// lane came down from `feature/receipts`.
#[test]
fn on_the_stacked_branchs_commit_everything_names_that_branch() {
    let scratch = main_screen_repo();
    let (app, state) = opened(scratch.path());
    let frame = drawn(&state, 120);
    let y = frame
        .iter()
        .position(|line| line.contains("Start the wallet release"))
        .expect("the release's first commit is drawn");
    let state = app.mouse_down(
        &state,
        MouseButton::Left,
        40,
        u16::try_from(y).expect("short"),
    );
    let state = at_rest(&app, state);
    let frame = drawn(&state, 120);
    let at = frame
        .iter()
        .position(|line| line.contains("Start the wallet release"))
        .expect("still drawn");
    let second = &frame[at + 1];
    assert!(second.contains("made on wallet-release"), "{second:?}");
    let heading = frame
        .iter()
        .filter_map(|line| line.split_once('\u{2502}'))
        .map(|(_, pane)| pane.trim())
        .find(|pane| !pane.is_empty())
        .expect("the pane has a heading");
    assert!(heading.starts_with("wallet-release"), "{heading:?}");
    // `C` and `m` read the same focus as the pane (`focused`).
    let focused = mergewright::update::branch_pane::focused(&state).expect("a local branch");
    assert_eq!(focused.branch.name.as_bytes(), b"refs/heads/wallet-release");
    drop(state);
    app.close();
}
