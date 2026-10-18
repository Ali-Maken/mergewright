//! The clash bar (ease-pass/11, mock M11): `clash_bar` picks where the
//! cursor's clash and file stand, and the footer draws its words. Over
//! `clash_look`'s fixture.

use std::path::PathBuf;

use mergewright_git::conflict::Take;
use mergewright_git::state::InProgress;
use mergewright_term::sanitize_line;
use mergewright_ui::graph_screen::hints::ClashAt;

use super::clash_look::{merge, stopped, stopped_with, taken};
use super::frame;
use crate::update::AppState;
use crate::update::resolver::RowState;
use crate::view::rows::clash_bars::clash_bar;

/// Where `state`'s bar stands, or `None` for today's resolver bar.
fn at(state: &AppState) -> Option<ClashAt> {
    clash_bar(state).map(|bar| bar.at)
}

/// The footer: the frame's last row.
fn footer(state: &AppState) -> String {
    let drawn = frame(state, 160, 30);
    drawn.lines().last().expect("a frame has rows").to_owned()
}

/// `state` with `notes.md` read and every clash in it taken.
fn notes_solved(mut state: AppState) -> AppState {
    let open = state.resolver.as_mut().expect("open");
    open.seen
        .insert(PathBuf::from("notes.md"), RowState::AllTaken);
    state
}

#[test]
fn an_open_clash_bars_the_picks() {
    let state = merge();
    assert_eq!(at(&state), Some(ClashAt::Open));
    assert!(
        footer(&state)
            .trim_start()
            .starts_with("o take yours (left)")
    );
}

#[test]
fn a_taken_clash_with_one_open_bars_next_and_undo() {
    let state = taken(merge(), &[(0, Take::Ours)]);
    assert_eq!(at(&state), Some(ClashAt::Picked));
    let bar = footer(&state);
    assert!(bar.contains("n next open clash · u undo pick"), "{bar:?}");
}

#[test]
fn a_solved_file_bars_the_next_file_by_its_whole_path() {
    let state = taken(merge(), &[(0, Take::Ours), (1, Take::Ours)]);
    assert_eq!(
        at(&state),
        Some(ClashAt::FileSolved {
            next: sanitize_line("notes.md")
        })
    );
    assert!(footer(&state).contains("n next file: notes.md"));
}

/// The bar's `next` is the next file's whole path; its words name the file.
#[test]
fn a_solved_file_beside_one_in_a_folder_bars_its_whole_path() {
    let both = [(0, Take::Ours), (1, Take::Ours)];
    let state = taken(stopped_with(InProgress::Merge, "docs/notes.md"), &both);
    let next = sanitize_line("docs/notes.md");
    assert_eq!(at(&state), Some(ClashAt::FileSolved { next }));
    assert!(footer(&state).contains("n next file: notes.md"));
}

#[test]
fn a_merge_all_solved_bars_the_review() {
    let state = notes_solved(taken(merge(), &[(0, Take::Ours), (1, Take::Ours)]));
    assert_eq!(at(&state), Some(ClashAt::AllSolved));
    assert!(footer(&state).contains("y review the merge"));
}

#[test]
fn a_rebase_names_its_sides_and_never_offers_the_review() {
    let open = stopped(InProgress::Rebase);
    assert_eq!(at(&open), Some(ClashAt::Open));
    assert!(
        footer(&open)
            .trim_start()
            .starts_with("o take upstream (left)")
    );
    let solved = notes_solved(taken(open, &[(0, Take::Ours), (1, Take::Ours)]));
    // Today's resolver bar: `⏎` marks the file, `c` once every file is.
    assert_eq!(at(&solved), None);
    assert!(!footer(&solved).contains("review the merge"));
}
