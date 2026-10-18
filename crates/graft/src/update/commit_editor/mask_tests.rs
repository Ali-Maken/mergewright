//! A secrets-looking file's values never reach the screen (08s Q1): its
//! diff is masked as it arrives, in or out; any other file's is not.

use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_ui::detail::{DiffView, LineKindView};

use super::screen_tests::{key, mock_tree, walk};
use super::*;
use crate::update::{AppEvent, Product, update};

/// `path`'s diff as git prints it: a hunk whose heading git took from a
/// line of the file (its default funcname picks `KEY=value` lines), one
/// context line, and a removed/added pair (word-tinted).
fn git_diff(path: &str) -> graft_git::diff::FileDiff {
    let text = format!(
        "diff --git a/{path} b/{path}\n\
         --- a/{path}\n\
         +++ b/{path}\n\
         @@ -4,2 +4,2 @@ OLD_KEY=heading-value\n \
         KEEP=context-value\n\
         -TOKEN=removed-value\n\
         +TOKEN=added-value\n"
    );
    graft_git::diff::parse_unified(text.as_bytes())
        .expect("a well-formed diff")
        .remove(0)
}

/// The newest diff read in `commands`, answered with `path`'s diff the way
/// the job assembles it (word tint on).
fn answered(state: &AppState, commands: &[Command], path: &str) -> AppState {
    let (id, key) = commands
        .iter()
        .rev()
        .find_map(|command| match command {
            Command::ReadFileDiff(id, key, _) => Some((*id, *key)),
            _ => None,
        })
        .unwrap_or_else(|| panic!("a diff read: {commands:?}"));
    let raw = git_diff(path);
    let view = crate::jobs::detail::assemble_diff(&raw, true);
    let event = AppEvent::Produced(
        id,
        Product::FileDiff(key, Ok((Arc::new(view), Arc::new(raw)))),
    );
    update(state, event).0
}

/// The diff the screen holds for the cursor file.
fn shown(state: &AppState) -> Arc<DiffView> {
    let editor = state.commit_editor.as_ref().expect("the screen is open");
    match &editor.diff.view {
        Some(Ok(view)) => Arc::clone(view),
        other => panic!("no diff on show: {other:?}"),
    }
}

/// Every line's text, heading first, as `(kind, text)`.
fn lines(view: &DiffView) -> Vec<(LineKindView, String)> {
    view.hunks
        .iter()
        .flat_map(|hunk| &hunk.lines)
        .map(|line| (line.kind, line.text.as_str().to_owned()))
        .collect()
}

/// Nothing of the values is left anywhere on the screen's model: not in the
/// state's diff, not in what the view hands graft-ui.
fn assert_masked(state: &AppState) {
    let view = shown(state);
    assert_eq!(
        lines(&view),
        [
            (LineKindView::HunkHeading, "OLD_KEY=••••••••".to_owned()),
            (LineKindView::Context, "KEEP=••••••••".to_owned()),
            (LineKindView::Removed, "TOKEN=••••••••".to_owned()),
            (LineKindView::Added, "TOKEN=••••••••".to_owned()),
        ]
    );
    assert_eq!(view.hunks[0].heading.as_str(), "OLD_KEY=••••••••");
    // Word spans and syntax spans index the old text: gone with it.
    assert!(
        view.hunks[0]
            .lines
            .iter()
            .all(|line| line.words.is_empty() && line.syntax.is_empty()),
        "{view:?}"
    );
    let drawn = format!("{:?}", crate::view::commit_screen_view(state));
    assert!(!drawn.contains("-value"), "{drawn}");
}

#[test]
fn a_secrets_row_is_masked_in_and_out() {
    let open = super::screen_tests::opened(&mock_tree());
    // `.env` is the fourth row, staying out.
    let (on_env, commands) = walk(
        &open,
        vec![key(KeyCode::Down), key(KeyCode::Down), key(KeyCode::Down)],
    );
    let out = answered(&on_env, &commands, ".env");
    assert_masked(&out);
    // `⇥ ⇥` to the files, `space` puts it in; `k` and `j` read it again.
    let (went_in, _) = walk(
        &out,
        vec![
            key(KeyCode::Tab),
            key(KeyCode::Tab),
            key(KeyCode::Char(' ')),
        ],
    );
    let (again, commands) = walk(
        &went_in,
        vec![key(KeyCode::Char('k')), key(KeyCode::Char('j'))],
    );
    let editor = again.commit_editor.as_ref().expect("open");
    assert!(
        editor.files[editor.file_cursor].goes_in,
        "`.env` goes in now"
    );
    assert_masked(&answered(&again, &commands, ".env"));
}

#[test]
fn an_ordinary_file_is_not_masked() {
    let (open, commands) = update(&mock_tree(), key(KeyCode::Char('c')));
    let state = answered(&open, &commands, "docs/toc.md");
    let view = shown(&state);
    assert_eq!(
        lines(&view),
        [
            (
                LineKindView::HunkHeading,
                "OLD_KEY=heading-value".to_owned()
            ),
            (LineKindView::Context, "KEEP=context-value".to_owned()),
            (LineKindView::Removed, "TOKEN=removed-value".to_owned()),
            (LineKindView::Added, "TOKEN=added-value".to_owned()),
        ]
    );
    // Exactly what the job assembled: spans and all.
    let raw = git_diff("docs/toc.md");
    assert_eq!(*view, crate::jobs::detail::assemble_diff(&raw, true));
    assert!(
        view.hunks[0]
            .lines
            .iter()
            .any(|line| !line.syntax.is_empty()),
        "syntax spans kept: {view:?}"
    );
}
