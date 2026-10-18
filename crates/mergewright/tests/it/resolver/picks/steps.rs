//! `←`/`→` (`h`/`l`): the previous / next clash in the file on screen,
//! open or taken, clamped at its ends — against a real merge begun in the
//! shell (ticket ease-pass/05r, review ease-pass-05 M1).

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::KeyCode;
use mergewright_git::conflict::{BlockState, Take};

use super::super::{opened_on, resolver_on};
use super::{bytes, clashing_merge, cursor, doc, notice, on, pressed};
use crate::app_harness::App;

/// `keys` pressed in order, nothing to settle: movement sends nothing.
fn moved(app: &App, state: &AppState, keys: &[KeyCode]) -> AppState {
    keys.iter()
        .fold(state.clone(), |state, key| app.key(&state, *key))
}

fn clash_text(repo: &Path, line: usize, side: &str) -> bool {
    let file = String::from_utf8(bytes(repo, "c.txt")).expect("UTF-8 text");
    file.contains(&format!("line {line} from {side}\n"))
}

/// The review's case: one file, three clashes, all taken; back on clash 1
/// and re-taken, clash 3 is reached by keyboard — `→ →` — and changed:
/// three keys from clash 1 to a changed clash 3 (`→ → t`; lazygit's
/// `→ → ↓ space`, four).
fn the_review_walk(back: KeyCode, ahead: KeyCode) {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "c.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'o');
    assert_eq!((doc(&state).open(), cursor(&state)), (0, Some(2)), "solved");
    let state = moved(&app, &state, &[KeyCode::Char('N'), KeyCode::Char('N')]);
    assert_eq!(cursor(&state), Some(0), "clash 1");
    let state = pressed(&app, &state, 't');
    assert_eq!(cursor(&state), Some(0), "nothing open: the re-take stays");
    assert!(clash_text(repo, 2, "feature"), "clash 1 is theirs on disk");

    let state = moved(&app, &state, &[ahead, ahead]);
    assert_eq!(cursor(&state), Some(2), "{ahead:?}: clash 3, taken");
    let state = pressed(&app, &state, 'o');
    assert!(clash_text(repo, 12, "main"), "clash 3 reads kept-yours");
    assert_eq!(doc(&state).blocks()[2].state, BlockState::Took(Take::Ours));
    let state = pressed(&app, &state, 't');
    assert!(clash_text(repo, 12, "feature"), "and changes in one key");
    assert!(clash_text(repo, 2, "feature"), "clash 1 untouched");
    assert!(clash_text(repo, 7, "main"), "clash 2 untouched");

    let state = moved(&app, &state, &[back, back]);
    assert_eq!(cursor(&state), Some(0), "{back:?}: back to clash 1");
    assert_eq!(on(&state), Path::new("c.txt"));
    drop(state);
    app.close();
}

#[test]
fn the_arrows_reach_a_taken_clash_below_and_change_it() {
    the_review_walk(KeyCode::Left, KeyCode::Right);
}

#[test]
fn h_and_l_reach_a_taken_clash_below_and_change_it() {
    the_review_walk(KeyCode::Char('h'), KeyCode::Char('l'));
}

/// Clamped to the file: at either end the cursor stays, the notice names
/// the end and the file, and no other file opens — `c.txt` is the list's
/// last, so `→` past it would cross or wrap if it could.
#[test]
fn the_arrows_stop_at_the_files_ends_and_say_which() {
    let scratch = clashing_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "c.txt");
    let state = moved(&app, &state, &[KeyCode::Right, KeyCode::Right]);
    assert_eq!(cursor(&state), Some(2));
    let state = app.key(&state, KeyCode::Right);
    assert_eq!((on(&state), cursor(&state)), (Path::new("c.txt"), Some(2)));
    assert_eq!(notice(&state), "last clash in c.txt");
    assert!(open_job_free(&state), "nothing read, nothing written");

    let state = moved(&app, &state, &[KeyCode::Left, KeyCode::Left]);
    assert_eq!(cursor(&state), Some(0), "back to clash 1");
    let state = app.key(&state, KeyCode::Left);
    assert_eq!((on(&state), cursor(&state)), (Path::new("c.txt"), Some(0)));
    assert_eq!(notice(&state), "first clash in c.txt");
    assert!(open_job_free(&state));
    drop(state);
    app.close();
}

fn open_job_free(state: &AppState) -> bool {
    state
        .resolver
        .as_ref()
        .is_some_and(|open| open.job.is_none())
}
