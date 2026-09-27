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

/// Two of the three captures (rule 6: the user judges them; colour
/// unverified).
/// Right: the put-in box titled `Preview: put your staged changes into
/// <sha7> "Add OAuth callback"` with its `gains` row and `feature-b` marked
/// `built on yours · follows`; after `y`, the cursor on the new `HEAD`,
/// `✎ changed` after "Add OAuth callback" only, and the status
/// `put into "Add OAuth callback" ✓ · your unstaged edits are back · u
/// undoes` (09b Q3: `src/tokens.rs` is edited and not staged). The box
/// says one thing about his edits (09b Q4) and `undo: u takes this back`.
#[test]
fn capture_amend_into() {
    let dir = oauth_repo(true);
    let repo = dir.path();
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
/// had no rewrite arm).
fn assert_edits_rows_agree(body: &str) {
    let rows = edits_rows(body);
    assert!(
        rows.iter()
            .any(|row| row.contains("set aside, put back after")),
        "{body}"
    );
    assert!(
        !body.contains("nothing is put back"),
        "the rows disagree: {rows:?}"
    );
}

#[test]
fn the_edits_rows_agree_in_the_put_in_preview() {
    let dir = oauth_repo(true);
    let repo = dir.path();
    stage_auth_fix(repo);
    let (app, state) = open(repo, 187, 50);
    let state = cursor_to(&app, state, "Add OAuth callback", &mut Vec::new());
    let state = planned(&app, &state, KeyCode::Char('a'), "a", &mut Vec::new());
    assert_edits_rows_agree(&frame(&state, 187, 50));
    drop(state);
    app.close();
}

#[test]
fn the_edits_rows_agree_in_the_absorb_preview() {
    let dir = oauth_repo(false);
    let repo = dir.path();
    stage_three_hunks(repo);
    let (app, state) = open(repo, 120, 40);
    let state = app.key(&state, KeyCode::Char('k'));
    let state = app.key(&state, KeyCode::Char('.'));
    let state = planned(&app, &state, KeyCode::Enter, "\u{23ce}", &mut Vec::new());
    assert_edits_rows_agree(&frame(&state, 120, 40));
    drop(state);
    app.close();
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

/// The dependents check `a` on `HEAD` waits on when another local branch
/// exists (ticket 09 "Decided", 09b A1: under one frame on the fixture).
/// Run by `scripts/m7-09-perf.sh` in release; lines start `m7-09b`.
/// Ignored: a measurement, not a test.
///
/// Phase 1: `a` to the amend editor open, as graft runs it, 3 rounds × 7,
/// with the check (HEAD on `feature-a`, `main`, and `spike` — a branch that
/// is not the default and not built on his) and without it (`feature-a`
/// alone: today's amend at once). Phase 2: each call the plan job makes on
/// the `HEAD` path, timed alone in the order it makes them (the job's own
/// `repos.shared()` and `repos.branches()` are cached reads of the store;
/// `Branches::resolve` is timed cold, the upper bound). Phase 3 is
/// [`perf_dependents_spawns`]. The 300-branch line is not gated.
#[test]
#[ignore = "a measurement: scripts/m7-09-perf.sh"]
fn perf_dependents_check() {
    const ROUNDS: usize = 3;
    let mut with_check = Vec::new();
    let mut without = Vec::new();
    for round in 1..=ROUNDS {
        let with = a_to_editor(Fixture::Spike);
        let base = a_to_editor(Fixture::OnlyBranch);
        report("phase 1: a to the editor, with the check", round, &with);
        report("phase 1: a to the editor, today's amend", round, &base);
        with_check.push(median(&with));
        without.push(median(&base));
    }
    for round in 1..=ROUNDS {
        for (call, took) in calls_alone() {
            report(&format!("phase 2: {call}"), round, &took);
        }
    }
    let wide = a_to_editor(Fixture::Wide);
    report("300 unrelated branches, a to the editor", 1, &wide);

    let ms = |took: std::time::Duration| took.as_secs_f64() * 1e3;
    let rounds: Vec<String> = with_check
        .iter()
        .map(|took| format!("{:.1}", ms(*took)))
        .collect();
    let (with, base) = (median(&with_check), median(&without));
    println!(
        "m7-09b summary: a to the amend editor, median of 3\u{d7}7: {:.1} ms through the dependents check (rounds {}), {:.1} ms today's amend; the check adds {:.1} ms (budget 16.7 ms)",
        ms(with),
        rounds.join("/"),
        ms(base),
        ms(with.saturating_sub(base))
    );
}

#[derive(Clone, Copy)]
enum Fixture {
    /// `main`, `feature-a` (HEAD) and `spike`, off `main` with its own
    /// commit: the check runs and finds nothing built on `feature-a`.
    Spike,
    /// `feature-a` alone: no check.
    OnlyBranch,
    /// `Spike` plus 300 branches at `spike`.
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

/// Seven `a`s on `HEAD`, each to the amend editor open and `Esc`, sorted.
fn a_to_editor(fixture: Fixture) -> Vec<std::time::Duration> {
    let dir = perf_repo(fixture);
    let (app, mut state) = open_on_head(dir.path());
    let mut took = Vec::new();
    for _ in 0..7 {
        let started = std::time::Instant::now();
        state = a_once(&app, &state);
        took.push(started.elapsed());
    }
    took.sort();
    drop(state);
    app.close();
    took
}

/// Opened with the cursor on `HEAD`'s row (`spike`'s newer commit is the
/// top row when it exists).
fn open_on_head(repo: &Path) -> (App, AppState) {
    let (app, state) = open(repo, 120, 40);
    let state = cursor_to(&app, state, "Handle expired tokens", &mut Vec::new());
    (app, state)
}

/// One `a` to the editor, then `Esc`.
fn a_once(app: &App, state: &AppState) -> AppState {
    let asked = app.key(state, KeyCode::Char('a'));
    let opened = app.pump(asked, "the amend editor", |state| {
        state.commit_editor.is_some()
    });
    let closed = app.key(&opened, KeyCode::Escape);
    assert!(closed.commit_editor.is_none(), "Esc closes the editor");
    closed
}

/// Phase 2: the `HEAD` path's calls, seven times each, in the job's order
/// (`jobs/rewrite.rs::plan`), then `HEAD`'s message (`commit_message.rs`).
fn calls_alone() -> Vec<(&'static str, Vec<std::time::Duration>)> {
    use graft_git::branches::Branches;
    use graft_git::repo::RepoHandle;
    use graft_git::version::system_git;
    let dir = perf_repo(Fixture::Spike);
    let handle = RepoHandle::open(dir.path()).unwrap_or_else(|why| panic!("open: {why}"));
    let head = graft_ops::operation::Tip {
        name: "refs/heads/feature-a".to_owned(),
        oid: graft_core::oid::Oid::from_hex(&read(dir.path(), "feature-a"))
            .unwrap_or_else(|why| panic!("oid: {why}")),
    };
    let time = |run: &dyn Fn()| {
        let mut took: Vec<std::time::Duration> = (0..7)
            .map(|_| {
                let started = std::time::Instant::now();
                run();
                started.elapsed()
            })
            .collect();
        took.sort();
        took
    };
    let git = system_git()
        .unwrap_or_else(|why| panic!("git: {why}"))
        .version();
    let default = Branches::resolve(&handle, None)
        .unwrap_or_else(|why| panic!("branches: {why}"))
        .default_branch()
        .cloned();
    vec![
        (
            "system_git (git --version)",
            time(&|| {
                system_git().unwrap_or_else(|why| panic!("git: {why}"));
            }),
        ),
        (
            "Branches::resolve, cold",
            time(&|| {
                Branches::resolve(&handle, None).unwrap_or_else(|why| panic!("branches: {why}"));
            }),
        ),
        (
            "rewrite::dependents",
            time(&|| {
                graft_ops::rewrite::dependents(&handle, &git, &head, default.as_ref(), &())
                    .unwrap_or_else(|why| panic!("dependents: {why:?}"));
            }),
        ),
        (
            "HEAD's message (in process)",
            time(&|| {
                graft_git::history::read_commit(&handle, head.oid)
                    .unwrap_or_else(|why| panic!("read: {why}"));
            }),
        ),
    ]
}

/// Phase 3, its own run so the trace's cost stays out of phases 1 and 2:
/// seven `a`s with git tracing into the directory `GIT_TRACE2_PERF` names
/// (the script sets it; Graft's spawns keep the environment); every trace
/// file that appeared during them is one git process: argv and time.
#[test]
#[ignore = "a measurement: scripts/m7-09-perf.sh"]
fn perf_dependents_spawns() {
    let dir = std::env::var_os("GIT_TRACE2_PERF")
        .map(std::path::PathBuf::from)
        .filter(|dir| dir.is_dir())
        .unwrap_or_else(|| panic!("GIT_TRACE2_PERF names no directory"));
    spawns(&dir);
}

fn spawns(dir: &Path) {
    let dir_files = || -> std::collections::BTreeSet<std::path::PathBuf> {
        std::fs::read_dir(dir)
            .unwrap_or_else(|why| panic!("read {}: {why}", dir.display()))
            .map(|entry| entry.unwrap_or_else(|why| panic!("entry: {why}")).path())
            .collect()
    };
    let repo = perf_repo(Fixture::Spike);
    let (app, mut state) = open_on_head(repo.path());
    // Let the opening reads finish tracing before the window starts.
    state = a_once(&app, &state);
    let before = dir_files();
    for _ in 0..7 {
        state = a_once(&app, &state);
    }
    let after = dir_files();
    drop(state);
    app.close();
    let mut by_argv: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for file in after.difference(&before) {
        let text = std::fs::read_to_string(file)
            .unwrap_or_else(|why| panic!("read {}: {why}", file.display()));
        let mut argv = None;
        let mut took = None;
        for line in text.lines() {
            let fields: Vec<&str> = line.split('|').map(str::trim).collect();
            match fields.get(3).copied() {
                Some("start") => argv = fields.get(8).map(|argv| (*argv).to_owned()),
                Some("atexit") => took = fields.get(5).and_then(|at| at.parse::<f64>().ok()),
                _ => {}
            }
        }
        let (Some(argv), Some(took)) = (argv, took) else {
            println!("m7-09b phase 3: unreadable trace {}", file.display());
            continue;
        };
        // The fixture's scratch path names this machine and changes every
        // run: said as `<repo>` (the repo may be published).
        let argv = [repo.path().to_path_buf(), canonical(repo.path())]
            .iter()
            .fold(argv, |argv, path| {
                argv.replace(&path.display().to_string(), "<repo>")
            });
        by_argv.entry(argv).or_default().push(took * 1e3);
    }
    println!(
        "m7-09b phase 3: {} git processes over 7 a's",
        after.difference(&before).count()
    );
    for (argv, mut took) in by_argv {
        took.sort_by(f64::total_cmp);
        println!(
            "m7-09b phase 3: {}× {:.1} ms median (git's own clock)  {argv}",
            took.len(),
            took[took.len() / 2]
        );
    }
}

use std::collections::BTreeMap;

/// `path` with symlinks resolved (`/var` is `/private/var` on macOS).
fn canonical(path: &Path) -> std::path::PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|why| panic!("{}: {why}", path.display()))
}

fn median(took: &[std::time::Duration]) -> std::time::Duration {
    let mut sorted = took.to_vec();
    sorted.sort();
    sorted[sorted.len() / 2]
}

/// `m7-09b <what>, round <n>: median <x> ms, spread <min>–<max> ms`.
fn report(what: &str, round: usize, took: &[std::time::Duration]) {
    let ms = |took: std::time::Duration| took.as_secs_f64() * 1e3;
    let (low, high) = (
        took.iter().min().copied().unwrap_or_default(),
        took.iter().max().copied().unwrap_or_default(),
    );
    println!(
        "m7-09b {what}, round {round}: median {:.1} ms, spread {:.1}\u{2013}{:.1} ms",
        ms(median(took)),
        ms(low),
        ms(high)
    );
}
