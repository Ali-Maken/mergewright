//! `n`: the first open clash of the operation, from the top (ease-pass 05).
//! What it skips, where it crosses to and what it says are in the
//! integration tests (`tests/it/resolver/picks.rs`); these are the two
//! states a real repository lands in only by luck — the cursor already
//! there, on a block and on a card.

use std::path::Path;

use graft_core::event::KeyCode;

use super::advance::{cursor, notice, on, open_path, sent, three, three_alone};
use super::{WALK, press, resolver_card_on_screen};
use crate::update::Command;
use crate::update::resolver::walk::ON_FIRST_OPEN;

fn opens_a_file(commands: &[Command]) -> bool {
    commands
        .iter()
        .any(|command| matches!(command, Command::OpenConflict(..)))
}

/// `n` already on the first open clash: said, so the bar's `n` is never a
/// dead key; the cursor stays.
#[test]
fn n_on_the_first_open_block_says_so() {
    let (state, commands) = press(&three_alone(), KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some(ON_FIRST_OPEN));
    assert!(sent(&commands).is_empty(), "nothing written");
    assert!(!opens_a_file(&commands), "{commands:?}");
}

/// A card is a clash the cursor cannot sit on a block of: the first file
/// open, `n` says so and stays.
#[test]
fn n_on_a_card_that_is_the_first_open_file_says_so() {
    let (state, commands) = press(&resolver_card_on_screen(WALK), KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("gone.txt"));
    assert_eq!(notice(&state), Some(ON_FIRST_OPEN));
    assert!(!opens_a_file(&commands), "{commands:?}");
}

/// `n` from `a`'s last block: the first open clash from the top is `a`'s
/// block 0, above the cursor — not `b`, which the walk crossed to before.
#[test]
fn n_goes_up_to_the_first_open_block_rather_than_cross() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, commands) = press(&state, KeyCode::Char('n'));
    assert_eq!(open_path(&state), Path::new("a"));
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), None, "from the top is no wrap");
    assert!(!opens_a_file(&commands), "{commands:?}");
}
