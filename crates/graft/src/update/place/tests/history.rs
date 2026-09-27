//! The history place (m7-power-tools/21), through `update` as the event
//! loop delivers keys and job answers: `L` in blame, `F` in the pane, its
//! keys, its `Esc`s, and the rows it draws.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::event::KeyCode;
use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::Rev;
use graft_git::trace::{BlobSizes, LineRange, SHALLOW_END, TraceCommit, TraceEnd};
use graft_term::sanitize::sanitize_line;
use graft_ui::detail::{DetailFocus, DiffView, FileStatusView, HunkView, LineKindView, LineView};
use graft_ui::history::{FilePane, HistoryFocus, HistoryList, HistoryRow, HistoryView, columns};

use crate::update::detail::Side;
use crate::update::events::WholeFileAnswer;

use super::super::history::next_change;
use super::super::history_keys::NO_MORE_CHANGES;
use super::*;

/// Priya's rename, the newest commit that touched the lines.
fn priya() -> Oid {
    Oid::from_bytes(&[0xd4; Oid::SHA1_BYTES]).expect("an oid")
}

/// The commit that added the file.
pub(super) fn adder() -> Oid {
    Oid::from_bytes(&[0xe5; Oid::SHA1_BYTES]).expect("an oid")
}

fn row(id: Oid, author: &str, range_at: Option<(u32, u32)>) -> TraceCommit {
    TraceCommit {
        id,
        author: author.to_owned(),
        when: 0,
        subject: format!("{author}'s change"),
        path: "a.rs".into(),
        renamed_from: None,
        added: 2,
        removed: 1,
        range_at,
    }
}

/// Priya's, Lina's, the adder's: newest first.
pub(super) fn three_rows() -> Vec<TraceCommit> {
    vec![
        row(priya(), "Priya", Some((20, 31))),
        row(lina(), "Lina", Some((18, 29))),
        row(adder(), "Ada", Some((10, 12))),
    ]
}

pub(super) fn refresh_token() -> LineRange {
    LineRange::Function {
        name: "    pub fn refresh_token(&mut self) -> Result<()> {".to_owned(),
        start: 18,
        end: 29,
    }
}

/// The blame place over `pane_at(22)`, cursor on line 22 (Lina's).
pub(super) fn blame_on_22() -> AppState {
    let mut state = pane_at(22);
    place_with(&mut state, a_blame(false, false), 21);
    state
}

/// `HEAD` on Lina's commit.
fn head_on_lina() -> Arc<crate::refs::RefBadges> {
    let refs = crate::refs::badges(
        &[],
        &graft_git::repo::HeadState::Detached(lina()),
        &std::collections::HashMap::new(),
        &[],
        |_, _| Err("no upstreams on this fixture".to_owned()),
    )
    .expect("badges");
    Arc::new(refs)
}

/// The changes pane, its files cursor on `a.rs` (unstaged); `HEAD` is
/// Lina's commit. `open.rs`' `changes_at`, without a diff: `F` on a file
/// row needs none.
fn changes_on_a_file() -> AppState {
    use graft_ui::detail::WorkingTreeSection;
    let mut detail =
        crate::update::detail::tests::pane_on(&[(WorkingTreeSection::Unstaged, &["a.rs"])], 1, 0);
    detail.focus = DetailFocus::Files;
    AppState {
        size: crate::update::Size {
            columns: 120,
            rows: 40,
        },
        detail,
        refs: Some(head_on_lina()),
        ..AppState::default()
    }
}

/// The trace job `commands` asks for.
pub(super) fn trace_job(commands: &[Command]) -> (JobId, TraceRequest) {
    commands
        .iter()
        .find_map(|command| match command {
            Command::TraceHistory { id, request } => Some((*id, (**request).clone())),
            _ => None,
        })
        .unwrap_or_else(|| panic!("expected a trace job, got {commands:?}"))
}

/// The whole-file read `commands` asks for: its id and commit.
fn file_job(commands: &[Command]) -> Option<(JobId, Oid)> {
    commands.iter().find_map(|command| match command {
        Command::ReadWholeFile { id, commit, .. } => Some((*id, *commit)),
        _ => None,
    })
}

pub(super) fn traced(
    state: &AppState,
    job: JobId,
    answer: Result<TraceAnswer, TraceFailure>,
) -> (AppState, Vec<Command>) {
    update(
        state,
        AppEvent::Produced(job, Product::Trace(answer.map(Arc::new))),
    )
}

fn file_read(state: &AppState, job: JobId, view: DiffView) -> (AppState, Vec<Command>) {
    file_read_sized(state, job, view, None)
}

/// A whole-file answer with its size read (binary only).
fn file_read_sized(
    state: &AppState,
    job: JobId,
    view: DiffView,
    sizes: Option<Result<BlobSizes, String>>,
) -> (AppState, Vec<Command>) {
    let answer = WholeFileAnswer {
        view: Arc::new(view),
        sizes,
    };
    update(
        state,
        AppEvent::Produced(job, Product::WholeFile(Ok(answer))),
    )
}

/// The whole-file read `commands` asks for: whether it ignores whitespace.
fn file_job_ignores_whitespace(commands: &[Command]) -> Option<bool> {
    commands.iter().find_map(|command| match command {
        Command::ReadWholeFile {
            ignore_whitespace, ..
        } => Some(*ignore_whitespace),
        _ => None,
    })
}

fn line(kind: LineKindView, old_no: Option<u32>, new_no: Option<u32>) -> LineView {
    LineView {
        kind,
        old_no,
        new_no,
        text: sanitize_line("x"),
        words: Vec::new(),
        syntax: Vec::new(),
        pair: None,
    }
}

/// A whole file of 60 lines: 1–60 kept, but line 19 added and a line
/// removed before new line 22, and line 45 added.
fn whole(binary: bool) -> DiffView {
    use LineKindView::{Added, Context, Removed};
    let mut lines = Vec::new();
    let mut old = 0;
    for new in 1..=60 {
        if new == 22 {
            old += 1;
            lines.push(line(Removed, Some(old), None));
        }
        if new == 19 || new == 45 {
            lines.push(line(Added, None, Some(new)));
        } else {
            old += 1;
            lines.push(line(Context, Some(old), Some(new)));
        }
    }
    DiffView {
        path: sanitize_line("a.rs"),
        old_path: None,
        status: FileStatusView::Modified,
        hunks: vec![HunkView {
            heading: sanitize_line(""),
            lines,
        }],
        binary: binary.then(|| sanitize_line("Binary files differ")),
        truncated: None,
        untracked: false,
    }
}

pub(super) fn history(state: &AppState) -> &HistoryState {
    state
        .trail
        .history_on_screen()
        .unwrap_or_else(|| panic!("no history on screen: {:?}", state.trail.places.len()))
}

/// `L` on line 22, the trace answered with `rows` and `range`, and (with
/// rows) the selected commit's file read.
pub(super) fn lines_history(rows: Vec<TraceCommit>, range: LineRange) -> AppState {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let answer = TraceAnswer {
        range: Some(range),
        rows,
        end: TraceEnd::Start { added_by: adder() },
    };
    let (state, commands) = traced(&opened, job, Ok(answer));
    match file_job(&commands) {
        Some((job, _)) => file_read(&state, job, whole(false)).0,
        None => state,
    }
}

/// The view the history place hands `graft_ui::history::draw_history`,
/// checked by `check`. `update` never draws (`update_cannot_reach_the_world`):
/// how each part of the view looks is `graft_ui::history`'s tests and
/// snapshots; these check what the place puts in it.
fn seen(state: &AppState, check: impl FnOnce(&HistoryView)) {
    let theme = crate::session::for_tests::session().theme;
    history(state).view(0, &theme, check);
}

/// The list's rows and end words, or a panic naming what the list is.
fn list_of<'a>(
    view: &'a HistoryView<'a>,
) -> (&'a [HistoryRow<'a>], Option<usize>, Option<&'a str>) {
    match &view.list {
        HistoryList::Rows {
            rows,
            selected,
            end,
            ..
        } => (rows, *selected, *end),
        HistoryList::Failed(words) => panic!("a failed list: {words}"),
    }
}

// ---- Open ----------------------------------------------------------------

#[test]
fn l_in_blame_opens_history_of_the_function() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, request) = trace_job(&commands);
    assert_eq!(
        request,
        TraceRequest::Lines {
            at: lina(),
            path: "a.rs".into(),
            line: 22,
            // Lines 6–30 are Lina's: 5 is Mo's.
            block: LineRange::Block { start: 6, end: 30 },
            from_disk: false,
        }
    );
    assert_eq!(opened.trail.places.len(), 2, "pushed over blame");
    assert!(matches!(history(&opened).trace, TraceLoad::Tracing { job: at } if at == job));
    let (state, _) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: three_rows(),
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    assert_eq!(
        history(&state).title(),
        "a.rs \u{b7} fn refresh_token, lines 18\u{2013}29"
    );
}

#[test]
fn l_with_no_function_uses_the_block_and_says_so() {
    let state = lines_history(three_rows(), LineRange::Block { start: 6, end: 30 });
    assert_eq!(
        history(&state).title(),
        "a.rs \u{b7} lines 6\u{2013}30 \u{b7} the block blame showed"
    );
}

#[test]
fn l_when_the_function_lookup_fails_draws_the_failure() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, commands) = traced(
        &opened,
        job,
        Err(TraceFailure::Failed("fatal: no such path".to_owned())),
    );
    assert!(file_job(&commands).is_none(), "no file to read");
    assert_eq!(
        history(&state).trace,
        TraceLoad::Failed("fatal: no such path".to_owned())
    );
    // `graft_ui::history` words it `couldn't trace: …`, on one row.
    seen(&state, |view| {
        assert_eq!(view.list, HistoryList::Failed("fatal: no such path"));
        assert!(!view.tracing, "the title drops tracing…");
        assert_eq!(view.file, FilePane::Empty);
    });
}

#[test]
fn l_on_an_uncommitted_line_says_so_and_opens_nothing() {
    let mut state = pane_at(22);
    place_with(&mut state, a_blame(false, true), 2);
    let (next, commands) = press(&state, KeyCode::Char('L'));
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::TraceHistory { .. })),
        "{commands:?}"
    );
    assert_eq!(next.trail.places.len(), 1, "blame only");
    assert_eq!(
        next.status,
        Status::sticky("line 3 isn't committed yet \u{b7} no history".to_owned())
    );
}

#[test]
fn a_working_tree_line_with_no_head_counterpart_says_so_and_pops() {
    let mut state = blame_on_22();
    state.refs = Some(head_on_lina());
    if let Some(Place::Blame(blame)) = state.trail.places.last_mut() {
        blame.rev = Rev::WorkingTree;
    }
    let (opened, commands) = press(&state, KeyCode::Char('L'));
    let (job, request) = trace_job(&commands);
    assert!(
        matches!(request, TraceRequest::Lines { at, from_disk: true, .. } if at == lina()),
        "traced from HEAD, the line mapped by the job: {request:?}"
    );
    let (next, _) = traced(&opened, job, Err(TraceFailure::Uncommitted(22)));
    assert_eq!(next.trail.places.len(), 1, "back on blame");
    assert!(next.trail.on_screen().is_some(), "blame is shown");
    assert_eq!(
        next.status,
        Status::sticky("line 22 isn't committed yet \u{b7} no history".to_owned())
    );
}

#[test]
fn the_list_cursor_starts_on_the_commit_blame_showed() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, commands) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: three_rows(),
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    assert_eq!(history(&state).selected, Some(1), "Lina's, not the newest");
    assert_eq!(file_job(&commands).map(|(_, commit)| commit), Some(lina()));
}

#[test]
fn f_on_a_file_row_opens_file_history() {
    let mut state = pane_at(22);
    state.detail.focus = DetailFocus::Files;
    let (commit, _) = pane_commit(&state);
    let (opened, commands) = press(&state, KeyCode::Char('F'));
    let (_, request) = trace_job(&commands);
    assert_eq!(
        request,
        TraceRequest::File {
            at: commit,
            path: "a.rs".into()
        }
    );
    assert!(!opened.focus, "not Focus mode");
    let place = history(&opened);
    assert_eq!(place.anchor, None, "a file row opens at the top");
    assert_eq!(opened.trail.places.len(), 1, "a fresh trail");
}

#[test]
fn f_on_a_diff_line_scrolls_the_file_pane_to_that_line() {
    let state = pane_at(22);
    let at = diff_cursor(&state)
        .and_then(|cursor| cursor.at)
        .expect("the cursor is on a line");
    assert_eq!(
        at.side,
        Side::New,
        "the fixture's row 22 is a new-side line"
    );
    let (opened, commands) = press(&state, KeyCode::Char('F'));
    let (job, _) = trace_job(&commands);
    assert_eq!(history(&opened).anchor, Some(at));
    let rows = vec![row(pane_commit(&state).0, "Lina", None)];
    let (traced_state, commands) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: None,
            rows,
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    let (file, _) = file_job(&commands).expect("a file read");
    let (state, _) = file_read(&traced_state, file, whole(false));
    let place = history(&state);
    assert_eq!(
        place.top_line(),
        Some(at.line),
        "the pane opens on his line"
    );
}

#[test]
fn f_in_the_changes_pane_opens_history_of_that_file() {
    let (opened, commands) = press(&changes_on_a_file(), KeyCode::Char('F'));
    let (_, request) = trace_job(&commands);
    assert_eq!(
        request,
        TraceRequest::File {
            at: lina(),
            path: "a.rs".into()
        },
        "HEAD's history of the file"
    );
    assert_eq!(
        history(&opened).anchor,
        None,
        "disk lines don't count in HEAD"
    );
}

#[test]
fn f_with_the_graph_focused_is_still_focus_mode() {
    let mut state = pane_at(22);
    state.detail.focus = DetailFocus::Graph;
    let (next, commands) = press(&state, KeyCode::Char('F'));
    assert!(next.focus, "Focus mode: {:?}", next.status);
    assert!(!next.trail.is_live(), "no history");
    assert!(
        !commands
            .iter()
            .any(|command| matches!(command, Command::TraceHistory { .. })),
        "{commands:?}"
    );
}

#[test]
fn f_with_the_pane_on_no_file_falls_through() {
    // The files cursor on `commit_pane`'s `src/` directory row.
    let mut state = commit_pane(JobId::new(1), true);
    state.detail.focus = DetailFocus::Files;
    let (next, commands) = press(&state, KeyCode::Char('F'));
    assert!(!next.trail.is_live(), "no history: {commands:?}");
}

// ---- Keys ----------------------------------------------------------------

#[test]
fn j_steps_older_and_the_file_pane_follows() {
    let state = lines_history(three_rows(), refresh_token());
    let reading = match history(&state).file {
        FileLoad::Ready { commit, .. } => commit,
        ref other => panic!("not ready: {other:?}"),
    };
    assert_eq!(reading, lina());
    let (next, commands) = press(&state, KeyCode::Char('j'));
    assert_eq!(history(&next).selected, Some(2), "one older");
    assert_eq!(file_job(&commands).map(|(_, commit)| commit), Some(adder()));
    assert!(matches!(history(&next).file, FileLoad::Reading { commit, .. } if commit == adder()));
    let (back, _) = press(&next, KeyCode::Char('k'));
    assert_eq!(history(&back).selected, Some(1), "k: newer");
}

#[test]
fn j_while_a_file_reads_cancels_the_read() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, commands) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: three_rows(),
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    let (reading, _) = file_job(&commands).expect("a read");
    let (next, commands) = press(&state, KeyCode::Char('j'));
    assert!(
        commands.contains(&Command::CancelJob(reading)),
        "{commands:?}"
    );
    // The stale read's answer lands on nothing.
    let (after, _) = file_read(&next, reading, whole(false));
    assert!(matches!(history(&after).file, FileLoad::Reading { commit, .. } if commit == adder()));
}

#[test]
fn the_range_stays_in_view_as_it_moves() {
    let state = lines_history(three_rows(), refresh_token());
    // Lina's range starts at 18: the pane opens a line above it.
    assert_eq!(history(&state).top_line(), Some(17));
    let (next, commands) = press(&state, KeyCode::Char('k'));
    let (job, commit) = file_job(&commands).expect("a read");
    assert_eq!(commit, priya());
    let (landed, _) = file_read(&next, job, whole(false));
    // Priya's range starts at 20.
    assert_eq!(history(&landed).top_line(), Some(19));
    assert_eq!(history(&landed).range_at(), Some((20, 31)));
}

#[test]
fn n_goes_to_the_next_change_from_either_focus() {
    let state = lines_history(three_rows(), refresh_token());
    let (next, _) = press(&state, KeyCode::Char('n'));
    // From 17 at the top, the next run is new line 19.
    assert_eq!(history(&next).top_line(), Some(19));
    let (tabbed, _) = press(&next, KeyCode::Tab);
    assert_eq!(history(&tabbed).focus, HistoryFocus::File);
    let (again, _) = press(&tabbed, KeyCode::Char('n'));
    // The removed row before 22 starts the next run: its top line is 22.
    assert_eq!(history(&again).top_line(), Some(22));
    let (back, _) = press(&again, KeyCode::Char('N'));
    assert_eq!(history(&back).top_line(), Some(19));
}

#[test]
fn n_past_the_last_change_says_so_and_stays() {
    let state = lines_history(three_rows(), refresh_token());
    let state = press_all(&state, &[KeyCode::Char('n'); 3]);
    let top = history(&state).file_top;
    let (next, _) = press(&state, KeyCode::Char('n'));
    assert_eq!(history(&next).file_top, top, "stays");
    assert_eq!(next.status, Status::Waiting(NO_MORE_CHANGES.to_owned()));
    assert_eq!(NO_MORE_CHANGES, "no more changes this way");
}

#[test]
fn next_change_matches_next_blocks_contract() {
    let changed = [false, true, true, false, false, true, false, true];
    // `from: None`: the first, or the last.
    assert_eq!(next_change(&changed, None, true), Some(1));
    assert_eq!(next_change(&changed, None, false), Some(7));
    // A run is one change: from inside it, the next run.
    assert_eq!(next_change(&changed, Some(1), true), Some(5));
    assert_eq!(next_change(&changed, Some(2), true), Some(5));
    assert_eq!(next_change(&changed, Some(5), false), Some(1));
    // Past the end, both ways: `None`, no wrap.
    assert_eq!(next_change(&changed, Some(7), true), None);
    assert_eq!(next_change(&changed, Some(1), false), None);
    assert_eq!(next_change(&changed, Some(0), false), None);
    // An all-`false` slice, and an empty one.
    assert_eq!(next_change(&[false; 4], None, true), None);
    assert_eq!(next_change(&[false; 4], Some(2), false), None);
    assert_eq!(next_change(&[], None, false), None);
}

#[test]
fn b_in_history_blames_that_commit_at_that_line() {
    let state = lines_history(three_rows(), refresh_token());
    let (blamed, commands) = press(&state, KeyCode::Char('B'));
    let (_, request) = blame_job(&commands);
    assert_eq!(request.rev, Rev::Commit(lina()));
    assert_eq!(request.path, PathBuf::from("a.rs"));
    assert_eq!(blamed.trail.places.len(), 3, "pushed over history");
    let top = blamed.trail.on_screen().expect("blame on screen");
    assert!(
        matches!(
            top.body,
            BlameLoad::Reading {
                want: Want::Line(17),
                ..
            }
        ),
        "at the pane's top line: {:?}",
        top.body
    );
}

/// m7-power-tools/21b: the frame draws the place for a history too
/// (`AppState::graph_overlay`, `view::frame::draw_screen`), tracing or read.
#[test]
fn graph_overlay_is_place_while_history_is_shown() {
    use crate::update::state::GraphOverlay;
    let (tracing, _) = press(&blame_on_22(), KeyCode::Char('L'));
    assert!(tracing.trail.history_on_screen().is_some());
    assert_eq!(
        tracing.graph_overlay(),
        Some(GraphOverlay::Place),
        "tracing"
    );
    let read = lines_history(three_rows(), refresh_token());
    assert_eq!(read.graph_overlay(), Some(GraphOverlay::Place), "read");
    let (shown, _) = press(&read, KeyCode::Enter);
    assert!(shown.trail.hidden());
    assert_ne!(shown.graph_overlay(), Some(GraphOverlay::Place), "hidden");
}

#[test]
fn esc_comes_back_to_history() {
    let state = lines_history(three_rows(), refresh_token());
    let (blamed, _) = press(&state, KeyCode::Char('B'));
    let (back, _) = press(&blamed, KeyCode::Escape);
    let place = history(&back);
    assert_eq!(place.selected, Some(1), "the same commit");
    assert_eq!(place.top_line(), Some(17), "the same line");
}

#[test]
fn enter_shows_the_commit_and_esc_comes_back() {
    let state = lines_history(three_rows(), refresh_token());
    let (shown, _) = press(&state, KeyCode::Enter);
    assert!(shown.trail.hidden(), "the trail waits under the graph");
    let landing = shown.line_landing.as_ref().expect("a landing on its way");
    assert_eq!(landing.spec, DetailSpec::Commit(lina()));
    let (back, _) = press(&shown, KeyCode::Escape);
    assert!(back.trail.history_on_screen().is_some(), "one Esc: history");
    assert_eq!(history(&back).selected, Some(1));
}

#[test]
fn esc_on_the_graph_with_the_pane_open_returns_to_history() {
    let state = lines_history(three_rows(), refresh_token());
    let (shown, _) = press(&state, KeyCode::Enter);
    let mut graph = shown;
    graph.detail.focus = DetailFocus::Graph;
    assert!(graph.detail.shown, "the pane stays open");
    assert!(esc_goes_back(&graph), "the predicate is place-generic");
    // The bar half (21b): the way back leads, named for history.
    let drawn = super::hints::bar(&graph);
    assert_eq!(
        drawn.first(),
        Some(&("esc", "back to history")),
        "{drawn:?}"
    );
    let (back, _) = press(&graph, KeyCode::Escape);
    let place = history(&back);
    assert_eq!(place.selected, Some(1), "the same row");
    assert_eq!(place.top_line(), Some(17), "the file pane at the same line");
}

#[test]
fn esc_during_trace_stops_git_and_goes_back() {
    let blame = blame_on_22();
    let (tracing, commands) = press(&blame, KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (back, commands) = press(&tracing, KeyCode::Escape);
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
    assert_eq!(back.trail.places.len(), 1, "popped");
    assert_eq!(cursor_line(&back), 22, "on the line he pressed L on");
}

#[test]
fn esc_during_an_f_trace_lands_where_f_was_pressed() {
    let state = pane_at(22);
    let before = diff_cursor(&state).and_then(|cursor| cursor.at);
    let (tracing, commands) = press(&state, KeyCode::Char('F'));
    let (job, _) = trace_job(&commands);
    let (back, commands) = press(&tracing, KeyCode::Escape);
    assert!(commands.contains(&Command::CancelJob(job)), "{commands:?}");
    assert!(!back.trail.is_live(), "the trail is over");
    assert_eq!(diff_cursor(&back).and_then(|cursor| cursor.at), before);
}

#[test]
fn a_trace_answer_after_esc_is_dropped() {
    let (tracing, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (back, _) = press(&tracing, KeyCode::Escape);
    let (late, commands) = traced(
        &back,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: three_rows(),
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    assert_eq!(late.trail.places.len(), 1, "nothing reopens");
    assert!(late.trail.on_screen().is_some(), "blame stays");
    assert!(file_job(&commands).is_none(), "{commands:?}");
}

#[test]
fn esc_from_history_returns_to_blame_same_line() {
    let state = lines_history(three_rows(), refresh_token());
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "blame");
    assert_eq!(cursor_line(&back), 22);
}

#[test]
fn tab_swaps_below_120() {
    let mut state = lines_history(three_rows(), refresh_token());
    state.size = crate::update::Size {
        columns: 100,
        rows: 30,
    };
    seen(&state, |view| {
        let panes = columns(100, view.focus);
        assert_eq!((panes.list.is_some(), panes.file.is_some()), (true, false));
    });
    let (tabbed, _) = press(&state, KeyCode::Tab);
    assert_eq!(history(&tabbed).focus, HistoryFocus::File);
    seen(&tabbed, |view| {
        let panes = columns(100, view.focus);
        assert_eq!((panes.list.is_some(), panes.file.is_some()), (false, true));
        assert!(
            matches!(view.file, FilePane::Lines { header, .. } if header.ends_with("this file +2 \u{2212}1")),
            "{:?}",
            view.file
        );
    });
    let (back, _) = press(&tabbed, KeyCode::Tab);
    assert_eq!(history(&back).focus, HistoryFocus::List);
}

#[test]
fn a_click_selects_and_a_double_click_is_enter() {
    use graft_core::event::{Event, Modifiers, MouseButton, MouseInput, MouseKind};
    let mut state = lines_history(three_rows(), refresh_token());
    state.size = crate::update::Size {
        columns: 187,
        rows: 50,
    };
    state.harness.clock = crate::session::Clock::Fixed(1_000);
    let click = AppEvent::Core(Event::Mouse(MouseInput {
        kind: MouseKind::Down(MouseButton::Left),
        column: 5,
        row: 1,
        modifiers: Modifiers::NONE,
    }));
    let (once, _) = update(&state, click.clone());
    assert_eq!(history(&once).selected, Some(0), "Priya's row");
    let (twice, _) = update(&once, click);
    assert!(twice.trail.hidden(), "a double-click is ⏎");
}

// ---- Rows ----------------------------------------------------------------

#[test]
fn a_rename_row_says_was() {
    let mut rows = three_rows();
    rows[0].renamed_from = Some("src/session.rs".into());
    let state = lines_history(rows, refresh_token());
    // `graft_ui::history` draws it `↳ was src/session.rs` under the row.
    seen(&state, |view| {
        let (rows, _, _) = list_of(view);
        assert_eq!(rows[0].was, Some("src/session.rs"));
        assert_eq!(rows[1].was, None);
    });
}

#[test]
fn the_last_row_says_the_file_starts_here() {
    let state = lines_history(three_rows(), refresh_token());
    let added = format!("the file starts here ({} added it)", short(adder()));
    seen(&state, |view| {
        assert_eq!(list_of(view).2, Some(added.as_str()));
    });
}

#[test]
fn the_cap_row_says_how_many() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, _) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: three_rows(),
            end: TraceEnd::Capped { shown: TRACE_CAP },
        }),
    );
    seen(&state, |view| {
        assert_eq!(list_of(view).2, Some("1 000 shown \u{b7} older not traced"));
    });
}

/// A binary commit, its file read answered with `sizes`.
fn binary_answered(sizes: Option<Result<BlobSizes, String>>) -> AppState {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, commands) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: None,
            rows: three_rows(),
            end: TraceEnd::Start { added_by: adder() },
        }),
    );
    let (read, _) = file_job(&commands).expect("a read");
    file_read_sized(&state, read, whole(true), sizes).0
}

#[test]
fn a_binary_commit_shows_sizes() {
    let state = binary_answered(Some(Ok(BlobSizes {
        old: Some(700),
        new: Some(1500),
    })));
    seen(&state, |view| {
        assert!(
            matches!(
                view.file,
                FilePane::Binary {
                    old: Some(700),
                    new: Some(1500),
                    sizes_failed: None,
                    ..
                }
            ),
            "{:?}",
            view.file
        );
    });
}

#[test]
fn a_size_read_failure_is_drawn() {
    let state = binary_answered(Some(Err("git was stopped".to_owned())));
    seen(&state, |view| {
        assert!(
            matches!(
                view.file,
                FilePane::Binary {
                    old: None,
                    new: None,
                    sizes_failed: Some("git was stopped"),
                    ..
                }
            ),
            "{:?}",
            view.file
        );
    });
}

/// The job always reads sizes for a binary diff; a binary answer without
/// them is a wiring bug, and shows as one rather than `none → none`.
#[test]
fn a_binary_answer_without_sizes_says_so() {
    let state = binary_answered(None);
    seen(&state, |view| {
        assert!(
            matches!(
                view.file,
                FilePane::Binary {
                    sizes_failed: Some("sizes not read"),
                    ..
                }
            ),
            "{:?}",
            view.file
        );
    });
}

// ---- w: ignore whitespace (21c) --------------------------------------------

#[test]
fn w_rereads_ignoring_whitespace_from_either_focus() {
    let state = lines_history(three_rows(), refresh_token());
    assert!(!history(&state).ignore_whitespace, "off by default");
    let (on, commands) = press(&state, KeyCode::Char('w'));
    assert!(history(&on).ignore_whitespace);
    assert_eq!(
        file_job_ignores_whitespace(&commands),
        Some(true),
        "{commands:?}"
    );
    let (job, commit) = file_job(&commands).expect("a read");
    assert_eq!(commit, lina(), "the same commit");
    let (landed, _) = file_read(&on, job, whole(false));
    let (tabbed, _) = press(&landed, KeyCode::Tab);
    assert_eq!(history(&tabbed).focus, HistoryFocus::File);
    let (off, commands) = press(&tabbed, KeyCode::Char('w'));
    assert!(!history(&off).ignore_whitespace);
    assert_eq!(
        file_job_ignores_whitespace(&commands),
        Some(false),
        "{commands:?}"
    );
}

#[test]
fn w_cancels_a_read_in_flight() {
    let state = lines_history(three_rows(), refresh_token());
    let (reading, commands) = press(&state, KeyCode::Char('j'));
    let (first, _) = file_job(&commands).expect("a read");
    let (next, commands) = press(&reading, KeyCode::Char('w'));
    assert!(
        commands.contains(&Command::CancelJob(first)),
        "{commands:?}"
    );
    let (second, commit) = file_job(&commands).expect("asked again");
    assert_eq!(commit, adder());
    assert_ne!(second, first);
    // The stale read's answer lands on nothing.
    let (after, _) = file_read(&next, first, whole(false));
    assert!(matches!(history(&after).file, FileLoad::Reading { job, .. } if job == second));
}

#[test]
fn w_holds_as_j_steps() {
    let state = lines_history(three_rows(), refresh_token());
    let (on, commands) = press(&state, KeyCode::Char('w'));
    let (job, _) = file_job(&commands).expect("a read");
    let (landed, _) = file_read(&on, job, whole(false));
    let (stepped, commands) = press(&landed, KeyCode::Char('j'));
    assert!(history(&stepped).ignore_whitespace, "still on");
    assert_eq!(
        file_job_ignores_whitespace(&commands),
        Some(true),
        "{commands:?}"
    );
}

/// [`whole`] as `-w` would read it: only line 45 still changed.
fn whole_ignoring_whitespace() -> DiffView {
    let mut view = whole(false);
    for hunk in &mut view.hunks {
        hunk.lines.retain(|line| line.kind != LineKindView::Removed);
        for line in &mut hunk.lines {
            if line.new_no != Some(45) {
                line.kind = LineKindView::Context;
                line.old_no = line.new_no;
            }
        }
    }
    view
}

#[test]
fn n_follows_the_changes_after_w() {
    let state = lines_history(three_rows(), refresh_token());
    let (on, commands) = press(&state, KeyCode::Char('w'));
    let (job, _) = file_job(&commands).expect("a read");
    let view = whole_ignoring_whitespace();
    let at_45 = graft_ui::history::row_of_new_line(&view.hunks, 45).expect("line 45");
    let (landed, _) = file_read(&on, job, view);
    let (next, _) = press(&landed, KeyCode::Char('n'));
    assert_eq!(
        history(&next).change,
        Some(at_45),
        "19 and 22 are no longer changes"
    );
}

#[test]
fn a_depth_one_clone_shows_the_end_row_alone() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let (state, commands) = traced(
        &opened,
        job,
        Ok(TraceAnswer {
            range: Some(refresh_token()),
            rows: Vec::new(),
            end: TraceEnd::GitStops(SHALLOW_END.to_owned()),
        }),
    );
    assert!(file_job(&commands).is_none(), "nothing to read");
    assert_eq!(history(&state).selected, None, "nothing selected");
    let end = format!("git stops tracing here: {SHALLOW_END}");
    seen(&state, |view| {
        let (rows, selected, words) = list_of(view);
        assert!(rows.is_empty(), "the end row alone");
        assert_eq!(selected, None);
        assert_eq!(words, Some(end.as_str()));
        // `graft_ui::history` draws `no change to show`, dim.
        assert_eq!(view.file, FilePane::NoChange);
    });
    // j/k, n/N, B and ⏎ do nothing.
    for code in [
        KeyCode::Char('j'),
        KeyCode::Char('k'),
        KeyCode::Char('n'),
        KeyCode::Char('N'),
        KeyCode::Char('B'),
        KeyCode::Enter,
    ] {
        let (next, commands) = press(&state, code);
        assert!(commands.is_empty(), "{code:?}: {commands:?}");
        assert_eq!(history(&next), history(&state), "{code:?}");
        assert_eq!(next.status, state.status, "{code:?}");
    }
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "Esc goes back");
}

/// The place's half of `a_trace_error_is_drawn_as_a_failure`
/// (`tests/it/history_place.rs`, real git): a failure's words in the list,
/// the file pane empty, `Esc` back.
#[test]
fn a_trace_failure_is_the_lists_words() {
    let (opened, commands) = press(&blame_on_22(), KeyCode::Char('L'));
    let (job, _) = trace_job(&commands);
    let words = "cannot read .git/shallow: Is a directory".to_owned();
    let (state, _) = traced(&opened, job, Err(TraceFailure::Failed(words.clone())));
    seen(&state, |view| {
        assert_eq!(view.list, HistoryList::Failed(words.as_str()));
        assert_eq!(view.file, FilePane::Empty);
    });
    let (back, _) = press(&state, KeyCode::Escape);
    assert!(back.trail.on_screen().is_some(), "Esc goes back");
}

#[test]
fn history_is_a_read_not_a_write() {
    let request = Box::new(TraceRequest::File {
        at: lina(),
        path: "a.rs".into(),
    });
    assert!(!is_write(&Command::TraceHistory {
        id: JobId::new(9),
        request,
    }));
    assert!(!is_write(&Command::ReadWholeFile {
        id: JobId::new(9),
        commit: lina(),
        path: "a.rs".into(),
        old_path: None,
        max_lines: 10,
        ignore_whitespace: false,
    }));
}
