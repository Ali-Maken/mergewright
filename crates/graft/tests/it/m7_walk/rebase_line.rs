//! Journeys `rebase-line-onto-main` and `squash-merged-bottom`
//! (m7-power-tools/05b): from his branch, the cursor to `main`, `r`, `y`.
//! The whole line moves as the preview drew it; a bottom branch main already
//! has (squash-merged) is dropped. A fork is refused before `y`, on the
//! graph, in words naming both branches.
//!
//! The fixtures are built here, dated so `main`'s tip sorts second, one
//! row under the cursor's start on the top row: the motion is one key.

use std::path::Path;

use graft::update::AppState;
use graft::update::detail::Loading;
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
/// (#42)" · dropped`, no `R`, one ghost (b1); the fork leaves the graph
/// with the status line naming feature-b and feature-c.
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
                    body.contains("\"Add OAuth (#42)\" \u{b7} dropped"),
                    "{body}"
                );
                assert!(!body.contains(" R to keep"), "{body}");
            }
            _ => assert!(body.contains("feature-b and feature-c"), "{body}"),
        }
        drop(state);
        app.close();
    }
}
