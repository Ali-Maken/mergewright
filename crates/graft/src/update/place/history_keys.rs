//! A shown history place's keys and mouse (keymap §2.17; design note H1,
//! P1; m7-power-tools/21 "Keys").

use graft_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use graft_git::trace::TraceCommit;
use graft_ui::history::{HistoryFocus, columns};

use super::history::{FileLoad, HistoryState, file_height, list_height, next_change};
use super::{BlameFrom, open_blame, trail_back};
use crate::update::detail::{DetailSpec, LineRef, Side, show_line};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// Two clicks on one row this close together are a double-click (blame's).
const DOUBLE_CLICK_MS: i64 = 500;

/// Rows the wheel scrolls per notch, as the graph's.
const WHEEL_ROWS: usize = 3;

/// `n`/`N` past the last change: the resolver's words for the same miss.
pub(crate) const NO_MORE_CHANGES: &str = "no more changes this way";

/// A key while a history place is on screen (not a live global).
pub(super) fn history_key(state: &mut AppState, key: KeyPress) -> Vec<Command> {
    if key.modifiers.control || key.modifiers.alt {
        return Vec::new();
    }
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => return step(state, 1, true),
        KeyCode::Char('k') | KeyCode::Up => return step(state, 1, false),
        KeyCode::PageDown => return step(state, page(state), true),
        KeyCode::PageUp => return step(state, page(state), false),
        KeyCode::Tab => {
            if let Some(history) = top(state) {
                history.focus = match history.focus {
                    HistoryFocus::List => HistoryFocus::File,
                    HistoryFocus::File => HistoryFocus::List,
                };
            }
        }
        KeyCode::Char('n') => next(state, true),
        KeyCode::Char('N') => next(state, false),
        KeyCode::Char('v') => {
            if let Some(history) = top(state) {
                history.side_by_side = !history.side_by_side;
            }
        }
        KeyCode::Char('w') => return whitespace(state),
        KeyCode::Char('L') => return load_all(state),
        KeyCode::Char('B') => return blame_here(state),
        KeyCode::Enter => return enter(state),
        KeyCode::Escape => return escape(state),
        _ => {}
    }
    Vec::new()
}

/// The history place on top of the trail.
fn top(state: &mut AppState) -> Option<&mut HistoryState> {
    state.trail.top_history_mut()
}

/// A page of the focused pane.
fn page(state: &AppState) -> usize {
    let focus = state.trail.top_history().map(|history| history.focus);
    match focus {
        Some(HistoryFocus::File) => file_height(state),
        Some(HistoryFocus::List) | None => list_height(state),
    }
    .max(1)
}

/// `j`/`k` (and the page keys): on the list, older/newer by `by` commits,
/// the file pane following; on the file pane, scroll. With no commits
/// (a depth-1 clone) nothing moves.
fn step(state: &mut AppState, by: usize, older: bool) -> Vec<Command> {
    let heights = (list_height(state), file_height(state));
    let job = state.local_jobs.next_id();
    let Some(history) = top(state) else {
        return Vec::new();
    };
    match history.focus {
        HistoryFocus::File => {
            history.file_top = if older {
                history.file_top + by
            } else {
                history.file_top.saturating_sub(by)
            };
            history.change = None;
            history.keep_file_in_view(heights.1);
            Vec::new()
        }
        HistoryFocus::List => {
            let Some(selected) = history.selected else {
                return Vec::new();
            };
            let last = history.rows().len().saturating_sub(1);
            let moved = if older {
                (selected + by).min(last)
            } else {
                selected.saturating_sub(by)
            };
            if moved == selected {
                return Vec::new();
            }
            // The next file opens where this one's top was (`F`'s line
            // carries on); `L` places by its range, which moves with it.
            if let Some(line) = history.top_line() {
                history.anchor = Some(LineRef {
                    side: Side::New,
                    line,
                });
            }
            history.selected = Some(moved);
            history.keep_list_in_view(heights.0);
            history.read_selected(job)
        }
    }
}

/// `n`/`N`: the file pane's next/previous change moves to its top, from
/// either focus. None that way: a notice, and the pane stays.
fn next(state: &mut AppState, forward: bool) {
    let height = file_height(state);
    let Some(history) = top(state) else {
        return;
    };
    if history.selected.is_none() {
        return;
    }
    let changed = history.changed();
    let from = history.change.unwrap_or(history.file_top);
    match next_change(&changed, Some(from), forward) {
        Some(row) => {
            history.change = Some(row);
            history.file_top = row;
            history.keep_file_in_view(height);
        }
        None => state.status = Status::Waiting(NO_MORE_CHANGES.to_owned()),
    }
}

/// `L` on the file pane (§2.5): read the whole file, however long.
fn load_all(state: &mut AppState) -> Vec<Command> {
    let job = state.local_jobs.next_id();
    let Some(history) = top(state) else {
        return Vec::new();
    };
    if history.max_lines == usize::MAX || history.selected.is_none() {
        return Vec::new();
    }
    history.max_lines = usize::MAX;
    history.anchor = history.top_line().map(|line| LineRef {
        side: Side::New,
        line,
    });
    history.read_selected(job)
}

/// `w` (§2.5 via §2.17), from either focus as `v`: flip ignore-whitespace
/// and read the selected commit's file again, stopping a read in flight.
/// The file opens where its top was, as `L` does.
fn whitespace(state: &mut AppState) -> Vec<Command> {
    let job = state.local_jobs.next_id();
    let Some(history) = top(state) else {
        return Vec::new();
    };
    history.ignore_whitespace = !history.ignore_whitespace;
    if history.selected.is_none() {
        return Vec::new();
    }
    history.anchor = history.top_line().map(|line| LineRef {
        side: Side::New,
        line,
    });
    history.read_selected(job)
}

/// The selected commit and the new-side line at the file pane's top (the
/// first line without a file read yet).
fn selected_line(state: &AppState) -> Option<(TraceCommit, u32)> {
    let history = state.trail.top_history()?;
    let row = history.selected_row()?.clone();
    let line = match history.file {
        FileLoad::Ready { .. } => history.top_line().unwrap_or(1),
        FileLoad::Idle | FileLoad::Reading { .. } | FileLoad::Failed { .. } => 1,
    };
    Some((row, line))
}

/// `B`: blame the file as of the selected commit, at the file pane's line;
/// pushed on the trail, so `Esc` comes back here.
fn blame_here(state: &mut AppState) -> Vec<Command> {
    let Some((row, line)) = selected_line(state) else {
        return Vec::new();
    };
    open_blame(
        state,
        BlameFrom::Commit {
            commit: row.id,
            path: row.path,
            line,
        },
    )
}

/// `⏎`: the graph on the selected commit, its diff on this file at the
/// pane's line. The trail stays, hidden, for `Esc` (P1).
fn enter(state: &mut AppState) -> Vec<Command> {
    let Some((row, line)) = selected_line(state) else {
        return Vec::new();
    };
    state.trail.shown = false;
    show_line(
        state,
        DetailSpec::Commit(row.id),
        &row.path,
        Some(LineRef {
            side: Side::New,
            line,
        }),
        None,
    )
}

/// `Esc`: a held failure on the status line first, then back along the
/// trail — while tracing, that stops git (`trail_back` cancels the job) and
/// lands where `L`/`F` was pressed (ruling a).
fn escape(state: &mut AppState) -> Vec<Command> {
    if matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
        state.status = Status::Ready;
        return Vec::new();
    }
    trail_back(state).unwrap_or_default()
}

/// The commit drawn `line` rows under the list's first row, from commit
/// `top`; a `↳ was` row is its commit's (`graft_ui::history::list_index_at`
/// over the trace's own rows).
fn commit_at(rows: &[TraceCommit], top: usize, line: usize) -> Option<usize> {
    let mut at = 0;
    for (index, row) in rows.iter().enumerate().skip(top) {
        at += 1 + usize::from(row.renamed_from.is_some());
        if line < at {
            return Some(index);
        }
    }
    None
}

/// The wheel over the file pane: `WHEEL_ROWS` a notch, kept in the file.
fn wheel_file(history: &mut HistoryState, down: bool, height: usize) {
    history.file_top = if down {
        history.file_top + WHEEL_ROWS
    } else {
        history.file_top.saturating_sub(WHEEL_ROWS)
    };
    history.change = None;
    history.keep_file_in_view(height);
}

/// The wheel over the list: `WHEEL_ROWS` a notch, the last commit at most.
fn wheel_list(history: &mut HistoryState, down: bool) {
    let last = history.rows().len().saturating_sub(1);
    history.list_top = if down {
        (history.list_top + WHEEL_ROWS).min(last)
    } else {
        history.list_top.saturating_sub(WHEEL_ROWS)
    };
}

/// The mouse while a history place is on screen: a click on a commit
/// selects it, a second click on it straight after is `⏎`; the wheel
/// scrolls the pane under it. Rows count from the title (row 0).
pub(super) fn history_mouse(state: &mut AppState, mouse: MouseInput) -> Vec<Command> {
    let now = state.harness.clock.now_millis();
    let width = state.size.columns;
    let heights = (list_height(state), file_height(state));
    let job = state.local_jobs.next_id();
    let Some(history) = top(state) else {
        return Vec::new();
    };
    let panes = columns(width, history.focus);
    let inside = |pane: Option<(u16, u16)>| {
        pane.is_some_and(|(x, w)| mouse.column >= x && mouse.column < x.saturating_add(w))
    };
    let on_list = inside(panes.list);
    let on_file = inside(panes.file);
    match mouse.kind {
        MouseKind::ScrollDown | MouseKind::ScrollUp => {
            let down = matches!(mouse.kind, MouseKind::ScrollDown);
            if on_file {
                wheel_file(history, down, heights.1);
            } else if on_list {
                wheel_list(history, down);
            }
            Vec::new()
        }
        MouseKind::Down(MouseButton::Left) if on_list => {
            let Some(below) = usize::from(mouse.row)
                .checked_sub(1)
                .filter(|&below| below < heights.0)
            else {
                return Vec::new();
            };
            let Some(index) = commit_at(history.rows(), history.list_top, below) else {
                return Vec::new();
            };
            let double = history
                .last_click
                .is_some_and(|(at, when)| at == index && now - when <= DOUBLE_CLICK_MS);
            history.focus = HistoryFocus::List;
            if double {
                history.last_click = None;
                return enter(state);
            }
            history.last_click = Some((index, now));
            if history.selected == Some(index) {
                return Vec::new();
            }
            if let Some(line) = history.top_line() {
                history.anchor = Some(LineRef {
                    side: Side::New,
                    line,
                });
            }
            history.selected = Some(index);
            history.read_selected(job)
        }
        MouseKind::Down(MouseButton::Left) if on_file => {
            history.focus = HistoryFocus::File;
            Vec::new()
        }
        _ => Vec::new(),
    }
}
