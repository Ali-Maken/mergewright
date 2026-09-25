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
/// keeps its own hints.
#[test]
fn n_past_the_last_block_says_no_more_conflicts_this_way() {
    use crate::update::resolver::walk::NO_MORE_THIS_WAY;

    let state = nothing_open_on_three(&["three.txt"]);
    let status_before = state.status.clone();
    let (after, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(cursor(&after), Some(2), "unmoved");
    assert_eq!(
        after.resolver.as_ref().expect("open").notice,
        Some((NO_MORE_THIS_WAY.to_owned(), false)),
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
    assert_eq!(notice(&back), Some(NO_MORE_THIS_WAY));
    assert_eq!(back.status, status_before);
}

/// The words are said once, on the notice row: the bar under the document
/// does not repeat them, and it still offers `⏎ done`.
#[test]
fn no_more_conflicts_is_not_repeated_in_the_bar() {
    let (after, _) = press(&nothing_open_on_three(&["three.txt"]), KeyCode::Char('n'));
    let frame = super::frame_text(&after);
    assert_eq!(
        frame.matches("no more conflicts").count(),
        1,
        "once, on the notice row:\n{frame}"
    );
    let bar = bottom_row(&after);
    assert!(!bar.contains("no more conflicts"), "{bar}");
    assert!(bar.contains("\u{23ce} done"), "{bar}");
}

/// The rule-3 notice keeps today's lifetime (Decisions, conductor
/// 2026-09-25): gone after `j` onto another row, replaced by a take landing,
/// replaced once `⏎`'s stage lands. `j`/`k` that go nowhere leave it up
/// (`the_notice_lifetime_today`).
#[test]
fn the_no_more_notice_is_gone_after_the_next_move() {
    use crate::update::resolver::walk::NO_MORE_THIS_WAY;

    let (said, _) = press(
        &nothing_open_on_three(&["a.txt", "three.txt"]),
        KeyCode::Char('n'),
    );
    assert_eq!(notice(&said), Some(NO_MORE_THIS_WAY));

    let (moved, _) = press(&said, KeyCode::Char('k'));
    assert_eq!(open_path(&moved), Path::new("a.txt"));
    assert_eq!(notice(&moved), None, "`k` onto another row");

    let (taking, commands) = press(&said, KeyCode::Char('o'));
    let (took, _) = landed(&taking, &sent(&commands)[0]);
    assert_ne!(notice(&took), Some(NO_MORE_THIS_WAY), "a take landed");

    let (pressed, _) = press(&said, KeyCode::Enter);
    let id = pressed
        .resolver
        .as_ref()
        .and_then(|open| open.staging)
        .expect("`⏎` launched a stage");
    let (staged, _) = update(
        &pressed,
        AppEvent::Produced(id, super::words_and_c::staged("three.txt")),
    );
    assert_ne!(notice(&staged), Some(NO_MORE_THIS_WAY), "the stage landed");
}

/// Rule 3 on a file not yet done: not every file is done, so there is no
/// finish row, and the notice is what the row draws.
#[test]
fn no_more_this_way_shows_before_the_file_is_done() {
    let (after, _) = press(&nothing_open_on_three(&["three.txt"]), KeyCode::Char('n'));
    assert!(crate::view::resolver::finish_row(&after).is_none());
    let frame = super::frame_text(&after);
    assert!(frame.contains("no more conflicts this way"), "{frame}");
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
        Some("all blocks taken · ⏎ marks three.txt done")
    );

    // `⏎`: up while the stage is in flight, gone once it lands.
    let (pressed, _) = press(&one, KeyCode::Enter);
    assert_eq!(notice(&pressed), Some("today's notice"), "stage in flight");
    let id = pressed
        .resolver
        .as_ref()
        .and_then(|open| open.staging)
        .expect("`⏎` launched a stage");
    let (landed_state, _) = update(
        &pressed,
        AppEvent::Produced(id, super::words_and_c::staged("three.txt")),
    );
    assert_ne!(
        notice(&landed_state),
        Some("today's notice"),
        "the stage landed"
    );
}
