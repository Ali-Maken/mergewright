//! The commit screen's captures, next to the commit mock
//! (`prototypes/ease-mocks/mergewright-commit-screen.html`): each a frame of
//! the real app over real `git`, for the user to judge by eye (rule 6).
//!
//! Regenerate on purpose: `MWR_UPDATE_SHOTS=1 cargo nextest run -E
//! 'test(capture_the_commit_screen)'`.

use graft::update::AppState;
use graft_core::event::{KeyCode, KeyPress, Modifiers};

use super::{SECRET_VALUE, landed, opened, ready, three_changed_and_a_secret, typed};
use crate::app_harness::{App, git};

/// Each capture, in the order [`screens`] draws them: its file name (the
/// width is appended), what right looks like, and its size.
const SHOTS: [(&str, &str, u16, u16); 7] = [
    (
        "commit-open",
        "`c` on three edits and a new `.env`. Cursor in the subject, placeholder `What did you change? (one line)`; header `committing to main`; files `Going in (3)` then `Staying out (1)`; the first file's changes on the right; bar `⏎ type a message first  ⇥ more detail …`.",
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
        "`fix toc` typed, `⇥ ⇥`: the file list has focus; bar `space leave it out  j/k move  ⏎ commit 3 files  esc back to the message`.",
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
        "commit",
        "`fix toc` typed, at 90 columns: under 100 the changes pane is hidden and the files take the width; the bar keeps whole keys, the last dropped rather than cut.",
        90,
        30,
    ),
    (
        "commit-landed",
        "after `⏎`: the graph, the cursor on `fix toc`; the bottom row is the status `Committed 3 files to main.`, its tick, `u undo · P push` (the row's bar, `u undo the commit, your edits come back · P push · c commit again`, is under it, not drawn while the status stands); the top line ends `1 commit to push`; `Uncommitted changes (1 untracked)`: `.env`, left out. Ids zeroed (made by the run).",
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

/// A run of its own: `c`, `fix toc`, `⏎`, landed.
fn committed_in_a_fresh_run() -> (tempfile::TempDir, AppState) {
    let scratch = three_changed_and_a_secret();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, &ready(&app, state));
    let state = typed(&app, &state, "fix toc");
    let state = landed(&app, app.key(&state, KeyCode::Enter));
    let state = crate::captures::at_rest(&app, crate::captures::settled_frame(&app, state));
    app.close();
    (scratch, state)
}

/// The six screens before the commit, in [`SHOTS`]' order, from one run.
fn screens() -> Vec<AppState> {
    let scratch = three_changed_and_a_secret();
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
    app.close();
    vec![open, secrets, files, refused, options, message]
}

#[test]
fn capture_the_commit_screen() {
    let mut states = screens();
    // The landing from a run of its own: the states above are forks of one
    // run, and a fork's job ids repeat its siblings', so a write from one
    // would take a sibling's read answer as its own.
    let (landed_scratch, committed) = committed_in_a_fresh_run();
    let landed_repo = landed_scratch.path();
    let made = git(landed_repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let parent = git(landed_repo, &["rev-parse", "HEAD~1"]).trim().to_owned();
    states.push(committed);

    let mut captures = graft_fixtures::Captures::new();
    for ((name, judge, columns, rows), state) in SHOTS.into_iter().zip(&states) {
        let frame = crate::captures::frame_at(state, columns, rows, None);
        assert!(
            !frame.iter().any(|row| row.contains(SECRET_VALUE)),
            "{name}: `.env`'s value drawn"
        );
        let body = format!(
            "# TestBackend capture (not a pty), no colour recorded (rule 5).\n\
             # Fixture: capture_the_commit_screen in\n\
             # crates/graft/tests/it/commit_screen/shots.rs, real git.\n\
             # Judge by eye: {judge}\n\
             === {columns}x{rows} ===\n{}\n",
            frame.join("\n")
        );
        captures.check(
            format!(
                "{}/../../docs/shots/ease-pass/{name}-{columns}.txt",
                env!("CARGO_MANIFEST_DIR"),
            ),
            // Only the landing shows ids: the ones its own run made.
            if name == "commit-landed" {
                crate::captures::without_invented_id(
                    &crate::captures::without_invented_id(&body, &made),
                    &parent,
                )
            } else {
                body
            },
        );
    }
    captures.finish();
}
