//! A merge stages nothing while the user picks; `y` reviews it
//! (ease-pass/05b). Over hand-built states: the merge from
//! `advance::on`, the same states turned into a rebase where `y` is not
//! the review's.

use std::path::Path;
use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_git::conflict::{ConflictFile, Take};
use graft_git::state::InProgress;

use super::advance::{cursor, landed, notice, on, open_path, sent, three};
use super::{press, sticky};
use crate::update::finish::{FinishFocus, ReturnTo, ReviewFailed};
use crate::update::resolver::Pick;
use crate::update::{AppEvent, AppState, Command, Product, update};

/// `three()` with every block taken: solved, still unmerged.
fn solved() -> ConflictFile {
    let mut file = three();
    file.take_all(Take::Theirs);
    file
}

/// Nothing that stages, resumes or writes the index left the keys.
fn nothing_staged(commands: &[Command]) {
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::Operate(..) | Command::Resume(..))),
        "{commands:?}"
    );
}

fn reads_the_review(commands: &[Command]) -> Option<JobId> {
    commands.iter().find_map(|command| match command {
        Command::ReadFinishSummary(id) => Some(*id),
        _ => None,
    })
}

fn as_rebase(mut state: AppState) -> AppState {
    if let Some(detail) = state.in_progress.as_mut() {
        detail.kind = InProgress::Rebase;
    }
    state
}

#[test]
fn y_in_a_merge_with_nothing_open_opens_the_review_and_stages_nothing() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    nothing_staged(&commands);
    let finish = state.finish.as_ref().expect("the review is open");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert_eq!(finish.focus, FinishFocus::Files);
    assert_eq!(reads_the_review(&commands), finish.job);
    assert!(state.resolver.is_some(), "the resolver stays under it");
}

#[test]
fn c_in_a_merge_is_the_review_s_door_too() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('c'));
    nothing_staged(&commands);
    assert!(state.finish.is_some());
    assert!(reads_the_review(&commands).is_some());
}

#[test]
fn y_with_clashes_still_open_says_how_many_and_where_next() {
    let state = on("a", three(), &["a"], &["a"]);
    for key in ['y', 'c'] {
        let (after, commands) = press(&state, KeyCode::Char(key));
        assert!(after.finish.is_none(), "{key}");
        assert_eq!(reads_the_review(&commands), None, "{key}");
        assert_eq!(
            notice(&after),
            Some("3 clashes still open · n goes to the next"),
            "{key}"
        );
    }
}

/// The review read found a file the resolver had not read still open: it
/// closes, and the resolver says how many files.
#[test]
fn a_review_read_that_finds_a_file_open_closes_back_to_the_resolver() {
    let state = on("a", solved(), &["a", "b"], &["a", "b"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let id = reads_the_review(&commands).expect("b is unread: the read judges it");
    let (state, _) = update(
        &state,
        AppEvent::Produced(
            id,
            Product::FinishSummary(Err(ReviewFailed::StillOpen { count: 1 })),
        ),
    );
    assert!(state.finish.is_none());
    assert_eq!(
        notice(&state),
        Some("1 file still open · n goes to the next")
    );
}

#[test]
fn y_outside_a_merge_answers_as_an_unbound_key() {
    let state = as_rebase(on("a", solved(), &["a"], &["a"]));
    let (after, commands) = press(&state, KeyCode::Char('y'));
    assert_eq!(reads_the_review(&commands), None, "{commands:?}");
    nothing_staged(&commands);
    assert_eq!(after.finish, None);
    assert_eq!(after.resolver, state.resolver, "nothing moved");
}

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

// ------------------------------------------------------------- review `u`

/// A pick in `a` on the stack: block 0 taken from `three()`.
fn with_a_pick(mut state: AppState) -> AppState {
    let was = three();
    let mut after = three();
    after.take(0, Take::Ours).expect("block 0");
    let pick = Pick::between(Path::new("a"), &was, Arc::new(after)).expect("it changed");
    state.picks.push(pick);
    state
}

fn review_over_the_resolver() -> AppState {
    let state = with_a_pick(on("a", solved(), &["a"], &["a"]));
    let (state, _) = press(&state, KeyCode::Char('y'));
    assert!(state.finish.is_some());
    state
}

fn untake(commands: &[Command]) -> Option<JobId> {
    commands.iter().find_map(|command| match command {
        Command::UntakeSide(id, _) => Some(*id),
        _ => None,
    })
}

/// `u` in the review: back to the resolver, the last pick put back.
#[test]
fn u_in_the_review_goes_back_to_the_resolver_and_undoes_the_last_pick() {
    let (state, commands) = press(&review_over_the_resolver(), KeyCode::Char('u'));
    assert!(state.finish.is_none(), "the review closed");
    assert!(state.resolver.is_some(), "on the resolver");
    assert!(untake(&commands).is_some(), "{commands:?}");
    nothing_staged(&commands);
}

/// `⌃Z`, `app.undo`'s other chord, is the same `u` — from the subject
/// too, where it is never text.
#[test]
fn ctrl_z_in_the_review_undoes_the_last_pick_from_any_focus() {
    let ctrl_z = AppEvent::Core(graft_core::event::Event::Key(
        graft_core::event::KeyPress::control(KeyCode::Char('z')),
    ));
    for focus in [FinishFocus::Files, FinishFocus::Subject] {
        let mut state = review_over_the_resolver();
        if let Some(finish) = state.finish.as_mut() {
            finish.focus = focus;
        }
        let (state, commands) = update(&state, ctrl_z.clone());
        assert!(state.finish.is_none(), "{focus:?}: the review closed");
        assert!(untake(&commands).is_some(), "{focus:?}: {commands:?}");
    }
}

#[test]
fn u_in_the_review_with_nothing_to_undo_says_so_and_stays() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, _) = press(&state, KeyCode::Char('y'));
    let (state, commands) = press(&state, KeyCode::Char('u'));
    let finish = state.finish.as_ref().expect("the review stays");
    assert_eq!(
        finish.notice.as_ref().map(|(words, _)| words.as_str()),
        Some("nothing to undo here")
    );
    assert_eq!(untake(&commands), None);
}

/// `u` typed in the subject is text, never an undo.
#[test]
fn u_in_the_subject_is_typed_text() {
    let mut state = review_over_the_resolver();
    if let Some(finish) = state.finish.as_mut() {
        finish.focus = FinishFocus::Subject;
    }
    let (state, commands) = press(&state, KeyCode::Char('u'));
    assert!(state.finish.is_some());
    assert_eq!(untake(&commands), None);
    assert!(state.picks.in_file(Path::new("a")) == 1, "the pick is kept");
}

/// `esc` from the review: the resolver, unchanged, picks kept.
#[test]
fn esc_from_the_review_is_the_resolver_unchanged() {
    let before = review_over_the_resolver();
    let (state, commands) = press(&before, KeyCode::Escape);
    assert!(state.finish.is_none());
    assert_eq!(state.resolver, before.resolver);
    assert_eq!(state.picks, before.picks);
    nothing_staged(&commands);
}

/// The review asked for from the banner, with no resolver under it: `u`
/// sends the untake, and its landing opens the resolver on the pick.
#[test]
fn u_in_a_review_from_the_banner_opens_the_resolver_on_the_pick() {
    let mut state = with_a_pick(on("a", solved(), &["a"], &["a"]));
    state.resolver = None;
    state.banner_focus = true;
    let (state, _) = press(&state, KeyCode::Char('c'));
    assert_eq!(
        state.finish.as_ref().map(|finish| finish.return_to),
        Some(ReturnTo::Banner)
    );
    let (state, commands) = press(&state, KeyCode::Char('u'));
    assert!(state.finish.is_none());
    let id = untake(&commands).expect("the untake is sent");
    let (state, _) = update(
        &state,
        AppEvent::Produced(id, Product::Untaken(Ok(Arc::new(three())))),
    );
    assert_eq!(
        open_path(&state),
        Path::new("a"),
        "the resolver, on the pick"
    );
}
