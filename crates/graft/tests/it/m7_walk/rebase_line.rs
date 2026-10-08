//! Journeys `rebase-line-onto-main` and `squash-merged-bottom`
//! (m7-power-tools/05b): from his branch, the cursor to `main`, `r`, `y`.
//! The whole line moves as the preview drew it; a bottom branch main already
//! has (squash-merged) is dropped. A fork is refused before `y`, on the
//! graph, in words naming both branches. And `keep-a-suggested-drop`
//! (rebase-preview-keeps-drops): `r R p y` keeps one suggested drop.
//!
//! The fixtures are built here, dated so `main`'s tip sorts second, one
//! row under the cursor's start on the top row: the motion is one key.

use std::path::Path;

use graft::update::detail::Loading;
use graft::update::{AppState, GraphOverlay};
use graft_core::event::KeyCode;
use graft_ops::preview::{PreviewPlan, Target};

use crate::app_harness::App;

pub(crate) fn git(repo: &Path, args: &[&str], hour: u8) -> String {
    let date = format!("2020-01-01T{hour:02}:00:00+00:00");
    let out = graft_fixtures::git::run_at(repo, args, &date)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    String::from_utf8(out).unwrap_or_else(|why| panic!("git speaks utf-8 here: {why}"))
}

pub(crate) fn commit(repo: &Path, name: &str, hour: u8) {
    std::fs::write(repo.join(format!("{name}.txt")), format!("{name}\n"))
        .unwrap_or_else(|why| panic!("write: {why}"));
    git(repo, &["add", "-A"], hour);
    git(repo, &["commit", "-q", "-m", name], hour);
}

fn read(repo: &Path, rev: &str) -> String {
    git(repo, &["rev-parse", rev], 0).trim().to_owned()
}

fn init() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    git(dir.path(), &["init", "-q", "-b", "main"], 0);
    commit(dir.path(), "m1", 0);
    dir
}

fn finish(repo: &Path) {
    git(repo, &["switch", "-q", "feature-a"], 9);
    git(repo, &["commit-graph", "write", "--reachable"], 9);
}

/// `feature-a` (a1 a2) ← `feature-b` (b1) ← `feature-c` (c1), one line;
/// `main` moved on by m2. HEAD on `feature-a`. Rows: c1, m2, b1, a2, …
pub fn line_repo() -> tempfile::TempDir {
    let dir = init();
    let repo = dir.path();
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(repo, "a1", 1);
    commit(repo, "a2", 2);
    git(repo, &["switch", "-q", "-c", "feature-b"], 4);
    commit(repo, "b1", 4);
    git(repo, &["switch", "-q", "-c", "feature-c"], 6);
    commit(repo, "c1", 6);
    git(repo, &["switch", "-q", "main"], 5);
    commit(repo, "m2", 5);
    finish(repo);
    dir
}

/// `feature-a` (a1 a2 a3), squash-merged into `main` as "Add OAuth (#42)";
/// `feature-b` (b1) built on it. HEAD on `feature-a`. Rows: b1, the squash
/// (main), a3, …
pub fn squash_repo() -> tempfile::TempDir {
    let dir = init();
    let repo = dir.path();
    git(repo, &["switch", "-q", "main"], 1);
    commit(repo, "m2", 1);
    git(repo, &["switch", "-q", "-c", "feature-a", "HEAD~1"], 2);
    commit(repo, "a1", 2);
    commit(repo, "a2", 3);
    commit(repo, "a3", 4);
    git(repo, &["switch", "-q", "-c", "feature-b"], 6);
    commit(repo, "b1", 6);
    git(repo, &["switch", "-q", "main"], 5);
    git(repo, &["merge", "-q", "--squash", "feature-a"], 5);
    git(repo, &["commit", "-q", "-m", "Add OAuth (#42)"], 5);
    finish(repo);
    dir
}

/// [`squash_repo`], and then `main` takes a3's change back out ("Remove
/// a3"). The squash still matches every commit of `feature-a` (it is
/// unchanged), so a1–a3 are still suggested as drops; but a3's change is no
/// longer on `main`, so keeping it re-applies clean and non-empty (ticket
/// Decided 6). Rows: b1, "Remove a3" (main), the squash, a3, …
pub fn squash_revert_repo() -> tempfile::TempDir {
    let dir = squash_repo();
    let repo = dir.path();
    git(repo, &["switch", "-q", "main"], 5);
    git(repo, &["rm", "-q", "a3.txt"], 5);
    git(repo, &["commit", "-q", "-m", "Remove a3"], 5);
    finish(repo);
    dir
}

/// `feature-b` and `feature-c` both built on `feature-a` (a1): a fork.
pub fn fork_repo() -> tempfile::TempDir {
    let dir = init();
    let repo = dir.path();
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    commit(repo, "a1", 1);
    git(repo, &["switch", "-q", "-c", "feature-b"], 2);
    commit(repo, "b1", 2);
    git(repo, &["switch", "-q", "-c", "feature-c", "feature-a"], 4);
    commit(repo, "c1", 4);
    git(repo, &["switch", "-q", "main"], 3);
    commit(repo, "m2", 3);
    finish(repo);
    dir
}

pub(crate) fn open(repo: &Path, columns: u16, rows: u16) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, columns, rows);
    let state = crate::preview::opened(&app, state);
    (app, state)
}

/// The cursor onto `main`'s row, one key at a time: up first, then down if
/// the top was reached. Every key pressed is on `route`.
pub(crate) fn cursor_to_main(
    app: &App,
    state: AppState,
    route: &mut Vec<&'static str>,
) -> AppState {
    let mut state = state;
    let mut up = true;
    for _ in 0..20 {
        if crate::preview::on_main(&state) {
            return state;
        }
        let at = state.history.as_ref().map_or(0, |history| history.cursor);
        if up && at == 0 {
            up = false;
        }
        let (key, name) = if up { ('k', "k") } else { ('j', "j") };
        route.push(name);
        state = app.key(&state, KeyCode::Char(key));
    }
    panic!(
        "never reached main's row: {:?}",
        crate::preview::rows_drawn(&state)
    );
}

/// `r` pressed, and the job's answer: a drawn plan, or the box closed.
pub(crate) fn r(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("r");
    let state = app.key(state, KeyCode::Char('r'));
    assert!(
        state.preview.is_some(),
        "`r` opens a preview: {:?}",
        state.status
    );
    app.pump(state, "the preview's answer", |state| {
        state
            .preview
            .as_ref()
            .is_none_or(|preview| !preview.rows.is_empty())
    })
}

fn plan_of(state: &AppState) -> PreviewPlan {
    let preview = state
        .preview
        .as_ref()
        .unwrap_or_else(|| panic!("the preview is open: {:?}", state.status));
    match &preview.plan {
        Loading::Ready(planned) => planned.plan.clone(),
        other => panic!("the plan has not landed: {other:?}"),
    }
}

/// `y`, run to its end.
pub(crate) fn y(app: &App, state: &AppState, route: &mut Vec<&'static str>) -> AppState {
    route.push("y");
    let state = app.key(state, KeyCode::Char('y'));
    app.settle_write(state, "the rebase")
}

/// Every branch `drawn` moves landed as it was drawn: down each ref's
/// commits, the ghosts' subjects in order, then the real commit they stand
/// on.
fn lands_as_drawn(repo: &Path, drawn: &PreviewPlan) {
    for moved in &drawn.ref_moves {
        let mut at = moved.to;
        let mut commit = read(repo, &moved.name);
        loop {
            match at {
                Some(Target::Real(real)) => {
                    assert_eq!(
                        commit,
                        real.to_string(),
                        "{} lands on its commit",
                        moved.name
                    );
                    break;
                }
                Some(Target::Ghost(index)) => {
                    let ghost = &drawn.ghosts[index];
                    let subject = git(repo, &["log", "-1", "--format=%s", &commit], 0);
                    assert_eq!(
                        subject.trim(),
                        ghost.subject,
                        "{}: ghost {index}",
                        moved.name
                    );
                    at = ghost.parents.first().copied();
                    commit = read(repo, &format!("{commit}^"));
                }
                None => panic!("{}: drawn deleted", moved.name),
            }
        }
    }
}

pub(crate) fn frame(state: &AppState, columns: u16, rows: u16) -> String {
    let lines = crate::captures::frame_at(
        state,
        columns,
        rows,
        Some(graft::session::Clock::Fixed(graft_fixtures::CORPUS_NOW)),
    );
    format!("{}\n", lines.join("\n"))
}

pub(crate) fn within(journey: &str, route: &[&str]) {
    let allowed = super::noted_keys(journey);
    assert!(
        route.len() <= allowed,
        "{journey} took {} inputs {route:?}, the note allows {allowed}",
        route.len()
    );
}

/// `rebase-line-onto-main`: the cursor to main, `r`, `y`. Every branch of
/// the line lands where its ghosts were drawn, bottom first.
#[test]
fn r_y_moves_the_line() {
    let dir = line_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 187, 50);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let drawn = plan_of(&state);
    let names: Vec<&str> = drawn
        .ref_moves
        .iter()
        .map(|moved| moved.name.as_str())
        .collect();
    assert_eq!(
        names,
        [
            "refs/heads/feature-a",
            "refs/heads/feature-b",
            "refs/heads/feature-c"
        ]
    );
    let state = y(&app, &state, &mut route);
    within("rebase-line-onto-main", &route);
    lands_as_drawn(repo, &drawn);
    assert_eq!(read(repo, "feature-a~2"), read(repo, "main"));
    drop(state);
    app.close();
}

/// `squash-merged-bottom`: the cursor to main, `r`, `y`. feature-a's three
/// commits are dropped (main has them as "Add OAuth (#42)"); feature-b's b1
/// lands on main.
#[test]
fn r_y_drops_the_squash_merged_branch() {
    let dir = squash_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 120, 40);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let drawn = plan_of(&state);
    let replayed: Vec<&str> = drawn
        .ghosts
        .iter()
        .map(|ghost| ghost.subject.as_str())
        .collect();
    assert_eq!(replayed, ["b1"], "no ghost for a dropped commit");
    let state = y(&app, &state, &mut route);
    within("squash-merged-bottom", &route);
    lands_as_drawn(repo, &drawn);
    assert_eq!(read(repo, "feature-a"), read(repo, "main"));
    assert_eq!(read(repo, "feature-b^"), read(repo, "main"));
    drop(state);
    app.close();
}

/// A fork: `r` is refused before `y`. The box is closed, the graph's status
/// line names both branches, and no ref moved.
#[test]
fn r_over_a_fork_lands_on_the_graph_with_the_words() {
    let dir = fork_repo();
    let repo = dir.path();
    let before = git(
        repo,
        &["for-each-ref", "--format=%(refname) %(objectname)"],
        0,
    );
    let (app, state) = open(repo, 120, 40);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    assert!(state.preview.is_none(), "the box closed");
    let screen = frame(&state, 120, 40);
    assert!(
        screen.contains("feature-b and feature-c are both built on yours"),
        "{screen}"
    );
    assert_eq!(
        git(
            repo,
            &["for-each-ref", "--format=%(refname) %(objectname)"],
            0
        ),
        before,
        "no ref moved"
    );
    drop(state);
    app.close();
}

/// The three captures (rule 6: the user judges them; colour unverified).
/// Right: the line's box has a `branches:` label and three rows bottom
/// first, the two above his marked `built on yours · follows`; the squash
/// box has `already in main: feature-a's 3 commits match <sha7> "Add OAuth
/// (#42)" · dropped · R to keep them`, one ghost (b1), and its footer reads
/// `y confirm  r rebase  R keep  ←/→ choose  ⏎ act  esc cancel`; the fork
/// leaves the graph with the status line naming feature-b and feature-c.
#[test]
fn capture_rebase_line() {
    type Fixture = fn() -> tempfile::TempDir;
    let shots: [(&str, Fixture, u16, u16); 3] = [
        ("m7/rebase-line-preview-187x50.txt", line_repo, 187, 50),
        (
            "m7/rebase-squash-merged-preview-120x40.txt",
            squash_repo,
            120,
            40,
        ),
        ("m7/rebase-refused-fork-120x40.txt", fork_repo, 120, 40),
    ];
    for (name, fixture, columns, rows) in shots {
        let dir = fixture();
        let (app, state) = open(dir.path(), columns, rows);
        let mut route = Vec::new();
        let state = cursor_to_main(&app, state, &mut route);
        let state = r(&app, &state, &mut route);
        let body = frame(&state, columns, rows);
        crate::captures::check_shot(name, &body);
        match name {
            "m7/rebase-line-preview-187x50.txt" => {
                assert!(body.contains("branches: feature-a "), "{body}");
                assert!(!body.contains("R keep"), "no suggestion, no key: {body}");
                assert_eq!(
                    body.matches("built on yours \u{b7} follows").count(),
                    2,
                    "{body}"
                );
            }
            "m7/rebase-squash-merged-preview-120x40.txt" => {
                assert!(
                    body.contains("already in main: feature-a's 3 commits match "),
                    "{body}"
                );
                assert!(
                    body.contains("\"Add OAuth (#42)\" \u{b7} dropped \u{b7} R to keep them"),
                    "{body}"
                );
                assert!(
                    body.contains("y confirm  r rebase  R keep  ←/→ choose  ⏎ act  esc cancel"),
                    "{body}"
                );
            }
            _ => {
                assert!(body.contains("feature-b and feature-c"), "{body}");
                assert!(!body.contains("R keep"), "{body}");
            }
        }
        drop(state);
        app.close();
    }
}

// ── the target on the top row (m7/37, usability M1) ─────────────────────

/// `feature-a` (`mine` commits) ← `feature-b` (b1), one line; `main` moved
/// on by m2, dated last, so `main` is the top row and the cursor starts on
/// it. HEAD on `feature-a`. The ghosts land above the user's top row.
fn top_row_repo(mine: usize) -> tempfile::TempDir {
    let dir = init();
    let repo = dir.path();
    git(repo, &["switch", "-q", "-c", "feature-a"], 1);
    for at in 1..=mine {
        commit(repo, &format!("a{at}"), 1);
    }
    git(repo, &["switch", "-q", "-c", "feature-b"], 2);
    commit(repo, "b1", 2);
    git(repo, &["switch", "-q", "main"], 8);
    commit(repo, "m2", 8);
    finish(repo);
    dir
}

/// The graph rows of `screen`: under the header, above the box.
fn graph_rows(screen: &str) -> Vec<&str> {
    screen
        .lines()
        .skip(1)
        .take_while(|line| !line.starts_with('\u{256d}'))
        .filter(|line| !line.trim().is_empty())
        .collect()
}

/// `r` with the cursor on `main`, the top row, at this size.
fn r_on_the_top_row(dir: &tempfile::TempDir, columns: u16, rows: u16) -> (App, AppState) {
    let (app, state) = open(dir.path(), columns, rows);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    assert_eq!(route, Vec::<&str>::new(), "main is the top row: no key");
    assert_eq!(
        state.history.as_ref().map(|history| history.top),
        Some(0),
        "scrolled to the top"
    );
    let state = r(&app, &state, &mut route);
    (app, state)
}

/// Usability M1: the ghosts of a rebase onto the top row sit above the
/// user's top row; the picture starts at their label, so they are drawn.
#[test]
fn a_rebase_onto_the_top_row_shows_after_rebase() {
    let dir = top_row_repo(3);
    let (app, state) = r_on_the_top_row(&dir, 187, 44);
    let screen = frame(&state, 187, 44);
    let rows = graph_rows(&screen);
    assert!(
        rows.first().is_some_and(|row| row.contains("after rebase")),
        "{screen}"
    );
    let ghosts: Vec<&&str> = rows.iter().filter(|row| row.contains('\u{25cb}')).collect();
    assert_eq!(ghosts.len(), 4, "a1 a2 a3 b1 as ghosts: {screen}");
    assert!(rows.iter().any(|row| row.contains("[main] m2")), "{screen}");
    crate::captures::check_shot("m7/rebase-preview-target-top-187x44.txt", &screen);
    assert!(
        screen
            .lines()
            .any(|line| line.contains("branches: feature-a ")
                && line.contains("\u{2192} 3 new commits ")),
        "{screen}"
    );
    assert!(
        screen.contains("\u{2192} 1 new commit \u{b7} built on yours \u{b7} follows"),
        "{screen}"
    );
    drop(state);
    app.close();
}

/// A ghost block taller than the picture: the target is the picture's last
/// row, the ghosts fill the rows above it, and `after rebase` is pinned to
/// the top row.
#[test]
fn a_preview_too_tall_pins_its_label() {
    let dir = top_row_repo(30);
    let (app, state) = r_on_the_top_row(&dir, 120, 40);
    let screen = frame(&state, 120, 40);
    let rows = graph_rows(&screen);
    assert!(
        rows.first().is_some_and(|row| row.contains("after rebase")),
        "the label on the top row: {screen}"
    );
    assert!(
        rows.last().is_some_and(|row| row.contains("[main] m2")),
        "the target on the last row: {screen}"
    );
    assert!(
        rows[1..rows.len() - 1]
            .iter()
            .all(|row| row.contains('\u{25cb}')),
        "ghosts between: {screen}"
    );
    drop(state);
    app.close();
}

// ── `keep-a-suggested-drop` (rebase-preview-keeps-drops) ─────────────────

/// The rewrite editor is up (opened by `R` in the preview) and nothing on
/// it is still reading or checking.
fn editor_settled(state: &AppState, columns: u16, rows: u16) -> bool {
    let screen = frame(state, columns, rows);
    state.graph_overlay() == Some(GraphOverlay::Rewrite)
        && screen.contains("REWRITE ")
        && !screen.contains("checking\u{2026}")
}

/// `R` in the squash-merged preview: the preview closes and the editor
/// opens on that rebase, its drops marked.
fn keep_key(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    columns: u16,
    rows: u16,
) -> AppState {
    route.push("R");
    let state = app.key(state, KeyCode::Char('R'));
    assert!(state.preview.is_none(), "`R` closes the preview");
    app.pump(state, "the editor opened from the rebase", |state| {
        editor_settled(state, columns, rows)
    })
}

/// Every local branch and its tip, by name.
fn branch_tips(repo: &Path) -> String {
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

/// `keep-a-suggested-drop`: the cursor to main, `r`, `R`, `p`, `y`. The
/// editor opens on the rebase with a1–a3 marked `d` and the cursor on a3;
/// `p` keeps a3; `y` lands feature-a as a3 alone on main (a1, a2 dropped),
/// feature-b following. `u` puts every branch back where it was.
#[test]
fn keep_a_suggested_drop() {
    let dir = squash_revert_repo();
    let repo = dir.path();
    let (app, state) = open(repo, 120, 40);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let onto_tip = read(repo, "main");
    let screen = frame(&state, 120, 40);
    assert!(screen.contains("R to keep them"), "{screen}");

    let state = keep_key(&app, &state, &mut route, 120, 40);
    let screen = frame(&state, 120, 40);
    assert_eq!(
        screen.matches("already in main \u{b7} p keeps it").count(),
        3,
        "{screen}"
    );
    let cursor_line = screen
        .lines()
        .find(|line| line.contains('\u{25b8}'))
        .unwrap_or_else(|| panic!("a cursor row: {screen}"));
    assert!(cursor_line.contains(" a3"), "on the newest drop: {screen}");

    let state = letter_replanned(&app, &state, &mut route, 'p', "p", 120, 40);
    let before = branch_tips(repo);
    let state = y(&app, &state, &mut route);
    within("keep-a-suggested-drop", &route);

    // feature-a: a3 alone, straight on main's tip as `R` saw it.
    let above: Vec<String> = git(repo, &["log", "--format=%s", "main..feature-a"], 0)
        .lines()
        .map(str::to_owned)
        .collect();
    assert_eq!(
        above,
        ["a3"],
        "the kept commit, and none of the other drops"
    );
    assert_eq!(read(repo, "feature-a^"), onto_tip, "parent = main's tip");
    assert!(
        repo.join("a3.txt").exists(),
        "a3 re-applied, not empty (main took it out)"
    );
    // feature-b follows, on the new feature-a.
    assert_eq!(read(repo, "feature-b^"), read(repo, "feature-a"));
    assert_eq!(
        git(repo, &["log", "-1", "--format=%s", "feature-b"], 0).trim(),
        "b1"
    );

    let state = app.key(&state, KeyCode::Char('u'));
    let state = app.settle_write(state, "the undo");
    for line in before.lines() {
        let (name, tip) = line
            .split_once(' ')
            .unwrap_or_else(|| panic!("a ref line: {line}"));
        assert_eq!(read(repo, name), tip, "`u` puts {name} back");
    }
    drop(state);
    app.close();
}

/// A letter in the editor, and its replan landed.
fn letter_replanned(
    app: &App,
    state: &AppState,
    route: &mut Vec<&'static str>,
    key: char,
    name: &'static str,
    columns: u16,
    rows: u16,
) -> AppState {
    route.push(name);
    let state = app.key(state, KeyCode::Char(key));
    app.pump(state, "the replan", |state| {
        editor_settled(state, columns, rows)
    })
}

/// The editor as `R` opens it from the squash-merged preview, before `p`
/// (rule 6: the user judges it; colour unverified). Right: each suggested
/// row shows 08's drop action and `already in main · p keeps it`; no row
/// says `marked by its name`; the last row is `─ main` at `onto_tip`; the
/// cursor `▸` is on the newest drop (a3).
#[test]
fn capture_keep_drops_editor() {
    let dir = squash_repo();
    let (app, state) = open(dir.path(), 120, 40);
    let mut route = Vec::new();
    let state = cursor_to_main(&app, state, &mut route);
    let state = r(&app, &state, &mut route);
    let state = keep_key(&app, &state, &mut route, 120, 40);
    let body = frame(&state, 120, 40);
    crate::captures::check_shot("m7/rebase-keep-drops-editor-120x40.txt", &body);
    assert_eq!(
        body.matches("already in main \u{b7} p keeps it").count(),
        3,
        "{body}"
    );
    assert!(!body.contains("marked by its name"), "{body}");
    // Arrival is not a refusal (30n Q1): where he is and the next key.
    assert!(
        body.contains("nothing kept yet \u{b7} p keeps a commit"),
        "{body}"
    );
    assert!(!body.contains("every commit dropped"), "{body}");
    drop(state);
    app.close();
}
