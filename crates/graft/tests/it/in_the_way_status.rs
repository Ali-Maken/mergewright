//! The in-the-way refusal on the status line (m7-power-tools/30t, 26's
//! F-guard): real `git`, the real job pool, the switch (`C`, the name, `⏎`)
//! refused because a file git doesn't keep stands where it would write. At
//! 80 columns the line keeps the file's name and the next step whole; where
//! the whole sentence fits it reads as it always did.

use graft::session::Clock;
use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::{App, git};
use crate::preview::{FORK_EPOCH, dated_commit, frame_at, opened};

const LONG_BRANCH: &str = "feature/sara-long-running-oauth-token-refresh-rework";
const DEEP: &str = "src/services/authentication/providers/oauth2/callbacks";
const NAME: &str = "session-cache.json";

/// How the file in the way stands on `main`.
#[derive(Clone, Copy)]
enum Kept {
    Ignored,
    Untracked,
}

/// `main` from the trivial shape; `branch` tracks `DEEP/NAME`. Back on
/// `main`, the same path is on disk (ignored through `.git/info/exclude`,
/// or plainly untracked): switching would overwrite it.
fn repo(branch: &str, kept: Kept) -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let path = scratch.path();
    let deep = path.join(DEEP);
    std::fs::create_dir_all(&deep).unwrap_or_else(|why| panic!("{why}"));
    git(path, &["switch", "-q", "-c", branch]);
    dated_commit(
        path,
        &format!("{DEEP}/{NAME}"),
        "theirs\n",
        "two on the branch",
        1,
    );
    git(path, &["switch", "-q", "main"]);
    std::fs::create_dir_all(&deep).unwrap_or_else(|why| panic!("{why}"));
    if matches!(kept, Kept::Ignored) {
        let exclude = path.join(".git/info/exclude");
        std::fs::create_dir_all(path.join(".git/info")).unwrap_or_else(|why| panic!("{why}"));
        std::fs::write(exclude, format!("{NAME}\n")).unwrap_or_else(|why| panic!("{why}"));
    }
    std::fs::write(deep.join(NAME), "mine\n").unwrap_or_else(|why| panic!("{why}"));
    scratch
}

/// `C`, the branch's name typed, `⏎`: the switch, refused.
fn switch(app: &App, state: &AppState, branch: &str) -> AppState {
    let state = app.key(state, KeyCode::Char('C'));
    let mut state = app.pump(state, "the switch list", |state| {
        state
            .picker
            .as_ref()
            .is_some_and(|picker| !picker.rows().is_empty())
    });
    for character in branch.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the refusal", |state| {
        state.operating.is_none() && matches!(state.status, Status::Sticky { .. })
    })
}

/// The refused switch at `columns` x `rows`: the frame, and the repo still
/// on `main`.
fn refused_at(branch: &str, kept: Kept, columns: u16, rows: u16) -> Vec<String> {
    let scratch = repo(branch, kept);
    let path = scratch.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, columns, rows);
    let state = opened(&app, state);
    let state = app.pump(state, "the first status read", |state| {
        state.working_tree.is_some() && state.status_read.is_none()
    });
    let state = switch(&app, &state, branch);
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main",
        "refused: still on main"
    );
    let frame = frame_at(
        &state,
        columns,
        rows,
        Some(Clock::Fixed(FORK_EPOCH + 2 * 60)),
    );
    drop(state);
    app.close();
    frame
}

fn bottom(frame: &[String]) -> &str {
    frame.last().map_or("", String::as_str)
}

#[test]
fn a_refusal_at_80_keeps_the_file_and_the_next_step() {
    let frame = refused_at(LONG_BRANCH, Kept::Ignored, 80, 24);
    let body = frame.join("\n");
    let row = bottom(&frame);
    assert!(row.contains(NAME), "the file's name whole: {row:?}\n{body}");
    // Rung 5: `(ignored)` goes before any word is cut.
    assert_eq!(
        row.trim(),
        format!("\u{2026}/{NAME} is in the way \u{b7} move it aside first \u{b7} esc dismiss"),
        "no word cut: {row:?}\n{body}"
    );
    crate::captures::check_shot("m7/in-the-way-at-80-80x24.txt", &format!("{body}\n"));
}

#[test]
fn a_refusal_at_187_reads_as_before() {
    // A branch short enough that today's whole sentence fits 187 columns:
    // 26's long branch makes it 202 cells, so no rung-1 row exists for it.
    let frame = refused_at("other", Kept::Ignored, 187, 50);
    let row = bottom(&frame);
    assert_eq!(
        row.trim(),
        format!(
            "{DEEP}/{NAME} (ignored) is in the way of switching to other \u{b7} move it aside \
             first \u{b7} esc dismiss"
        ),
        "{}",
        frame.join("\n")
    );
}

#[test]
fn an_untracked_file_in_the_way_names_s() {
    let frame = refused_at(LONG_BRANCH, Kept::Untracked, 80, 24);
    let row = bottom(&frame);
    assert!(
        row.trim_end().ends_with("\u{b7} S stashes it first"),
        "{row:?}\n{}",
        frame.join("\n")
    );
    assert!(!row.contains("esc dismiss"), "{row:?}");
    assert!(row.contains(NAME), "{row:?}");
}
