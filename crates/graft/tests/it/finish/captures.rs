//! The finish summary's captures (m6-conflicts/35).

use super::choices::{advanced_to, c_on_the_banner, gone_kept_last, stopped_on_gone_and_notes};
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
    let state = c_on_the_banner(&app, state);

    closed_captures(&state, &repo);
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
            body.contains("Finish · merging feature → main · nothing is written until ⏎"),
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
        let bar = frame.last().expect("a bar");
        assert!(bar.trim_start().starts_with("⏎ commit"), "{columns}: {bar}");
        crate::captures::check_shot(
            &format!("m6/fix-finish-summary-{columns}x{rows}.txt"),
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
    let under: Vec<&String> = frame
        .iter()
        .skip(at + 1)
        .take_while(|line| !line.contains("MESSAGE"))
        .filter(|line| !line.trim().is_empty())
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
