//! Blame's `⏎` and the trail's last `Esc` in a **partly staged** file
//! (m7-power-tools/20m A). Real `git`, the real job pool.
//!
//! The fixture: `f.txt`, lines `l1`…`l10`, committed. Staged: `l8` → `S8`.
//! Then, unstaged: `U` inserted above `l3`, `l6` → `U6`. On disk `U` is line
//! 3, `U6` line 7, `S8` line 9 — but the Staged diff counts in the index,
//! where `S8` is line 8.
//!
//! The oracle is independent of the code under test: the diff row under the
//! cursor must read exactly line `L` of `f.txt` as `std::fs` reads it from
//! disk, and be an added line.
#![allow(
    clippy::expect_used,
    reason = "a test fixture that cannot be built has nothing to test; the panic names why"
)]

use std::path::Path;

use graft::update::AppState;
use graft::update::detail::{DetailSpec, Loading, Side, diff_cursor};
use graft_core::event::KeyCode;
use graft_ui::detail::{BodyRow, DetailFocus, LineKindView, WorkingTreeSection};

use crate::app_harness::{App, changes_pane, git};
use crate::m7_walk::blame::{blame, cursor_line, drawn};

/// The file every test here blames.
const FILE: &str = "f.txt";

/// The diff's line cursor glyph (`▎`).
const DIFF_CURSOR: char = '\u{258e}';

/// The fixture above, in a scratch copy of the trivial repository.
fn partly_staged() -> graft_fixtures::Scratch {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    let lines = |edit: &dyn Fn(u32) -> Vec<String>| -> String {
        (1..=10).flat_map(edit).map(|line| line + "\n").collect()
    };
    let write = |body: String| {
        std::fs::write(repo.join(FILE), body).expect("the scratch repo is writable");
    };
    write(lines(&|n| vec![format!("l{n}")]));
    git(repo, &["add", FILE]);
    // Dated a day before the corpus's clock, so the capture's age and id
    // are the same every run.
    let at = format!("{} +0000", graft_fixtures::CORPUS_NOW - 86_400);
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.name=Graft Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-m",
            "add f",
        ])
        .env("GIT_AUTHOR_DATE", &at)
        .env("GIT_COMMITTER_DATE", &at)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git commit: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    write(lines(&|n| match n {
        8 => vec!["S8".to_owned()],
        n => vec![format!("l{n}")],
    }));
    git(repo, &["add", FILE]);
    write(lines(&|n| match n {
        3 => vec!["U".to_owned(), "l3".to_owned()],
        6 => vec!["U6".to_owned()],
        8 => vec!["S8".to_owned()],
        n => vec![format!("l{n}")],
    }));
    // No commit-graph offer: it would take `B` (keymap §2.1).
    git(repo, &["commit-graph", "write", "--reachable"]);
    scratch
}

/// Line `line` of `f.txt` as it is on disk.
fn on_disk(repo: &Path, line: u32) -> String {
    let body = std::fs::read_to_string(repo.join(FILE)).expect("f.txt reads");
    let index = usize::try_from(line - 1).expect("small");
    body.lines()
        .nth(index)
        .unwrap_or_else(|| panic!("f.txt has no line {line}"))
        .to_owned()
}

/// The kind and text of the diff row under the line cursor.
fn cursor_row(state: &AppState) -> (LineKindView, String) {
    let Loading::Ready(view) = &state.detail.diff else {
        panic!("no diff on show: {:?}", state.detail.diff);
    };
    let row = state.detail.body_cursor().expect("a line cursor");
    let Some(BodyRow::Line { hunk, line }) = view.body_row(row) else {
        panic!("the cursor is on no line: {:?}", view.body_row(row));
    };
    let line = &view.hunks[hunk].lines[line];
    (line.kind, line.text.as_str().to_owned())
}

/// The changes pane, its diff arrived, the files cursor on `f.txt` in
/// `section`.
fn on_the_file(app: &App, state: AppState, section: WorkingTreeSection) -> AppState {
    let mut state = changes_pane(app, state);
    for _ in 0..8 {
        if state.detail.selected_section() == Some(section)
            && diff_cursor(&state).is_some_and(|cursor| cursor.path == Path::new(FILE))
        {
            return app.pump(state, "the file's diff", |state| {
                matches!(state.detail.diff, Loading::Ready(_))
            });
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("no {section:?} row for {FILE}: {:?}", state.detail.files);
}

/// `j`/`k` in blame until its cursor is on `line`.
fn blame_to(app: &App, mut state: AppState, line: u32) -> AppState {
    for _ in 0..20 {
        let at = cursor_line(&state).final_line;
        if at == line {
            return state;
        }
        let key = if at < line { 'j' } else { 'k' };
        state = app.key(&state, KeyCode::Char(key));
    }
    panic!("blame's cursor never reached line {line}");
}

/// Pump until no landing is pending and the pane's diff is on show.
fn landed(app: &App, state: AppState) -> AppState {
    app.pump(state, "the landing", |state| {
        state.line_landing.is_none() && matches!(state.detail.diff, Loading::Ready(_))
    })
}

/// `B` on the file (from the Staged row), blame's cursor to disk `line`,
/// `⏎`, and the landing.
fn enter_on(repo: &Path, line: u32) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = on_the_file(&app, state, WorkingTreeSection::Staged);
    let state = blame(&app, &state);
    let state = blame_to(&app, state, line);
    assert_eq!(
        cursor_line(&state).commit,
        None,
        "line {line} is uncommitted"
    );
    let state = app.key(&state, KeyCode::Enter);
    let state = landed(&app, state);
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    (app, state)
}

#[test]
fn enter_on_an_unstaged_line_of_a_partly_staged_file_lands_on_it() {
    let scratch = partly_staged();
    for line in [3, 7] {
        let (app, state) = enter_on(scratch.path(), line);
        assert_eq!(
            state.detail.selected_section(),
            Some(WorkingTreeSection::Unstaged),
            "line {line}: the Unstaged row"
        );
        assert_eq!(
            cursor_row(&state),
            (LineKindView::Added, on_disk(scratch.path(), line)),
            "line {line}: the cursor on the disk line"
        );
        app.close();
    }
}

/// Also writes `docs/shots/m7/blame-enter-partly-staged-120x40.txt`. Right:
/// the changes pane after `⏎` on disk line 9 (`S8`, staged only): the Staged
/// row selected; exactly one `▎`, on `+ S8`.
#[test]
fn enter_on_a_staged_line_of_a_partly_staged_file_lands_on_it() {
    let scratch = partly_staged();
    let (app, state) = enter_on(scratch.path(), 9);
    assert_eq!(
        state.detail.selected_section(),
        Some(WorkingTreeSection::Staged),
        "the Staged row"
    );
    assert_eq!(
        cursor_row(&state),
        (LineKindView::Added, on_disk(scratch.path(), 9)),
        "the cursor on S8"
    );
    let state = crate::captures::settled_frame(&app, state);
    let frame = drawn(&state, 120, 40);
    let marked: Vec<&String> = frame
        .iter()
        .filter(|row| row.contains(DIFF_CURSOR))
        .collect();
    assert_eq!(marked.len(), 1, "one `▎`:\n{}", frame.join("\n"));
    assert!(
        marked[0].contains("\u{258e}+ S8"),
        "`▎` on `+ S8`: {}",
        marked[0]
    );
    crate::captures::check_shot(
        "m7/blame-enter-partly-staged-120x40.txt",
        &format!("{}\n", frame.join("\n")),
    );
    app.close();
}

/// The second case (20m A): `B` from `U6` in the Unstaged section, `⏎` on a
/// committed line, `Esc` back to blame, `Esc` out — back on the Unstaged
/// row, the cursor on `U6`.
#[test]
fn esc_restores_an_origin_in_the_unstaged_section() {
    let scratch = partly_staged();
    let (app, state) = App::open(scratch.path());
    let state = on_the_file(&app, state, WorkingTreeSection::Unstaged);
    let mut state = app.key(&state, KeyCode::Enter);
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    for _ in 0..20 {
        let at = diff_cursor(&state).and_then(|cursor| cursor.at);
        if at.is_some_and(|at| at.side == Side::New && at.line == 7) {
            break;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    assert_eq!(cursor_row(&state), (LineKindView::Added, "U6".to_owned()));

    let state = blame(&app, &state);
    let state = blame_to(&app, state, 1);
    assert!(cursor_line(&state).commit.is_some(), "line 1 is committed");
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the landing on the commit", |state| {
        state.line_landing.is_none()
            && matches!(state.detail.spec, Some(DetailSpec::Commit(_)))
            && matches!(state.detail.diff, Loading::Ready(_))
    });
    let state = app.key(&state, KeyCode::Escape);
    assert!(state.trail.on_screen().is_some(), "one `Esc` is blame");
    let state = app.key(&state, KeyCode::Escape);
    assert!(!state.trail.is_live(), "the second leaves it");
    let state = landed(&app, state);
    assert_eq!(state.detail.spec, Some(DetailSpec::WorkingTree));
    assert_eq!(
        state.detail.selected_section(),
        Some(WorkingTreeSection::Unstaged),
        "back on the Unstaged row"
    );
    assert_eq!(cursor_row(&state), (LineKindView::Added, "U6".to_owned()));
    app.close();
}
