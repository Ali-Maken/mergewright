//! Journeys `fix-into-older`, `absorb-staged` and `amend-with-dependents`
//! (m7-power-tools/09): staged changes put into the commit they belong to,
//! through the Ghost Preview and `y`; `u` takes each back whole.
//!
//! The fixtures are built here and dated, so the rows sort the same way on
//! every run. None has a remote: every commit is unpushed (06e carry), so
//! the target of `fix-into-older` is `feature-a`'s "Add OAuth callback".

use std::path::Path;

use graft::update::{AppState, Status};
use graft_core::event::KeyCode;

use crate::app_harness::App;

fn git(repo: &Path, args: &[&str], hour: u8) -> String {
    let date = format!("2020-01-01T{hour:02}:00:00+00:00");
    let out = graft_fixtures::git::run_at(repo, args, &date)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    String::from_utf8(out).unwrap_or_else(|why| panic!("git speaks utf-8 here: {why}"))
}

/// `name` with ten numbered lines, `tag` in each.
fn ten_lines(tag: &str) -> String {
    let lines: Vec<String> = (1..=10).map(|n| format!("{tag} {n}\n")).collect();
    lines.concat()
}

fn write(repo: &Path, path: &str, body: &str) {
    let path = repo.join(path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).unwrap_or_else(|why| panic!("mkdir: {why}"));
    }
    std::fs::write(path, body).unwrap_or_else(|why| panic!("write: {why}"));
}

fn commit(repo: &Path, path: &str, body: &str, subject: &str, hour: u8) {
    write(repo, path, body);
    git(repo, &["add", "-A"], hour);
    git(repo, &["commit", "-q", "-m", subject], hour);
}

fn read(repo: &Path, rev: &str) -> String {
    git(repo, &["rev-parse", rev], 0).trim().to_owned()
}

/// `main` (m1) ← `feature-a` ("Add OAuth callback" adds `src/auth.rs`,
/// "Handle expired tokens" adds `src/tokens.rs`), and with `dependent`,
/// `feature-b` ("Show the login page") built on it. HEAD on `feature-a`.
/// The graph's top row is `feature-b`'s commit when there is one, else
/// `HEAD`'s.
fn oauth_repo(dependent: bool) -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"], 0);
    commit(repo, "README", "readme\n", "m1", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(
        repo,
        "src/auth.rs",
        &ten_lines("auth"),
        "Add OAuth callback",
        1,
    );
    commit(
        repo,
        "src/tokens.rs",
        &ten_lines("tokens"),
        "Handle expired tokens",
        2,
    );
    if dependent {
        git(repo, &["switch", "-q", "-c", "feature-b"], 3);
        commit(
            repo,
            "src/login.rs",
            &ten_lines("login"),
            "Show the login page",
            3,
        );
        git(repo, &["switch", "-q", "feature-a"], 4);
    }
    git(repo, &["commit-graph", "write", "--reachable"], 4);
    dir
}

/// `src/auth.rs` line 5 fixed, staged.
fn stage_auth_fix(repo: &Path) {
    let body = ten_lines("auth").replace("auth 5\n", "auth 5 fixed\n");
    write(repo, "src/auth.rs", &body);
    git(repo, &["add", "src/auth.rs"], 5);
}

/// Line 5 of both of his files fixed, and a line added at the end of
/// `README` (from `main`), all staged. The first two have one commit each;
/// the third no commit on his branch changed.
fn stage_three_hunks(repo: &Path) {
    stage_auth_fix(repo);
    let body = ten_lines("tokens").replace("tokens 5\n", "tokens 5 fixed\n");
    write(repo, "src/tokens.rs", &body);
    write(repo, "README", "readme\nmore\n");
    git(repo, &["add", "-A"], 5);
}

fn open(repo: &Path, columns: u16, rows: u16) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, columns, rows);
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the working tree", |state| {
        state.working_tree.is_some()
    });
    (app, state)
}

fn subject_under_cursor(state: &AppState) -> Option<String> {
    if state.on_working_tree {
        return None;
    }
    let row = state
        .history
        .as_ref()
        .and_then(graft::history::History::selected)?;
    let message = String::from_utf8_lossy(&row.commit.message).into_owned();
    message.lines().next().map(str::to_owned)
}

/// The cursor down to the row whose subject is `subject`, a key at a time.
fn cursor_to(app: &App, state: AppState, subject: &str, route: &mut Vec<&'static str>) -> AppState {
    let mut state = state;
    for _ in 0..10 {
        if subject_under_cursor(&state).as_deref() == Some(subject) {
            return state;
        }
        route.push("j");
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("never reached {subject:?}")
}

/// A key that plans a put-in, and the plan's answer: the preview drawn, or
/// the words refusing it.
fn planned(
    app: &App,
    state: &AppState,
    key: KeyCode,
    name: &'static str,
    route: &mut Vec<&'static str>,
) -> AppState {
    route.push(name);
    let state = app.key(state, key);
    app.pump(state, "the put-in's plan", |state| {
        let drawn = state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty());
        drawn || matches!(state.status, Status::Sticky { .. }) || state.commit_editor.is_some()
    })
}

/// `y`, run to its end, and the tag read after it.
fn y(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("y");
    let state = app.key(state, KeyCode::Char('y'));
    let state = app.settle_write(state, "the put-in");
    app.pump(state, "the refresh after the put-in", |state| {
        state.refs.is_some() && state.working_tree.is_some()
    })
}

/// `u`, run to its end.
fn u(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('u'));
    app.settle_write(state, "the undo")
}

fn frame(state: &AppState, columns: u16, rows: u16) -> String {
    let lines = crate::captures::frame_at(
        state,
        columns,
        rows,
        Some(graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    );
    format!("{}\n", lines.join("\n"))
}

fn within(journey: &str, route: &[&str]) {
    let allowed = super::noted_keys(journey);
    assert!(
        route.len() <= allowed,
        "{journey} took {} inputs {route:?}, the note allows {allowed}",
        route.len()
    );
}

fn refs(repo: &Path) -> String {
    git(
        repo,
        &[
            "for-each-ref",
            "--format=%(refname) %(objectname)",
            "refs/heads",
        ],
        0,
    )
}

fn staged(repo: &Path) -> String {
    git(repo, &["diff", "--cached", "--no-ext-diff"], 0)
}

/// `fix-into-older`: the cursor to "Add OAuth callback" (one row below
/// `HEAD`, the top row), `a`, `y`. The commit's diff now has the fix,
/// nothing is staged, and "Handle expired tokens" was replayed on it. `u`
/// puts refs and index back.
#[test]
fn fix_into_older() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_auth_fix(repo);
    let (before_refs, before_index) = (refs(repo), staged(repo));
    let (app, state) = open(repo, 187, 50);
    let mut route = Vec::new();
    let state = cursor_to(&app, state, "Add OAuth callback", &mut route);
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut route);
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let state = y(&app, &state, &mut route);
    within("fix-into-older", &route);

    let shown = git(repo, &["show", "feature-a~1", "--", "src/auth.rs"], 0);
    assert!(shown.contains("+auth 5 fixed"), "{shown}");
    assert_eq!(staged(repo), "", "nothing is staged");
    let replayed = git(repo, &["log", "-1", "--format=%s", "feature-a"], 0);
    assert_eq!(replayed.trim(), "Handle expired tokens");

    let state = u(&app, &state);
    assert_eq!(refs(repo), before_refs, "u puts every branch back");
    assert_eq!(staged(repo), before_index, "u puts the index back");
    drop(state);
    app.close();
}

/// `absorb-staged`: `k` to the uncommitted row, `.` (the menu opens on
/// absorb), `⏎`, `y`. Two hunks go into their commits; the README line stays staged.
/// `u` puts it all back.
#[test]
fn absorb_staged() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_three_hunks(repo);
    let (before_refs, before_index) = (refs(repo), staged(repo));
    let (app, state) = open(repo, 120, 40);
    let state = app.key(&state, KeyCode::Char('k'));
    assert!(
        state.on_working_tree,
        "`k` from the top row: the uncommitted row"
    );
    let mut route = vec!["k", "."];
    let state = app.key(&state, KeyCode::Char('.'));
    // 09b ruling B: absorb is the menu's first item and the cursor opens
    // on it — no motion inside the menu.
    assert!(
        state.context_menu.as_ref().is_some_and(|open| {
            open.menu.selected() == graft_ui::context_menu::ContextAction::Absorb
        }),
        "the menu opens on absorb"
    );
    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut route);
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let state = y(&app, &state, &mut route);
    assert_eq!(route, ["k", ".", "\u{23ce}", "y"]);
    within("absorb-staged", &route);

    let auth = git(repo, &["show", "feature-a~1", "--", "src/auth.rs"], 0);
    assert!(auth.contains("+auth 5 fixed"), "{auth}");
    let tokens = git(repo, &["show", "feature-a", "--", "src/tokens.rs"], 0);
    assert!(tokens.contains("+tokens 5 fixed"), "{tokens}");
    let still = staged(repo);
    assert!(
        still.contains("+more"),
        "the README line stays staged: {still}"
    );
    assert!(!still.contains("fixed"), "{still}");

    let state = u(&app, &state);
    assert_eq!(refs(repo), before_refs, "u puts every branch back");
    assert_eq!(staged(repo), before_index, "u puts the index back");
    drop(state);
    app.close();
}

/// `amend-with-dependents`: from the changes pane, where he staged, `a`
/// with `feature-b` built on his branch opens the preview rather than the
/// editor; `y` amends `HEAD` and `feature-b` follows. `u` puts it back.
#[test]
fn amend_with_dependents() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    let body = ten_lines("tokens").replace("tokens 9\n", "tokens 9 fixed\n");
    write(repo, "src/tokens.rs", &body);
    git(repo, &["add", "-A"], 5);
    let (before_refs, before_index) = (refs(repo), staged(repo));
    let (app, state) = open(repo, 120, 40);
    let state = crate::app_harness::changes_pane(&app, state);
    assert!(state.on_changes_pane());
    let mut route = Vec::new();
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut route);
    assert!(state.commit_editor.is_none(), "not the editor");
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let state = y(&app, &state, &mut route);
    within("amend-with-dependents", &route);

    let head = git(repo, &["show", "feature-a", "--", "src/tokens.rs"], 0);
    assert!(head.contains("+tokens 9 fixed"), "{head}");
    assert_eq!(
        read(repo, "feature-b~1"),
        read(repo, "feature-a"),
        "feature-b followed"
    );
    assert_eq!(staged(repo), "");

    let state = u(&app, &state);
    assert_eq!(refs(repo), before_refs);
    assert_eq!(staged(repo), before_index);
    drop(state);
    app.close();
}

/// `reword-head` (m8-polish-and-ship/14, J19): `a` on `HEAD` with
/// `feature-b` built on it and nothing staged opens the commit editor on
/// `HEAD`'s message (not the `nothing staged` refusal); the edit, `⏎`, and
/// the preview's `y` reword `HEAD` and `feature-b` follows. `u` puts both
/// back. Truth read with git.
///
/// Counted as the M8 note counts it (`a <edit> ⏎ y`, 4 — note "Pass 2c"):
/// the typing is the edit, the preview's `y` the fourth.
#[test]
fn reword_head_with_a_line_built_on_it() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    let before_refs = refs(repo);
    let tree = git(repo, &["rev-parse", "feature-a^{tree}"], 0);
    let (app, state) = open(repo, 120, 40);
    let state = cursor_to(&app, state, "Handle expired tokens", &mut Vec::new());
    let mut route = Vec::new();
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut route);
    assert!(state.preview.is_none(), "not a preview yet");
    let editor = state
        .commit_editor
        .as_ref()
        .unwrap_or_else(|| panic!("the editor: {:?}", state.status));
    assert_eq!(editor.subject.text(), "Handle expired tokens");
    route.push("<edit>");
    let mut state = state;
    for typed in " on refresh".chars() {
        state = app.key(&state, KeyCode::Char(typed));
    }
    let state = planned(&app, &state, KeyCode::Enter, "⏎", &mut route);
    assert!(state.commit_editor.is_none(), "the editor closed");
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    capture_reword_preview(&state);
    let state = y(&app, &state, &mut route);
    crate::m8_walk::within_the_note("reword-head", &route);

    assert_eq!(
        git(repo, &["log", "-1", "--format=%s", "feature-a"], 0).trim(),
        "Handle expired tokens on refresh"
    );
    assert_eq!(
        git(repo, &["rev-parse", "feature-a^{tree}"], 0),
        tree,
        "only the message changed"
    );
    assert_eq!(
        read(repo, "feature-b~1"),
        read(repo, "feature-a"),
        "feature-b followed"
    );
    assert_eq!(
        git(repo, &["symbolic-ref", "HEAD"], 0).trim(),
        "refs/heads/feature-a"
    );

    let state = u(&app, &state);
    assert_eq!(refs(repo), before_refs, "u puts both back");
    drop(state);
    app.close();
}

/// Review m8-14 Major 4: the reword box is on screen, not only in the state.
fn capture_reword_preview(state: &AppState) {
    let body = frame(state, 120, 40);
    assert!(
        body.contains("reword ") && body.contains("\"Handle expired tokens\""),
        "{body}"
    );
    assert!(
        body.contains("new message  Handle expired tokens on refresh"),
        "{body}"
    );
    assert!(body.contains("follows"), "feature-b follows: {body}");
    crate::captures::check_shot(
        "m8/reword-preview-120x40.txt",
        &format!("{REWORD_HEADER}\n{body}"),
    );
}

const REWORD_HEADER: &str = "\
# m8/reword-preview-120x40.txt: drawn by `view::view` from a real AppState
# (crates/graft/tests/it/m7_walk/amend_into.rs,
# reword_head_with_a_line_built_on_it, asserted). Not a pty capture: a
# character grid with no colour (CLAUDE.md rule 5, colour unverified).
#
# Right: `a` on HEAD with nothing staged, ` on refresh` typed, `⏎` — a box
# titled `Preview: reword <sha7> \"Handle expired tokens\"`, the row
# `<sha7>  new message  Handle expired tokens on refresh`, feature-b marked
# as following, `y` on the box edge. Nothing written yet.
";

/// Two of the three captures (rule 6: the user judges them; colour
/// unverified).
/// Right: the put-in box titled `Preview: put your staged changes into
/// <sha7> "Add OAuth callback"` with its `gains` row and `feature-b` marked
/// `built on yours · follows`; after `y`, the cursor on the new `HEAD`,
/// `✎ changed` after "Add OAuth callback" only, and the status
/// `put into "Add OAuth callback" ✓ · your unstaged edits are back · u
/// undoes` (09b Q3: `src/tokens.rs` is edited and not staged). The box
/// says one thing about his edits (09b Q4) and `undo: u takes this back`.
///
/// Run in a child of this test binary with git's own clock pinned
/// (`GIT_COMMITTER_DATE`, inherited by every git the app spawns), so the
/// rewritten commits are dated before `CORPUS_NOW` like the rest of the
/// fixture — not today, which the fixed clock draws as `futu…` (review
/// m7-09..21c Minor 7) — and their SHAs are the same every run.
#[test]
fn capture_amend_into() {
    let status = std::process::Command::new(std::env::current_exe().expect("the test binary"))
        .args([
            "--exact",
            "m7_walk::amend_into::capture_amend_into_git_clock_pinned",
            "--ignored",
            "--nocapture",
            "--test-threads=1",
        ])
        .env(PINNED_CHILD, "1")
        .env("GIT_COMMITTER_DATE", PINNED_COMMITTER_DATE)
        .status()
        .expect("the child runs");
    assert!(
        status.success(),
        "the pinned-clock capture failed: {status:?}"
    );
}

/// Set for [`capture_amend_into_git_clock_pinned`] by its parent.
const PINNED_CHILD: &str = "TRIB_M7_AMEND_INTO_PINNED_CHILD";

/// The committer date the rewrite's commits get: an hour after the
/// fixture's last (`stage_auth_fix`, hour 5), before `CORPUS_NOW`.
const PINNED_COMMITTER_DATE: &str = "2020-01-01T06:00:00+00:00";

/// The child half of [`capture_amend_into`]. Does nothing when run on its
/// own.
#[test]
#[ignore = "the child of capture_amend_into; runs only under it, git's clock pinned"]
fn capture_amend_into_git_clock_pinned() {
    if std::env::var_os(PINNED_CHILD).is_none() {
        return;
    }
    let dir = oauth_repo(true);
    let repo = dir.path();
    // The rewrite's commits carry a committer, and the capture draws their
    // ids: the developer's own identity there made the ids per machine.
    crate::app_harness::pin_identity(repo);
    stage_auth_fix(repo);
    write(
        repo,
        "src/tokens.rs",
        &ten_lines("tokens").replace("tokens 9\n", "tokens 9 edited\n"),
    );
    let (app, state) = open(repo, 187, 50);
    let mut route = Vec::new();
    let state = cursor_to(&app, state, "Add OAuth callback", &mut route);
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut route);
    let body = frame(&state, 187, 50);
    crate::captures::check_shot("m7/amend-into-preview-187x50.txt", &body);
    assert!(
        body.contains("put your staged changes into ") && body.contains("\"Add OAuth callback\""),
        "{body}"
    );
    assert!(body.contains("gains  src/auth.rs +1 \u{2212}1"), "{body}");
    assert!(body.contains("built on yours \u{b7} follows"), "{body}");
    assert_edits_rows_agree(&body);
    assert!(body.contains("undo: u takes this back"), "{body}");

    let state = app.resize(&state, 120, 40);
    let state = y(&app, &state, &mut route);
    let state = app.pump(state, "the tag", |state| {
        frame(state, 120, 40).contains("\u{270e} changed")
    });
    // git's clock is pinned (above), so the rewritten commits' dates and
    // SHAs are the same every run: nothing is masked.
    let body = frame(&state, 120, 40);
    crate::captures::check_shot("m7/amend-into-landed-120x40.txt", &body);
    assert!(
        !body.contains("futu"),
        "a rewritten commit dated ahead: {body}"
    );
    assert!(
        body.contains(
            "put into \"Add OAuth callback\" \u{2713} \u{b7} your unstaged edits are back \u{b7} u undoes"
        ),
        "{body}"
    );
    assert_eq!(body.matches("\u{270e} changed").count(), 1, "{body}");
    assert_eq!(
        subject_under_cursor(&state).as_deref(),
        Some("Handle expired tokens"),
        "the cursor lands on the new HEAD"
    );
    let tagged = body
        .lines()
        .find(|line| line.contains("\u{270e} changed"))
        .expect("a tagged row");
    assert!(tagged.contains("Add OAuth callback"), "{tagged}");
    drop(state);
    app.close();
}

/// The absorb box (rule 6; colour unverified). Right: two `→ <sha>` rows
/// and one `stays staged · no commit on your branch changed these lines`.
#[test]
fn capture_absorb() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_three_hunks(repo);
    let (app, state) = open(repo, 120, 40);
    let state = app.key(&state, KeyCode::Char('k'));
    let state = app.key(&state, KeyCode::Char('.'));
    // 09b: the menu, the cursor on absorb, first; the old five below.
    let menu = frame(&state, 120, 40);
    crate::captures::check_shot("m7/absorb-menu-120x40.txt", &menu);
    let items: Vec<&str> = menu
        .lines()
        .filter(|line| {
            [
                "Put staged changes into their commits (absorb)",
                "Open details (\u{23ce})",
                "Fetch (f)",
                "Pull (p)",
                "Push (P)",
                "Undo (u)",
            ]
            .iter()
            .any(|label| line.contains(label))
        })
        .collect();
    assert_eq!(items.len(), 6, "{menu}");
    assert!(items[0].contains("(absorb)"), "{menu}");
    assert!(items[1].contains("Open details"), "{menu}");
    assert!(items[5].contains("Undo (u)"), "{menu}");

    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut Vec::new());
    let body = frame(&state, 120, 40);
    crate::captures::check_shot("m7/absorb-preview-120x40.txt", &body);
    let placed = body
        .lines()
        .filter(|line| line.contains(" line ") && line.contains("\u{2192} "))
        .count();
    assert_eq!(placed, 2, "{body}");
    assert!(body.contains("stays staged \u{b7} "), "{body}");
    assert!(body.contains("undo: u takes this back"), "{body}");
    drop(state);
    app.close();
}

/// The box's statements about his edits, as one: every line that says
/// where his edits go.
fn edits_rows(body: &str) -> Vec<String> {
    body.lines()
        .filter(|line| line.contains("your edits"))
        .map(|line| line.trim_matches(['\u{2502}', ' ']).to_owned())
        .collect()
}

/// 09b Q4: a box that says the edits are set aside and put back never
/// also says nothing is put back (`preview/put_back.rs`'s `landing_of`
/// had no rewrite arm). With one unstaged file, the count is that one file:
/// the staged edits go into commits, and only the unstaged come back, as
/// the landing says (review m7-09..21c Minor 6).
fn assert_edits_rows_agree(body: &str) {
    let rows = edits_rows(body);
    assert!(
        rows.iter()
            .any(|row| row.contains("your edits: 1 file set aside, put back after")),
        "{body}"
    );
    assert!(
        !body.contains("nothing is put back"),
        "the rows disagree: {rows:?}"
    );
}

/// `src/tokens.rs` line 9 edited and not staged.
fn edit_tokens_unstaged(repo: &Path) {
    let body = std::fs::read_to_string(repo.join("src/tokens.rs"))
        .unwrap_or_else(|why| panic!("read: {why}"));
    write(
        repo,
        "src/tokens.rs",
        &body.replace("tokens 9\n", "tokens 9 edited\n"),
    );
}

/// The put-in's box, `a` on "Add OAuth callback".
fn put_in_box(repo: &Path) -> String {
    let (app, state) = open(repo, 187, 50);
    let state = cursor_to(&app, state, "Add OAuth callback", &mut Vec::new());
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut Vec::new());
    let body = frame(&state, 187, 50);
    drop(state);
    app.close();
    body
}

/// The absorb box, from the uncommitted row's `.` menu.
fn absorb_box(repo: &Path) -> String {
    let (app, state) = open(repo, 120, 40);
    let state = app.key(&state, KeyCode::Char('k'));
    let state = app.key(&state, KeyCode::Char('.'));
    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut Vec::new());
    let body = frame(&state, 120, 40);
    drop(state);
    app.close();
    body
}

#[test]
fn the_edits_rows_agree_in_the_put_in_preview() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    stage_auth_fix(repo);
    edit_tokens_unstaged(repo);
    assert_edits_rows_agree(&put_in_box(repo));
}

#[test]
fn the_edits_rows_agree_in_the_absorb_preview() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_three_hunks(repo);
    edit_tokens_unstaged(repo);
    assert_edits_rows_agree(&absorb_box(repo));
}

/// Nothing unstaged: nothing of his is put back, and the box has no
/// set-aside row (review m7-09..21c Minor 6: it used to count the staged
/// files the put-in puts into commits).
#[test]
fn a_put_in_with_nothing_unstaged_has_no_set_aside_row() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    stage_auth_fix(repo);
    let body = put_in_box(repo);
    assert!(body.contains("gains  src/auth.rs"), "{body}");
    assert!(!body.contains("set aside"), "{body}");

    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_three_hunks(repo);
    let body = absorb_box(repo);
    assert!(body.contains("stays staged"), "{body}");
    assert!(!body.contains("set aside"), "{body}");
}

/// 09b Q4, the truth by the write: a put-in over an unstaged edit puts
/// the file back byte for byte, still unstaged; the staged fix is in the
/// commit, so nothing stays staged (ADR-0045).
#[test]
fn a_put_in_puts_the_unstaged_file_back() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_auth_fix(repo);
    let edited = ten_lines("tokens").replace("tokens 9\n", "tokens 9 edited\n");
    write(repo, "src/tokens.rs", &edited);
    let (app, state) = open(repo, 120, 40);
    let state = cursor_to(&app, state, "Add OAuth callback", &mut Vec::new());
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut Vec::new());
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let state = y(&app, &state, &mut Vec::new());
    let on_disk = std::fs::read_to_string(repo.join("src/tokens.rs"))
        .unwrap_or_else(|why| panic!("read: {why}"));
    assert_eq!(on_disk, edited, "the unstaged edit is back, byte for byte");
    assert_eq!(staged(repo), "", "nothing stays staged");
    let unstaged = git(repo, &["diff", "--no-ext-diff", "--name-only"], 0);
    assert_eq!(unstaged.trim(), "src/tokens.rs");
    let shown = git(repo, &["show", "feature-a~1", "--", "src/auth.rs"], 0);
    assert!(shown.contains("+auth 5 fixed"), "{shown}");
    drop(state);
    app.close();
}

/// 09b A2(a)'s candidate, refuted and pinned: with nothing built on his
/// branch, `a` on `HEAD` answers from the dependents check alone
/// (`jobs/rewrite.rs::plan` returns before `rewrite::plan`), so no
/// `write-tree`/`commit-tree` object is made on the way to the editor.
#[test]
fn a_on_head_with_nothing_built_on_it_writes_no_object() {
    let dir = perf_repo(Fixture::Spike);
    let objects = || {
        let counts = git(dir.path(), &["count-objects", "-v"], 0);
        counts
            .lines()
            .filter(|line| line.starts_with("count:") || line.starts_with("in-pack:"))
            .map(|line| line.split(':').nth(1).unwrap_or("").trim().to_owned())
            .collect::<Vec<_>>()
    };
    let before = objects();
    let (app, state) = open_on_head(dir.path());
    let asked = app.key(&state, KeyCode::Char('a'));
    let opened = app.pump(asked, "the amend editor", |state| {
        state.commit_editor.is_some() || matches!(state.status, Status::Sticky { .. })
    });
    assert!(opened.commit_editor.is_some(), "{:?}", opened.status);
    assert_eq!(objects(), before, "loose and packed object counts");
    drop(opened);
    app.close();
}

/// The shapes the put-in check is measured on ([`perf`]); `Spike` also
/// serves the fast test above.
#[derive(Clone, Copy)]
enum Fixture {
    /// `main`, `feature-a` (HEAD) and `spike`, off `main` with its own
    /// commit: the check runs and finds nothing built on `feature-a`.
    Spike,
    /// `feature-a` alone: no check.
    #[cfg_attr(
        not(feature = "heavy-tests"),
        allow(dead_code, reason = "built only by the heavy tier's measurements")
    )]
    OnlyBranch,
    /// `Spike` plus 300 branches at `spike`.
    #[cfg_attr(
        not(feature = "heavy-tests"),
        allow(dead_code, reason = "built only by the heavy tier's measurements")
    )]
    Wide,
}

/// The oauth repo, shaped as `fixture` says, one file staged.
fn perf_repo(fixture: Fixture) -> tempfile::TempDir {
    let dir = oauth_repo(false);
    let repo = dir.path();
    match fixture {
        Fixture::OnlyBranch => {
            git(repo, &["branch", "-q", "-D", "main"], 4);
        }
        Fixture::Spike | Fixture::Wide => {
            git(repo, &["switch", "-q", "-c", "spike", "main"], 4);
            commit(repo, "SPIKE", "spike\n", "Try a spike", 4);
            git(repo, &["switch", "-q", "feature-a"], 4);
        }
    }
    if let Fixture::Wide = fixture {
        // Loose ref files: one write each, no git process per branch.
        let spike = read(repo, "spike");
        for n in 0..300 {
            let path = repo.join(format!(".git/refs/heads/unrelated-{n:03}"));
            std::fs::write(path, format!("{spike}\n")).unwrap_or_else(|why| panic!("write: {why}"));
        }
    }
    stage_auth_fix(repo);
    dir
}

/// Review m8-14 Major 2: a reword the plan would refuse is refused at `a`,
/// before he types anything — not after `⏎`, when his text would be lost.
/// `HEAD` is the first commit, `feature-b` built on it, nothing staged.
#[test]
fn a_reword_that_cannot_run_is_refused_at_a() {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "feature-a"], 0);
    commit(repo, "README", "readme\n", "The first commit", 0);
    git(repo, &["switch", "-q", "-c", "feature-b"], 1);
    commit(
        repo,
        "src/login.rs",
        &ten_lines("login"),
        "Show the login page",
        1,
    );
    git(repo, &["switch", "-q", "feature-a"], 2);
    let (app, state) = open(repo, 120, 40);
    let state = cursor_to(&app, state, "The first commit", &mut Vec::new());
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut Vec::new());
    assert!(
        state.commit_editor.is_none(),
        "no editor to type into: {:?}",
        state.status
    );
    let Status::Sticky { reason, .. } = &state.status else {
        panic!("a refusal in words: {:?}", state.status);
    };
    assert!(reason.contains("is the first commit"), "{reason}");
    drop(state);
    app.close();
}

/// Opened with the cursor on `HEAD`'s row (`spike`'s newer commit is the
/// top row when it exists).
fn open_on_head(repo: &Path) -> (App, AppState) {
    let (app, state) = open(repo, 120, 40);
    let state = cursor_to(&app, state, "Handle expired tokens", &mut Vec::new());
    (app, state)
}

/// The put-in check's measurements: the heavy tier (review m7-09..21c
/// Minor 12), so every review runs them; `scripts/m7-09-perf.sh` runs them
/// in release.
#[cfg(feature = "heavy-tests")]
mod perf;
