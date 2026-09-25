//! The finish summary's captures (m6-conflicts/35).

use super::choices::{advanced_to, c_for_the_summary, gone_kept_last, stopped_on_gone_and_notes};
use super::*;

/// The finish summary the user judges by eye (m6-conflicts/19, built by
/// 35): `notes.txt` taken theirs in both blocks (`Ctrl+T ⏎`), `gone.txt`
/// kept yours (`o`), `README.md` combined by git. Colour unverified
/// (rule 5); the look is the user's (rule 6).
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

/// The three widths, nothing expanded: [`summary_captures`]' first half.
fn closed_captures(state: &AppState, repo: &Path) {
    let merge_msg = std::fs::read_to_string(repo.join(".git/MERGE_MSG")).expect("git wrote it");
    let first = merge_msg.lines().next().expect("a first line").to_owned();
    for (columns, rows) in [(120, 40), (160, 48), (200, 50)] {
        let frame = crate::captures::frame(state, columns, rows);
        let body = frame.join("\n");
        assert!(
            frame[1].starts_with("╭ Preview: finish merging feature into main ─"),
            "{columns}: {body}"
        );
        assert!(
            frame[2].starts_with("│ 1 merge commit on main · nothing is committed until y"),
            "{columns}: {body}"
        );
        assert!(body.contains("RESOLVED BY YOU · 2"), "{columns}: {body}");
        assert!(
            body.contains("COMBINED BY GIT ON ITS OWN · 1"),
            "{columns}: {body}"
        );
        let readme = frame
            .iter()
            .find(|line| line.contains("README.md"))
            .unwrap_or_else(|| panic!("{columns}: no README.md row: {body}"));
        assert!(
            readme.contains("▸ README.md")
                && readme.contains("modified")
                && readme.contains("+1 −1"),
            "{columns}: {readme}"
        );
        assert!(
            body.contains(&first),
            "{columns}: git's first line {first:?}"
        );
        // The keys are in the frame's bottom border; the app's last row
        // carries none while nothing is committing.
        let border = &frame[frame.len() - 2];
        assert!(
            border.starts_with("╰ y commit  ⏎/space diff"),
            "{columns}: {border}"
        );
        let bar = frame.last().expect("a bar");
        assert!(bar.trim().is_empty(), "{columns}: {bar:?}");
        crate::captures::check_shot(
            &format!("m6/fix-finish-summary-{columns}x{rows}.txt"),
            &format!("{body}\n"),
        );
    }
}

/// The summary as a preview (m6-conflicts/59), at the two sizes the ticket
/// names: the frame's title, its two preview lines, `y commit` leading the
/// bottom border, and nothing on the app's last row. Colour unverified
/// (rule 5).
fn preview_captures(state: &AppState) {
    for (columns, rows) in [(187, 50), (80, 24)] {
        let frame = crate::captures::frame(state, columns, rows);
        let body = frame.join("\n");
        let top = frame
            .iter()
            .position(|line| line.starts_with('╭'))
            .unwrap_or_else(|| panic!("{columns}: no frame: {body}"));
        assert!(
            frame[top].starts_with("╭ Preview: finish merging feature into main ─"),
            "{columns}: {body}"
        );
        assert!(
            frame[top + 1].starts_with("│ 1 merge commit on main · nothing is committed until y"),
            "{columns}: {body}"
        );
        assert!(
            frame[top + 2].starts_with("│ undo: u takes this back"),
            "{columns}: {body}"
        );
        let border = &frame[frame.len() - 2];
        assert!(
            border.starts_with("╰ y commit  ⏎/space diff"),
            "{columns}: {border}"
        );
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
    // `space` on `README.md`, the cursor's row: its diff under it.
    let state = app.key(state, KeyCode::Char(' '));
    let state = app.pump(state, "the README.md diff", |state| {
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
        .position(|line| line.contains("▸ README.md"))
        .expect("the README.md row");
    // The first row under it is the hunk's heading row, blank here (git
    // gives this hunk no function context); the diff lines follow it.
    // Inside the frame: its `│ ` border and gutter come off first.
    let under: Vec<&str> = frame
        .iter()
        .skip(at + 1)
        .take_while(|line| !line.contains("MESSAGE"))
        .map(|line| line.strip_prefix("│ ").unwrap_or(line))
        .filter(|line| !line.trim_end_matches('│').trim().is_empty())
        .collect();
    assert!(
        !under.is_empty() && under.iter().all(|line| line.starts_with("    ")),
        "the diff sits under the row, indented: {body}"
    );
    assert!(
        under.iter().any(|line| line.contains("- line two"))
            && under.iter().any(|line| line.contains("+ line TWO")),
        "the change itself: {body}"
    );
    crate::captures::check_shot(
        "m6/fix-finish-summary-expanded-120x40.txt",
        &format!("{body}\n"),
    );
    state
}
