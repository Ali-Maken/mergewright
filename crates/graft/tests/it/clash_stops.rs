//! Every clash stop lands on the clash (ease-pass/05b Q17 a, built in
//! 05c), and `esc` pauses (Q7 a): the graph with the banner focused, its
//! bar leading `⏎ back to the clashes`, and `⏎` back to the first open
//! clash with the picks kept. A relaunch reads files solved on disk as
//! solved. Over real repositories, through the app.

#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::{Path, PathBuf};

use graft::update::AppState;
use graft::update::resolver::{OpenedFrom, RowState, listed_files};
use graft_core::event::KeyCode;
use graft_git::state::InProgress;

use crate::app_harness::{App, git};
use crate::conflict_interim::about_to_conflict;

/// Opened, with the first rows, the refs and the status on hand.
fn opened(app: &App, state: AppState) -> AppState {
    app.pump(state, "the first rows and the refs", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state.system_git.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// Is the cursor's row carrying branch `name`'s badge?
pub(crate) fn on_branch(state: &AppState, name: &[u8]) -> bool {
    let (Some(refs), Some(row)) = (
        state.refs.as_deref(),
        state
            .history
            .as_ref()
            .and_then(graft::history::History::selected),
    ) else {
        return false;
    };
    refs.on(row.commit.id)
        .iter()
        .any(|label| matches!(label, graft::refs::RefLabel::Branch { name: n, .. } if n == name))
}

/// The cursor onto `branch`'s row, `key`, the preview's plan, `y`: the
/// operation runs, stops, and the stop lands where it lands.
pub(crate) fn stop_by(
    app: &App,
    state: AppState,
    branch: &[u8],
    key: char,
    kind: InProgress,
) -> AppState {
    let mut state = opened(app, state);
    for _ in 0..20 {
        if on_branch(&state, branch) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert!(on_branch(&state, branch), "never found the branch's row");
    let state = app.key(&state, KeyCode::Char(key));
    assert!(state.preview.is_some(), "`{key}` opens a preview");
    let state = app.pump(state, "the preview's plan", |state| {
        state.preview.as_ref().is_some_and(|preview| {
            !matches!(preview.plan, graft::update::detail::Loading::Pending(_))
        })
    });
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the operation", |state| state.operating.is_none());
    app.pump(state, "the stop's first file", |state| {
        state
            .in_progress
            .as_ref()
            .is_some_and(|detail| detail.kind == kind)
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.file.is_some() || open.whole_file.is_some())
            && !graft::update::resolver::listed_files(state).is_empty()
            && state
                .resolver
                .as_ref()
                .is_some_and(|open| open.set_job.is_none())
    })
}

/// The resolver is open on git's first conflicted path, from the banner,
/// cursor on its first clash: 0 keys after the stop.
pub(crate) fn on_the_first_clash(state: &AppState) {
    let first = graft::update::banner::first_conflicted(state).expect("something unmerged");
    let open = state
        .resolver
        .as_ref()
        .expect("the stop lands on the resolver");
    assert_eq!(open.path, first);
    assert_eq!(open.opened_from, OpenedFrom::Banner);
    assert_eq!(open.cursor_block, Some(0), "the cursor on the first clash");
}

/// The footer as drawn, 200 columns wide.
fn bottom(state: &AppState) -> String {
    crate::captures::frame(state, 200, 40)
        .last()
        .cloned()
        .unwrap_or_default()
}

fn feature_checked_out() -> graft_fixtures::Scratch {
    let scratch = about_to_conflict();
    git(scratch.path(), &["switch", "--quiet", "feature"]);
    scratch
}

pub(crate) fn read(repo: &Path, name: &str) -> String {
    std::fs::read_to_string(repo.join(name)).expect("the file reads")
}

/// Q17 a: a Graft-begun rebase that stops on a clash lands on it.
#[test]
fn a_rebase_clash_stop_lands_on_the_first_clash() {
    let scratch = feature_checked_out();
    let (app, state) = App::open(scratch.path());
    let state = stop_by(&app, state, b"main", 'r', InProgress::Rebase);
    on_the_first_clash(&state);
    drop(state);
    app.close();
}

/// Q17 a: the same for a cherry-pick (`o` on the commit).
#[test]
fn a_cherry_pick_clash_stop_lands_on_the_first_clash() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stop_by(&app, state, b"feature", 'o', InProgress::CherryPick);
    on_the_first_clash(&state);
    drop(state);
    app.close();
}

/// Q7 a on a rebase: `esc` pauses on the banner, its bar leading `⏎ back
/// to the clashes`; `⏎` goes back to the first open clash.
#[test]
fn esc_pauses_a_rebase_on_the_banner_and_enter_goes_back() {
    let scratch = feature_checked_out();
    let (app, state) = App::open(scratch.path());
    let state = stop_by(&app, state, b"main", 'r', InProgress::Rebase);
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none());
    assert!(state.banner_focus, "the banner has the keyboard");
    let bar = bottom(&state);
    assert!(bar.contains("⏎ back to the clashes"), "{bar}");
    assert!(bar.contains("c continue"), "a rebase's `c`: {bar}");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the first file again", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some() && open.set_job.is_none())
    });
    on_the_first_clash(&state);
    drop(state);
    app.close();
}

/// Q7 a on a merge, with a pick: `esc ⏎` lands on the first **open**
/// clash — past the file the pick solved — and the pick is still taken on
/// disk, and `u` still undoes it.
#[test]
fn esc_enter_in_a_merge_skips_the_solved_file_and_keeps_its_pick() {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stop_by(&app, state, b"feature", 'm', InProgress::Merge);
    on_the_first_clash(&state);
    let solved = state.resolver.as_ref().expect("open").path.clone();
    assert_eq!(solved, PathBuf::from("notes.md"));
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the pick", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.job.is_none() && open.in_flight.is_none())
    });
    assert_eq!(read(repo, "notes.md"), "our plan\n");

    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none() && state.banner_focus);
    let bar = bottom(&state);
    assert!(bar.contains("⏎ back to the clashes"), "{bar}");
    assert!(bar.contains("c finish"), "a merge's `c`: {bar}");

    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the first open clash", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new("shared.rs") && open.file.is_some() && open.job.is_none()
        })
    });
    assert_eq!(
        state.resolver.as_ref().and_then(|open| open.cursor_block),
        Some(0)
    );
    assert_eq!(read(repo, "notes.md"), "our plan\n", "the pick is kept");

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.pump(state, "the pick undone", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == solved
                && open.job.is_none()
                && open.file.as_ref().is_some_and(|file| file.open() == 1)
        })
    });
    assert!(
        read(repo, "notes.md").contains("<<<<<<<"),
        "the clash is back"
    );
    drop(state);
    app.close();
}

/// §2.6a: from the graph mid-merge, `u` refuses with the operation's words.
#[test]
fn u_from_the_graph_mid_merge_refuses() {
    let scratch = about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = stop_by(&app, state, b"feature", 'm', InProgress::Merge);
    let state = app.key(&state, KeyCode::Escape);
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.resolver.is_none() && !state.banner_focus, "the graph");
    let state = app.key(&state, KeyCode::Char('u'));
    let said = format!("{:?}", state.status);
    assert!(said.contains("finish or abort the merge first"), "{said}");
    assert!(state.operating.is_none(), "nothing ran");
    drop(state);
    app.close();
}

/// A merge git began in the shell, stopped on `notes.md` and `shared.rs`,
/// with `solved` written over with its clash picked — not staged — and the
/// app opened on it: on the banner, `⏎ resolve` (first landing).
fn relaunched_with(solved: &str, body: &str) -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = about_to_conflict();
    let repo = scratch.path();
    let merge = std::process::Command::new("git")
        .args(["merge", "--quiet", "feature"])
        .current_dir(repo)
        .output()
        .expect("git runs");
    assert!(!merge.status.success(), "the merge stops on its clashes");
    std::fs::write(repo.join(solved), body).expect("writable");
    let (app, state) = App::open(repo);
    let state = app.pump(state, "the launch on the banner", |state| {
        state.in_progress.is_some() && state.banner_focus && state.working_tree.is_some()
    });
    let bar = bottom(&state);
    assert!(bar.contains("⏎ resolve"), "{bar}");
    (scratch, app, state)
}

/// Banner `⏎`, and every read it waits on landed.
fn entered(app: &App, state: &AppState, path: &str) -> AppState {
    let state = app.key(state, KeyCode::Enter);
    app.pump(state, "the first open clash", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.path == Path::new(path)
                && open.file.is_some()
                && open.job.is_none()
                && open.set_job.is_none()
                && !open.seek_open
        })
    })
}

/// The row `path` reads in the resolver's list.
pub(crate) fn row(state: &AppState, path: &str) -> RowState {
    listed_files(state)
        .into_iter()
        .find(|file| file.path == Path::new(path))
        .map(|file| file.state)
        .expect("listed")
}

/// Relaunch, git's first file solved on disk: `⏎` hands on past it to the
/// first open clash, and its row reads solved. Still unmerged in git.
#[test]
fn a_relaunch_hands_on_past_a_file_solved_on_disk() {
    let (scratch, app, state) = relaunched_with("notes.md", "our plan\n");
    let state = entered(&app, &state, "shared.rs");
    assert_eq!(
        state.resolver.as_ref().and_then(|open| open.cursor_block),
        Some(0)
    );
    assert_eq!(row(&state, "notes.md"), RowState::AllTaken);
    let unmerged = git(scratch.path(), &["ls-files", "-u", "--", "notes.md"]);
    assert!(!unmerged.trim().is_empty(), "nothing staged");
    drop(state);
    app.close();
}

/// Relaunch, a later file solved on disk and never opened here: its row
/// reads solved from the set read (`ConflictSet::solved`), drawn `✓ done`
/// in a merge (ease-pass/11), while git's first file — open — is where `⏎`
/// lands.
#[test]
fn a_relaunch_reads_an_unopened_file_solved_on_disk_as_solved() {
    let (_scratch, app, state) = relaunched_with("shared.rs", "fn main() { ours() }\n");
    let state = entered(&app, &state, "notes.md");
    assert_eq!(row(&state, "shared.rs"), RowState::AllTaken);
    assert!(matches!(row(&state, "notes.md"), RowState::Open { .. }));
    let drawn = crate::captures::frame(&state, 200, 40);
    let listed = crate::captures::list_row(&drawn, "shared.rs");
    assert_eq!(listed, "✓ shared.rs done", "{}", drawn.join("\n"));
    drop(state);
    app.close();
}
