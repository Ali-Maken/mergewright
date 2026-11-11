//! `not merged` on a branch's tip row (ease-pass 09b): only from a landed
//! read in the branch pane's cache, never `reading…`, gone with the cache.

use super::{TOC, cursor_on, drawn, opened, read, rest_with, with_sibling};

/// Wide enough for sara's row to hold every label beside the pane.
const WIDE: u16 = 160;

/// `subject`'s row in the frame, left of the branch pane's `│` rule (the
/// pane's own `reading…` is not the row's).
fn row_of(frame: &[String], subject: &str) -> String {
    let line = frame
        .iter()
        .find(|line| line.contains(subject))
        .unwrap_or_else(|| panic!("{subject} on screen: {frame:#?}"));
    line.rsplit_once('│')
        .map_or(line.as_str(), |(row, _)| row)
        .to_owned()
}

#[test]
fn before_the_read_lands_saras_row_has_no_label() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = rest_with(&app, state, None);
    let row = row_of(&drawn(&state, 120), TOC);
    assert!(!row.contains("not merged"), "{row}");
    assert!(!row.contains("reading"), "never `reading…` on a row: {row}");
    app.close();
}

#[test]
fn once_saras_read_lands_her_tip_row_says_not_merged() {
    // 160 wide: at 120 beside the pane her message column holds the subject
    // and `no upstream set` only, and labels give way first (09 ruling W).
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), WIDE);
    let state = cursor_on(&app, state, TOC);
    let state = read(&app, state, WIDE);
    let row = row_of(&drawn(&state, WIDE), TOC);
    assert!(
        row.contains("no upstream set  not merged"),
        "after her label: {row}"
    );
    app.close();
}

#[test]
fn a_failed_read_draws_no_label() {
    let repo = with_sibling();
    let (app, state) = opened(repo.path(), 120);
    let state = cursor_on(&app, state, TOC);
    let state = rest_with(&app, state, Some("the disk went away"));
    let row = row_of(&drawn(&state, 120), TOC);
    assert!(!row.contains("not merged"), "{row}");
    app.close();
}
