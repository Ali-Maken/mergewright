//! A merge stages nothing while the user picks; `y` reviews it
//! (ease-pass/05b). Over hand-built states: the merge from
//! `advance::on`, the same states turned into a rebase where `y` is not
//! the review's.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use mergewright_core::event::KeyCode;
use mergewright_core::job::JobId;
use mergewright_git::conflict::{ConflictFile, Take};
use mergewright_git::state::InProgress;

use mergewright_ops::finish::{FinishSummary, ResultIs};

use super::advance::{notice, on, open_path, three};
use super::press;
use crate::update::detail::Loading;
use crate::update::finish::{ReturnTo, ReviewFailed, Reviewed};
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

/// `three()` with its first two blocks taken: one clash open.
fn one_open() -> ConflictFile {
    let mut file = three();
    file.take(0, Take::Ours).expect("block 0");
    file.take(1, Take::Ours).expect("block 1");
    file
}

/// What the review's read finds: `a` solved, staged at the review's `y`.
fn reviewed() -> Reviewed {
    Reviewed {
        summary: Arc::new(FinishSummary {
            into: b"main".to_vec(),
            from: b"feature".to_vec(),
            by_you: vec![mergewright_ops::finish::ByYou {
                path: PathBuf::from("a"),
                result: ResultIs::Theirs,
            }],
            by_git: Vec::new(),
            message: b"Merge branch 'feature'\n".to_vec(),
            recorded: true,
        }),
        solved: vec![PathBuf::from("a")],
        incoming: None,
    }
}

/// The review's read, out under `commands`, answered `answer`.
fn read_lands(
    state: &AppState,
    commands: &[Command],
    answer: Result<Reviewed, ReviewFailed>,
) -> AppState {
    let id = reads_the_review(commands).expect("the review is read");
    update(
        state,
        AppEvent::Produced(id, Product::FinishSummary(answer)),
    )
    .0
}

pub(super) fn as_rebase(mut state: AppState) -> AppState {
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
    let state = read_lands(&state, &commands, Ok(reviewed()));
    let finish = state.finish.as_ref().expect("the review is open");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert!(state.resolver.is_some(), "the resolver stays under it");
}

#[test]
fn c_in_a_merge_is_the_review_s_door_too() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('c'));
    nothing_staged(&commands);
    assert!(
        state.finish.is_none(),
        "nothing drawn before the read lands"
    );
    let state = read_lands(&state, &commands, Ok(reviewed()));
    assert!(state.finish.is_some());
}

/// The door (ease-pass/05s): a clash open in a file the resolver read, and
/// no file it has not — the count is whole, so `y` refuses at once.
#[test]
fn y_with_a_clash_open_and_every_file_known_refuses_at_once() {
    let state = on("a", one_open(), &["a"], &["a"]);
    let (after, commands) = press(&state, KeyCode::Char('y'));
    assert_eq!(reads_the_review(&commands), None, "{commands:?}");
    assert!(after.finish.is_none());
    assert_eq!(
        notice(&after),
        Some("1 clash still open · n goes to the next")
    );
}

/// A file the resolver has not read: only the read can count it, so `y`
/// asks it — once — and nothing is drawn until it answers.
#[test]
fn y_with_an_unread_file_asks_the_read_and_draws_no_review_until_it_lands() {
    let state = on("a", one_open(), &["a", "b"], &["a", "b"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let reads = |commands: &[Command]| {
        commands
            .iter()
            .filter(|command| matches!(command, Command::ReadFinishSummary(_)))
            .count()
    };
    assert_eq!(reads(&commands), 1, "{commands:?}");
    assert!(state.finish.is_none(), "no review while the read runs");
    assert!(crate::view::finish::finish_view(&state).is_none());
    for key in ['y', 'c'] {
        let (again, more) = press(&state, KeyCode::Char(key));
        assert_eq!(reads(&more), 0, "`{key}` again: one read in flight");
        assert!(again.finish.is_none());
    }
    let state = read_lands(
        &state,
        &commands,
        Err(ReviewFailed::StillOpen {
            files: 1,
            clashes: 2,
        }),
    );
    assert!(state.finish.is_none(), "the review is never drawn");
    assert_eq!(
        notice(&state),
        Some("2 clashes still open · n goes to the next")
    );
    let (_, after) = press(&state, KeyCode::Char('y'));
    assert_eq!(reads(&after), 1, "the answer cleared the marker");
}

#[test]
fn the_review_opens_when_its_read_lands_ready() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    assert!(state.finish.is_none());
    let state = read_lands(&state, &commands, Ok(reviewed()));
    let finish = state.finish.as_ref().expect("the review is open");
    assert_eq!(finish.return_to, ReturnTo::Resolver);
    assert!(matches!(finish.summary, Loading::Ready(_)));
    assert_eq!(finish.solved, vec![PathBuf::from("a")]);
}

#[test]
fn a_failed_review_read_opens_the_review_showing_it() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let state = read_lands(
        &state,
        &commands,
        Err(ReviewFailed::Failed("git would not run".to_owned())),
    );
    let finish = state.finish.as_ref().expect("the review shows the failure");
    assert_eq!(
        finish.summary,
        Loading::Failed("git would not run".to_owned())
    );
    assert_eq!(finish.return_to, ReturnTo::Resolver);
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

/// The review read found a file the resolver had not read still open: no
/// review, and the resolver says how many clashes — the unit `y` says.
#[test]
fn a_review_read_that_finds_a_file_open_closes_back_to_the_resolver() {
    let state = on("a", solved(), &["a", "b"], &["a", "b"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let state = read_lands(
        &state,
        &commands,
        Err(ReviewFailed::StillOpen {
            files: 1,
            clashes: 1,
        }),
    );
    assert!(state.finish.is_none());
    assert_eq!(
        notice(&state),
        Some("1 clash still open · n goes to the next")
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
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let state = read_lands(&state, &commands, Ok(reviewed()));
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

/// `⌃Z`, `app.undo`'s other chord, is the same `u` (the review has no
/// text field since ease-pass/12).
#[test]
fn ctrl_z_in_the_review_undoes_the_last_pick() {
    let ctrl_z = AppEvent::Core(mergewright_core::event::Event::Key(
        mergewright_core::event::KeyPress::control(KeyCode::Char('z')),
    ));
    let state = review_over_the_resolver();
    let (state, commands) = update(&state, ctrl_z);
    assert!(state.finish.is_none(), "the review closed");
    assert!(untake(&commands).is_some(), "{commands:?}");
}

#[test]
fn u_in_the_review_with_nothing_to_undo_says_so_and_stays() {
    let state = on("a", solved(), &["a"], &["a"]);
    let (state, commands) = press(&state, KeyCode::Char('y'));
    let state = read_lands(&state, &commands, Ok(reviewed()));
    let (state, commands) = press(&state, KeyCode::Char('u'));
    let finish = state.finish.as_ref().expect("the review stays");
    assert_eq!(
        finish.notice.as_ref().map(|(words, _)| words.as_str()),
        Some("nothing to undo here")
    );
    assert_eq!(untake(&commands), None);
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
    let (state, commands) = press(&state, KeyCode::Char('c'));
    let state = read_lands(&state, &commands, Ok(reviewed()));
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

mod click;
mod stages_nothing;

/// Abort from the review (pre-publish-cleanup/15): the review has no `a` of
/// its own and `?` lists no banner keys, so the visible route is the one the
/// bars teach: `Esc` back to the clashes, `Esc` to the banner, `a` opens the
/// abort box.
#[test]
fn abort_is_reachable_from_the_review_by_esc_esc_a() {
    let mut state = review_over_the_resolver();
    assert!(state.in_progress.is_some(), "the merge is live");
    let mut presses = 0;
    while !state.banner_focus {
        assert!(presses < 4, "no banner after {presses} presses");
        let code = if presses < 2 {
            KeyCode::Escape
        } else {
            KeyCode::Tab
        };
        state = press(&state, code).0;
        presses += 1;
    }
    assert!(state.finish.is_none() && state.resolver.is_none());
    let (asked, _) = press(&state, KeyCode::Char('a'));
    assert!(
        asked.confirm.is_some(),
        "`a` on the banner opens the abort box"
    );
}
