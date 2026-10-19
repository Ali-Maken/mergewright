//! The commit screen's captures, next to the commit mock
//! (`prototypes/ease-mocks/mergewright-commit-screen.html`): each a frame of
//! the real app over real `git`, for the user to judge by eye (rule 6).
//!
//! Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo nextest run -E
//! 'test(capture_the_commit_screen)'`.

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::{KeyCode, KeyPress, Modifiers};

use super::rows::amend_toggled;
use super::{SECRET_VALUE, landed, opened, ready, three_changed_and_a_secret, typed};
use crate::app_harness::{App, git};

/// Each capture, in the order [`capture_the_commit_screen`] draws them: its
/// file name (the width is appended), what right looks like, and its size.
const SHOTS: [(&str, &str, u16, u16); 9] = [
    (
        "commit-open",
        "`c` on three edits and a new `.env`. Header `◆ api  ~/code/api  committing to main` (level with `origin/main`: no push words): the repository words the graph's top line starts with, then the branch. Cursor in the subject, placeholder `What did you change? (one line)`; files `Going in (3)` then `Staying out (1)`; the first file's changes on the right; bar `⏎ type a message first  ⇥ more detail …`.",
        120,
        40,
    ),
    (
        "commit-secrets",
        "`↓` three times from the subject: the cursor on `.env` under `Staying out`, marked `!` and `looks like a secrets file`; the note `! left out: looks like a secrets file. space puts it in.` over its contents on the right, its value drawn `TOKEN=••••••••`: the fixture's value is nowhere in this file.",
        120,
        40,
    ),
    (
        "commit-files-focus",
        "`fix toc` typed, `⇥ ⇥`: the file list has focus; bar `space leave it out  j/k move  ⏎ commit  esc back to the message`.",
        120,
        40,
    ),
    (
        "commit-refused",
        "every file left out with `space`, then `⏎`: nothing committed; all four under `Staying out`; the line above the files reads `Nothing goes in. Put a file in with space.`; the bar's `⏎` slot reads `put a file in first`.",
        120,
        40,
    ),
    (
        "commit-options",
        "`⌃S`, then `Skip the commit hooks` from `⌃P`: no option row anywhere; the header ends `· signing off · skipping hooks`.",
        120,
        40,
    ),
    (
        "commit-amend-from-c",
        "`c`, `fix toc`, then `Amend the last commit` from `⌃P`: the header reads `amending 0000000 start` in place of `committing to main` (the id zeroed: the run made it); no push words; the line above the files reads `3 files go in, +4 −0, into the last commit. 1 stays out.`; message and files as they were.",
        120,
        40,
    ),
    (
        "commit",
        "`fix toc` typed, at 90 columns: under 100 the changes pane is hidden and the files take the width; the bar has no `↑ ↓` (no changes beside the files to look at) and keeps `⏎ commit 3 files` and `esc cancel, your message is kept`.",
        90,
        30,
    ),
    (
        "commit-partial",
        "`src/a.rs` staged, then one more line added to it and left unstaged; `c`: only `src/a.rs` goes in, its row ending `+1 part`; the line above the files reads `1 file goes in, +1 −0, as a new commit on main. 3 stay out.`; the changes pane's top line says `only the part you staged goes in; the rest stays uncommitted.`; the changes show the staged `fn b() {}` and not the unstaged `fn c() {}`.",
        120,
        40,
    ),
    (
        "commit-landed",
        "after `⏎`: the graph, the cursor on `fix toc`; the second-last row is the done line `✓ Committed 3 files to main.  u undo · P push`, tick first; the last row is the bar: `. explain` first (the new commit carries a radar `ⓘ`), then `u undo the commit, your edits come back · P push · c commit`, `c commit again` at its short rung to make room; the top line ends `1 commit to push`; `Uncommitted changes (1 untracked)`: `.env`, left out.",
        120,
        40,
    ),
];

fn ctrl(c: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }
}

/// `n` presses of `code`.
fn pressed(app: &App, state: &AppState, code: KeyCode, n: usize) -> AppState {
    (0..n).fold(state.clone(), |state, _| app.key(&state, code))
}

/// Wait until the file under the cursor has its changes drawn.
fn diff_in(app: &App, state: AppState) -> AppState {
    app.pump(state, "the cursor file's changes", |state| {
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.diff.view.is_some())
    })
}

/// A run of its own on the mock's tree, changed first by `prepare`: the
/// screens [`screens`] draws are forks of one run, and a fork's job ids
/// repeat its siblings', so a screen with a tree or a write of its own
/// would take a sibling's answer as its own.
fn own_run(
    prepare: impl FnOnce(&Path),
    act: impl FnOnce(&App, AppState) -> AppState,
) -> (tempfile::TempDir, AppState) {
    let scratch = three_changed_and_a_secret();
    prepare(scratch.path());
    let (app, state) = App::open(scratch.path());
    let state = act(&app, ready(&app, state));
    app.close();
    (scratch, state)
}

/// The screens before the commit, in [`SHOTS`]' order, from one run, and
/// the id of that run's one commit (the amend header names it).
fn screens() -> (Vec<AppState>, String) {
    let scratch = three_changed_and_a_secret();
    let head = git(scratch.path(), &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    let (app, state) = App::open(scratch.path());
    let open = opened(&app, &ready(&app, state));
    // `.env` is the fourth row: the three going in come first.
    let secrets = diff_in(&app, pressed(&app, &open, KeyCode::Down, 3));
    let message = typed(&app, &open, "fix toc");
    let files = diff_in(&app, pressed(&app, &message, KeyCode::Tab, 2));
    // Leave all three out: to the top, `space`, again until none goes in.
    let refused = (0..3).fold(files.clone(), |state, _| {
        let top = pressed(&app, &state, KeyCode::Char('k'), 4);
        app.key(&top, KeyCode::Char(' '))
    });
    let refused = app.key(&refused, KeyCode::Enter);
    let refused = crate::captures::settled_frame(&app, refused);
    assert_eq!(
        refused
            .commit_editor
            .as_ref()
            .map(|editor| editor.going_in()),
        Some(0),
        "every file left out"
    );
    let signed = app.press(&message, ctrl('s'));
    let palette = app.press(&signed, ctrl('p'));
    let options = app.key(
        &typed(&app, &palette, "Skip the commit hooks"),
        KeyCode::Enter,
    );
    let editor = options.commit_editor.as_ref().expect("the screen stays");
    assert!(editor.sign_off && editor.no_verify, "both options on");
    // Flipping amend is a flag, no read: safe to fork (08s M2).
    let amended = amend_toggled(&app, &message);
    assert!(
        amended
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.amend),
        "amend on"
    );
    app.close();
    (
        vec![open, secrets, files, refused, options, amended, message],
        head,
    )
}

/// `src/a.rs` staged, then a line more left unstaged (08s M3), and `c`.
fn partly_staged() -> (tempfile::TempDir, AppState) {
    own_run(
        |repo| {
            git(repo, &["add", "src/a.rs"]);
            std::fs::write(repo.join("src/a.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n")
                .expect("writable");
        },
        |app, state| opened(app, &state),
    )
}

/// `c`, `fix toc`, `⏎`, landed.
fn committed() -> (tempfile::TempDir, AppState) {
    own_run(
        |_| {},
        |app, state| {
            let state = typed(app, &opened(app, &state), "fix toc");
            let state = landed(app, app.key(&state, KeyCode::Enter));
            crate::captures::at_rest(app, crate::captures::settled_frame(app, state))
        },
    )
}

#[test]
fn capture_the_commit_screen() {
    let (mut states, head) = screens();
    let (_partial_scratch, partial) = partly_staged();
    states.push(partial);
    let (_landed_scratch, committed) = committed();
    states.push(committed);
    assert_eq!(states.len(), SHOTS.len(), "one state per capture");

    let mut captures = mergewright_fixtures::Captures::new();
    for ((name, judge, columns, rows), state) in SHOTS.into_iter().zip(&states) {
        let frame = crate::captures::frame_at(state, columns, rows, None);
        assert!(
            !frame.iter().any(|row| row.contains(SECRET_VALUE)),
            "{name}: `.env`'s value drawn"
        );
        let body = format!(
            "# TestBackend capture (not a pty), no colour recorded (rule 5).\n\
             # Fixture: capture_the_commit_screen in\n\
             # crates/mergewright/tests/it/commit_screen/shots.rs, real git.\n\
             # Judge by eye: {judge}\n\
             === {columns}x{rows} ===\n{}\n",
            frame.join("\n")
        );
        // Only the amend's header shows an id the run made: rows draw none
        // since ease-pass 07.
        let made_by_the_run: &[&str] = match name {
            "commit-amend-from-c" => &[&head],
            _ => &[],
        };
        let body = made_by_the_run.iter().fold(body, |body, id| {
            crate::captures::without_invented_id(&body, id)
        });
        // The landed commit is the run's: its hash on the cursor's second
        // line (07b) is zeroed.
        let body = if name == "commit-landed" {
            crate::captures::without_cursor_sha(state, &body)
        } else {
            body
        };
        captures.check(
            format!(
                "{}/../../docs/shots/ease-pass/{name}-{columns}.txt",
                env!("CARGO_MANIFEST_DIR"),
            ),
            body,
        );
    }
    captures.finish();
}
