//! The history place on real `git` (m7-power-tools/21): what a trace does
//! at the edges — a depth-1 clone, a broken `.git/shallow`. The journeys
//! are `m7_walk::history`.

use mergewright::update::place::{FileLoad, TraceLoad};
use mergewright_core::event::KeyCode;
use mergewright_git::trace::{SHALLOW_END, TraceEnd};

use mergewright::update::AppState;
use mergewright_ui::history::HistoryFocus;

use crate::app_harness::App;
use crate::m7_walk::blame::{blame, drawn, on_line_22, open_on_the_file, session};
use crate::m7_walk::history::{history, traced};

/// The `List` form's bar, as the frame draws it at 140 columns and wider.
const LIST_BAR: &str = " j/k older\u{b7}newer  ⇥ file  n/N next change  B blame here  \u{23ce} that commit  esc back  ? keys";

/// `B L` from the diff cursor on line 22 at `columns`×`rows`, traced and
/// the selected commit's file read.
fn lines_history(columns: u16, rows: u16) -> (mergewright_fixtures::Scratch, App, AppState) {
    let scratch = session();
    let (app, state) = on_line_22(scratch.path(), columns, rows);
    let state = blame(&app, &state);
    let state = app.key(&state, KeyCode::Char('L'));
    let state = traced(&app, state);
    (scratch, app, state)
}

/// The frame's last row: the place's bar.
fn bar_row(frame: &[String]) -> &str {
    frame.last().map_or("", String::as_str).trim_end()
}

/// The frame, checked against `docs/shots/m7/<name>`.
fn shoot(state: &AppState, columns: u16, rows: u16, name: &str) -> Vec<String> {
    let frame = drawn(state, columns, rows);
    crate::captures::check_shot(&format!("m7/{name}"), &format!("{}\n", frame.join("\n")));
    frame
}

/// m7-power-tools/21b: the history place reaches the real frame —
/// `view::view` draws it over the graph (`AppState::graph_overlay`), title
/// on row 0, its own bar on the last.
#[test]
fn a_history_place_is_drawn_in_the_frame() {
    let (_scratch, app, state) = lines_history(187, 50);
    let frame = drawn(&state, 187, 50);
    assert!(
        frame[0].trim_start().starts_with("HISTORY \u{b7} "),
        "the title row:\n{}",
        frame.join("\n")
    );
    assert_eq!(bar_row(&frame), LIST_BAR.trim_end());
    app.close();
}

/// `docs/shots/m7/history-lines-187x50.txt`. Right: the commits that
/// changed `refresh_token`, `▸` on Lina's (the commit blame showed); her
/// whole file on the right, `┃` on the function's lines; bar = the list
/// form.
#[test]
fn capture_history_lines_187x50() {
    let (_scratch, app, state) = lines_history(187, 50);
    let state = crate::captures::settled_frame(&app, state);
    let frame = shoot(&state, 187, 50, "history-lines-187x50.txt");
    assert!(
        frame.iter().any(|row| row.contains('\u{2503}')),
        "the `┃` range"
    );
    assert_eq!(bar_row(&frame), LIST_BAR.trim_end());
    app.close();
}

/// `docs/shots/m7/history-file-140x40.txt`. Right: after `F` on the file's
/// row — every commit that changed it, a `↳ was src/session.rs` row for
/// Priya's rename; no `┃` (a file has no range).
#[test]
fn capture_history_file_140x40() {
    let scratch = session();
    let (app, state) = open_on_the_file(scratch.path(), 140, 40, false);
    let state = app.key(&state, KeyCode::Char('F'));
    let state = traced(&app, state);
    let state = crate::captures::settled_frame(&app, state);
    let frame = shoot(&state, 140, 40, "history-file-140x40.txt");
    let all = frame.join("\n");
    assert!(all.contains("was src/session.rs"), "the rename row:\n{all}");
    assert!(!all.contains('\u{2503}'), "no range:\n{all}");
    assert_eq!(bar_row(&frame), LIST_BAR.trim_end());
    app.close();
}

/// `docs/shots/m7/history-100x30.txt`. Right: one pane, the list (below
/// 120 the focused pane alone); the bar offers `⇥ file`.
#[test]
fn capture_history_100x30() {
    let (_scratch, app, state) = lines_history(100, 30);
    let state = crate::captures::settled_frame(&app, state);
    assert_eq!(history(&state).focus, HistoryFocus::List);
    let frame = shoot(&state, 100, 30, "history-100x30.txt");
    assert!(bar_row(&frame).contains("⇥ file"), "{}", bar_row(&frame));
    app.close();
}

/// `docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt`. Two
/// frames. (a) After `⏎` from history and `Tab` to the graph: the graph
/// shown, the bar leads `esc back to history`. (b) After one `Esc`: the
/// history place again, the same commit selected.
#[test]
fn capture_esc_back_to_history_from_the_graph() {
    use mergewright_ui::detail::DetailFocus;
    let (_scratch, app, state) = lines_history(120, 40);
    let selected = history(&state).selected;
    let state = app.key(&state, KeyCode::Enter);
    let mut state = crate::captures::settled_frame(&app, state);
    for _ in 0..3 {
        if state.detail.focus == DetailFocus::Graph {
            break;
        }
        state = app.key(&state, KeyCode::Tab);
    }
    assert_eq!(state.detail.focus, DetailFocus::Graph, "`Tab` to the graph");
    assert!(state.trail.hidden(), "history waits");
    let graph = drawn(&state, 120, 40);
    let lead = graph
        .iter()
        .find(|row| row.contains(" esc "))
        .unwrap_or_else(|| panic!("a bar:\n{}", graph.join("\n")));
    assert!(
        lead.trim_start().starts_with("esc back to history"),
        "(a) the bar leads esc back to history: {lead}"
    );
    let state = app.key(&state, KeyCode::Escape);
    assert_eq!(history(&state).selected, selected, "the same commit");
    let back = drawn(&state, 120, 40);
    assert!(
        back[0].trim_start().starts_with("HISTORY \u{b7} "),
        "(b) history again"
    );
    let body = format!(
        "Right: (a) `B L ⏎` then `Tab` to the graph — the bar leads `esc back \
         to history`.\n{}\n\nRight: (b) one `Esc` — the history place again, \
         the same commit selected.\n{}\n",
        graph.join("\n"),
        back.join("\n")
    );
    crate::captures::check_shot("m7/esc-back-to-history-from-the-graph-120x40.txt", &body);
    app.close();
}

/// A `git clone --depth 1` of the session fixture: `main` alone.
fn depth_one() -> (mergewright_fixtures::Scratch, tempfile::TempDir) {
    let source = mergewright_fixtures::copy_into_temp(mergewright_fixtures::Shape::Session);
    let clone = tempfile::tempdir().unwrap_or_else(|why| panic!("a temporary directory: {why}"));
    let url = format!("file://{}", source.path().display());
    crate::app_harness::git(
        clone.path(),
        &["clone", "--quiet", "--depth", "1", &url, "clone"],
    );
    (source, clone)
}

/// `F` on a depth-1 clone: the trace ends at the clone's edge, said in the
/// blame place's words; no panic.
#[test]
fn a_depth_one_clone_shows_the_end_row_alone() {
    let (_source, dir) = depth_one();
    let clone = dir.path().join("clone");
    let (app, state) = open_on_the_file(&clone, 187, 50, false);
    let state = app.key(&state, KeyCode::Char('F'));
    let state = traced(&app, state);
    let place = history(&state);
    let TraceLoad::Ready { rows, end } = &place.trace else {
        panic!("not traced: {:?}", place.trace);
    };
    assert_eq!(end, &TraceEnd::GitStops(SHALLOW_END.to_owned()), "{rows:?}");
    // The reader gives no rows on a depth-1 clone (review 18bcd Minor 6).
    assert!(rows.is_empty(), "no rows: {rows:?}");
    assert_eq!(place.selected, None);
    assert_eq!(place.file, FileLoad::Idle);
    // 21b: nothing acts but the way back, and the drawn bar says only that.
    assert_eq!(bar_row(&drawn(&state, 187, 50)), " esc back  ? keys");
    let back = app.key(&state, KeyCode::Escape);
    assert!(!back.trail.is_live(), "Esc goes back");
    app.close();
}

/// A `.git/shallow` that is a directory: git can't read the clone's edge,
/// and the trace fails — drawn as the list's failure, not a panic, not the
/// block fallback.
#[test]
fn a_trace_error_is_drawn_as_a_failure() {
    let scratch = session();
    let repo = scratch.path();
    let (app, state) = on_line_22(repo, 140, 40);
    let state = blame(&app, &state);
    std::fs::create_dir(repo.join(".git").join("shallow"))
        .unwrap_or_else(|why| panic!("a directory for .git/shallow: {why}"));
    let state = app.key(&state, KeyCode::Char('L'));
    let state = traced(&app, state);
    let place = history(&state);
    assert!(
        matches!(&place.trace, TraceLoad::Failed(words) if !words.is_empty()),
        "{:?}",
        place.trace
    );
    // 21b: nothing acts but the way back, and the drawn bar says only that.
    assert_eq!(bar_row(&drawn(&state, 140, 40)), " esc back  ? keys");
    let back = app.key(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "Esc: blame again");
    app.close();
}

/// 21c: a text commit's whole file comes with no size read.
#[test]
fn a_text_commit_reads_no_sizes() {
    let (_scratch, app, state) = lines_history(140, 40);
    match &history(&state).file {
        FileLoad::Ready { view, sizes, .. } => {
            assert!(view.binary.is_none(), "a text file");
            assert_eq!(sizes, &None, "no second read");
        }
        other => panic!("not read: {other:?}"),
    }
    app.close();
}

/// Bytes of `logo.bin` before and after the fixture's growing commit.
const LOGO_BEFORE: usize = 600;
const LOGO_AFTER: usize = 1500;

/// The session fixture with `logo.bin` added at [`LOGO_BEFORE`] bytes, then
/// grown to [`LOGO_AFTER`] on `main`: binary (it holds NULs).
fn with_a_binary() -> mergewright_fixtures::Scratch {
    let scratch = session();
    let repo = scratch.path();
    for (bytes, subject) in [(LOGO_BEFORE, "Add logo"), (LOGO_AFTER, "Grow logo")] {
        let content: Vec<u8> = b"\0\x01\x02\x03\x04\x05\x06"
            .iter()
            .copied()
            .cycle()
            .take(bytes)
            .collect();
        std::fs::write(repo.join("logo.bin"), content)
            .unwrap_or_else(|why| panic!("writing logo.bin: {why}"));
        crate::app_harness::git(repo, &["add", "logo.bin"]);
        // Both dates fixed, so the shas (and the capture) do not drift.
        let out = std::process::Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["commit", "-q", "-m", subject])
            .env("GIT_AUTHOR_NAME", "Test Maker")
            .env("GIT_AUTHOR_EMAIL", "test@example.invalid")
            .env("GIT_COMMITTER_NAME", "Test Maker")
            .env("GIT_COMMITTER_EMAIL", "test@example.invalid")
            .env("GIT_AUTHOR_DATE", "@1578000000 +0000")
            .env("GIT_COMMITTER_DATE", "@1578000000 +0000")
            .output()
            .unwrap_or_else(|why| panic!("git commit: {why}"));
        assert!(
            out.status.success(),
            "git commit: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    crate::app_harness::git(repo, &["commit-graph", "write", "--reachable"]);
    scratch
}

/// `docs/shots/m7/history-binary-140x40.txt`. Right: `F` on `logo.bin` in
/// `main`'s commit — the file pane reads `binary · 600 B → 2 KB` (600 and
/// 1 500 bytes, `size()`'s words), no line rows.
#[test]
fn capture_history_binary_140x40() {
    use mergewright::update::detail::{DetailSpec, Loading};
    let scratch = with_a_binary();
    let (app, state) = mergewright_open(scratch.path());
    let state = app.resize(&state, 140, 40);
    let state = crate::preview::opened(&app, state);
    let state = app.pump(state, "the repository's facts", |state| {
        state.repository.is_some() && state.system_git.is_some()
    });
    let state = app.key(&state, KeyCode::Enter);
    let state = app.pump(state, "main's file list and diff", |state| {
        matches!(state.detail.spec, Some(DetailSpec::Commit(_)))
            && matches!(state.detail.files, Loading::Ready(_))
            && matches!(state.detail.diff, Loading::Ready(_))
    });
    let state = app.key(&state, KeyCode::Char('F'));
    let state = traced(&app, state);
    let place = history(&state);
    match &place.file {
        FileLoad::Ready { view, sizes, .. } => {
            assert!(view.binary.is_some(), "binary");
            assert_eq!(
                sizes,
                &Some(Ok(mergewright_git::trace::BlobSizes {
                    old: u64::try_from(LOGO_BEFORE).ok(),
                    new: u64::try_from(LOGO_AFTER).ok(),
                }))
            );
        }
        other => panic!("not read: {other:?}"),
    }
    let state = crate::captures::settled_frame(&app, state);
    let frame = shoot(&state, 140, 40, "history-binary-140x40.txt");
    let all = frame.join("\n");
    assert!(
        all.contains("binary \u{b7} 600 B \u{2192} 2 KB"),
        "the sizes:\n{all}"
    );
    app.close();
}

/// The app on `repo`, its clock the corpus's.
fn mergewright_open(repo: &std::path::Path) -> (App, AppState) {
    App::open_with_clock(
        repo,
        mergewright::session::Clock::Fixed(mergewright_fixtures::CORPUS_NOW),
    )
}
