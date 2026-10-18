//! The live clash bar on a solved merge file (ease-pass/05c): in a merge
//! `⏎` marks nothing, so a solved file's bar points on — `n` to the next
//! file with a clash open, else `y` to the review — and its row reads
//! `solved`, never `⏎ marks done`. A rebase keeps today's bar and row.

use mergewright_git::conflict::{ConflictFile, Take};
use mergewright_ui::conflict_resolver::FileRowLook;

use super::advance::{on, three};
use super::bar;
use super::review_at_y::as_rebase;
use crate::update::AppState;

fn solved() -> ConflictFile {
    let mut file = three();
    file.take_all(Take::Theirs);
    file
}

fn row_of(state: &AppState, path: &str) -> FileRowLook {
    crate::view::resolver::file_list_view(state)
        .rows
        .into_iter()
        .find(|row| row.name.as_str() == path)
        .expect("listed")
        .state
}

#[test]
fn a_merge_with_every_clash_solved_bars_y_review_the_merge() {
    let state = on("a", solved(), &["a"], &["a"]);
    let words = bar(&state);
    assert!(
        words.starts_with(&[
            "y review the merge".to_owned(),
            "u undo pick".to_owned(),
            "esc pause, come back later".to_owned(),
        ]),
        "{words:?}"
    );
    assert!(
        !words
            .iter()
            .any(|hint| hint.contains("done") || hint.contains("start over")),
        "{words:?}"
    );
}

#[test]
fn a_solved_merge_file_with_a_clash_open_elsewhere_bars_n_to_that_file() {
    let state = on("a", solved(), &["a", "docs/b.md"], &["a", "docs/b.md"]);
    let words = bar(&state);
    assert_eq!(
        words.first().map(String::as_str),
        Some("n next file: b.md"),
        "{words:?}"
    );
    assert!(
        !words.iter().any(|hint| hint.starts_with("y ")),
        "{words:?}"
    );
    assert!(!words.iter().any(|hint| hint.contains("done")), "{words:?}");
}

#[test]
fn a_rebase_with_every_clash_solved_draws_no_y() {
    let state = as_rebase(on("a", solved(), &["a"], &["a"]));
    let words = bar(&state);
    assert!(
        !words.iter().any(|hint| hint.starts_with("y ")),
        "{words:?}"
    );
    assert!(words.iter().any(|hint| hint == "⏎ done"), "{words:?}");
}

#[test]
fn a_merge_still_open_keeps_the_take_bar() {
    let state = on("a", three(), &["a"], &["a"]);
    let words = bar(&state);
    assert!(
        !words.iter().any(|hint| hint.starts_with("y ")),
        "{words:?}"
    );
}

#[test]
fn a_solved_merge_file_s_row_reads_solved_and_a_rebase_s_still_marks_done() {
    let merge = on("a", solved(), &["a"], &["a"]);
    assert_eq!(row_of(&merge, "a"), FileRowLook::Solved);
    let rebase = as_rebase(merge);
    assert_eq!(row_of(&rebase, "a"), FileRowLook::AllTaken);
}
