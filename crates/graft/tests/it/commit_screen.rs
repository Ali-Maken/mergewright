//! The commit screen over the real job pool and real `git` (ease-pass/08,
//! the commit mock): the journeys the design note counts, the landing
//! (C13), `u` giving the message back (Q9 a), and sign-off reaching the
//! commit (Q8 a). The rest of the screen is pure and tested in
//! `update::commit_editor`.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::event::{KeyCode, KeyPress, Modifiers};
use graft_ui::graph_screen::hints::{HintContext, RowBar};

use crate::app_harness::{App, git, pin_identity};

mod rows;
mod shots;

/// A repository on `main` with `docs/toc.md`, `README.md` and `src/a.rs`
/// committed and pushed to `origin/main`, all three edited since, and a new
/// `.env` beside them — the mock's working tree.
pub(crate) fn three_changed_and_a_secret() -> tempfile::TempDir {
    let scratch = tempfile::TempDir::with_prefix("mwr-commit-screen-").expect("a temp directory");
    let repo = scratch.path();
    git(
        repo,
        &["init", "-q", "-b", "main", "--object-format=sha1", "."],
    );
    pin_identity(repo);
    std::fs::create_dir_all(repo.join("docs")).expect("writable");
    std::fs::create_dir_all(repo.join("src")).expect("writable");
    for (path, text) in [
        ("docs/toc.md", "# Contents\n"),
        ("README.md", "readme\n"),
        ("src/a.rs", "fn a() {}\n"),
    ] {
        std::fs::write(repo.join(path), text).expect("writable");
    }
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", "start"]);
    // An upstream level with `main`, so a new commit is one to push.
    git(
        repo,
        &["remote", "add", "origin", "https://example.invalid/r.git"],
    );
    git(repo, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
    git(repo, &["config", "branch.main.remote", "origin"]);
    git(repo, &["config", "branch.main.merge", "refs/heads/main"]);
    for (path, text) in [
        ("docs/toc.md", "# Contents\n\n- one\n"),
        ("README.md", "readme\nmore\n"),
        ("src/a.rs", "fn a() {}\nfn b() {}\n"),
        (".env", "TOKEN=x\n"),
    ] {
        std::fs::write(repo.join(path), text).expect("writable");
    }
    scratch
}

/// Type `text` into whatever has focus, one key per character.
pub(crate) fn typed(app: &App, state: &AppState, text: &str) -> AppState {
    text.chars()
        .fold(state.clone(), |state, c| app.key(&state, KeyCode::Char(c)))
}

/// Open, and wait until the rows, the working tree and the refs are in.
pub(crate) fn ready(app: &App, state: AppState) -> AppState {
    app.pump(state, "the rows, the working tree and the refs", |state| {
        state.working_tree.is_some()
            && state.refs.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    })
}

/// `c`, then wait for the line counts and the first file's diff.
pub(crate) fn opened(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('c'));
    assert!(state.commit_editor.is_some(), "`c` opens the screen");
    app.pump(state, "the counts and the diff", |state| {
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.counts.is_some() && editor.diff.view.is_some())
            && state.commit_template.is_none()
    })
}

/// The files `HEAD` changed, sorted.
pub(crate) fn committed_files(repo: &Path) -> Vec<String> {
    let mut files: Vec<String> = git(repo, &["show", "--name-only", "--format=", "HEAD"])
        .lines()
        .map(str::to_owned)
        .collect();
    files.sort();
    files
}

/// The commit has landed and its row is the cursor's, with the graph re-read.
pub(crate) fn landed(app: &App, state: AppState) -> AppState {
    let state = app.settle_write(state, "the commit");
    app.pump(state, "the graph on the new commit", |state| {
        let head = state.refs.as_deref().and_then(graft::refs::RefBadges::head);
        head.is_some()
            && state.last_commit.as_ref().map(|last| last.commit) == head
            && graft::view::graph_view(state, 0).push.is_some()
            && state
                .history
                .as_ref()
                .and_then(graft::history::History::selected)
                .is_some_and(|row| Some(row.commit.id) == head)
    })
}

fn bar(state: &AppState) -> HintContext {
    graft::view::graph_view(state, 0).hint_context
}

fn is_just_committed(context: &HintContext) -> bool {
    match context {
        HintContext::GraphRow { row, .. } => *row == RowBar::JustCommitted,
        HintContext::Marked(inner) | HintContext::Rewritable(inner) => is_just_committed(inner),
        _ => false,
    }
}

/// Journey `commit-all-subject` (3 keys: `c`, the message, `⏎`) and its
/// landing (C13): every change but `.env` goes in, the cursor rests on the
/// new commit, the status names it, the bar is `JustCommitted`, the top
/// line counts it to push; `j` gives the row's own bar, `k` brings it back.
#[test]
fn commit_all_subject_is_three_keys_and_lands_on_the_commit() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let mut keys = 0;
    let state = opened(&app, &state);
    keys += 1;
    let state = typed(&app, &state, "fix toc");
    keys += 1;
    let state = app.key(&state, KeyCode::Enter);
    keys += 1;
    assert_eq!(keys, 3, "the note's count");
    let state = landed(&app, state);

    assert_eq!(
        committed_files(repo),
        ["README.md", "docs/toc.md", "src/a.rs"],
        "all three, `.env` left out"
    );
    assert!(repo.join(".env").exists(), "the secret is untouched");
    let Status::Done { what, keys, .. } = &state.status else {
        panic!("{:?}", state.status);
    };
    assert_eq!(what, "Committed 3 files to main.");
    assert_eq!(keys.as_deref(), Some("u undo · P push"));
    assert!(is_just_committed(&bar(&state)), "{:?}", bar(&state));
    assert_eq!(
        graft::view::graph_view(&state, 0)
            .push
            .map(|words| words.as_str().to_owned())
            .as_deref(),
        Some("1 commit to push")
    );

    let down = app.key(&state, KeyCode::Char('j'));
    assert!(!is_just_committed(&bar(&down)), "{:?}", bar(&down));
    let back = app.key(&down, KeyCode::Char('k'));
    assert!(is_just_committed(&bar(&back)), "{:?}", bar(&back));
    app.close();
}

/// Q9 a: `u` after the commit takes it back, the edits uncommitted, and the
/// next `c` opens with its message and the files chosen afresh.
#[test]
fn u_after_the_commit_gives_its_message_to_the_next_c() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let before = git(repo, &["rev-parse", "HEAD"]);
    let state = opened(&app, &state);
    let state = typed(&app, &state, "fix toc");
    let state = landed(&app, app.key(&state, KeyCode::Enter));

    let undone = app.settle_write(app.key(&state, KeyCode::Char('u')), "the undo");
    assert_eq!(git(repo, &["rev-parse", "HEAD"]), before, "HEAD is back");
    assert!(undone.last_commit.is_none());
    let status = git(repo, &["status", "--porcelain"]);
    assert!(
        status.contains(" M docs/toc.md"),
        "edits uncommitted: {status}"
    );

    let undone = app.pump(undone, "the working tree re-read", |state| {
        state
            .working_tree
            .as_deref()
            .is_some_and(|tree| tree.entries.len() == 4)
    });
    let reopened = opened(&app, &undone);
    let editor = reopened.commit_editor.as_ref().expect("open");
    assert_eq!(editor.subject.text(), "fix toc");
    assert_eq!(editor.going_in(), 3, "chosen afresh, `.env` out");
    app.close();
}

/// Q8 a: `⌃S` on → the header says so, and the commit carries
/// `Signed-off-by:`.
#[test]
fn a_commit_made_with_sign_off_on_carries_the_trailer() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let state = opened(&app, &state);
    let state = typed(&app, &state, "fix toc");
    let state = app.press(
        &state,
        KeyPress {
            code: KeyCode::Char('s'),
            modifiers: Modifiers {
                control: true,
                ..Modifiers::NONE
            },
        },
    );
    let view = graft::view::commit_screen_view(&state).expect("open");
    assert_eq!(view.header.options, ["signing off"]);
    let state = app.key(&state, KeyCode::Enter);
    let _landed: AppState = app.settle_write(state, "the commit");
    let message = git(repo, &["log", "-1", "--format=%B"]);
    assert!(
        message.contains("Signed-off-by: Graft Test <test@example.invalid>"),
        "{message}"
    );
    app.close();
}

/// C13: an amend is not a commit `u` gives back as a message — its landing
/// earns the row's own bar, not `JustCommitted`.
#[test]
fn after_an_amend_lands_the_bar_is_not_just_committed() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    let (app, state) = App::open(repo);
    let state = ready(&app, state);
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.pump(state, "the amend screen", |state| {
        state.commit_editor.is_some()
    });
    let state = app.settle_write(app.key(&state, KeyCode::Enter), "the amend");
    assert!(state.last_commit.is_none());
    assert!(!is_just_committed(&bar(&state)), "{:?}", bar(&state));
    app.close();
}
