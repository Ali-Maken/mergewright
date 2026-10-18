//! PROMPT §9's seven tasks, each reachable "using only the hint bar and the
//! command palette" in the place it happens (m8-polish-and-ship/28, Scope
//! 4): one test per task, in the real app over a real repository. Either
//! the place's bar names the key, or `?` there, typed with the task's word,
//! puts the cursor on the action — its key spelled on the row.
//!
//! Doing the tasks is the tour's (`tour_tasks.rs`, one test per task, and
//! `tour_walk.rs`'s whole walk); this holds only that each is findable
//! where he stands.
#![allow(
    clippy::expect_used,
    reason = "a place that cannot be reached has nothing to show; the panic names why"
)]

use graft::session::Clock;
use graft::update::AppState;
use graft_core::event::{KeyCode, MouseButton};

use crate::app_harness::{App, changes_pane, git};
use crate::captures::{frame_at, settled_frame};

const COLUMNS: u16 = 120;
const ROWS: u16 = 36;
const CURSOR: char = '\u{25b8}';

fn drawn(state: &AppState) -> Vec<String> {
    frame_at(
        state,
        COLUMNS,
        ROWS,
        Some(Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    )
}

/// The key bar: the row ending `? keys`, or — a stop's banner, whose bar
/// ends `q quit` — the bottom row. With the detail pane open the bar sits
/// above it, not at the bottom.
fn bar(state: &AppState) -> String {
    let frame = drawn(state);
    frame
        .iter()
        .find(|line| line.trim_end().ends_with("? keys"))
        .or_else(|| frame.last())
        .cloned()
        .expect("a frame has a bottom row")
}

/// `?` in this place, then `word` typed: the palette row under the cursor.
fn asked(app: &App, state: &AppState, word: &str) -> String {
    let mut state = app.key(state, KeyCode::Char('?'));
    for character in word.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    let frame = drawn(&state);
    frame
        .iter()
        .find(|line| {
            line.char_indices()
                .any(|(at, glyph)| glyph == CURSOR && at > 0)
        })
        .cloned()
        .unwrap_or_else(|| panic!("`?` {word}: no cursor row\n{}", frame.join("\n")))
}

/// `?` `word` here lands on a row reading `label`, its key `key` spelled
/// with the cursor (`▸<key>`).
fn assert_asked(app: &App, state: &AppState, word: &str, label: &str, key: &str) {
    let row = asked(app, state, word);
    assert!(
        row.contains(label) && row.contains(&format!("{CURSOR}{key}")),
        "`?` {word}: {row:?}; the bar here: {:?}",
        bar(state)
    );
}

/// A repository with one tracked file edited, unstaged: the changes pane.
fn edited() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("notes.txt"), "one\ntwo\n").expect("writable");
    git(path, &["add", "notes.txt"]);
    graft_fixtures::git::run_at(path, &["commit", "-m", "add notes"], "@1609459260 +0000")
        .expect("commit");
    std::fs::write(path.join("notes.txt"), "one\ntwo, edited\n").expect("writable");
    scratch
}

fn opened(scratch: &graft_fixtures::Scratch) -> (App, AppState) {
    let (app, state) = App::open(scratch.path());
    let state = crate::look_polish::opened(&app, state);
    let state = app.resize(&state, COLUMNS, ROWS);
    let state = settled_frame(&app, state);
    (app, state)
}

/// §9 "stage a single hunk": in the changes pane, `⏎` puts the cursor on
/// the file's hunk; there the bar leads `space stage`, and `?` `stage` lands
/// on `space Stage / unstage`. (`?` `hunk` finds nothing: the label does not
/// say hunk — a question for the usability walk, not asserted here.)
#[test]
fn stage_a_hunk_is_found_in_the_changes_pane() {
    let scratch = edited();
    let (app, state) = opened(&scratch);
    let state = changes_pane(&app, state);
    let state = app.resize(&state, COLUMNS, ROWS);
    // The file's diff, its hunks under the cursor.
    let state = app.key(&state, KeyCode::Enter);
    let state = settled_frame(&app, state);
    assert!(bar(&state).contains("space stage"), "{:?}", bar(&state));
    assert_asked(&app, &state, "stage", "Stage / unstage", "space");
    app.close();
}

/// §9 "commit": with the edit staged, the graph's bar names `c commit`.
#[test]
fn commit_is_on_the_bar() {
    let scratch = edited();
    git(scratch.path(), &["add", "notes.txt"]);
    let (app, state) = opened(&scratch);
    let state = app.key(&state, KeyCode::Char('k'));
    assert!(bar(&state).contains("c commit"), "{:?}", bar(&state));
    app.close();
}

/// §9 "stash the rest and bring it back": `?` `stash` on the graph lands on
/// `S`; with an entry set aside, the stash list's bar names `p pop`.
#[test]
fn stash_and_bring_back_are_found() {
    let scratch = edited();
    let (app, state) = opened(&scratch);
    assert_asked(&app, &state, "stash", "Stash", "S");
    app.close();

    git(scratch.path(), &["stash", "push", "-m", "the rest"]);
    let (app, state) = opened(&scratch);
    let state = app.key(&state, KeyCode::Char('S'));
    assert!(state.stash.shown, "`S` opens the stash list");
    let state = app.pump(state, "the stash list read", |state| {
        drawn(state).iter().any(|line| line.contains("STASHES (1)"))
    });
    assert!(bar(&state).contains("p pop"), "{:?}", bar(&state));
    app.close();
}

/// The corpus octopus with the cursor clicked onto the unmerged `topic-4`'s
/// row ([`super::octopus_with_an_unmerged_branch`]: the octopus's own
/// topics fold by default).
fn on_a_branch_row() -> (graft_fixtures::Scratch, App, AppState) {
    let scratch = super::octopus_with_an_unmerged_branch();
    let (app, state) = opened(&scratch);
    let frame = drawn(&state);
    let y = frame
        .iter()
        .position(|line| line.contains("[topic-4]"))
        .unwrap_or_else(|| panic!("topic-4's row\n{}", frame.join("\n")));
    let row = u16::try_from(y).expect("short screen");
    let state = app.mouse_down(&state, MouseButton::Left, 40, row);
    (scratch, app, state)
}

/// §9 "create a branch": `?` `branch` on the graph lands on making one.
#[test]
fn create_a_branch_is_found() {
    let (_scratch, app, state) = on_a_branch_row();
    let row = asked(&app, &state, "new branch");
    assert!(
        row.to_lowercase().contains("branch") && row.contains(CURSOR),
        "{row:?}"
    );
    app.close();
}

/// §9 "merge it with a Ghost Preview": `?` `merge` on a branch row lands on
/// `m` — merge the selected branch (its preview is M5's, `y` confirms).
#[test]
fn merge_is_found_on_a_branch_row() {
    let (_scratch, app, state) = on_a_branch_row();
    assert_asked(
        &app,
        &state,
        "merge",
        "Merge selected branch into current",
        "m",
    );
    app.close();
}

/// §9 "resolve a conflict": at a merge stopped on a conflict, the banner's
/// bar leads with `⏎` — `back to the clashes` once `esc` has paused them
/// (ease-pass/05c, Q7 a), as here.
#[test]
fn resolve_is_on_the_stop_banner_bar() {
    let scratch = crate::conflict_interim::about_to_conflict();
    let (app, state) = App::open(scratch.path());
    let state = crate::conflict_interim::stopped_on_the_conflict(&app, state);
    let state = app.resize(&state, COLUMNS, ROWS);
    assert!(
        bar(&state).contains("\u{23ce} back to the clashes"),
        "{:?}",
        bar(&state)
    );
    app.close();
}

/// §9 "push": `?` `push` on the graph lands on `P`.
#[test]
fn push_is_found() {
    let (_scratch, app, state) = on_a_branch_row();
    assert_asked(&app, &state, "push", "Push", "P");
    app.close();
}
