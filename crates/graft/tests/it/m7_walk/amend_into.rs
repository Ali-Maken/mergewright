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

/// `absorb-staged`: `k` to the uncommitted row, `.`, down to absorb, `⏎`,
/// `y`. Two hunks go into their commits; the README line stays staged.
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
    let mut state = app.key(&state, KeyCode::Char('.'));
    for _ in 0..10 {
        let at_absorb = state.context_menu.as_ref().is_some_and(|open| {
            open.menu.selected() == graft_ui::context_menu::ContextAction::Absorb
        });
        if at_absorb {
            break;
        }
        route.push("j");
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut route);
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let state = y(&app, &state, &mut route);
    // The note allows 4 (`. motion ⏎ y`). Absorb is the menu's last item,
    // after today's five (ticket 09 "Decided"), and the menu does not wrap,
    // so the motion is five `j`: 9 inputs with the `k` that reaches the
    // row. Not held to the note here; the ticket's NOTES carry the
    // conflict for a ruling.
    assert_eq!(route, ["k", ".", "j", "j", "j", "j", "j", "\u{23ce}", "y"]);

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

/// Two of the three captures (rule 6: the user judges them; colour
/// unverified).
/// Right: the put-in box titled `Preview: put your staged changes into
/// <sha7> "Add OAuth callback"` with its `gains` row and `feature-b` marked
/// `built on yours · follows`; after `y`, the cursor on the new `HEAD`,
/// `✎ changed` after "Add OAuth callback" only, and the status
/// `put into "Add OAuth callback" ✓ · u undoes`.
#[test]
fn capture_amend_into() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    stage_auth_fix(repo);
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

    let state = app.resize(&state, 120, 40);
    let state = y(&app, &state, &mut route);
    let state = app.pump(state, "the tag", |state| {
        frame(state, 120, 40).contains("\u{270e} changed")
    });
    // The rewritten commits carry today's committer date (git's own
    // clock, not the fixture's), so their SHAs differ run to run: each is
    // drawn as `#######` in the committed capture. Nothing else is masked.
    let mut body = frame(&state, 120, 40);
    for rewritten in ["feature-a", "feature-a~1", "feature-b"] {
        let sha = read(repo, rewritten);
        body = body.replace(&sha[..7], "#######");
    }
    crate::captures::check_shot("m7/amend-into-landed-120x40.txt", &body);
    assert!(
        body.contains("put into \"Add OAuth callback\" \u{2713} \u{b7} u undoes"),
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
    let mut state = app.key(&state, KeyCode::Char('.'));
    for _ in 0..5 {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut Vec::new());
    let body = frame(&state, 120, 40);
    crate::captures::check_shot("m7/absorb-preview-120x40.txt", &body);
    let placed = body
        .lines()
        .filter(|line| line.contains(" line ") && line.contains("\u{2192} "))
        .count();
    assert_eq!(placed, 2, "{body}");
    assert!(body.contains("stays staged \u{b7} "), "{body}");
    drop(state);
    app.close();
}

/// The dependents check `a` on `HEAD` waits on when another local branch
/// exists (ticket 09 "Decided": under one frame on the fixture). From `a`
/// to the amend editor open — the check, then `HEAD`'s message — seven
/// times, median. Run by `scripts/m7-09-perf.sh` in release; lines start
/// `m7-09`. Ignored: a measurement, not a test.
#[test]
#[ignore = "a measurement: scripts/m7-09-perf.sh"]
fn perf_dependents_check() {
    let with_check = a_to_editor(false);
    // The baseline: `main` deleted, so `feature-a` is the only local branch
    // and `a` is today's amend at once (no check).
    let without = a_to_editor(true);
    for (what, took) in [("with the check", &with_check), ("today's amend", &without)] {
        for each in took {
            println!("m7-09 {what}: {:.1} ms", each.as_secs_f64() * 1e3);
        }
    }
    let (median, base) = (with_check[3], without[3]);
    println!(
        "m7-09 summary: a to the amend editor, median of 7: {:.1} ms through the dependents check, {:.1} ms today's amend; the check adds {:.1} ms (budget 16.7 ms)",
        median.as_secs_f64() * 1e3,
        base.as_secs_f64() * 1e3,
        median.saturating_sub(base).as_secs_f64() * 1e3
    );
}

/// Seven `a`s on `HEAD`, each to the amend editor open and `Esc`, sorted.
fn a_to_editor(only_branch: bool) -> Vec<std::time::Duration> {
    let dir = oauth_repo(false);
    let repo = dir.path();
    if only_branch {
        git(repo, &["branch", "-q", "-D", "main"], 4);
    }
    stage_auth_fix(repo);
    let (app, mut state) = open(repo, 120, 40);
    let mut took = Vec::new();
    for _ in 0..7 {
        let started = std::time::Instant::now();
        let asked = app.key(&state, KeyCode::Char('a'));
        let opened = app.pump(asked, "the amend editor", |state| {
            state.commit_editor.is_some()
        });
        took.push(started.elapsed());
        state = app.key(&opened, KeyCode::Escape);
        assert!(state.commit_editor.is_none(), "Esc closes the editor");
    }
    took.sort();
    drop(state);
    app.close();
    took
}
