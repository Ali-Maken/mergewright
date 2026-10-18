//! The line cursor in a commit's diff (m7-power-tools/20b): the same `▎`
//! the changes pane draws, moved by `j`. Real `git`, the real job pool, the
//! real `view::view`.

use mergewright::update::detail::{DetailSpec, Loading, Side, diff_cursor};
use mergewright_core::event::KeyCode;
use mergewright_ui::detail::DetailFocus;

use crate::app_harness::App;
use crate::preview::{FORK_NOW, dated_commit, opened};

/// The line cursor's glyph (`mergewright_ui::detail_pane::line_render::CURSOR_GLYPH`).
const CURSOR: char = '\u{258e}';

/// `docs/shots/m7/diff-cursor-commit-120x40.txt`. What right looks like: the
/// top commit's diff (`notes.txt`: `b` became `B`, `g` added), the cursor on
/// the added `+B` line after `⏎ ⏎ j j j` — that row carries `▎` beside its
/// sign, and no other row on screen does. Colour unverified (rule 5).
#[test]
fn capture_the_commit_diff_cursor() {
    let scratch = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Trivial);
    let repo = scratch.path();
    dated_commit(repo, "notes.txt", "a\nb\nc\nd\ne\nf\n", "add notes", 1);
    dated_commit(repo, "notes.txt", "a\nB\nc\nd\ne\nf\ng\n", "edit notes", 2);
    let (app, state) = App::open(repo);
    let state = app.resize(&state, 120, 40);
    let state = opened(&app, state);
    // `⏎` opens the pane on the commit under the cursor, `⏎` again the diff.
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "the commit's diff", |state| {
        matches!(state.detail.spec, Some(DetailSpec::Commit(_)))
            && matches!(state.detail.diff, Loading::Ready(_))
    });
    let mut state = app.key(&state, KeyCode::Enter);
    assert_eq!(state.detail.focus, DetailFocus::Diff);
    for _ in 0..3 {
        state = app.key(&state, KeyCode::Char('j'));
    }
    let under = diff_cursor(&state).expect("the diff has a cursor");
    let at = under.at.expect("on a line");
    assert_eq!((at.side, at.line), (Side::New, 2), "the added `B`");
    let state = crate::captures::settled_frame(&app, state);
    let frame = crate::captures::frame_at(
        &state,
        120,
        40,
        Some(mergewright::session::Clock::Fixed(FORK_NOW)),
    );
    let marked: Vec<&String> = frame.iter().filter(|row| row.contains(CURSOR)).collect();
    assert_eq!(marked.len(), 1, "one cursor row:\n{}", frame.join("\n"));
    assert!(
        marked[0].contains(&format!("{CURSOR}+ B")),
        "the cursor is on the added line: {}",
        marked[0]
    );
    let body = format!("{}\n", frame.join("\n"));
    crate::captures::check_shot("m7/diff-cursor-commit-120x40.txt", &body);
    drop(state);
    app.close();
}
