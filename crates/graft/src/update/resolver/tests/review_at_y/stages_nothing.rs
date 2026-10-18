//! In a merge `⏎`, `n`, `T` and `⌃T` stage nothing and point the way on
//! (ease-pass/05b). Moved from `review_at_y.rs` (ease-pass/05s), same names.

use std::path::Path;

use graft_core::event::KeyCode;

use super::super::advance::{cursor, landed, notice, on, open_path, sent, three};
use super::super::{press, sticky};
use super::{as_rebase, nothing_staged, solved};
use crate::update::{AppEvent, AppState, Command, update};

#[test]
fn enter_on_a_solved_file_in_a_merge_goes_on_and_stages_nothing() {
    let state = on("a", solved(), &["a", "b"], &["a", "b"]);
    let (state, commands) = press(&state, KeyCode::Enter);
    nothing_staged(&commands);
    assert_eq!(
        open_path(&state),
        Path::new("b"),
        "on to the next open file"
    );
}

#[test]
fn enter_with_nothing_open_anywhere_says_y_reviews_the_merge() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Enter);
    nothing_staged(&commands);
    assert_eq!(
        notice(&state),
        Some("every clash is solved · y reviews the merge")
    );
}

#[test]
fn enter_on_an_open_file_in_a_merge_is_refused_as_today() {
    let state = on("a", three(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Enter);
    nothing_staged(&commands);
    assert!(sticky(&state).0.contains("still"), "{:?}", state.status);
}

/// A rebase keeps today's staging: `⏎` on a solved file is `git add`.
#[test]
fn enter_on_a_solved_file_in_a_rebase_still_stages() {
    let state = as_rebase(on("a", solved(), &["a"], &["a"]));
    let (_, commands) = press(&state, KeyCode::Enter);
    assert!(
        commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..))),
        "{commands:?}"
    );
}

#[test]
fn n_with_nothing_open_in_a_merge_says_y_reviews_the_merge() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(
        notice(&state),
        Some("no open clashes left · y reviews the merge")
    );
}

/// `T` in a merge: every open block in one pick, then on — and no `Stage`.
#[test]
fn shift_t_in_a_merge_takes_every_open_block_and_stages_nothing() {
    let state = on("a", three(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('T'));
    nothing_staged(&commands);
    let takes = sent(&commands);
    assert_eq!(takes.len(), 1, "one pick");
    let (state, commands) = landed(&state, &takes[0]);
    nothing_staged(&commands);
    assert_eq!(state.picks.in_file(Path::new("a")), 1, "one pick to undo");
    assert_eq!(
        notice(&state),
        Some("every clash is solved · y reviews the merge")
    );
}

/// `⌃T` in a merge: the same pick, the cursor stays, nothing staged.
#[test]
fn ctrl_t_in_a_merge_takes_every_open_block_and_the_cursor_stays() {
    let state = on("a", three(), &["a"], &["a"]);
    let (state, commands) = update(
        &state,
        AppEvent::Core(graft_core::event::Event::Key(graft_core::event::KeyPress {
            code: KeyCode::Char('t'),
            modifiers: graft_core::event::Modifiers {
                control: true,
                ..graft_core::event::Modifiers::NONE
            },
        })),
    );
    let takes = sent(&commands);
    assert_eq!(takes.len(), 1);
    let (state, commands) = landed(&state, &takes[0]);
    nothing_staged(&commands);
    assert_eq!(cursor(&state), Some(0));
}

/// The words once a merge's file is solved point the way on: `n` while a
/// clash may be open in another file, `y` once none is.
#[test]
fn the_last_take_in_a_merge_points_to_y_once_nothing_is_open_elsewhere() {
    let after_ctrl_t = |state: &AppState| {
        let (state, commands) = update(
            state,
            AppEvent::Core(graft_core::event::Event::Key(graft_core::event::KeyPress {
                code: KeyCode::Char('t'),
                modifiers: graft_core::event::Modifiers {
                    control: true,
                    ..graft_core::event::Modifiers::NONE
                },
            })),
        );
        landed(&state, &sent(&commands)[0]).0
    };
    let alone = after_ctrl_t(&on("a", three(), &["a"], &["a"]));
    let words = notice(&alone).expect("a notice");
    assert!(words.ends_with(" · y reviews the merge"), "{words}");
    let beside = after_ctrl_t(&on("a", three(), &["a", "b"], &["a", "b"]));
    let words = notice(&beside).expect("a notice");
    assert!(words.ends_with(" · n goes to the next"), "{words}");
}
