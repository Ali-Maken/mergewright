//! `N` reaches every block, taken or not, and crosses files in the list's
//! order (m6-conflicts/15); `n` goes to the first open clash from the top
//! (ease-pass 05). Split out of `advance.rs` (review m6-amend-15 M3), whose
//! fixtures it shares.

use std::path::Path;
use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_git::conflict::{ConflictFile, Take};
use graft_git::status::UnmergedHow;

use super::advance::{cursor, landed, notice, on, open_path, sent, three};
use super::{press, sticky, takes};
use crate::jobs::conflict::{OpenedFile, Opening};
use crate::update::resolver::reviews_the_merge;
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
fn N_reaches_a_taken_block_and_n_skips_it() {
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
    // `n` is the first open clash from the top (ease-pass 05): back on the
    // taken block 0, it goes past it to block 1.
    let (state, _) = press(&state, KeyCode::Char('N'));
    assert_eq!(cursor(&state), Some(0));
    let (state, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(1), "n skips the taken block");
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
                unreadable: None,
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

/// Rule 1 before rule 2, for `N` (the rules are `n`'s no longer, ease-pass
/// 05): before `a`'s first block, `b` is not done, so `N` crosses to it
/// rather than wrap to `a`'s last open block.
#[test]
#[allow(non_snake_case, reason = "`N` is the key, not `n`")]
fn N_crosses_before_it_wraps() {
    let mut state = on("a", three(), &["a", "b"], &["a", "b"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(0);
    let (state, _) = press(&state, KeyCode::Char('N'));
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

/// `N` from block 0 with no other file to cross to wraps to the last open
/// block, and says so. `n` from block 2 reaches the same block from the
/// top: no wrap, so no words (ease-pass 05, `first_open.rs`).
#[test]
#[allow(non_snake_case, reason = "`N` is the key, not `n`")]
fn N_wraps_when_no_other_file_is_open() {
    let mut state = only_block_0_open();
    state.resolver.as_mut().expect("open").cursor_block = Some(0);
    let (state, commands) = press(&state, KeyCode::Char('N'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::OpenConflict(..))),
        "{commands:?}"
    );
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), Some("wrapped · 1 still open"));

    let (state, _) = press(&only_block_0_open(), KeyCode::Char('n'));
    assert_eq!(cursor(&state), Some(0));
    assert_eq!(notice(&state), None);
}

/// Every block of `three.txt` taken, the cursor on the last, in a list of
/// `paths` where only `three.txt` is unmerged: rule 3's ground.
fn nothing_open_on_three(paths: &[&str]) -> AppState {
    let mut file = three();
    for block in 0..3 {
        file.take(block, Take::Ours).expect("the block");
    }
    let mut state = on("three.txt", file, paths, &["three.txt"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    state
}

/// The frame's bottom row: the resolver's bar.
fn bottom_row(state: &AppState) -> String {
    let frame = super::frame_text(state);
    frame
        .lines()
        .last()
        .expect("a frame has rows")
        .trim_end()
        .to_owned()
}

/// Rule 3 (m6-conflicts/61): past the last block with nothing open anywhere,
/// `n` (and `N` before the first) says so on the notice row — an info
/// notice, not a warning — and leaves the status line alone, so the bar
/// keeps its own hints. The words are `none_open_words`' (ticket
/// ease-pass/05r): every clash here is taken, so the arrows are named.
#[test]
fn n_past_the_last_block_says_no_open_clashes_left() {
    let state = nothing_open_on_three(&["three.txt"]);
    let words = reviews_the_merge("no open clashes left", &state.keymap);
    let status_before = state.status.clone();
    let (after, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&after), Some(2), "unmoved");
    assert_eq!(
        after.resolver.as_ref().expect("open").notice,
        Some((words.clone(), false)),
        "an info notice"
    );
    assert_eq!(
        after.status, status_before,
        "the status line is not touched"
    );
    assert!(
        bottom_row(&after).contains("\u{23ce} done"),
        "{}",
        bottom_row(&after)
    );

    let mut back = state;
    back.resolver.as_mut().expect("open").cursor_block = Some(0);
    let (back, _) = press(&back, KeyCode::Char('N'));
    assert_eq!(cursor(&back), Some(0), "unmoved");
    assert_eq!(notice(&back), Some(words.as_str()));
    assert_eq!(back.status, status_before);
}

/// The words are said once, on the notice row: the bar under the document
/// does not repeat them, and it still offers `⏎ done`.
#[test]
fn no_open_clashes_left_is_not_repeated_in_the_bar() {
    let (after, _) = press(&nothing_open_on_three(&["three.txt"]), KeyCode::Char('n'));
    let frame = super::frame_text(&after);
    assert_eq!(
        frame.matches("no open clashes left").count(),
        1,
        "once, on the notice row:\n{frame}"
    );
    let bar = bottom_row(&after);
    assert!(!bar.contains("no open clashes left"), "{bar}");
    assert!(bar.contains("\u{23ce} done"), "{bar}");
}

/// The rule-3 notice keeps today's lifetime (Decisions, conductor
/// 2026-09-25): gone after `j` onto another row, replaced by a take landing,
/// replaced once `⏎`'s stage lands. `j`/`k` that go nowhere leave it up
/// (`the_notice_lifetime_today`).
#[test]
fn the_no_more_notice_is_gone_after_the_next_move() {
    let (said, _) = press(
        &nothing_open_on_three(&["a.txt", "three.txt"]),
        KeyCode::Char('n'),
    );
    let words = reviews_the_merge("no open clashes left", &said.keymap);
    assert_eq!(notice(&said), Some(words.as_str()));

    let (moved, _) = press(&said, KeyCode::Char('k'));
    assert_eq!(open_path(&moved), Path::new("a.txt"));
    assert_eq!(notice(&moved), None, "`k` onto another row");

    let (taking, commands) = press(&said, KeyCode::Char('o'));
    let (took, _) = landed(&taking, &sent(&commands)[0]);
    assert_ne!(notice(&took), Some(words.as_str()), "a take landed");

    // `⏎` in a merge stages nothing (ease-pass/05b): its own words
    // replace the notice at once.
    let (pressed, _) = press(&said, KeyCode::Enter);
    assert!(
        pressed
            .resolver
            .as_ref()
            .is_some_and(|open| open.staging.is_none()),
        "a merge stages nothing at ⏎"
    );
    assert_ne!(notice(&pressed), Some(words.as_str()), "⏎ answered");
}

/// Rule 3 on a file not yet done: not every file is done, so there is no
/// finish row, and the notice is what the row draws.
#[test]
fn no_open_clashes_left_shows_before_the_file_is_done() {
    let (after, _) = press(&nothing_open_on_three(&["three.txt"]), KeyCode::Char('n'));
    assert!(crate::view::resolver::finish_row(&after).is_none());
    let frame = super::frame_text(&after);
    assert!(frame.contains("no open clashes left"), "{frame}");
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

/// Every block of `three.txt` taken, the cursor on the last, and an info
/// notice standing, in a list of `paths` where only `three.txt` is unmerged.
fn all_taken_with_a_notice(paths: &[&str]) -> AppState {
    let mut file = three();
    for block in 0..3 {
        file.take(block, Take::Ours).expect("the block");
    }
    let mut state = on("three.txt", file, paths, &["three.txt"]);
    let open = state.resolver.as_mut().expect("open");
    open.cursor_block = Some(2);
    open.notice = Some(("today's notice".to_owned(), false));
    state
}

/// Find-and-pin 2 of m6-conflicts/61, pinned **as observed** (2026-09-25):
/// how long an info notice on the resolver's notice row lives today, after
/// `j`/`k`, a take landing and `⏎`.
///
/// Differs from the ticket's requirement ("after `j`/`k` … the words are
/// gone"): `j`/`k` with no row to step to (a one-file list, or the list's
/// edge) change nothing, so the notice stands (`files.rs:200-216
/// step_file`: no target, no `select_file`). With a row to step to, the
/// switch builds a fresh resolver and the notice is gone. `⏎` leaves it up
/// while its stage is in flight; the stage landing replaces it
/// (`resolved.rs:64-95 resolved_advance`: the next file, a reopen, or a
/// close — which branch this fixture takes is not asserted). A take landing
/// replaces it (`take.rs:212`).
#[test]
fn the_notice_lifetime_today() {
    let one = all_taken_with_a_notice(&["three.txt"]);
    for code in [KeyCode::Char('j'), KeyCode::Char('k')] {
        let (after, _) = press(&one, code);
        assert_eq!(open_path(&after), Path::new("three.txt"), "{code:?}");
        assert_eq!(
            notice(&after),
            Some("today's notice"),
            "{code:?} with nowhere to step: the notice stands"
        );
    }
    let three_rows = all_taken_with_a_notice(&["a.txt", "three.txt", "z.txt"]);
    for (code, lands_on) in [(KeyCode::Char('j'), "z.txt"), (KeyCode::Char('k'), "a.txt")] {
        let (after, _) = press(&three_rows, code);
        assert_eq!(open_path(&after), Path::new(lands_on), "{code:?}");
        assert_eq!(notice(&after), None, "{code:?} onto another row: gone");
    }

    // A take landing: the all-taken words replace it.
    let (after, commands) = press(&one, KeyCode::Char('o'));
    let (after, _) = landed(&after, &sent(&commands)[0]);
    assert_eq!(
        notice(&after),
        Some("all blocks taken · y reviews the merge")
    );

    // `⏎` in a merge stages nothing (ease-pass/05b): its own words replace
    // the notice at once.
    let (pressed, _) = press(&one, KeyCode::Enter);
    assert_eq!(
        notice(&pressed),
        Some("every clash is solved · y reviews the merge")
    );
}

/// `KNOWN_GAPS` :572: a whole-file card whose file could not be read lands
/// with the job's words as its notice, not as an empty card.
#[test]
fn an_unreadable_card_lands_its_words_as_the_notice() {
    let mut state = on("c", three(), &["a", "b", "c"], &["a", "c"]);
    state.resolver.as_mut().expect("open").cursor_block = Some(2);
    let (state, commands) = press(&state, KeyCode::Char('n'));
    let id = commands
        .iter()
        .find_map(|command| match command {
            Command::OpenConflict(id, _) => Some(*id),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no OpenConflict: {commands:?}"));
    let words = "can't read a: Permission denied (os error 13)";
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::ConflictOpened(Ok(Opening::Unmerged(Arc::new(OpenedFile {
                file: None,
                how: UnmergedHow::DeletedByThem,
                binary: false,
                on_disk: None,
                unreadable: Some(words.to_owned()),
                start_over: Some(crate::jobs::conflict::StartOver::Cannot {
                    why: "not asked".to_owned(),
                }),
            })))),
        ),
    );
    let open = state.resolver.as_ref().expect("open");
    assert_eq!(open.notice, Some((words.to_owned(), true)));
}
