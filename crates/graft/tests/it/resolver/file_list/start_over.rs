//! `d` on a done file, in a merge Graft began: offered where ops can start
//! the file over, and answered in ops' words where it cannot
//! (m6-conflicts/28) — and the captures the user judges by eye.

use super::*;

/// `gone.txt`'s refusal, in ops' own words.
const CANNOT: &str = "can't start over · gone.txt: deleted by them — no markers to take from";

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
    // until the next key; `k` at the top scrolls nowhere and gives it back.
    app.key(&state, KeyCode::Char('k'))
}

/// `shared.rs` done ([`shared_done`]), then clicked open read-only.
fn shared_done_and_opened(app: &App, state: &AppState) -> AppState {
    let state = shared_done(app, state);
    let state = done_open(app, &state, "shared.rs");
    app.key(&state, KeyCode::Char('k'))
}

/// The journal file's bytes, read without opening it (opening cuts a torn
/// tail, which is a write).
fn journal_bytes(repo: &Path) -> Option<Vec<u8>> {
    let path = crate::app_harness::git(
        repo,
        &["rev-parse", "--git-path", "graft/undo-journal.jsonl"],
    );
    std::fs::read(repo.join(path.trim())).ok()
}

/// The status line's reason, or a panic naming what it was.
fn reason(state: &AppState) -> &str {
    match &state.status {
        Status::Sticky { reason, .. } => reason,
        other => panic!("expected a sticky line, got {other:?}"),
    }
}

#[test]
fn a_whole_file_done_file_offers_no_start_over() {
    let scratch = super::super::restart::app_merge_with_a_whole_file_conflict();
    let repo = scratch.path().to_owned();
    let (app, state) = on_gone_card(&scratch);
    let state = gone_kept_and_opened(&app, &state);

    let frame = crate::captures::frame(&state, 120, 40);
    let body = frame.join("\n");
    assert_eq!(frame[3].trim_end(), "▸ gone.txt · done ✓", "{body}");
    assert!(
        frame
            .last()
            .is_some_and(|bar| bar.trim() == "e edit  esc close"),
        "{body}"
    );

    let status = crate::app_harness::git(&repo, &["status", "--porcelain"]);
    let journal = journal_bytes(&repo);
    let state = app.key(&state, KeyCode::Char('d'));
    assert_eq!(reason(&state), CANNOT);
    assert!(state.confirm.is_none(), "no box");
    assert!(state.operating.is_none(), "no write");
    assert_eq!(
        crate::app_harness::git(&repo, &["status", "--porcelain"]),
        status,
        "git status unchanged"
    );
    assert_eq!(journal_bytes(&repo), journal, "the journal unchanged");
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

/// `gone.txt` kept in Graft's own merge: no `d`, and `d` pressed says why.
fn capture_cannot() {
    let scratch = super::super::restart::app_merge_with_a_whole_file_conflict();
    let (app, state) = on_gone_card(&scratch);
    let state = gone_kept_and_opened(&app, &state);
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
