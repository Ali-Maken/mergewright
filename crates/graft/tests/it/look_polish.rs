//! m8-polish-and-ship/17 (G-look polish): captures for the user's eye.

#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use graft::update::AppState;
use graft_core::event::KeyCode;
use graft_ui::detail::DetailFocus;

use crate::app_harness::{App, git};

/// A two-commit repo whose tip changes only whitespace in one line and a
/// word in another, so `w` visibly changes the diff.
fn repo() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    std::fs::write(path.join("notes.txt"), "alpha beta\ngamma\n").expect("writable");
    git(path, &["add", "notes.txt"]);
    graft_fixtures::git::run_at(path, &["commit", "-m", "add notes"], "@1609459260 +0000")
        .expect("commit");
    std::fs::write(path.join("notes.txt"), "alpha   beta\ndelta\n").expect("writable");
    git(path, &["add", "notes.txt"]);
    graft_fixtures::git::run_at(path, &["commit", "-m", "reflow notes"], "@1609459320 +0000")
        .expect("commit");
    scratch
}

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

/// Item 8: with `w` on, the detail pane's title row ends
/// `· whitespace ignored` (muted; colour unverified, rule 5).
#[test]
fn capture_whitespace_ignored_title_for_docs_shots() {
    let scratch = repo();
    let (app, state) = App::open(scratch.path());
    let state = opened(&app, state);
    let tip = app.key(&state, KeyCode::Enter);
    let mut tip = app.pump(tip, "the tip's detail", |state| state.detail.shown);
    for _ in 0..3 {
        if tip.detail.focus == DetailFocus::Files {
            break;
        }
        tip = app.key(&tip, KeyCode::Tab);
    }
    let on = app.key(&tip, KeyCode::Char('w'));
    assert!(on.detail.options.ignore_whitespace, "`w` turned it on");
    let on = crate::captures::settled_frame(&app, on);
    let sized = app.resize(&on, 120, 36);
    let drawn = crate::captures::frame_at(
        &sized,
        120,
        36,
        Some(graft::session::Clock::Fixed(1_609_459_200 + 600)),
    );
    assert!(
        drawn
            .iter()
            .any(|line| line.contains("\u{b7} whitespace ignored")),
        "{drawn:#?}"
    );
    let mut out = String::from(
        "# m8/polish-whitespace-120x36: drawn by `view::view` from a real AppState\n\
         # (crates/graft/tests/it/look_polish.rs). Not a pty capture: a character grid,\n\
         # no colour (CLAUDE.md rule 5 — colour unverified).\n\
         #\n\
         # The tip's detail pane with `w` on. Right: the pane's title row ends\n\
         # `· whitespace ignored` (muted), right after the title.\n\n",
    );
    for line in &drawn {
        out.push_str(line);
        out.push('\n');
    }
    app.close();
    crate::captures::check_shot("m8/polish-whitespace-120x36.txt", &out);
}
