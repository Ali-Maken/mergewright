//! A one-file merge through the app (ease-pass/05b, built in 05c): its
//! clashes solved, the file reads solved and stays unmerged; the whole
//! one-clash merge is `m y o y y` and lands on the merge commit.

#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::update::resolver::RowState;
use graft_core::event::KeyCode;
use graft_git::state::InProgress;

use crate::app_harness::{App, git};
use crate::clash_stops::{on_branch, on_the_first_clash, read, row, stop_by};

/// `only.md` clashes, and nothing else: one file, its clashes where
/// `ours` and `theirs` differ from `base` line by line.
fn only(base: &str, theirs: &str, ours: &str) -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let commit = |subject: &str, minute: i64| {
        git(repo, &["add", "--all"]);
        let when = format!("@{} +0000", 1_609_459_200 + minute * 60);
        graft_fixtures::git::run_at(repo, &["commit", "-m", subject], &when)
            .unwrap_or_else(|why| panic!("committing {subject:?}: {why}"));
    };
    let write = |body: &str| std::fs::write(repo.join("only.md"), body).expect("writable");
    write(base);
    commit("the plan", 1);
    git(repo, &["switch", "--quiet", "-c", "feature"]);
    write(theirs);
    commit("their plan", 2);
    git(repo, &["switch", "--quiet", "main"]);
    write(ours);
    commit("our plan", 3);
    scratch
}

/// One file, one clash.
fn one_clash() -> graft_fixtures::Scratch {
    only("the plan\n", "their plan\n", "our plan\n")
}

/// The merge's journey (ease-pass.md Journeys, merge-one-clash): from the
/// cursor on the branch, `m y o y y` — five keys — and he is on the graph,
/// the cursor on the merge commit, nothing else open.
#[test]
fn a_one_clash_merge_is_m_y_o_y_y_and_lands_on_the_merge_commit() {
    let scratch = one_clash();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    // `m`, `y`: keys 1 and 2.
    let state = stop_by(&app, state, b"feature", 'm', InProgress::Merge);
    on_the_first_clash(&state);
    // `o`: key 3 — every clash solved, nothing staged.
    let state = app.key(&state, KeyCode::Char('o'));
    let state = app.pump(state, "the pick", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.job.is_none()
                && open.in_flight.is_none()
                && open.file.as_ref().is_some_and(|file| file.open() == 0)
        })
    });
    // `y`: key 4 — the review.
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the review", |state| {
        state
            .finish
            .as_ref()
            .is_some_and(|finish| finish.job.is_none())
    });
    // `y`: key 5 — the merge commit.
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the merge commit and its landing", |state| {
        state.in_progress.is_none() && state.operating.is_none() && !state.land_on_head
    });
    assert!(
        state.resolver.is_none() && state.finish.is_none() && state.commit_editor.is_none(),
        "the graph has the keyboard"
    );
    assert!(!state.banner_focus);
    assert!(on_branch(&state, b"main"), "the cursor on the merge commit");
    let parents = git(repo, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    assert_eq!(parents.split_whitespace().count(), 3, "a merge: {parents}");
    assert_eq!(read(repo, "only.md"), "our plan\n");
    drop(state);
    app.close();
}

/// One file, two clashes (lines 2 and 8, far enough apart to stay two):
/// the stop lands on clash 1 (0 keys); `o o` and the file reads solved —
/// drawn `solved ✓` — while git still lists it unmerged: nothing staged.
#[test]
fn two_clashes_picked_read_solved_and_stay_unmerged() {
    let lines = |two: &str, eight: &str| format!("1\n{two}\n3\n4\n5\n6\n7\n{eight}\n9\n");
    let scratch = only(
        &lines("2", "8"),
        &lines("their 2", "their 8"),
        &lines("our 2", "our 8"),
    );
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = stop_by(&app, state, b"feature", 'm', InProgress::Merge);
    on_the_first_clash(&state);
    let blocks = |state: &graft::update::AppState| {
        state
            .resolver
            .as_ref()
            .and_then(|open| open.file.as_ref())
            .map(|file| (file.open(), file.blocks().len()))
    };
    assert_eq!(blocks(&state), Some((2, 2)), "two clashes");
    let mut state = state;
    for left in [1, 0] {
        state = app.key(&state, KeyCode::Char('o'));
        state = app.pump(state, "the pick", |state| {
            state.resolver.as_ref().is_some_and(|open| {
                open.job.is_none()
                    && open.in_flight.is_none()
                    && open.file.as_ref().is_some_and(|file| file.open() == left)
            })
        });
    }
    assert_eq!(row(&state, "only.md"), RowState::AllTaken);
    let drawn = crate::captures::frame(&state, 200, 40);
    let listed = crate::captures::list_row(&drawn, "only.md");
    assert_eq!(listed, "▏✓ only.md done", "{}", drawn.join("\n"));
    assert_eq!(read(repo, "only.md"), lines("our 2", "our 8"));
    let unmerged = git(repo, &["ls-files", "-u", "--", "only.md"]);
    assert!(
        !unmerged.trim().is_empty(),
        "still unmerged: nothing staged"
    );
    drop(state);
    app.close();
}

/// Review `u` (ease-pass/05s, Minor 6): from the review over a file whose
/// two clashes are picked, `u` closes the review, puts the last pick back,
/// and lands the resolver's cursor on that clash — the second — with one
/// clash open again.
#[test]
fn u_in_the_review_lands_on_the_undone_clash() {
    let lines = |two: &str, eight: &str| format!("1\n{two}\n3\n4\n5\n6\n7\n{eight}\n9\n");
    let scratch = only(
        &lines("2", "8"),
        &lines("their 2", "their 8"),
        &lines("our 2", "our 8"),
    );
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let mut state = stop_by(&app, state, b"feature", 'm', InProgress::Merge);
    on_the_first_clash(&state);
    for left in [1, 0] {
        state = app.key(&state, KeyCode::Char('o'));
        state = app.pump(state, "the pick", |state| {
            state.resolver.as_ref().is_some_and(|open| {
                open.job.is_none()
                    && open.in_flight.is_none()
                    && open.file.as_ref().is_some_and(|file| file.open() == left)
            })
        });
    }
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "the review", |state| {
        state
            .finish
            .as_ref()
            .is_some_and(|finish| finish.job.is_none())
    });
    let state = app.key(&state, KeyCode::Char('u'));
    assert!(state.finish.is_none(), "the review closed");
    let state = app.pump(state, "the untake and its landing", |state| {
        state.resolver.as_ref().is_some_and(|open| {
            open.job.is_none()
                && open.in_flight.is_none()
                && open.file.as_ref().is_some_and(|file| file.open() == 1)
        })
    });
    let open = state.resolver.as_ref().expect("the resolver is open");
    assert_eq!(open.cursor_block, Some(1), "on the undone clash");
    assert_eq!(open.file.as_ref().map(|file| file.open()), Some(1));
    assert!(
        read(repo, "only.md").contains("<<<<<<<"),
        "the clash is back on disk"
    );
    drop(state);
    app.close();
}
