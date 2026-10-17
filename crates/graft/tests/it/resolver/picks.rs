//! ease-pass 05: picks. A take moves the cursor to the file's first open
//! clash; `n` goes to the first open clash of the whole operation; `u` in
//! the resolver puts the last pick back, byte for byte — against a real
//! merge begun in the shell, then (`lifetime`) a rebase, a stash put-back
//! and a merge begun in Graft.

use std::path::Path;

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress};
use graft_fixtures::{Scratch, Shape};
use graft_git::conflict::{BlockState, ConflictFile, Take};

use super::{git_stops, open, opened_on, resolver_on, settled};
use crate::app_harness::{App, git, git_as_graft};

mod lifetime;
mod reread;
mod steps;
mod stops;

/// Clashes on lines 2 and 7 of seven.
const TWO: &[usize] = &[2, 7];
/// Clashes on lines 2, 7 and 12 of twelve.
const THREE: &[usize] = &[2, 7, 12];

/// A file `clashes.last()` lines long, the lines in `clashes` worded for
/// `side` (plain on `base`). Four plain lines between two clashes keep git
/// from folding them into one.
fn lines(clashes: &[usize], side: &str) -> String {
    let last = clashes.last().copied().unwrap_or(1);
    (1..=last)
        .map(|n| {
            if side != "base" && clashes.contains(&n) {
                format!("line {n} from {side}\n")
            } else {
                format!("line {n}\n")
            }
        })
        .collect()
}

/// `files` written for `side` and committed.
pub(super) fn commit_side(repo: &Path, files: &[(&str, &[usize])], side: &str) {
    for (name, clashes) in files {
        std::fs::write(repo.join(name), lines(clashes, side)).expect("writable scratch");
    }
    git(repo, &["add", "--all"]);
    git_as_graft(repo, &["commit", "-q", "-m", side]);
}

/// A merge begun in the shell, stopped on `a.txt` and `b.txt` (two clashes
/// each) and `c.txt` (three) — the list's order.
fn clashing_merge() -> Scratch {
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let files: &[(&str, &[usize])] = &[("a.txt", TWO), ("b.txt", TWO), ("c.txt", THREE)];
    commit_side(repo, files, "base");
    git(repo, &["checkout", "-q", "-b", "feature"]);
    commit_side(repo, files, "feature");
    git(repo, &["checkout", "-q", "main"]);
    commit_side(repo, files, "main");
    git_stops(repo, &["merge", "feature"]);
    scratch
}

pub(super) fn bytes(repo: &Path, name: &str) -> Vec<u8> {
    std::fs::read(repo.join(name)).expect("the conflicted file reads")
}

pub(super) fn doc(state: &AppState) -> &ConflictFile {
    open(state).file.as_deref().expect("a document on screen")
}

pub(super) fn cursor(state: &AppState) -> Option<usize> {
    open(state).cursor_block
}

pub(super) fn notice(state: &AppState) -> String {
    open(state)
        .notice
        .as_ref()
        .map(|(words, _)| words.clone())
        .unwrap_or_default()
}

/// `key`, then whatever it sent — a take, an untake, a read — landed.
pub(super) fn pressed(app: &App, state: &AppState, key: char) -> AppState {
    settled(app, app.key(state, KeyCode::Char(key)))
}

fn on(state: &AppState) -> &Path {
    &open(state).path
}

#[test]
fn a_take_moves_to_the_first_open_clash_and_stays_once_the_file_is_solved() {
    let scratch = clashing_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    assert_eq!(cursor(&state), Some(0));
    let state = app.key(&state, KeyCode::Char('o'));
    assert_eq!(cursor(&state), Some(1), "at the keypress: clash 2");
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('o'));
    assert_eq!(cursor(&state), Some(1), "solved: the cursor stays");
    let state = settled(&app, state);
    assert_eq!(doc(&state).open(), 0);
    assert_eq!(on(&state), Path::new("a.txt"), "the solved file stays");
    drop(state);
    app.close();
}

/// Top-down, not "the next after the one taken": the cursor on c.txt's
/// clash 2 with clash 1 still open above it, `o` goes up to clash 1.
#[test]
fn a_take_goes_back_up_to_an_open_clash_above_the_cursor() {
    let scratch = clashing_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    // `N` from a.txt's first clash wraps to the last file's last clash.
    let state = settled(&app, app.key(&state, KeyCode::Char('N')));
    assert_eq!((on(&state), cursor(&state)), (Path::new("c.txt"), Some(2)));
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(1));
    let state = app.key(&state, KeyCode::Char('o'));
    assert_eq!(cursor(&state), Some(0), "clash 1 is the first open one");
    let state = settled(&app, state);
    assert_eq!(doc(&state).blocks()[1].state, BlockState::Took(Take::Ours));
    drop(state);
    app.close();
}

#[test]
fn u_puts_each_pick_back_byte_for_byte_newest_first() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let after_one = bytes(repo, "a.txt");
    let state = pressed(&app, &state, 'o');
    assert_ne!(bytes(repo, "a.txt"), after_one, "pick 2 wrote");

    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), after_one, "as before pick 2");
    assert_eq!(doc(&state).blocks()[1].state, BlockState::Open);
    assert_eq!(cursor(&state), Some(1), "on the clash it opened");

    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote, "the file git wrote");
    assert_eq!(doc(&state).open(), 2);
    assert_eq!(cursor(&state), Some(0));
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

#[test]
fn u_after_a_retake_restores_the_earlier_take() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let kept_yours = bytes(repo, "a.txt");
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(0), "back on clash 1");
    let state = pressed(&app, &state, 't');
    assert_eq!(
        doc(&state).blocks()[0].state,
        BlockState::Took(Take::Theirs)
    );

    let state = pressed(&app, &state, 'u');
    assert_eq!(
        doc(&state).blocks()[0].state,
        BlockState::Took(Take::Ours),
        "kept yours again, not open"
    );
    assert_eq!(bytes(repo, "a.txt"), kept_yours);
    assert_eq!(cursor(&state), Some(0));

    let state = pressed(&app, &state, 'u');
    assert_eq!(doc(&state).blocks()[0].state, BlockState::Open);
    assert_eq!(bytes(repo, "a.txt"), git_wrote);
    drop(state);
    app.close();
}

#[test]
fn one_u_undoes_a_whole_ctrl_o() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "c.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "c.txt");
    assert_eq!(doc(&state).open(), 3);
    let state = app.press(&state, KeyPress::control(KeyCode::Char('o')));
    let state = settled(&app, state);
    assert_eq!(doc(&state).open(), 0);
    assert_eq!(state.picks.in_file(Path::new("c.txt")), 1, "one pick");

    let state = pressed(&app, &state, 'u');
    assert_eq!(doc(&state).open(), 3);
    assert_eq!(bytes(repo, "c.txt"), git_wrote);
    assert_eq!(cursor(&state), Some(0));
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

#[test]
fn n_goes_to_the_first_open_clash_from_the_top() {
    let scratch = clashing_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'j');
    assert_eq!(on(&state), Path::new("b.txt"));

    // From file 2, with file 1 still open: back to file 1's open clash —
    // its taken clash still shown, its pick alive (ticket ease-pass/05r),
    // so the one left is the second.
    let state = pressed(&app, &state, 'n');
    assert_eq!(on(&state), Path::new("a.txt"));
    assert_eq!((doc(&state).open(), cursor(&state)), (1, Some(1)));

    // a.txt solved: `n` skips it, and b.txt's taken clash.
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'n');
    assert_eq!((on(&state), cursor(&state)), (Path::new("b.txt"), Some(0)));
    let state = pressed(&app, &state, 'o');
    let state = app.key(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(0), "N: the taken clash, as before");
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!((on(&state), cursor(&state)), (Path::new("b.txt"), Some(1)));

    // Nothing open anywhere, every clash here taken: none is open, and the
    // keys that reach them (review ease-pass-05 M1).
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'n');
    assert_eq!(on(&state), Path::new("c.txt"));
    let state = app.press(&state, KeyPress::control(KeyCode::Char('o')));
    let state = settled(&app, state);
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(on(&state), Path::new("c.txt"));
    assert_eq!(
        notice(&state),
        "no open clashes left \u{b7} y reviews the merge",
        "a merge: the next step is the review (ease-pass/05b)"
    );
    drop(state);
    app.close();
}

/// `n` is the first open clash from the top (ease-pass 05; J9's first step
/// was `n` crossing down from the card, m6-conflicts/15): from `notes.txt`
/// it crosses up to `gone.txt`'s card, the list's first file still open.
#[test]
fn n_crosses_up_to_a_card_still_open_above() {
    let scratch = super::file_list::gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "notes.txt");
    let state = app.key(&state, KeyCode::Char('n'));
    assert_eq!(open(&state).path, Path::new("gone.txt"));
    let state = settled(&app, state);
    assert!(open(&state).whole_file.is_some(), "its card");
    assert_eq!(open(&state).notice, Some(("▸ gone.txt".to_owned(), false)));
    drop(state);
    app.close();
    drop(scratch);
}

#[test]
fn a_pick_whose_file_changed_is_dropped_and_u_reaches_the_one_before() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let a_git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'j');
    let state = pressed(&app, &state, 'o');
    assert_eq!(state.picks.in_file(Path::new("b.txt")), 1);

    let edited = b"edited outside, between the pick and u\n";
    std::fs::write(repo.join("b.txt"), edited).expect("writable scratch");
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "b.txt"), edited, "the edit survives");
    let graft::update::Status::Sticky { reason, .. } = &state.status else {
        panic!("words for the stale pick: {:?}", state.status);
    };
    assert_eq!(
        reason, "b.txt changed on disk since that pick \u{b7} nothing undone",
        "said, never swallowed"
    );
    assert_eq!(state.picks.in_file(Path::new("b.txt")), 0, "its picks go");
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 1);

    let state = pressed(&app, &state, 'u');
    assert_eq!(on(&state), Path::new("a.txt"), "u crosses to the pick");
    assert_eq!(bytes(repo, "a.txt"), a_git_wrote);
    assert_eq!((doc(&state).open(), cursor(&state)), (2, Some(0)));
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

#[test]
fn u_with_no_pick_says_so() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let before = bytes(repo, "a.txt");
    let state = pressed(&app, &state, 'u');
    assert_eq!(notice(&state), "nothing to undo here");
    assert_eq!(bytes(repo, "a.txt"), before);
    drop(state);
    app.close();
}
