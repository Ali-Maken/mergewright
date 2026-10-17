//! Picks at a cherry-pick's and a revert's stops, and a rebase stop's `O`
//! (review ease-pass-05 minors 5 and 6; ticket ease-pass/05r): `u` puts a
//! pick back byte for byte at any stop, `c` leaves the next stop with an
//! empty stack, and a file `O` marked done has no pick left to undo.

use std::path::Path;

use graft::jobs::Dispatch as _;
use graft::update::{AppEvent, AppState, Command, Product, Status};
use graft_core::event::{KeyCode, KeyPress};
use graft_fixtures::{Scratch, Shape};
use graft_git::state::InProgress;

use super::super::file_list::rebased_onto_main_by_graft;
use super::super::{git_stops, open, opened_on, resolver_on};
use super::lifetime::{landed_on, stages, three_step_feature};
use super::{TWO, bytes, commit_side, pressed};
use crate::app_harness::{App, git, press_for_commands};

/// `a.txt` and `b.txt`, two clashes each, committed on `base`.
const BOTH: &[(&str, &[usize])] = &[("a.txt", TWO), ("b.txt", TWO)];

fn head(repo: &Path) -> String {
    git(repo, &["rev-parse", "HEAD"]).trim().to_owned()
}

/// `feature` changes `a.txt`, then `b.txt`, each in its own commit; `main`
/// changes both. `git cherry-pick` of the two from `main` stops on `a.txt`,
/// then on `b.txt`.
fn cherry_picking_two() -> Scratch {
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    commit_side(repo, BOTH, "base");
    git(repo, &["checkout", "-q", "-b", "feature"]);
    commit_side(repo, &BOTH[..1], "feature");
    let first = head(repo);
    commit_side(repo, &BOTH[1..], "feature");
    let second = head(repo);
    git(repo, &["checkout", "-q", "main"]);
    commit_side(repo, BOTH, "main");
    git_stops(repo, &["cherry-pick", &first, &second]);
    scratch
}

/// `first` changes `a.txt`, then `b.txt`, each in its own commit, and
/// `second` changes both again: reverting the two `first` commits stops on
/// `a.txt`, then on `b.txt`.
fn reverting_two() -> Scratch {
    let scratch = graft_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    commit_side(repo, BOTH, "base");
    commit_side(repo, &BOTH[..1], "first");
    let first = head(repo);
    commit_side(repo, &BOTH[1..], "first");
    let second = head(repo);
    commit_side(repo, BOTH, "second");
    git_stops(repo, &["revert", "--no-edit", &first, &second]);
    scratch
}

/// At the stop on `a.txt`: `o` writes, `u` puts git's bytes back; then both
/// clashes taken, `⏎` stages, `c` goes on, and the stop on `b.txt` starts
/// with no picks.
fn picks_across_two_stops(scratch: &Scratch) {
    let repo = scratch.path();
    let (app, state) = opened_on(scratch);
    let state = resolver_on(&app, state, "a.txt");
    let git_wrote = bytes(repo, "a.txt");
    let state = pressed(&app, &state, 'o');
    assert_ne!(bytes(repo, "a.txt"), git_wrote, "the pick wrote");
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote, "u: git's bytes");
    assert_eq!(stages(repo, "a.txt"), 3, "still conflicted");

    // Theirs: ours on both would leave the commit empty, and git stops
    // there for that instead of going on.
    let state = pressed(&app, &pressed(&app, &state, 't'), 't');
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 2);
    let state = staged(&app, app.key(&state, KeyCode::Enter));
    assert_eq!(stages(repo, "a.txt"), 0, "staged");
    let state = continued(&app, &state, repo);
    assert_eq!(stages(repo, "b.txt"), 3, "git stopped on b.txt");
    assert!(state.picks.is_empty(), "the next stop starts clean");
    drop(state);
    app.close();
}

/// `c` with the stop's every file staged, carried to git's next stop and
/// the app's read of it. A cherry-pick's `c` hands the terminal to `git
/// cherry-pick --continue` — the loop's to carry out, never the pool's, so
/// it is carried out here as the loop does: git run, its exit fed back. A
/// revert begun in the shell is finished there, and `c` says so: it is
/// continued in the shell, and the app reads it when it gets focus back.
fn continued(app: &App, state: &AppState, repo: &Path) -> AppState {
    let (state, commands) = press_for_commands(state, KeyPress::plain(KeyCode::Char('c')));
    let (state, commands) = if let [Command::ContinueInGit { id, kind }] = commands.as_slice() {
        assert!(matches!(kind, InProgress::CherryPick), "{kind:?}");
        git_stops(
            repo,
            &["-c", "core.editor=true", "cherry-pick", "--continue"],
        );
        graft::update::update(
            &state,
            AppEvent::Produced(*id, Product::ContinuedInGit(Ok(Some(1)))),
        )
    } else {
        for command in commands {
            app.jobs.dispatch(command);
        }
        let state = app.pump(state, "c's answer", |state| state.operating.is_none());
        let Status::Sticky { reason, .. } = &state.status else {
            panic!("c at a revert begun in the shell: {:?}", state.status);
        };
        assert!(
            reason.starts_with("nothing Mergewright started is in progress"),
            "{reason}"
        );
        git_stops(repo, &["-c", "core.editor=true", "revert", "--continue"]);
        graft::update::update(
            &state,
            AppEvent::Core(graft_core::event::Event::FocusGained),
        )
    };
    for command in commands {
        app.jobs.dispatch(command);
    }
    app.pump(state, "git's next stop, read", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.working_tree.as_ref().is_some_and(|tree| {
                tree.entries
                    .iter()
                    .any(|entry| entry.unmerged && entry.path == Path::new("b.txt"))
            })
    })
}

/// `⏎`'s stage landed and the file is done.
fn staged(app: &App, state: AppState) -> AppState {
    app.pump(state, "the stage", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.done.is_some())
    })
}

#[test]
fn a_cherry_pick_stop_takes_and_undoes_and_its_next_stop_starts_clean() {
    picks_across_two_stops(&cherry_picking_two());
}

#[test]
fn a_revert_stop_takes_and_undoes_and_its_next_stop_starts_clean() {
    picks_across_two_stops(&reverting_two());
}

/// Minor 6: `O` at a rebase stop takes every clash and marks the file done
/// — its picks go with the stage, and `u` gets the done file's words.
#[test]
fn shift_o_at_a_rebase_stop_leaves_no_pick_to_undo() {
    let scratch = three_step_feature();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = rebased_onto_main_by_graft(&app, state, "a.txt");
    let state = landed_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 1);
    let state = staged(&app, app.key(&state, KeyCode::Char('O')));
    assert_eq!(stages(repo, "a.txt"), 0, "O staged it");
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 0);
    let marked = bytes(repo, "a.txt");

    let state = app.key(&state, KeyCode::Char('u'));
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("u on a done file: {:?}", state.status);
    };
    assert!(reason.starts_with("a.txt is done"), "{reason}");
    assert_eq!(bytes(repo, "a.txt"), marked, "nothing undone");
    assert_eq!(open(&state).path, Path::new("a.txt"));
    drop(state);
    app.close();
}
