//! `d` on a done file, in a merge Graft began: offered where ops can start
//! the file over, and answered in ops' words where it cannot
//! (m6-conflicts/28) — and the captures the user judges by eye.

use super::*;

/// `gone.txt`'s refusal in a merge begun in the shell, in ops' own words.
const CANNOT: &str = "can't start over · gone.txt: the conflict's sides were not kept";

/// `gone.txt` kept with `o`, the resolver on to `shared.rs`, then
/// `gone.txt` clicked open read-only.
fn gone_kept_and_opened(app: &App, state: &AppState) -> AppState {
    let state = app.key(state, KeyCode::Char('o'));
    let state = app.pump(state, "the keep, and shared.rs", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new("shared.rs") && open.job.is_none() && open.file.is_some()
            })
            && rows(state)
                .iter()
                .any(|(path, row)| path == "gone.txt" && *row == RowState::Done)
    });
    let state = done_open(app, &state, "gone.txt");
    // The status line (`resolved gone.txt (ours) ✓`) holds the bar's row
    // until the next key; `PgUp` at the top scrolls nowhere and gives it
    // back (`k` steps files since m6-conflicts/17).
    app.key(&state, KeyCode::PageUp)
}

/// `shared.rs` done ([`shared_done`]), then clicked open read-only.
fn shared_done_and_opened(app: &App, state: &AppState) -> AppState {
    let state = shared_done(app, state);
    let state = done_open(app, &state, "shared.rs");
    app.key(&state, KeyCode::PageUp)
}

/// `d` on `gone.txt` done (kept with `o`) in Graft's own merge: the box
/// names the side that survives, and `→ ⏎` puts the keep-or-delete choice
/// back — the card, the list row, stages 1 and 2, our bytes on disk — with
/// no journal line of its own (m6-conflicts/58).
///
/// What right looks like (`fix-start-over-whole-file-120x40.txt`): frame 1's
/// bottom row has `d start over`; frame 2 has `the choice comes back: keep
/// or delete` and `git checkout --ours -- gone.txt`, no `--merge`, cursor on
/// `[ Cancel ]`; frame 3's card reads `deleted by them · o keeps the file  t
/// deletes it` and the list row `keep or delete`.
#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one walk, three frames, each checked where it is drawn"
)]
fn d_on_a_done_modify_delete_file_reopens_its_card() {
    let scratch = super::super::restart::app_merge_with_a_whole_file_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = on_gone_card(&scratch);
    let state = gone_kept_and_opened(&app, &state);

    let done = crate::captures::frame(&state, 120, 40);
    let body = done.join("\n");
    assert_eq!(
        done[3].trim_end(),
        "▸ gone.txt · done ✓ · d starts it over",
        "{body}"
    );
    assert!(
        done.last()
            .is_some_and(|bar| bar.trim() == "d start over  e edit  esc close"),
        "{body}"
    );

    let entries = journal_entries(&repo).len();
    let state = app.key(&state, KeyCode::Char('d'));
    let boxed = crate::captures::frame(&state, 120, 40);
    let body = boxed.join("\n");
    let confirm = state.confirm.as_ref().expect("`d` opened the box");
    assert_eq!(confirm.view.cursor, 0, "on Cancel");
    assert!(
        body.contains("the choice comes back: keep or delete"),
        "{body}"
    );
    assert!(body.contains("git checkout --ours -- gone.txt"), "{body}");
    assert!(!body.contains("--merge"), "{body}");
    assert!(body.contains("[ Cancel ]"), "{body}");

    let state = app.key(&state, KeyCode::Right);
    let state = app.key(&state, KeyCode::Enter);
    assert!(state.operating.is_some(), "launched: {:?}", state.status);
    let state = app.pump(state, "the start over, and the card", |state| {
        state.operating.is_none()
            && state.status_read.is_none()
            && state.resolver.as_ref().is_some_and(|open| {
                open.path == Path::new("gone.txt")
                    && open.job.is_none()
                    && open.set_job.is_none()
                    && open.whole_file.is_some()
            })
            && rows(state)
                .iter()
                .any(|(path, row)| path == "gone.txt" && *row != RowState::Done)
    });
    let card = crate::captures::frame(&state, 120, 40);
    let body = card.join("\n");
    assert!(
        body.contains("deleted by them · o keeps the file  t deletes it"),
        "{body}"
    );
    assert!(
        card.iter()
            .any(|row| row.contains("gone.txt") && row.contains("keep or delete")),
        "{body}"
    );
    assert_eq!(
        crate::app_harness::git(&repo, &["ls-files", "-u", "--", "gone.txt"])
            .lines()
            .filter_map(|line| line.split_whitespace().nth(2))
            .collect::<Vec<_>>(),
        ["1", "2"],
        "stages 1 and 2 are back"
    );
    assert_eq!(
        std::fs::read_to_string(repo.join("gone.txt")).expect("gone.txt is on disk"),
        "our plan\n"
    );
    assert_eq!(journal_entries(&repo).len(), entries, "folded: no entry");

    crate::captures::check_shot(
        "m6/fix-start-over-whole-file-120x40.txt",
        &format!(
            "{}\n── d ──\n{}\n── → ⏎ ──\n{}\n",
            done.join("\n"),
            boxed.join("\n"),
            card.join("\n")
        ),
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// What right looks like — can: `▸ shared.rs · done ✓ · d starts it over`
/// over the file, bar `d start over  e edit  esc close`. Cannot: `▸
/// gone.txt · done ✓` and nothing after it, bar `e edit  esc close`; after
/// `d`, the status row says why. At 200 columns the same identity row
/// beside the list column, one card.
#[test]
fn done_start_over_capture() {
    capture_can();
    capture_cannot();
}

/// `shared.rs` done in Graft's own merge: `d` offered.
fn capture_can() {
    let scratch = super::super::restart::app_merge_with_a_whole_file_conflict();
    let (app, state) = on_gone_card(&scratch);
    let state = shared_done_and_opened(&app, &state);
    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert!(
        body.contains("▸ shared.rs · done ✓ · d starts it over"),
        "{body}"
    );
    assert!(
        frame
            .last()
            .is_some_and(|bar| bar.trim() == "d start over  e edit  esc close"),
        "{body}"
    );
    crate::captures::check_shot(
        "m6/fix-done-start-over-can-120x40.txt",
        &format!("{body}\n"),
    );
    drop(state);
    app.close();
    drop(scratch);
}

/// `gone.txt` kept in a merge **begun in the shell**: nothing kept its
/// stages, so no `d`, and `d` pressed says why. (Until m6-conflicts/58 this
/// was Graft's own merge, whose done whole-file file can now start over.)
fn capture_cannot() {
    let scratch = gone_and_notes_merge();
    let (app, state) = opened_on(&scratch);
    let state = gone_kept(&app, state);
    let state = done_open(&app, &state, "gone.txt");
    // As `gone_kept_and_opened`: `PgUp` gives the bar its row back.
    let state = app.key(&state, KeyCode::PageUp);
    let before = crate::captures::frame(&state, 120, 40);
    let after = crate::captures::frame(&app.key(&state, KeyCode::Char('d')), 120, 40);
    assert_eq!(before[3].trim_end(), "▸ gone.txt · done ✓");
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
