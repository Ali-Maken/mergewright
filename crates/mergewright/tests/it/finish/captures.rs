//! The merge review's captures (m6-conflicts/35; the look since
//! ease-pass/12, mock `mergewright-merge-screen.html:155-162`).

use super::choices::{advanced_to, c_for_the_summary, gone_kept_last, stopped_on_gone_and_notes};
use super::*;

/// The review the user judges by eye: `notes.txt` taken theirs in both
/// blocks (`Ctrl+T ⏎`), `gone.txt` kept yours (`o`), `README.md` combined
/// by git. Colour unverified (rule 5); the look is the user's (rule 6).
#[test]
fn summary_captures() {
    let (scratch, app, state) = stopped_on_gone_and_notes();
    let repo = scratch.path().to_owned();
    let state = crate::resolver::resolver_on(&app, state, "notes.txt");
    let state = app.press(&state, KeyPress::control(KeyCode::Char('t')));
    let state = crate::resolver::settled(&app, state);
    let state = app.key(&state, KeyCode::Enter);
    let state = advanced_to(&app, state, "gone.txt");
    let state = gone_kept_last(&app, &state);
    let state = c_for_the_summary(&app, state);

    closed_captures(&state, &repo);
    preview_captures(&state);
    let state = expanded_capture(&app, &state);
    drop(state);
    app.close();
    drop(scratch);
}

/// The frame's lines as the mock reads them, each asserted at `columns`.
fn the_mock_lines(frame: &[String], columns: u16, first: &str) {
    let body = frame.join("\n");
    let has = |words: &str| {
        assert!(body.contains(words), "{columns}: no {words:?}:\n{body}");
    };
    has("All 3 clashes solved. Nothing committed yet.");
    has("This merge will put into main");
    has("1 commit from feature");
    has("✓ notes.txt  you chose: ▶ theirs");
    has("✓ gone.txt   you chose: ◀ yours, whole file");
    has("✓ 1 file git combined on its own");
    has(&format!("Message  {first}"));
    // The mock has no undo line (ticket decision 3, Minor 7).
    assert!(!body.contains("u takes this back"), "{columns}: {body}");
    let border = &frame[frame.len() - 2];
    assert!(
        border.starts_with("╰ y commit the merge · j/k move"),
        "{columns}: {border}"
    );
    let bar = frame.last().expect("a bar");
    assert!(bar.trim().is_empty(), "{columns}: {bar:?}");
}

/// Three widths, nothing expanded: [`summary_captures`]' first half.
fn closed_captures(state: &AppState, repo: &Path) {
    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    let first = merge_msg.lines().next().expect("a first line").to_owned();
    for (columns, rows) in [(120, 40), (160, 48), (200, 50)] {
        let frame = crate::captures::frame(state, columns, rows);
        the_mock_lines(&frame, columns, &first);
        crate::captures::check_shot(
            &format!("m6/fix-finish-summary-{columns}x{rows}.txt"),
            &format!("{}\n", frame.join("\n")),
        );
    }
}

/// The two sizes m6-conflicts/59 named, recaptured in the review's look.
fn preview_captures(state: &AppState) {
    for (columns, rows) in [(187, 50), (80, 24)] {
        let frame = crate::captures::frame(state, columns, rows);
        let body = frame.join("\n");
        assert!(
            body.contains("All 3 clashes solved. Nothing committed yet."),
            "{columns}: {body}"
        );
        assert!(!body.contains("u takes this back"), "{columns}: {body}");
        let last = frame.last().expect("a last row");
        assert!(last.trim().is_empty(), "{columns}: {last:?}");
        crate::captures::check_shot(
            &format!("m6/fix-summary-preview-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
}

/// `space` on the cursor's row: [`summary_captures`]' second half.
fn expanded_capture(app: &App, state: &AppState) -> AppState {
    // `j` from `gone.txt` (the first clashed file, git's order) to
    // `notes.txt`, then `space`: its change under it.
    let state = app.key(state, KeyCode::Char('j'));
    let state = app.key(&state, KeyCode::Char(' '));
    let state = app.pump(state, "the notes.txt diff", |state| {
        state
            .finish
            .as_ref()
            .is_some_and(|open| open.diff_job.is_none() && open.expanded.is_some())
    });
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(!body.contains("load all"), "no `L load all`: {body}");
    let at = frame
        .iter()
        .position(|line| line.contains("▸ ✓ notes.txt"))
        .unwrap_or_else(|| panic!("the notes.txt row: {body}"));
    // Inside the frame: its `│ ` border and gutter come off first.
    let under: Vec<&str> = frame
        .iter()
        .skip(at + 1)
        .take_while(|line| !line.contains("git combined on its own"))
        .map(|line| line.strip_prefix("│ ").unwrap_or(line))
        .filter(|line| !line.trim_end_matches('│').trim().is_empty())
        .collect();
    assert!(
        !under.is_empty() && under.iter().all(|line| line.starts_with("    ")),
        "the diff sits under the row, indented: {body}"
    );
    assert!(
        under.iter().any(|line| line.contains("+ TWO from feature")),
        "the change itself: {body}"
    );
    crate::captures::check_shot(
        "m6/fix-finish-summary-expanded-120x40.txt",
        &format!("{body}\n"),
    );
    state
}
