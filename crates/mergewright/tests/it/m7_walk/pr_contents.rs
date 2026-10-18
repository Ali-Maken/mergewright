//! Journey `pr-contents` (m7-power-tools/14): from `main` with the pane
//! closed, `x` then one motion shows what `feature-a` carries since it left
//! `main` — split..head, not two-dot — with the header saying so in words.
//!
//! The fixture is built here: `feature-a` 3 commits off the root, `main`
//! moved on by 2 after, dates interleaved so `feature-a`'s tip sorts one row
//! under `main`'s.

use std::path::Path;

use mergewright::update::AppState;
use mergewright::update::detail::{DetailSpec, Loading};
use mergewright_core::event::KeyCode;
use mergewright_core::oid::Oid;

use crate::app_harness::App;

fn git(repo: &Path, args: &[&str], date: &str) -> String {
    let out = mergewright_fixtures::git::run_at(repo, args, date)
        .unwrap_or_else(|why| panic!("git {args:?}: {why}"));
    String::from_utf8(out).unwrap_or_else(|why| panic!("git speaks utf-8 here: {why}"))
}

fn commit(repo: &Path, file: &str, body: &str, message: &str, date: &str) {
    let path = repo.join(file);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap_or_else(|why| panic!("mkdir: {why}"));
    }
    std::fs::write(path, body).unwrap_or_else(|why| panic!("write: {why}"));
    git(repo, &["add", file], date);
    git(repo, &["commit", "-q", "-m", message], date);
}

fn oid(repo: &Path, rev: &str) -> Oid {
    Oid::from_hex(git(repo, &["rev-parse", rev], "2020-01-01T00:00:00+00:00").trim())
        .unwrap_or_else(|why| panic!("an oid: {why}"))
}

/// Rows, newest first: main (m2), feature-a (a3), m1, a2, a1, root.
pub fn split_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let repo = dir.path();
    let at = |hour: u8| format!("2020-01-01T{hour:02}:00:00+00:00");
    git(repo, &["init", "-q", "-b", "main"], &at(0));
    commit(repo, "README.md", "readme\n", "root", &at(0));
    git(repo, &["checkout", "-q", "-b", "feature-a"], &at(0));
    commit(repo, "src/login.rs", "fn login() {}\n", "Add login", &at(1));
    commit(repo, "src/token.rs", "fn token() {}\n", "Add token", &at(2));
    git(repo, &["checkout", "-q", "main"], &at(3));
    commit(repo, "CHANGELOG.md", "0.1\n", "Changelog", &at(3));
    git(repo, &["checkout", "-q", "feature-a"], &at(4));
    commit(
        repo,
        "src/login.rs",
        "fn login() { token() }\n",
        "Use token",
        &at(4),
    );
    git(repo, &["checkout", "-q", "main"], &at(5));
    commit(repo, "README.md", "readme\nmore\n", "Readme", &at(5));
    git(repo, &["commit-graph", "write", "--reachable"], &at(5));
    dir
}

/// The app on `repo` at `columns`×`rows`, cursor on `main`, pane closed.
fn on_main(repo: &Path, columns: u16, rows: u16) -> (App, AppState) {
    let (app, state) = App::open_with_clock(
        repo,
        mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW),
    );
    let state = app.resize(&state, columns, rows);
    let state = crate::preview::opened(&app, state);
    assert!(!state.detail.shown, "the pane starts closed");
    (app, state)
}

/// `x`, `j`, then pump until the compare's file list lands.
fn walk(app: &App, state: &AppState) -> (AppState, Vec<&'static str>) {
    let route = vec!["x", "j"];
    let state = app.key(state, KeyCode::Char('x'));
    let state = app.key(&state, KeyCode::Char('j'));
    let state = app.pump(state, "the compare's file list", |state| {
        matches!(state.detail.spec, Some(DetailSpec::Compare { base, head }) if base != head)
            && matches!(state.detail.files, Loading::Ready(_))
    });
    (state, route)
}

fn frame(state: &AppState, columns: u16, rows: u16) -> Vec<String> {
    crate::captures::frame_at(
        state,
        columns,
        rows,
        Some(mergewright::session::Clock::Fixed(
            mergewright_fixtures::CORPUS_NOW,
        )),
    )
}

/// The journey, counted against the note (`pr-contents` = 2).
#[test]
fn pr_contents() {
    let dir = split_repo();
    let repo = dir.path();
    let (app, state) = on_main(repo, 187, 50);
    let (state, route) = walk(&app, &state);
    let allowed = super::noted_keys("pr-contents");
    assert!(
        route.len() <= allowed,
        "pr-contents took {} inputs {route:?}, the note allows {allowed}",
        route.len()
    );
    assert_eq!(
        state.detail.spec,
        Some(DetailSpec::Compare {
            base: oid(repo, "main"),
            head: oid(repo, "feature-a"),
        })
    );
    let screen = frame(&state, 187, 50).join("\n");
    assert!(
        screen.contains("changes on feature-a since it left main \u{b7} 3 commits"),
        "{screen}"
    );
    // m8-polish-and-ship/20 item 1: the tally is wired from the file list.
    assert!(
        screen.contains("3 commits \u{b7} 2 files +2 \u{2212}0"),
        "{screen}"
    );
    app.close();
}

/// The two captures. Right: the header's words (at 80x24 the ladder's cut,
/// numbers whole); the file list holds only feature-a's files
/// (`src/login.rs`, `src/token.rs`), never `README.md` or `CHANGELOG.md`.
/// Under 100 columns the pane is a tab and `x` leaves the keys on the
/// graph, so the narrow capture presses `Tab` to show it (not counted).
#[test]
fn capture_compare_since_split() {
    for (columns, rows) in [(187u16, 50u16), (80, 24)] {
        let dir = split_repo();
        let (app, state) = on_main(dir.path(), columns, rows);
        let (state, _) = walk(&app, &state);
        let state = if columns < 100 {
            app.key(&state, KeyCode::Tab)
        } else {
            state
        };
        let lines = frame(&state, columns, rows);
        let body = format!("{}\n", lines.join("\n"));
        crate::captures::check_shot(
            &format!("m7/compare-since-split-{columns}x{rows}.txt"),
            &body,
        );
        assert!(
            body.contains("changes on feature-a since it left main"),
            "{body}"
        );
        assert!(
            body.contains("login.rs") && body.contains("token.rs"),
            "{body}"
        );
        assert!(!body.contains("CHANGELOG"), "{body}");
        app.close();
    }
}

/// The screen columns (in cells) where `needle` starts and ends on `line`.
fn cells_of(line: &str, needle: &str) -> std::ops::Range<u16> {
    let byte = line
        .find(needle)
        .unwrap_or_else(|| panic!("{needle:?} not on {line:?}"));
    let start = line[..byte].chars().count();
    let end = start + needle.chars().count();
    u16::try_from(start).unwrap_or(u16::MAX)..u16::try_from(end).unwrap_or(u16::MAX)
}

/// `x` on main: its row reads `[x base]` first, before `[◆ main]`; the ▏
/// rule stays. A drag from main's own badge picks up `main`; the `x base`
/// badge is not a ref and picks up nothing.
#[test]
fn the_base_row_carries_x_base() {
    use mergewright_core::event::{MouseButton, MouseKind};

    let dir = split_repo();
    let (app, state) = on_main(dir.path(), 187, 50);
    let state = app.key(&state, KeyCode::Char('x'));
    let lines = frame(&state, 187, 50);
    let (row, line) = lines
        .iter()
        .enumerate()
        .find(|(_, line)| line.contains("[\u{25c6} main]"))
        .unwrap_or_else(|| panic!("main's row: {lines:#?}"));
    let base = line
        .find("[x base]")
        .unwrap_or_else(|| panic!("no x base on main's row: {line}"));
    assert!(
        base < line.find("[\u{25c6} main]").unwrap_or(0),
        "x base first: {line}"
    );
    assert!(line.contains('\u{258f}'), "the rule stays: {line}");
    let row = u16::try_from(row).unwrap_or(u16::MAX);
    let on_main = cells_of(line, "main]").start;
    let grabbed = app.mouse(&state, MouseKind::Down(MouseButton::Left), on_main, row);
    let source = grabbed
        .drag
        .as_ref()
        .unwrap_or_else(|| panic!("main's badge picks up a drag at {on_main}"));
    assert_eq!(source.source.short, "main");
    let on_base = cells_of(line, "x base").start;
    let pressed = app.mouse(&state, MouseKind::Down(MouseButton::Left), on_base, row);
    assert!(pressed.drag.is_none(), "x base is not a ref");
    app.close();
}
