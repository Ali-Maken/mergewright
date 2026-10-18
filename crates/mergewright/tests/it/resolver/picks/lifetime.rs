//! How long picks last, and where they never reach: a merge begun in the
//! shell journals none of them; closing the resolver keeps them and a
//! relaunch does not; a rebase's next stop and its abort clear them; a
//! stash put-back's clash undoes like any other; the graph's and the
//! banner's `u` still refuse mid-merge (§2.6a).

use std::path::Path;

use mergewright::update::{AppState, Command, Status};
use mergewright_core::event::{KeyCode, KeyPress};
use mergewright_fixtures::{Scratch, Shape};

use super::super::file_list::rebased_onto_main_by_graft;
use super::super::{journal_entries, on_the_banner, open, opened_on, resolver_on, settled};
use super::{TWO, bytes, clashing_merge, commit_side, cursor, doc, notice, pressed};
use crate::app_harness::{App, git, press_for_commands};

/// git's own word on `path`: its conflict stages, one line each.
pub(super) fn stages(repo: &Path, path: &str) -> usize {
    git(repo, &["ls-files", "-u", "--", path]).lines().count()
}

#[test]
fn a_pick_in_a_merge_begun_in_the_shell_is_a_file_write_and_no_entry() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let entries = journal_entries(repo).len();
    let state = resolver_on(&app, state, "a.txt");
    assert_eq!(stages(repo, "a.txt"), 3, "before the pick");
    let state = pressed(&app, &state, 'o');
    assert_eq!(stages(repo, "a.txt"), 3, "after the pick");
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote);
    assert_eq!(stages(repo, "a.txt"), 3, "after u");
    assert_eq!(journal_entries(repo).len(), entries, "no entry added");
    drop(state);
    app.close();
}

#[test]
fn picks_outlive_a_closed_resolver_but_not_a_relaunch() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none(), "esc closed it");
    // Esc left the pane's cursor on a.txt's row: `⏎` opens it again.
    let state = settled(&app, app.key(&state, KeyCode::Enter));
    assert_eq!(open(&state).path, Path::new("a.txt"));
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote, "undone after a close");
    assert_eq!((doc(&state).open(), cursor(&state)), (2, Some(0)));

    let state = pressed(&app, &state, 'o');
    let picked = bytes(repo, "a.txt");
    drop(state);
    app.close();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'u');
    assert_eq!(notice(&state), "nothing to undo here", "memory only");
    assert_eq!(bytes(repo, "a.txt"), picked);
    drop(state);
    app.close();
}

/// `feature`'s three commits, one file each, every one clashing with
/// `main`'s: Mergewright's rebase stops three times, on `a.txt` (two clashes),
/// `b.txt`, then `c.txt`.
pub(super) fn three_step_feature() -> Scratch {
    let scratch = mergewright_fixtures::copy_into_temp(Shape::Trivial);
    let repo = scratch.path();
    let all: &[(&str, &[usize])] = &[("a.txt", TWO), ("b.txt", TWO), ("c.txt", TWO)];
    commit_side(repo, all, "base");
    git(repo, &["checkout", "-q", "-b", "feature"]);
    for one in all {
        commit_side(repo, std::slice::from_ref(one), "feature");
    }
    git(repo, &["checkout", "-q", "main"]);
    commit_side(repo, all, "main");
    git(repo, &["checkout", "-q", "feature"]);
    scratch
}

/// The stop landed in the resolver on `path`, its read in.
pub(super) fn landed_on(app: &App, state: AppState, path: &str) -> AppState {
    app.pump(state, "the stop's resolver", |state| {
        state.operating.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new(path) && open.file.is_some() && open.job.is_none()
            })
    })
}

/// Esc to the banner, then its `key` and the box's second button.
fn from_the_banner(app: &App, state: &AppState, key: char) -> AppState {
    let state = app.key(state, KeyCode::Escape);
    let state = on_the_banner(app, state);
    let state = app.key(&state, KeyCode::Char(key));
    assert!(state.confirm.is_some(), "{key} asks first");
    let state = app.key(&state, KeyCode::Right);
    app.key(&state, KeyCode::Enter)
}

#[test]
fn a_rebase_stop_takes_and_undoes_and_its_next_stop_starts_clean() {
    let scratch = three_step_feature();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = rebased_onto_main_by_graft(&app, state, "a.txt");
    let state = landed_on(&app, state, "a.txt");
    let git_wrote = bytes(repo, "a.txt");

    let state = app.key(&state, KeyCode::Char('o'));
    assert_eq!(cursor(&state), Some(1), "advances within the file");
    let state = pressed(&app, &settled(&app, state), 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote);
    assert_eq!(cursor(&state), Some(0));

    let state = pressed(&app, &pressed(&app, &state, 'o'), 'o');
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.done.is_some())
    });
    assert_eq!(stages(repo, "a.txt"), 0, "staged");
    let state = app.key(&state, KeyCode::Char('u'));
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("u on a done file: {:?}", state.status);
    };
    assert!(reason.starts_with("a.txt is done"), "{reason}");

    let state = app.key(&state, KeyCode::Char('c'));
    assert!(state.operating.is_some(), "c ran: {:?}", state.status);
    let state = landed_on(&app, state, "b.txt");
    assert!(state.picks.is_empty(), "the next stop starts clean");

    // A pick left standing at a stop goes with it: `s` skips to step 3.
    let state = pressed(&app, &state, 'o');
    assert_eq!(state.picks.in_file(Path::new("b.txt")), 1);
    let state = from_the_banner(&app, &state, 's');
    let state = landed_on(&app, state, "c.txt");
    assert!(state.picks.is_empty(), "b.txt's pick went with its stop");

    // And with the operation: `a` aborts.
    let state = pressed(&app, &state, 'o');
    let state = from_the_banner(&app, &state, 'a');
    let state = app.pump(state, "the abort", |state| {
        state.operating.is_none() && state.in_progress.is_none()
    });
    assert!(state.picks.is_empty(), "the abort cleared them");
    drop(state);
    app.close();
}

#[test]
fn a_stash_clash_takes_and_undoes_then_closes_as_today() {
    use crate::stash_clash::{NOTES, a_stash_that_clashes, landed, overlay_open, put_back};
    let scratch = a_stash_that_clashes();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = landed(
        &app,
        put_back(&app, &overlay_open(&app, state), KeyCode::Char('p')),
    );
    let git_wrote = bytes(repo, NOTES);
    let state = pressed(&app, &state, 'o');
    assert_ne!(bytes(repo, NOTES), git_wrote);
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, NOTES), git_wrote);
    assert_eq!(doc(&state).open(), 1);
    // The stack empty, a stash clash's `u` keeps today's clause.
    let state = pressed(&app, &state, 'u');
    assert_eq!(notice(&state), "esc, u puts it all back");

    let state = pressed(&app, &state, 'o');
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the stage, the drop and their reads", |state| {
        state.resolver.is_none()
            && state.operating.is_none()
            && matches!(&state.status, Status::Done { what, .. } if what.contains("dropped"))
    });
    assert!(state.detail.shown, "closed to the changes pane");
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

#[test]
fn mid_merge_u_off_the_resolver_still_refuses_and_keeps_the_pick() {
    use crate::conflict_interim::{about_to_conflict, landed_in_the_resolver};
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = landed_in_the_resolver(&app, state);
    let path = open(&state).path.clone();
    let name = path.to_str().expect("a UTF-8 path");
    let git_wrote = bytes(repo, name);
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, name), git_wrote, "a merge begun in Mergewright too");

    let state = pressed(&app, &state, 'o');
    let picked = bytes(repo, name);
    let banner = app.key(&state, KeyCode::Escape);
    assert!(banner.banner_focus, "esc gives the banner the keyboard");
    let graph = app.key(&banner, KeyCode::Escape);
    assert!(!graph.banner_focus);
    for (place, state) in [("banner", &banner), ("graph", &graph)] {
        let (after, commands) = press_for_commands(state, KeyPress::plain(KeyCode::Char('u')));
        assert!(
            !commands
                .iter()
                .any(|command| matches!(command, Command::Undo(..))),
            "{place}: {commands:?}"
        );
        let Status::Sticky { reason, .. } = &after.status else {
            panic!("{place}: {:?}", after.status);
        };
        assert_eq!(reason, "finish or abort the merge first", "{place}");
        assert_eq!(after.picks.in_file(&path), 1, "{place}: the pick stays");
    }
    assert_eq!(bytes(repo, name), picked, "nothing undone");
    drop(graph);
    app.close();
}
