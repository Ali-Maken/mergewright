//! m6-conflicts/86: `⏎` and `p` on the stash list open a Ghost Preview over
//! the list; `y` runs it, `esc` goes back to the same stash having written
//! nothing. Walked against a real repository; the three frames the user
//! judges by eye are written under `docs/shots/m6/`.

use std::path::Path;

use graft::session::Clock;
use graft::update::AppState;
use graft::update::detail::Loading;
use graft_core::event::KeyCode;

use super::{Keys, planned};
use crate::app_harness::{App, git};
use crate::captures::{check_shot, frame_at};
use crate::preview::{FORK_EPOCH, FORK_NOW, dated_commit};
use crate::stash_clash::{NOTES, a_stash_that_clashes};

/// The note's row: `S ⏎ esc`, a look at what the apply would do, then back.
pub const STASH_LOOK: &str = "stash-look-and-leave";

/// What the repository holds that a stash apply or pop would change: the
/// stash list, the index and the files, read with git's read-only commands.
fn truth(repo: &Path) -> [String; 3] {
    [
        git(repo, &["stash", "list", "--format=%H %gs"]),
        git(repo, &["diff", "--cached", "--name-status"]),
        git(repo, &["status", "--porcelain=v1", "--untracked-files=all"]),
    ]
}

/// The stash list is open (`S`, counted).
fn listed(keys: &mut Keys<'_>, state: AppState) -> AppState {
    let app = keys.app;
    let state = app.pump(state, "the working tree and the refs", |state| {
        state.working_tree.is_some() && state.refs.is_some()
    });
    let state = keys.press(&state, KeyCode::Char('S'));
    app.pump(state, "the stash list", |state| {
        !matches!(state.stash.entries, Loading::Idle | Loading::Pending(_))
    })
}

fn drawn(state: &AppState, width: u16, height: u16) -> Vec<String> {
    frame_at(state, width, height, Some(Clock::Fixed(FORK_NOW)))
}

fn shows(frame: &[String], words: &str) -> bool {
    frame.iter().any(|row| row.contains(words))
}

/// Review 86, M2: while the box is open the stash list's bar is not drawn —
/// the box's footer is the bar, as on the graph — and the frame's last row
/// is blank.
fn no_list_bar(frame: &[String]) {
    assert!(
        !shows(frame, "space files") && !shows(frame, "s save"),
        "the list's bar under the box:\n{}",
        frame.join("\n")
    );
    assert_eq!(
        frame.last().map(|row| row.trim()),
        Some(""),
        "the bar row is blank:\n{}",
        frame.join("\n")
    );
}

/// `stash-look-and-leave`: `S ⏎ esc` — the apply preview for a stash that
/// would clash, looked at and left. Nothing is written: the stash list, the
/// index and the files are as they were, and the list is back on the same
/// stash. Writes `docs/shots/m6/stash-apply-preview-120x40.txt`.
#[test]
fn stash_look_and_leave_writes_nothing() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let before = truth(path);
    let (app, state) = App::open(path);
    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = listed(&mut keys, state);
    let selected = state.stash.selected;
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(&app, state);

    let frame = drawn(&state, 120, 40);
    for words in [
        "Preview: apply stash@{0}",
        "working tree: notes.md",
        "may conflict: notes.md",
        "stash@{0} stays in the list",
        "y apply",
    ] {
        assert!(shows(&frame, words), "`{words}`:\n{}", frame.join("\n"));
    }
    no_list_bar(&frame);
    check_shot(
        "m6/stash-apply-preview-120x40.txt",
        &format!("{APPLY_HEADER}\n{}\n", frame.join("\n")),
    );

    let state = keys.press(&state, KeyCode::Escape);
    assert!(state.preview.is_none(), "esc closed the preview");
    assert!(state.stash.shown, "esc went back to the stash list");
    assert_eq!(state.stash.selected, selected, "on the same stash");
    let allowed = crate::m5_walk::noted_keys(STASH_LOOK).unwrap();
    assert!(
        keys.count <= allowed,
        "{STASH_LOOK}: {} keys, the note allows {allowed}",
        keys.count
    );
    assert_eq!(truth(path), before, "the look wrote nothing");

    // Not counted: closing the list is leaving the stash, not the look.
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.stash.shown);
    drop(state);
    app.close();
    assert_eq!(truth(path), before, "still nothing written");
}

/// The note's row: `S p y`, a pop that does not clash.
pub const STASH_POP_CLEAN: &str = "stash-pop-clean";

/// `stash-pop-clean`: `S p y` on a stash that pops cleanly. The preview says
/// `no conflicts expected`, `y` pops it, the change is back in the file and
/// the stash is gone. Keys within the note's row (review 86, m7).
#[test]
fn a_clean_stash_pop_is_three_keys() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    dated_commit(path, NOTES, "base\n", "add the notes", 1);
    std::fs::write(path.join(NOTES), "stashed\n").expect("the scratch repo is writable");
    let when = format!("@{} +0000", FORK_EPOCH + 2 * 60);
    graft_fixtures::git::run_at(path, &["stash", "push"], &when)
        .unwrap_or_else(|why| panic!("stashing: {why}"));
    let (app, state) = App::open(path);
    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = listed(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('p'));
    let state = planned(&app, state);
    let frame = drawn(&state, 120, 40);
    assert!(
        shows(&frame, "no conflicts expected") && shows(&frame, "y pop"),
        "a clean pop preview:\n{}",
        frame.join("\n")
    );
    let state = keys.press(&state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the pop");

    let allowed = crate::m5_walk::noted_keys(STASH_POP_CLEAN).unwrap();
    assert!(
        keys.count <= allowed,
        "{STASH_POP_CLEAN}: {} keys, the note allows {allowed}",
        keys.count
    );
    assert_eq!(
        std::fs::read_to_string(path.join(NOTES)).expect("the file"),
        "stashed\n",
        "the stash's change is back"
    );
    assert!(
        git(path, &["stash", "list"]).trim().is_empty(),
        "the stash is popped"
    );
    drop(state);
    app.close();
}

/// `S p` at 80x24: the pop preview for the same stash, its fate row saying
/// the stash is dropped and `u` brings it back. Writes
/// `docs/shots/m6/stash-pop-preview-80x24.txt`.
#[test]
fn the_pop_preview_fits_80_columns() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    let before = truth(path);
    let (app, state) = App::open(path);
    let state = app.resize(&state, 80, 24);
    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = listed(&mut keys, state);
    let state = keys.press(&state, KeyCode::Char('p'));
    let state = planned(&app, state);

    let frame = drawn(&state, 80, 24);
    for words in [
        "Preview: pop stash@{0}",
        "may conflict: notes.md",
        "stash@{0} is dropped",
        "y pop",
    ] {
        assert!(shows(&frame, words), "`{words}`:\n{}", frame.join("\n"));
    }
    no_list_bar(&frame);
    check_shot(
        "m6/stash-pop-preview-80x24.txt",
        &format!("{POP_HEADER}\n{}\n", frame.join("\n")),
    );
    drop(state);
    app.close();
    assert_eq!(truth(path), before, "the preview wrote nothing");
}

/// `S ⏎` with an unsaved edit to `notes.md`: the box says the edit is in the
/// way instead of forecasting, and offers nothing to confirm. Writes
/// `docs/shots/m6/stash-preview-in-the-way-120x40.txt`.
#[test]
fn unsaved_edits_in_the_way_are_named_in_the_box() {
    let scratch = a_stash_that_clashes();
    let path = scratch.path();
    std::fs::write(path.join(NOTES), "unsaved\n").expect("the scratch repo is writable");
    let before = truth(path);
    let (app, state) = App::open(path);
    let mut keys = Keys {
        app: &app,
        count: 0,
    };
    let state = listed(&mut keys, state);
    let state = keys.press(&state, KeyCode::Enter);
    let state = planned(&app, state);

    let frame = drawn(&state, 120, 40);
    let words = "your unsaved edits to notes.md are in the way";
    assert!(shows(&frame, words), "`{words}`:\n{}", frame.join("\n"));
    assert!(
        !shows(&frame, "y apply"),
        "a refusal offers no `y`:\n{}",
        frame.join("\n")
    );
    // Review 86, m3: one `Close`, so nothing to choose; the keys are
    // `esc back`.
    assert!(
        shows(&frame, "╰ esc back ─") && !shows(&frame, "choose"),
        "a refusal's footer is `esc back`:\n{}",
        frame.join("\n")
    );
    no_list_bar(&frame);
    check_shot(
        "m6/stash-preview-in-the-way-120x40.txt",
        &format!("{IN_THE_WAY_HEADER}\n{}\n", frame.join("\n")),
    );

    // `y` on a refusal runs nothing.
    let state = app.key(&state, KeyCode::Char('y'));
    let state = app.pump(state, "any write the `y` queued", |state| {
        state.operating.is_none()
    });
    drop(state);
    app.close();
    assert_eq!(truth(path), before, "the refusal wrote nothing");
}

const APPLY_HEADER: &str = "\
# m6/stash-apply-preview-120x40.txt: 120x40, drawn by `view::view` from a
# real AppState (crates/graft/tests/it/m6_stash_put_back/stash_preview.rs,
# stash_look_and_leave_writes_nothing, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `S ⏎` on a stash whose notes.md clashes with the
# commit made since.
#  - A box over the stash list titled `Preview: apply stash@{0}`.
#  - `working tree: notes.md`, then `may conflict: notes.md`.
#  - The fate row: `stash@{0} stays in the list`.
#  - The footer offers `y apply`; `esc` goes back to the list, nothing written.
#  - No stash-list bar under the box: the box's footer is the bar, and the
#    last row is blank (review 86, M2).
";

const POP_HEADER: &str = "\
# m6/stash-pop-preview-80x24.txt: 80x24, drawn by `view::view` from a real
# AppState (crates/graft/tests/it/m6_stash_put_back/stash_preview.rs,
# the_pop_preview_fits_80_columns, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# What right looks like: `S p` on the same stash, at 80 columns.
#  - A box titled `Preview: pop stash@{0}`, whole inside 80 columns.
#  - `may conflict: notes.md`.
#  - The fate row: `stash@{0} is dropped · u brings it back`.
#  - The footer offers `y pop`.
#  - No stash-list bar under the box; the last row is blank.
";

const IN_THE_WAY_HEADER: &str = "\
# m6/stash-preview-in-the-way-120x40.txt: 120x40, drawn by `view::view`
# from a real AppState (crates/graft/tests/it/m6_stash_put_back/
# stash_preview.rs, unsaved_edits_in_the_way_are_named_in_the_box,
# asserted). Not a pty capture: a character grid with no colour (CLAUDE.md
# rule 5, colour unverified).
#
# What right looks like: `S ⏎` with an unsaved edit to notes.md, which the
# stash also changes.
#  - The box over the stash list says `your unsaved edits to notes.md are in
#    the way` — no forecast, no `y apply`.
#  - One `[ Close ]`; the footer reads `esc back` (review 86, m3), and no
#    stash-list bar under the box.
#  - Nothing was written: `y` does nothing, `esc` goes back to the list.
";
