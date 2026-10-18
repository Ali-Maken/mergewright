//! The rows are the commit (08s M2, M3), over real `git`: amend turned on
//! from `c` amends with the rows shown, turned off on `a` commits them,
//! and a partly staged file shows and commits only the part that goes in.

use std::path::Path;

use mergewright::update::AppState;
use mergewright_core::event::{KeyCode, KeyPress, Modifiers};

use super::{committed_files, opened, ready, three_changed_and_a_secret, typed};
use crate::app_harness::{App, git};

fn ctrl(c: char) -> KeyPress {
    KeyPress {
        code: KeyCode::Char(c),
        modifiers: Modifiers {
            control: true,
            ..Modifiers::NONE
        },
    }
}

/// `⌃P`, `Amend the last commit`, `⏎`: the option flipped from the palette.
pub(super) fn amend_toggled(app: &App, state: &AppState) -> AppState {
    let palette = app.press(state, ctrl('p'));
    app.key(
        &typed(app, &palette, "Amend the last commit"),
        KeyCode::Enter,
    )
}

/// The screen as drawn at 120×40, one string per row.
fn drawn(state: &AppState) -> Vec<String> {
    crate::captures::frame_at(state, 120, 40, None)
}

fn short_head(repo: &Path) -> String {
    git(repo, &["rev-parse", "--short=7", "HEAD"])
        .trim()
        .to_owned()
}

#[test]
fn amend_from_ctrl_p_amends_with_the_rows_shown() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    let head = short_head(repo);
    let (app, state) = App::open(repo);
    let state = opened(&app, &ready(&app, state));
    let state = amend_toggled(&app, &typed(&app, &state, "fix toc"));
    assert!(
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.amend)
    );
    let frame = drawn(&state).join("\n");
    assert!(frame.contains(&format!("amending {head} start")), "{frame}");
    assert!(frame.contains("into the last commit."), "{frame}");
    let _landed: AppState = app.settle_write(app.key(&state, KeyCode::Enter), "the amend");
    assert_eq!(git(repo, &["log", "-1", "--format=%s"]).trim(), "fix toc");
    // The root was rewritten, not built on: still one commit, no parent.
    assert_eq!(git(repo, &["rev-list", "--count", "HEAD"]).trim(), "1");
    let tree = git(repo, &["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(!tree.lines().any(|path| path == ".env"), "{tree}");
    for (path, text) in [
        ("docs/toc.md", "# Contents\n\n- one\n"),
        ("README.md", "readme\nmore\n"),
        ("src/a.rs", "fn a() {}\nfn b() {}\n"),
    ] {
        assert_eq!(
            git(repo, &["show", &format!("HEAD:{path}")]),
            text,
            "{path}"
        );
    }
    app.close();
}

#[test]
fn amend_turned_off_commits_the_rows_shown() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    git(repo, &["add", "docs/toc.md"]);
    let start = git(repo, &["rev-parse", "HEAD"]).trim().to_owned();
    let (app, state) = App::open(repo);
    let state = app.key(&ready(&app, state), KeyCode::Char('a'));
    let state = app.pump(state, "the amend screen and its counts", |state| {
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| editor.counts.is_some())
    });
    let state = amend_toggled(&app, &state);
    assert!(
        state
            .commit_editor
            .as_ref()
            .is_some_and(|editor| !editor.amend)
    );
    let frame = drawn(&state).join("\n");
    assert!(frame.contains("committing to main"), "{frame}");
    assert!(frame.contains("as a new commit on main."), "{frame}");
    let _landed: AppState = app.settle_write(app.key(&state, KeyCode::Enter), "the commit");
    assert_eq!(
        git(repo, &["rev-parse", "HEAD~1"]).trim(),
        start,
        "built on"
    );
    assert_eq!(committed_files(repo), ["docs/toc.md"]);
    app.close();
}

#[test]
fn a_partly_staged_file_shows_and_commits_only_its_staged_part() {
    let scratch = three_changed_and_a_secret();
    let repo = scratch.path();
    git(repo, &["add", "src/a.rs"]);
    std::fs::write(repo.join("src/a.rs"), "fn a() {}\nfn b() {}\nfn c() {}\n").expect("writable");
    let (app, state) = App::open(repo);
    let state = opened(&app, &ready(&app, state));
    let frame = drawn(&state);
    let row = frame
        .iter()
        .find(|line| line.contains("src/a.rs"))
        .unwrap_or_else(|| panic!("{}", frame.join("\n")));
    let files_column: String = row.chars().take(44).collect();
    assert!(files_column.trim_end().ends_with("+1 part"), "{row}");
    let frame = frame.join("\n");
    assert!(frame.contains("1 file goes in, +1 \u{2212}0"), "{frame}");
    assert!(
        frame.contains("only the part you staged goes in; the rest stays uncommitted."),
        "{frame}"
    );
    assert!(frame.contains("fn b() {}"), "{frame}");
    assert!(
        !frame.contains("fn c() {}"),
        "the unstaged line is not shown: {frame}"
    );
    let state = typed(&app, &state, "add b");
    let _landed: AppState = app.settle_write(app.key(&state, KeyCode::Enter), "the commit");
    let shown = git(repo, &["show", "--format=", "HEAD", "--", "src/a.rs"]);
    assert!(shown.contains("+fn b() {}"), "{shown}");
    assert!(!shown.contains("fn c"), "{shown}");
    let file = std::fs::read_to_string(repo.join("src/a.rs")).expect("readable");
    assert!(file.contains("fn c() {}"), "the rest stays: {file}");
    app.close();
}
