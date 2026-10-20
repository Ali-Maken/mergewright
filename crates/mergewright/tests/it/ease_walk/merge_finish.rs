//! M11–M19: the merge mock's bars, pick undo, edit by hand, pause, review,
//! look and landing on the mock-shaped repository, one test per id. Words
//! and shapes only (rule 5).

use std::path::PathBuf;

use mergewright::update::Command;
use mergewright_core::event::{KeyCode, KeyPress};

use super::flow::{
    at_clash, committed, head_parents, pick, read_in, reviewed, stopped, unmerged, wall,
};
use super::screen::{bar, drawn, line_with, shown, status};
use crate::app_harness::{App, git, press_for_commands};

const CLASH_BAR: &str = "o take yours (left) · t take theirs (right) · b take both · e edit by \
                         hand · u undo pick · esc pause, come back later";
const SOLVED_BAR: &str = "y review the merge · u undo pick · esc pause, come back later";
const REVIEW_BAR: &str = "╰ y commit the merge · j/k move · ⏎ look at it · u undo last pick · \
                          esc back to the clashes (y brings you here again) ─";

/// `integration` stopped, `o` taken, `y`: the review.
fn on_the_review() -> (tempfile::TempDir, App, mergewright::update::AppState) {
    let (scratch, app, state) = stopped("integration");
    let state = pick(&app, &state, 'o');
    let state = reviewed(&app, &state);
    assert!(state.finish.is_some(), "`y` opens the review");
    (scratch, app, state)
}

/// M11: the three bars — an open clash, every clash solved, the review.
#[test]
fn m11_the_three_bars() {
    let (_scratch, app, state) = stopped("integration");
    assert_eq!(bar(&drawn(&state)).trim(), CLASH_BAR);
    let solved = pick(&app, &state, 'o');
    // The pick's done line holds the bottom row until the next key; the
    // review and back (M16) shows the bar with every clash solved.
    let review = reviewed(&app, &solved);
    line_with(&drawn(&review), REVIEW_BAR);
    let back = app.key(&review, KeyCode::Escape);
    let frame = drawn(&back);
    assert!(
        bar(&frame).trim().starts_with(SOLVED_BAR),
        "{:?}",
        bar(&frame)
    );
    app.close();
}

/// M12: `u` puts the last pick back open with the cursor on it, on the clash
/// screen and from the review.
#[test]
fn m12_u_undoes_the_last_pick_from_the_screen_and_the_review() {
    let (scratch, app, state) = stopped("pay-307");
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'u');
    assert_eq!(at_clash(&state), ("notes.md".to_owned(), Some(0)));
    assert!(line_with(&drawn(&state), "notes.md   2 open").contains('?'));
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'o');
    let state = pick(&app, &state, 'n');
    let state = pick(&app, &state, 'o');
    let review = reviewed(&app, &state);
    assert!(review.finish.is_some());
    let undone = pick(&app, &review, 'u');
    assert!(undone.finish.is_none(), "back on the clash screen");
    assert_eq!(at_clash(&undone), ("report.md".to_owned(), Some(0)));
    assert!(
        std::fs::read_to_string(scratch.path().join("report.md"))
            .expect("readable")
            .contains("<<<<<<<"),
        "the clash is open again on disk"
    );
    line_with(
        &drawn(&undone),
        "✓ undid the last pick in report.md · 1 of 1 open",
    );
    app.close();
}

/// M13: `e` hands the file to `$EDITOR` at the cursor's clash.
#[test]
fn m13_e_edits_by_hand_at_the_cursor_block() {
    let (_scratch, app, state) = stopped("integration");
    let (_, commands) = press_for_commands(&state, KeyPress::plain(KeyCode::Char('e')));
    let aimed = commands.iter().find_map(|command| match command {
        Command::OpenExternalEditor { path, line } => Some((path.clone(), *line)),
        _ => None,
    });
    assert_eq!(
        aimed,
        Some((PathBuf::from("report.md"), Some(5))),
        "{commands:?}"
    );
    app.close();
}

/// M14: `esc` pauses: the graph, the banner focused, its bar leading `⏎ back
/// to the clashes`; `⏎` comes back to the first open clash.
#[test]
fn m14_esc_pauses_and_enter_comes_back() {
    let (_scratch, app, state) = stopped("integration");
    let paused = app.key(&state, KeyCode::Escape);
    assert!(paused.resolver.is_none() && paused.banner_focus);
    let frame = drawn(&paused);
    assert_eq!(
        first_key(bar(&frame)),
        "⏎ back to the clashes",
        "{:?}",
        bar(&frame)
    );
    let back = app.pump(app.key(&paused, KeyCode::Enter), "the clash", |state| {
        state
            .resolver
            .as_ref()
            .is_some_and(|open| open.file.is_some() && open.set_job.is_none())
    });
    crate::clash_stops::on_the_first_clash(&back);
    app.close();
}

/// The banner bar's first key hint: the bar opens with the stop's words
/// (`merge stopped · 1 file conflicted · `), then its keys, two spaces
/// apart.
pub fn first_key(bar: &str) -> &str {
    let keys = bar.rsplit(" · ").next().unwrap_or_default();
    keys.split("  ").next().unwrap_or_default().trim()
}

/// M15: the review's words and bar.
#[test]
fn m15_review_words_and_bar() {
    let (_scratch, app, state) = on_the_review();
    let frame = drawn(&state);
    for words in [
        "│ 1 clash solved. Nothing committed yet.",
        "│ This merge will put into wallet-release:",
        "│   1 commit from integration",
        "│ ▸ ✓ report.md  you chose: ◀ yours",
        "│ Message  Merge branch 'integration' into wallet-release",
        REVIEW_BAR,
    ] {
        line_with(&frame, words);
    }
    app.close();
}

/// M16: `esc` on the review goes back to the clashes; `y` brings the review
/// back.
#[test]
fn m16_review_esc_back_and_y_again() {
    let (_scratch, app, state) = on_the_review();
    let back = app.key(&state, KeyCode::Escape);
    assert!(
        back.finish.is_none() && back.resolver.is_some(),
        "the clashes"
    );
    line_with(&drawn(&back), "Clashing files");
    let again = reviewed(&app, &back);
    line_with(&drawn(&again), "│ 1 clash solved. Nothing committed yet.");
    app.close();
}

/// M17: `⏎` on the review is a read-only look; on the combined row, per file
/// `yours +a −d theirs +a −d` and a sample marked `from theirs`.
#[test]
fn m17_enter_looks_read_only_and_the_combined_row() {
    let (_scratch, app, state) = stopped("pay-307");
    let mut state = state;
    for key in ['o', 'o', 'n', 'o'] {
        state = pick(&app, &state, key);
    }
    let review = reviewed(&app, &state);
    let look = read_in(&app, app.key(&review, KeyCode::Enter));
    let frame = drawn(&look);
    line_with(&frame, "│ notes.md · as it will land · read only");
    line_with(&frame, "╰ esc back to the review · y commit the merge ─");
    let mut on = review;
    for _ in 0..3 {
        on = app.key(&on, KeyCode::Char('j'));
    }
    line_with(&drawn(&on), "│ ▸ ✓ 1 file git combined on its own");
    let combined = read_in(&app, app.key(&on, KeyCode::Enter));
    let frame = drawn(&combined);
    let counts = line_with(&frame, "✓ checks.md");
    assert!(
        counts.contains("yours +1 −1") && counts.contains("theirs +1 −1"),
        "{counts:?}"
    );
    assert!(
        line_with(&frame, "Check line 1, by Omar").contains("from theirs"),
        "{}",
        shown(&frame)
    );
    app.close();
}

/// M18: the commit lands on the merge commit, folded `▸ 1 commit from
/// integration`, `✓ Merged …`, bar `u undo the merge · P push · … space show
/// the 1 commit inside`.
#[test]
fn m18_commit_lands_on_the_folded_merge() {
    let (scratch, app, state) = on_the_review();
    let state = committed(&app, &state);
    assert_eq!(head_parents(scratch.path()), 2);
    assert!(
        super::screen::cursor_on_head(&state),
        "the cursor on the merge"
    );
    let state = super::screen::sized(&app, &state, 200, super::screen::ROWS);
    let frame = wall(&state);
    let row = line_with(&frame, "Merge branch 'integration' into wallet-release");
    assert!(row.starts_with('▸'), "the cursor's row: {row:?}");
    assert!(row.contains("▸ 1 commit from integration"), "{row:?}");
    assert_eq!(
        status(&frame).trim(),
        "✓ Merged integration into wallet-release."
    );
    let bar = bar(&frame).trim();
    assert!(
        bar.starts_with("u undo the merge · P push · ")
            && bar.contains("· space show the 1 commit inside"),
        "{bar:?}"
    );
    app.close();
}

/// M19: `u` after the merge undoes it.
#[test]
fn m19_u_undoes_the_merge() {
    let (scratch, app, state) = on_the_review();
    let before = git(scratch.path(), &["rev-parse", "HEAD"]);
    let state = committed(&app, &state);
    assert_ne!(git(scratch.path(), &["rev-parse", "HEAD"]), before);
    let undone = app.settle_write(app.key(&state, KeyCode::Char('u')), "the undo");
    assert_eq!(
        git(scratch.path(), &["rev-parse", "HEAD"]),
        before,
        "HEAD is back"
    );
    assert!(unmerged(scratch.path()).is_empty());
    assert!(!scratch.path().join(".git/MERGE_HEAD").exists());
    assert!(status(&drawn(&undone)).contains("undo: merge integration into wallet-release"));
    app.close();
}
