//! The history place on real `git` (m7-power-tools/21): what a trace does
//! at the edges — a depth-1 clone, a broken `.git/shallow`. The journeys
//! are `m7_walk::history`.

use graft::update::place::{FileLoad, TraceLoad};
use graft_core::event::KeyCode;
use graft_git::trace::{SHALLOW_END, TraceEnd};

use graft::update::AppState;
use graft_ui::history::HistoryFocus;

use crate::app_harness::App;
use crate::m7_walk::blame::{blame, drawn, on_line_22, open_on_the_file, session};
use crate::m7_walk::history::{history, traced};

/// The `List` form's bar, as the frame draws it at 140 columns and wider.
const LIST_BAR: &str = " j/k older\u{b7}newer  tab file  n/N next change  B blame here  \u{23ce} that commit  esc back";

/// `B L` from the diff cursor on line 22 at `columns`×`rows`, traced and
/// the selected commit's file read.
fn lines_history(columns: u16, rows: u16) -> (graft_fixtures::Scratch, App, AppState) {
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
/// 120 the focused pane alone); the bar offers `tab file`.
#[test]
fn capture_history_100x30() {
    let (_scratch, app, state) = lines_history(100, 30);
    let state = crate::captures::settled_frame(&app, state);
    assert_eq!(history(&state).focus, HistoryFocus::List);
    let frame = shoot(&state, 100, 30, "history-100x30.txt");
    assert!(bar_row(&frame).contains("tab file"), "{}", bar_row(&frame));
    app.close();
}

/// `docs/shots/m7/esc-back-to-history-from-the-graph-120x40.txt`. Two
/// frames. (a) After `⏎` from history and `Tab` to the graph: the graph
/// shown, the bar leads `esc back to history`. (b) After one `Esc`: the
/// history place again, the same commit selected.
#[test]
fn capture_esc_back_to_history_from_the_graph() {
    use graft_ui::detail::DetailFocus;
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
fn depth_one() -> (graft_fixtures::Scratch, tempfile::TempDir) {
    let source = graft_fixtures::copy_into_temp(graft_fixtures::Shape::Session);
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
    assert_eq!(bar_row(&drawn(&state, 187, 50)), " esc back");
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
    assert_eq!(bar_row(&drawn(&state, 140, 40)), " esc back");
    let back = app.key(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "Esc: blame again");
    app.close();
}
