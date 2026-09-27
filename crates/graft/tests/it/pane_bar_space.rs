//! The detail pane's bar and `space` on a commit's file list
//! (m7-power-tools/20h): offered on a directory row, which it folds; not on a
//! file row, where it does nothing. Real `git`, the real job pool, the real
//! `view::view`.

use graft::update::AppState;
use graft::update::detail::{DetailSpec, Loading};
use graft_core::event::KeyCode;
use graft_ui::detail::{DetailFocus, FileRowKind};

use crate::app_harness::App;
use crate::preview::{FORK_NOW, opened};

/// The kind of row the files cursor is on.
fn cursor_kind(state: &AppState) -> FileRowKind {
    match &state.detail.files {
        Loading::Ready(files) => files.rows[files.cursor].kind.clone(),
        _ => panic!("the file list is loaded"),
    }
}

/// `j` until the files cursor is on a row `want` accepts.
fn move_to(app: &App, mut state: AppState, want: fn(&FileRowKind) -> bool) -> AppState {
    for _ in 0..8 {
        if want(&cursor_kind(&state)) {
            return state;
        }
        state = app.key(&state, KeyCode::Char('j'));
    }
    panic!("no such row in the file list: {:?}", state.detail.files);
}

/// The frame at 120×40 and its hint bar (the row that offers `⏎ open`).
fn shot(app: &App, state: AppState) -> (AppState, Vec<String>, String) {
    let state = crate::captures::settled_frame(app, state);
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(graft::session::Clock::Fixed(FORK_NOW)),
    );
    let bar = frame
        .iter()
        .find(|row| row.contains("⏎ open"))
        .unwrap_or_else(|| panic!("a bar offering ⏎ open:\n{}", frame.join("\n")))
        .clone();
    (state, frame, bar)
}

/// `docs/shots/m7/pane-bar-file-vs-directory-120x40.txt`. What right looks
/// like: two frames of the detail pane focused on a commit's file list. On a
/// file row the bar has no `space`; on the `src/` directory row it reads
/// `… space fold …`, and nothing else in the two bars differs except what
/// the row changes (`B blame` needs a file). Colour unverified (rule 5).
#[test]
fn capture_the_pane_bar_on_a_file_and_on_a_directory() {
    let scratch = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Trivial);
    let repo = scratch.path();
    std::fs::create_dir_all(repo.join("src")).expect("the scratch repo is writable");
    for (name, body) in [
        ("src/a.rs", "fn a() {}\n"),
        ("src/b.rs", "fn b() {}\n"),
        ("notes.txt", "a\n"),
    ] {
        std::fs::write(repo.join(name), body).expect("the scratch repo is writable");
    }
    graft_fixtures::git::run_at(repo, &["add", "."], "@1600000000 +0000")
        .unwrap_or_else(|why| panic!("adding: {why}"));
    graft_fixtures::git::run_at(repo, &["commit", "-m", "add src"], "@1600000060 +0000")
        .unwrap_or_else(|why| panic!("committing: {why}"));

    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    // `⏎` opens the pane on the commit under the cursor, its file list focused.
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the commit's file list", |state| {
        matches!(state.detail.spec, Some(DetailSpec::Commit(_)))
            && matches!(state.detail.files, Loading::Ready(_))
    });
    assert_eq!(state.detail.focus, DetailFocus::Files);

    let state = move_to(&app, state, |kind| matches!(kind, FileRowKind::File { .. }));
    let (state, on_file, file_bar) = shot(&app, state);
    let state = move_to(&app, state, |kind| matches!(kind, FileRowKind::Dir { .. }));
    let (state, on_dir, dir_bar) = shot(&app, state);

    assert!(!file_bar.contains("space"), "file row: {file_bar}");
    assert!(dir_bar.contains("space fold"), "directory row: {dir_bar}");
    // And the key does what the bar says: the directory folds.
    let folded = app.key(&state, KeyCode::Char(' '));
    assert_eq!(cursor_kind(&folded), FileRowKind::Dir { collapsed: true });

    let body = format!(
        "== the files cursor on a file row: no `space` ==\n{}\n\n\
         == the files cursor on the `src/` directory row: `space fold` ==\n{}\n",
        on_file.join("\n"),
        on_dir.join("\n")
    );
    crate::captures::check_shot("m7/pane-bar-file-vs-directory-120x40.txt", &body);
    drop(folded);
    app.close();
}
