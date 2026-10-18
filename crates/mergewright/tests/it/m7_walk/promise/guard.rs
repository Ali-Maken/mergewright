//! Review 28 Q2 (carried into m7-acceptance): the ignored-file guard's
//! refusal at 80 columns, with a long branch name and a deep path. **Right:**
//! the file's name and the next step (`move it aside first`) are both on the
//! status line.

use mergewright::update::{AppState, Status};
use mergewright_core::event::KeyCode;

use crate::app_harness::{App, git};

const BRANCH: &str = "feature/sara-long-running-oauth-token-refresh-rework";
const DEEP: &str = "src/services/authentication/providers/oauth2/callbacks";
const NAME: &str = "session-cache.json";

/// `main` with `a.txt`; `BRANCH` tracks `DEEP/NAME`. Back on `main`, the
/// same path is ignored (`.git/info/exclude`) and on disk: switching would
/// overwrite it.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let path = dir.path();
    let write = |name: &str, text: &str| {
        let file = path.join(name);
        if let Some(parent) = file.parent() {
            std::fs::create_dir_all(parent).unwrap_or_else(|why| panic!("{why}"));
        }
        std::fs::write(file, text).unwrap_or_else(|why| panic!("{why}"));
    };
    git(path, &["init", "-q", "-b", "main"]);
    write("a.txt", "a\n");
    git(path, &["add", "-A"]);
    git(path, &["commit", "-q", "-m", "one"]);
    git(path, &["switch", "-q", "-c", BRANCH]);
    write(&format!("{DEEP}/{NAME}"), "theirs\n");
    git(path, &["add", "-A"]);
    git(path, &["commit", "-q", "-m", "two on the branch"]);
    git(path, &["switch", "-q", "main"]);
    write(".git/info/exclude", &format!("{NAME}\n"));
    write(&format!("{DEEP}/{NAME}"), "mine\n");
    assert_eq!(
        git(path, &["status", "--porcelain"]),
        "",
        "status lists nothing"
    );
    git(path, &["commit-graph", "write", "--reachable"]);
    dir
}

/// `C`, the branch's name typed, `⏎`: the switch, refused.
fn switch(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('C'));
    let mut state = app.pump(state, "the switch list", |state| {
        state
            .picker
            .as_ref()
            .is_some_and(|picker| !picker.rows().is_empty())
    });
    for character in BRANCH.chars() {
        state = app.key(&state, KeyCode::Char(character));
    }
    let state = app.key(&state, KeyCode::Enter);
    app.pump(state, "the refusal", |state| {
        state.operating.is_none() && matches!(state.status, Status::Sticky { .. })
    })
}

#[test]
fn guard_refusal_at_80_names_the_file_and_the_next_step() {
    let dir = repo();
    let path = dir.path();
    let (app, state) = App::open(path);
    let state = app.resize(&state, 80, 24);
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the first status read", |state| {
        state.working_tree.is_some() && state.status_read.is_none()
    });
    let state = switch(&app, &state);
    assert_eq!(
        git(path, &["symbolic-ref", "HEAD"]).trim(),
        "refs/heads/main",
        "refused: still on main"
    );
    let frame = crate::captures::frame_at(&state, 80, 24, None);
    let body = frame.join("\n");
    let status_line = frame.last().map_or("", String::as_str);
    assert!(
        status_line.contains(NAME),
        "the file's name on the status line: {status_line:?}\n{body}"
    );
    assert!(
        status_line.contains("move it aside first"),
        "the next step on the status line: {status_line:?}\n{body}"
    );
    drop(state);
    app.close();
}
