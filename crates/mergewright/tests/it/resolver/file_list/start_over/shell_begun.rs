//! Start over in a merge begun in the shell: nothing kept the stages, so
//! `d` is refused in ops' words.

use super::*;

/// `gone.txt`'s refusal in a merge begun in the shell, in ops' own words.
const CANNOT: &str =
    "git can't bring back the conflict in gone.txt: the conflict's sides were not kept";

/// `gone.txt` kept in a merge **begun in the shell**: nothing kept its
/// stages, so no `d`, and `d` pressed says why. (Until m6-conflicts/58 this
/// was Mergewright's own merge, whose done whole-file file can now start over.)
pub(super) fn capture_cannot() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let state = done_open(&app, &state, "gone.txt");
    // As `gone_kept_and_opened`: `PgUp` gives the bar its row back.
    let state = app.key(&state, KeyCode::PageUp);
    let before = crate::captures::frame(&state, 120, 40);
    let after = crate::captures::frame(&app.key(&state, KeyCode::Char('d')), 120, 40);
    assert_eq!(
        crate::captures::doc_rows(&before)[1].trim_end(),
        "▸ gone.txt · done ✓"
    );
    assert!(
        before
            .last()
            .is_some_and(|bar| bar.trim() == "e edit  esc close")
    );
    assert!(
        after.iter().any(|row| row.trim().starts_with(CANNOT)),
        "{}",
        after.join("\n")
    );
    crate::captures::check_shot(
        "m6/fix-done-start-over-cannot-120x40.txt",
        &format!(
            "{}\n── after d ──\n{}\n",
            before.join("\n"),
            after.join("\n")
        ),
    );
    let wide = crate::captures::frame(&state, 200, 50);
    let body = wide.join("\n");
    assert!(body.contains("▸ gone.txt · done ✓"), "{body}");
    assert!(!body.contains("d starts it over"), "{body}");
    assert!(
        wide.last()
            .is_some_and(|bar| bar.trim() == "e edit  esc close"),
        "{body}"
    );
    crate::captures::check_shot(
        "m6/fix-done-start-over-cannot-200x50.txt",
        &format!("{body}\n"),
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// Where the conflict can't come back (a merge begun in the shell kept no
/// stages), `d` answers in words naming the file and the reason, and writes
/// nothing (m7-power-tools/17).
#[test]
fn a_restart_git_cannot_do_is_refused_in_words() {
    let scratch = gone_and_notes_merge();
    let repo = scratch.path().to_owned();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let state = done_open(&app, &state, "gone.txt");
    let staged = crate::app_harness::git(&repo, &["ls-files", "-s"]);
    let state = app.key(&state, KeyCode::Char('d'));
    assert!(state.confirm.is_none());
    assert!(state.operating.is_none());
    let frame = crate::captures::frame(&state, 120, 40);
    assert!(
        frame.iter().any(|row| row.trim().starts_with(CANNOT)),
        "{}",
        frame.join("\n")
    );
    assert_eq!(crate::app_harness::git(&repo, &["ls-files", "-s"]), staged);
    drop(state);
    app.close();
    drop(scratch);
}
