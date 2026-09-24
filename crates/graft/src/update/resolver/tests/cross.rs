//! `n`/`N` reach every block, taken or not, and cross files in the list's
//! order (m6-conflicts/15). Split out of `advance.rs` (review
//! m6-amend-15 M3), whose fixtures it shares.

use std::path::Path;
use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_git::conflict::{ConflictFile, Take};
use graft_git::status::UnmergedHow;

use super::advance::{cursor, landed, notice, on, open_path, sent, three};
use super::{press, sticky, takes};
use crate::jobs::conflict::{OpenedFile, Opening};
use crate::update::{AppEvent, AppState, Command, Product, update};

/// `three.txt` alone.
fn three_alone() -> AppState {
    on("three.txt", three(), &["three.txt"], &["three.txt"])
}

#[test]
#[allow(
    non_snake_case,
    reason = "`n` and `N` are two different keys, and the test is named for them"
)]
fn n_and_N_reach_a_taken_block() {
    let (state, commands) = press(&three_alone(), KeyCode::Char('t'));
    let (state, _) = landed(&state, &sent(&commands)[0]);
    assert_eq!(cursor(&state), Some(1));
    let (state, _) = press(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(0), "N lands on the taken block");
    let (state, commands) = press(&state, KeyCode::Char('o'));
    let out = sent(&commands);
    assert_eq!(out.len(), 1);
    assert_eq!(
        (out[0].1, out[0].2),
        (crate::update::TakeAt::Block(0), Take::Ours),
        "a re-take"
    );
    assert_eq!(cursor(&state), Some(1), "and on to block 1");
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(2));
}

/// The opening read of the file the walk crossed to lands with `file`.
fn crossed_read_lands(state: &AppState, commands: &[Command], file: ConflictFile) -> AppState {
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no OpenConflict: {commands:?}"));
    update(
        state,
        AppEvent::Produced(
            id,
            Product::ConflictOpened(Ok(Opening::Unmerged(Arc::new(OpenedFile {
                file: Some(file),
                how: UnmergedHow::BothModified,
                binary: false,
                on_disk: None,
                start_over: None,
            })))),
        ),
    )
    .0
}

#[test]
fn n_at_the_last_block_crosses_in_list_order() {
    let mut state = on("c", three(), &["a", "b", "c"], &["a", "c"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, commands) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("a"), "wrapped, b skipped");
    assert_eq!(notice(&state), Some("▸ a"));
    let state = crossed_read_lands(&state, &commands, three());
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some("▸ a"));

    let (state, commands) = press(&state, KeyCode::Char('N'));
    assert_eq!(open_path(&state), Path::new("c"), "backward, wrapped");
    assert_eq!(notice(&state), Some("▸ c"));
    let state = crossed_read_lands(&state, &commands, three());
    assert_eq!(cursor(&state), Some(2), "N lands on the last block");
}

#[test]
fn n_crosses_before_it_wraps() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("b"));
    assert_eq!(notice(&state), Some("▸ b"));
}

/// `three.txt` alone, blocks 1 and 2 taken, block 0 still open, the
/// cursor on the last block.
fn only_block_0_open() -> AppState {
    let mut file = three();
    file.take(1, Take::Ours).expect("block 1");
    file.take(2, Take::Ours).expect("block 2");
    let mut state = on("three.txt", file, &["three.txt"], &["three.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    state
}

#[test]
fn n_wraps_when_no_other_file_is_open() {
    let (state, commands) = press(&only_block_0_open(), KeyCode::Char('n'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{commands:?}"
    );
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some("wrapped · 1 still open"));
}

#[test]
fn n_past_the_end_with_nothing_open_says_no_more() {
    let mut file = three();
    for block in 0..3 {
        file.take(block, Take::Ours).expect("the block");
    }
    let mut state = on("three.txt", file, &["three.txt"], &["three.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(2), "unmoved");
    assert_eq!(
        sticky(&state),
        ("no more conflicts", Some("⏎ marks three.txt done"))
    );
    let mut back = state.clone();
    back.resolver.as_mut().expect("open").cursor_block = Some(0);
    let (back, _) = press(&back, KeyCode::Char('N'));
    assert_eq!(cursor(&back), Some(0), "unmoved");
    assert_eq!(
        sticky(&back),
        ("no more conflicts", Some("⏎ marks three.txt done"))
    );
}

#[test]
fn a_cross_waits_for_the_take_in_flight() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    // Blocks 0 and 1 taken already, so `t` on 2 takes the last open one.
    {
        let open = state.resolver.as_mut().expect("open");
        let mut file = (**open.file.as_ref().expect("a document")).clone();
        file.take(0, Take::Ours).expect("block 0");
        file.take(1, Take::Ours).expect("block 1");
        open.file = Some(Arc::new(file));
    }
    let (state, first) = press(&state, KeyCode::Char('t'));
    assert_eq!(takes(&first), 1);
    let (state, second) = press(&state, KeyCode::Char('n'));
    assert!(
        !second
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{second:?}"
    );
    assert_eq!(open_path(&state), Path::new("a"), "nothing selected");
    assert_eq!(sticky(&state).0, "still writing the last take");
}
