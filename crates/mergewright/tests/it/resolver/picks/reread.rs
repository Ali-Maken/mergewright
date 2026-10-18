//! A fresh read of a file with picks (ticket ease-pass/05r): its taken
//! clashes stay while the bytes are the ones its newest pick wrote (review
//! ease-pass-05 M1 part 2, minor 8); other bytes drop its picks, and the
//! status line says so (M2). And `u` on a file the shell staged meanwhile
//! drops, never sticks (M3).

use std::path::Path;

use mergewright::jobs::Dispatch as _;
use mergewright::update::resolver::{RowState, listed_files};
use mergewright::update::{AppState, Status};
use mergewright_core::event::KeyCode;
use mergewright_git::conflict::{BlockState, Take};

use super::super::{opened_on, resolver_on};
use super::{bytes, clashing_merge, cursor, doc, on, pressed};
use crate::app_harness::{App, git};

/// The list's row state for `path`.
fn row(state: &AppState, path: &str) -> RowState {
    listed_files(state)
        .into_iter()
        .find(|file| file.path == Path::new(path))
        .map(|file| file.state)
        .expect("listed")
}

fn sticky(state: &AppState) -> &str {
    match &state.status {
        Status::Sticky { reason, .. } => reason,
        other => panic!("expected a sticky line, got {other:?}"),
    }
}

/// What someone else writes to `name`: a line added at the top, its
/// markers kept — still a conflict document, other bytes.
fn edited_outside(repo: &Path, name: &str) {
    let mut text = b"added outside\n".to_vec();
    text.extend(bytes(repo, name));
    std::fs::write(repo.join(name), text).expect("writable scratch");
}

#[test]
fn taken_clashes_survive_j_and_k() {
    let scratch = clashing_merge();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let listed = row(&state, "a.txt");
    let state = pressed(&app, &state, 'j');
    assert_eq!(on(&state), Path::new("b.txt"));
    let state = pressed(&app, &state, 'k');
    assert_eq!(on(&state), Path::new("a.txt"));

    let blocks: Vec<BlockState> = doc(&state).blocks().iter().map(|b| b.state).collect();
    assert_eq!(
        blocks,
        [BlockState::Took(Take::Ours), BlockState::Open],
        "clash 1 still taken, clash 2 still open"
    );
    assert_eq!(cursor(&state), Some(1), "on its first open clash");
    assert_eq!(row(&state, "a.txt"), listed, "the list's row unchanged");
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 1, "the pick lives");
    let state = app.key(&state, KeyCode::Left);
    assert_eq!(cursor(&state), Some(0), "`←` reaches the taken clash");
    let state = app.key(&state, KeyCode::Right);
    assert_eq!(cursor(&state), Some(1));
    drop(state);
    app.close();
}

/// Review minor 8: a pick before a re-read and one after it, both undone,
/// leave the bytes git wrote.
#[test]
fn two_picks_across_a_reread_undo_to_the_bytes_git_wrote() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let git_wrote = bytes(repo, "a.txt");
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &pressed(&app, &state, 'j'), 'k');
    let state = pressed(&app, &state, 'o');
    assert_eq!(doc(&state).open(), 0, "both clashes taken");
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 2);
    let state = pressed(&app, &pressed(&app, &state, 'u'), 'u');
    assert_eq!(bytes(repo, "a.txt"), git_wrote);
    assert_eq!(doc(&state).open(), 2);
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

/// Picks in `b.txt` then `a.txt`, the resolver back on `b.txt`, and
/// `a.txt` written outside: returns the state and `b.txt`'s bytes before
/// its pick.
fn picked_both_then_a_edited(app: &App, state: AppState, repo: &Path) -> (AppState, Vec<u8>) {
    let state = resolver_on(app, state, "b.txt");
    let b_before = bytes(repo, "b.txt");
    let state = pressed(app, &state, 'o');
    let state = pressed(app, &state, 'k');
    assert_eq!(on(&state), Path::new("a.txt"));
    let state = pressed(app, &state, 'o');
    let state = pressed(app, &state, 'j');
    assert_eq!(on(&state), Path::new("b.txt"));
    edited_outside(repo, "a.txt");
    (state, b_before)
}

/// Review M2 through `k`: the drop is said, with the file `u` reaches
/// next, and that `u` undoes it.
#[test]
fn a_file_changed_since_its_picks_says_so_when_read_again() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let (state, b_before) = picked_both_then_a_edited(&app, state, repo);
    let edited = bytes(repo, "a.txt");
    let state = pressed(&app, &state, 'k');
    assert_eq!(on(&state), Path::new("a.txt"));
    assert_eq!(
        sticky(&state),
        "a.txt changed since your picks \u{b7} u can't undo them now \u{b7} next u: b.txt"
    );
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 0, "dropped");
    assert_eq!(doc(&state).open(), 1, "the read as it is: one clash open");

    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "b.txt"), b_before, "b.txt's pick undone");
    assert_eq!(bytes(repo, "a.txt"), edited, "the edit survives");
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

/// Review M2 through `e`: `$EDITOR` changed the file, the hand-back's
/// re-read says so — the short form, with no other pick left.
#[test]
fn a_file_changed_in_the_editor_says_its_picks_went() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    edited_outside(repo, "a.txt");
    let state = handed_back(&app, &state);
    assert_eq!(
        sticky(&state),
        "a.txt changed since your picks \u{b7} u can't undo them now"
    );
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}

/// The same through `e` with another file's pick below: the long form.
#[test]
fn a_file_changed_in_the_editor_names_the_next_u() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "b.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'k');
    let state = pressed(&app, &state, 'o');
    assert_eq!(on(&state), Path::new("a.txt"));
    edited_outside(repo, "a.txt");
    let state = handed_back(&app, &state);
    assert_eq!(
        sticky(&state),
        "a.txt changed since your picks \u{b7} u can't undo them now \u{b7} next u: b.txt"
    );
    drop(state);
    app.close();
}

/// `d`'s start over rewrites the file on purpose: its picks go with it,
/// silently — nothing "changed since your picks" that he did not ask for.
#[test]
fn start_over_drops_the_files_picks_without_words() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "a.txt");
    let state = pressed(&app, &state, 'o');
    let picked = bytes(repo, "a.txt");
    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_some(), "`d` asks first");
    let state = app.key(&app.key(&state, KeyCode::Right), KeyCode::Enter);
    let state = app.pump(state, "the restart and its read", |state| {
        state.operating.is_none()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.job.is_none() && open.file.is_some())
    });
    // `checkout --merge` labels its markers `ours`/`theirs`, not git's
    // merge labels: the same clashes, other bytes.
    assert_ne!(bytes(repo, "a.txt"), picked, "started over");
    assert_eq!(doc(&state).open(), 2, "both clashes open again");
    assert!(state.picks.is_empty());
    assert!(
        !format!("{:?}", state.status).contains("changed since your picks"),
        "{:?}",
        state.status
    );
    drop(state);
    app.close();
}

/// `e`'s answer fed back the way the loop feeds it (the editor itself not
/// run: the edit is the caller's), and the re-read it queues landed.
fn handed_back(app: &App, state: &AppState) -> AppState {
    let (state, commands) = mergewright::update::update(
        state,
        mergewright::update::AppEvent::Produced(
            mergewright_core::job::JobId::new(9_310),
            mergewright::update::Product::HandedOver(Ok(())),
        ),
    );
    for command in commands {
        app.jobs.dispatch(command);
    }
    app.pump(state, "the re-read", |state| {
        state
            .resolver
            .as_ref()
            .is_none_or(|open| open.job.is_none())
    })
}

/// Review M3: `u` on a pick whose file the shell staged meanwhile writes
/// nothing, drops that file's picks and says why; the next `u` reaches the
/// pick before it.
#[test]
fn u_on_a_file_the_shell_staged_drops_its_picks() {
    let scratch = clashing_merge();
    let repo = scratch.path();
    let (app, state) = opened_on(&scratch);
    let state = resolver_on(&app, state, "b.txt");
    let b_before = bytes(repo, "b.txt");
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'k');
    let state = pressed(&app, &state, 'o');
    let state = pressed(&app, &state, 'j');
    assert_eq!(on(&state), Path::new("b.txt"));
    git(repo, &["add", "a.txt"]);
    let a_staged = bytes(repo, "a.txt");

    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "a.txt"), a_staged, "nothing written");
    assert_eq!(git(repo, &["ls-files", "-u", "--", "a.txt"]), "");
    assert_eq!(
        sticky(&state),
        "a.txt is not conflicted any more \u{b7} nothing undone"
    );
    assert_eq!(state.picks.in_file(Path::new("a.txt")), 0, "dropped");

    let state = pressed(&app, &state, 'u');
    assert_eq!(bytes(repo, "b.txt"), b_before, "b.txt's pick undone");
    assert!(state.picks.is_empty());
    drop(state);
    app.close();
}
