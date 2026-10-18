//! Journeys `squash-last-three` and `swap-adjacent` (m7-power-tools/08):
//! `R` on his branch opens the rewrite editor over every commit since it left
//! `main`; letters mark rows, `Alt+↑` moves one, `y` writes the lot as one
//! operation that `u` takes back. `R s esc` writes nothing.
//!
//! The fixture is R1's mock: `feature-a` has five commits since `main`, one of
//! them a `fixup!` (it opens pre-marked), and `feature-b` is built on it.

use std::path::Path;

use mergewright::update::{AppState, GraphOverlay, Status};
use mergewright_core::event::{KeyCode, KeyPress, Modifiers};

use super::rebase_line::{fork_repo, frame, git, open, within};
use crate::app_harness::App;
use mergewright_core::on_disk::CURRENT_DIR;

/// `subject` (with `body`, if any) committed as a change to `file`.
fn commit(repo: &Path, file: &str, subject: &str, body: &str, hour: u8) {
    let path = repo.join(file);
    // A first commit to `file` creates it; any other read failure is the
    // fixture's own bug and panics.
    let mut text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(why) => panic!("read {file}: {why}"),
    };
    text.push_str(subject);
    text.push('\n');
    std::fs::write(&path, text).unwrap_or_else(|why| panic!("write {file}: {why}"));
    git(repo, &["add", "-A"], hour);
    let message = if body.is_empty() {
        subject.to_owned()
    } else {
        format!("{subject}\n\n{body}")
    };
    git(repo, &["commit", "-q", "-m", &message], hour);
}

/// `main` (m1) ← `feature-a`: "Add OAuth callback", its `fixup!`, "Handle
/// expired tokens", "wip: tests", "wip" ← `feature-b` (b1). HEAD on
/// `feature-a`.
pub(super) fn mock_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"], 0);
    commit(repo, "readme.txt", "m1", "", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(
        repo,
        "oauth.txt",
        "Add OAuth callback",
        "The callback body.",
        1,
    );
    commit(repo, "oauth.txt", "fixup! Add OAuth callback", "", 2);
    commit(repo, "tokens.txt", "Handle expired tokens", "", 3);
    commit(
        repo,
        "tests.txt",
        "wip: tests",
        "Covers the expired path.",
        4,
    );
    commit(repo, "wip.txt", "wip", "", 5);
    git(repo, &["switch", "-q", "-c", "feature-b"], 6);
    commit(repo, "b.txt", "b1", "", 6);
    git(repo, &["switch", "-q", "feature-a"], 7);
    git(repo, &["commit-graph", "write", "--reachable"], 7);
    dir
}

pub(super) fn read(repo: &Path, rev: &str) -> String {
    git(repo, &["rev-parse", rev], 0).trim().to_owned()
}

pub(super) fn subjects(repo: &Path, range: &str) -> Vec<String> {
    git(repo, &["log", "--format=%s", range], 0)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// Every branch and where it points. The engine's own safety refs
/// (`<prefix in force>backup/…`, `Generation::ref_prefix`, written by each rewrite) are not his refs.
pub(super) fn every_ref(repo: &Path) -> String {
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

pub(super) fn selected(state: &AppState) -> Option<String> {
    state
        .history
        .as_ref()
        .and_then(mergewright::history::History::selected)
        .map(|row| row.commit.id.to_string())
}

/// The cursor onto `rev`'s row, placed: these keys are not the journey's.
pub(super) fn place_on(app: &App, state: AppState, repo: &Path, rev: &str) -> AppState {
    let want = read(repo, rev);
    let mut state = state;
    let mut up = true;
    for _ in 0..20 {
        if selected(&state).as_deref() == Some(want.as_str()) {
            return state;
        }
        let at = state.history.as_ref().map_or(0, |history| history.cursor);
        if up && at == 0 {
            up = false;
        }
        state = app.key(&state, KeyCode::Char(if up { 'k' } else { 'j' }));
    }
    panic!("never reached {rev}'s row");
}

/// `R`: the editor is up and its first AFTER plan landed (`y rewrite` on
/// the bar).
pub(super) fn capital_r(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    columns: u16,
    rows: u16,
) -> AppState {
    route.push("R");
    let state = app.key(state, KeyCode::Char('R'));
    app.pump(state, "the editor and its plan", |state| {
        settled(state, columns, rows)
    })
}

/// The editor is up and its AFTER plan is in: no `checking…` anywhere (at
/// 120 columns and wider the AFTER header says it while the plan is out).
fn settled(state: &AppState, columns: u16, rows: u16) -> bool {
    let screen = frame(state, columns, rows);
    state.graph_overlay() == Some(GraphOverlay::Rewrite)
        && screen.contains("REWRITE ")
        && !screen.contains("checking\u{2026}")
}

fn letter(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    key: char,
    name: &'static str,
) -> AppState {
    route.push(name);
    app.key(state, KeyCode::Char(key))
}

/// A letter that replans, and its plan landed.
pub(super) fn replanned(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    key: KeyPress,
    name: &'static str,
    columns: u16,
    rows: u16,
) -> AppState {
    route.push(name);
    let state = app.press(state, key);
    app.pump(state, "the replan", |state| settled(state, columns, rows))
}

/// `y`, run to its end.
fn y(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("y");
    let state = app.key(state, KeyCode::Char('y'));
    app.settle_write(state, "the rewrite")
}

pub(super) fn undo(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('u'));
    app.settle_write(state, "the undo")
}

/// The message block's lines as drawn: every line after the `┊` gutter.
pub(super) fn message_shown(screen: &str) -> Vec<String> {
    screen
        .lines()
        .filter_map(|line| line.split_once('\u{250a}'))
        .map(|(_, text)| {
            // At 187 columns the AFTER side's `│` rule ends each line.
            let text = text.trim_end().strip_suffix('\u{2502}').unwrap_or(text);
            text.trim().to_owned()
        })
        .collect()
}

/// `squash-last-three`: on HEAD, `R s s y`. The three newest commits are
/// one, with the message the editor showed; the fixup melded on its own;
/// `feature-b` follows; `u` puts every ref back.
#[test]
fn squash_last_three() {
    let dir = mock_repo();
    let repo = dir.path();
    let before = every_ref(repo);
    let (app, state) = open(repo, 187, 50);
    let state = place_on(&app, state, repo, "HEAD");
    let mut route = Vec::new();
    let state = capital_r(&app, &state, &mut route, 187, 50);
    let press = |c| KeyPress::plain(KeyCode::Char(c));
    let state = replanned(&app, &state, &mut route, press('s'), "s", 187, 50);
    let state = replanned(&app, &state, &mut route, press('s'), "s", 187, 50);
    let screen = frame(&state, 187, 50);
    let shown = message_shown(&screen);
    assert!(!shown.is_empty(), "the message block is drawn: {screen}");
    let state = y(&app, &state, &mut route);
    within("squash-last-three", &route);

    assert_eq!(
        subjects(repo, "main..feature-a"),
        ["Handle expired tokens", "Add OAuth callback"],
        "one commit where there were three; the fixup melded"
    );
    let message = git(repo, &["log", "-1", "--format=%B", "feature-a"], 0);
    let written: Vec<&str> = message.trim_end().lines().map(str::trim).collect();
    assert_eq!(
        written, shown,
        "the message written is the one the editor showed"
    );
    assert_eq!(
        read(repo, "feature-b^"),
        read(repo, "feature-a"),
        "feature-b rebased onto it"
    );
    // `moves_head` covers `Rewrite` (mergewright-ops `operation.rs:1278`), so the
    // launch asks for the landing (`update/operate.rs:141`) and `Done`
    // spends it once the refresh's refs and walk are in (`:1011`).
    let head = read(repo, "HEAD");
    let state = app.pump(state, "the cursor on the new HEAD", |state| {
        selected(state).as_deref() == Some(head.as_str())
    });

    let state = undo(&app, &state);
    assert_eq!(every_ref(repo), before, "`u` restores every branch");
    drop(state);
    app.close();
}

/// `swap-adjacent`: on HEAD's parent, `R Alt+↑ y`. The two newest commits
/// trade places; `u` puts them back.
#[test]
fn swap_adjacent() {
    let dir = mock_repo();
    let repo = dir.path();
    let before = every_ref(repo);
    let (app, state) = open(repo, 187, 50);
    let state = place_on(&app, state, repo, "HEAD^");
    let mut route = Vec::new();
    let state = capital_r(&app, &state, &mut route, 187, 50);
    let alt_up = KeyPress {
        code: KeyCode::Up,
        modifiers: Modifiers {
            alt: true,
            ..Modifiers::default()
        },
    };
    let state = replanned(&app, &state, &mut route, alt_up, "Alt+↑", 187, 50);
    let state = y(&app, &state, &mut route);
    within("swap-adjacent", &route);
    let top_two: Vec<String> = subjects(repo, "main..feature-a")
        .into_iter()
        .take(2)
        .collect();
    assert_eq!(top_two, ["wip: tests", "wip"], "the two traded places");

    let state = undo(&app, &state);
    assert_eq!(every_ref(repo), before, "`u` restores every branch");
    drop(state);
    app.close();
}

/// `R s esc`: refs, index, files and journal as they were.
#[test]
fn rewrite_look_and_leave_writes_nothing() {
    let dir = mock_repo();
    let repo = dir.path();
    let refs = every_ref(repo);
    let index = git(repo, &["ls-files", "-s"], 0);
    let files = git(repo, &["status", "--porcelain", "--ignored"], 0);
    let (app, state) = open(repo, 187, 50);
    let state = place_on(&app, state, repo, "HEAD");
    let mut route = Vec::new();
    let state = capital_r(&app, &state, &mut route, 187, 50);
    let state = letter(&app, &state, &mut route, 's', "s");
    route.push("esc");
    let state = app.key(&state, KeyCode::Escape);
    assert_eq!(state.graph_overlay(), None, "esc closes the editor");
    drop(state);
    app.close();
    assert_eq!(route, ["R", "s", "esc"]);
    assert_eq!(every_ref(repo), refs, "refs");
    assert_eq!(git(repo, &["ls-files", "-s"], 0), index, "index");
    assert_eq!(
        git(repo, &["status", "--porcelain", "--ignored"], 0),
        files,
        "files"
    );
    // The app makes the journal file at some point after opening (empty);
    // what counts is that no entry was written.
    let journal = mergewright_ops::journal::Journal::open(
        &repo.join(format!(".git/{CURRENT_DIR}/undo-journal.jsonl")),
    )
    .unwrap_or_else(|why| panic!("the journal is readable: {why}"));
    assert!(
        journal.entries().is_empty(),
        "journal: {:?}",
        journal.entries()
    );
}

/// The editor's three captures (rule 6: the user judges them; colour
/// unverified).
/// Right: see TESTING.md, "The rewrite editor".
#[test]
fn capture_rewrite_editor() {
    for (columns, rows) in [(187, 50), (140, 40), (100, 30)] {
        let dir = mock_repo();
        let repo = dir.path();
        let (app, state) = open(repo, columns, rows);
        let state = place_on(&app, state, repo, "HEAD");
        let mut route = Vec::new();
        let state = capital_r(&app, &state, &mut route, columns, rows);
        let press = |c| KeyPress::plain(KeyCode::Char(c));
        let state = if columns >= 120 {
            let state = replanned(&app, &state, &mut route, press('s'), "s", columns, rows);
            replanned(&app, &state, &mut route, press('s'), "s", columns, rows)
        } else {
            let state = letter(&app, &state, &mut route, 's', "s");
            letter(&app, &state, &mut route, 's', "s")
        };
        let body = frame(&state, columns, rows);
        crate::captures::check_shot(&format!("m7/rewrite-editor-{columns}x{rows}.txt"), &body);
        assert!(
            body.contains(
                "REWRITE feature-a \u{b7} 5 commits since it left main \u{b7} newest first"
            ),
            "{body}"
        );
        match columns {
            187 => {
                assert!(body.contains("AFTER y \u{b7} not written yet"), "{body}");
                assert!(body.contains("3 in one"), "{body}");
                assert!(body.contains("MESSAGE of the squashed commit"), "{body}");
                assert!(
                    body.lines()
                        .last()
                        .is_some_and(|bar| bar.trim_start().starts_with("y rewrite")),
                    "{body}"
                );
            }
            140 => assert!(body.contains("AFTER y"), "{body}"),
            _ => {
                assert!(!body.contains("AFTER y"), "rows only: {body}");
                assert!(body.contains("⇥ after"), "{body}");
            }
        }
        drop(state);
        app.close();
    }
}

/// `R` over a fork: no editor, the graph as it was, the words on the status
/// line (rule 6; colour unverified).
#[test]
fn capture_rewrite_refused_fork() {
    let dir = fork_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 120, 40);
    let state = place_on(&app, state, repo, "HEAD");
    let state = app.key(&state, KeyCode::Char('R'));
    let state = app.pump(state, "the refusal", |state| {
        matches!(state.status, Status::Sticky { .. })
    });
    assert_eq!(state.graph_overlay(), None, "no overlay");
    let body = frame(&state, 120, 40);
    crate::captures::check_shot("m7/rewrite-refused-fork-120x40.txt", &body);
    assert!(
        body.contains(
            "feature-b and feature-c are both built on yours \u{b7} Mergewright moves one line"
        ),
        "{body}"
    );
    drop(state);
    app.close();
}

// ── left behind (m7/37, usability M6) ───────────────────────────────────

/// The walk's world: `main` (m1) ← `feature-a`: "feature a: part 1",
/// "part 2", "part 3"; `feature-b` (b1) forked from part 1, not from
/// `feature-a`'s tip, so it is not built on his and does not follow. HEAD
/// on `feature-a`, a fix to part 1's file staged.
fn forked_mid_chain_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    git(repo, &["init", "-q", "-b", "main"], 0);
    commit(repo, "readme.txt", "m1", "", 0);
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(repo, "one.txt", "feature a: part 1", "", 1);
    commit(repo, "two.txt", "feature a: part 2", "", 2);
    commit(repo, "three.txt", "feature a: part 3", "", 3);
    git(repo, &["switch", "-q", "-c", "feature-b", "feature-a~2"], 4);
    commit(repo, "b.txt", "b1", "", 4);
    git(repo, &["switch", "-q", "feature-a"], 5);
    git(repo, &["commit-graph", "write", "--reachable"], 5);
    std::fs::write(repo.join("one.txt"), "feature a: part 1, fixed\n")
        .unwrap_or_else(|why| panic!("write: {why}"));
    git(repo, &["add", "one.txt"], 5);
    dir
}

/// `a y` on part 1: `feature-b` forks off part 1 with a clean replay, so it
/// follows (ADR-0059, m8-polish-and-ship/14b) where it used to be left
/// behind. The box says so before `y`, under feature-a's ref row, and no
/// `left behind:` line is drawn; at 80 columns a shorter rung still names it.
/// The name is kept from m7 (the journey); 14c draws every fate.
/// Rule 6: the capture is for the user's eye; colour unverified.
#[test]
fn the_box_says_who_stays_behind() {
    let dir = forked_mid_chain_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 187, 44);
    let state = app.pump(state, "the working tree", |state| {
        state.working_tree.is_some()
    });
    let state = place_on(&app, state, repo, "feature-a~2");
    let state = app.key(&state, KeyCode::Char('a'));
    let state = app.pump(state, "the put-in's preview", |state| {
        state
            .preview
            .as_ref()
            .is_some_and(|preview| !preview.rows.is_empty())
            || matches!(state.status, Status::Sticky { .. })
    });
    assert!(state.preview.is_some(), "the preview: {:?}", state.status);
    let body = frame(&state, 187, 44);
    crate::captures::check_shot("m7/amend-into-left-behind-187x44.txt", &body);
    let lines: Vec<&str> = body.lines().collect();
    let moved = lines
        .iter()
        .position(|line| line.contains("feature-a ") && line.contains("\u{2192} 3 new commits"))
        .unwrap_or_else(|| panic!("feature-a's row: {body}"));
    let follows = lines
        .iter()
        .position(|line| line.contains("feature-b ") && line.contains("follows"))
        .unwrap_or_else(|| panic!("feature-b follows: {body}"));
    assert_eq!(follows, moved + 1, "right under feature-a's row: {body}");
    assert!(
        !body.contains("left behind"),
        "a clean fork is not left behind: {body}"
    );

    let state = app.resize(&state, 80, 40);
    let narrow = frame(&state, 80, 40);
    assert!(
        narrow
            .lines()
            .any(|line| line.contains("feature-b") && line.contains("follows")),
        "{narrow}"
    );
    assert!(!narrow.contains("left behind"), "{narrow}");
    drop(state);
    app.close();
}

/// The rewrite editor's AFTER side says it too: `feature-b` follows
/// (ADR-0059, 14b), named above the forecast, and nothing is left behind.
/// He confirms from there as well (the M5 promise).
#[test]
fn the_rewrite_editor_after_says_who_stays_behind() {
    let dir = forked_mid_chain_repo();
    let repo = dir.path();
    git(repo, &["restore", "--staged", "one.txt"], 5);
    git(repo, &["checkout", "--", "one.txt"], 5);
    let (app, state) = open(repo, 187, 44);
    let state = place_on(&app, state, repo, "HEAD");
    let mut route = Vec::new();
    let state = capital_r(&app, &state, &mut route, 187, 44);
    let body = frame(&state, 187, 44);
    crate::captures::check_shot("m7/rewrite-editor-left-behind-187x44.txt", &body);
    let lines: Vec<&str> = body.lines().collect();
    let left = lines
        .iter()
        .position(|line| line.contains("feature-b follows \u{b7} 1 commit from "))
        .unwrap_or_else(|| panic!("feature-b follows: {body}"));
    assert!(
        !body.contains("left behind"),
        "a clean fork is not left behind: {body}"
    );
    let forecast = lines
        .iter()
        .position(|line| line.contains("no conflicts expected"))
        .unwrap_or_else(|| panic!("the forecast: {body}"));
    assert!(left < forecast, "above the forecast: {body}");
    drop(state);
    app.close();
}
