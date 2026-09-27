//! A shown place's keys and mouse (design note pass 2b B1; ticket 20's
//! "Keys in blame" and "`Esc`").

use graft_core::event::{KeyCode, KeyPress, MouseButton, MouseInput, MouseKind};
use graft_git::blame::Rev;
use graft_git::trace::SHALLOW_END;

use crate::update::detail::{DetailSpec, LineRef, Side, show_line};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

use super::{BlameLoad, BlameState, CameFromRequest, Want, open_place, short, trail_back};

/// Two clicks on one row this close together are a double-click.
const DOUBLE_CLICK_MS: i64 = 500;

/// Rows the wheel scrolls per notch, as the graph's.
const WHEEL_ROWS: usize = 3;

/// The keys a shown place leaves to the app: `?`, `q`, `Ctrl+C`, `Ctrl+R`.
/// Everything else is the place's, most of it swallowed.
#[must_use]
pub(crate) fn live_global(key: KeyPress) -> bool {
    let control = key.modifiers.control;
    matches!(
        (key.code, control),
        (KeyCode::Char('?' | 'q'), false) | (KeyCode::Char('c' | 'r'), true)
    )
}

/// A key while a place is on screen (not a live global: the caller passes
/// those on).
pub fn place_key(state: &mut AppState, key: KeyPress) -> Vec<Command> {
    let Some(blame) = state.trail.top_mut() else {
        return Vec::new();
    };
    if blame.find_prompt.is_some() {
        prompt_key(state, key);
        return Vec::new();
    }
    if key.modifiers.control || key.modifiers.alt {
        return Vec::new();
    }
    let page = body_rows(state).max(1);
    let Some(blame) = state.trail.top_mut() else {
        return Vec::new();
    };
    let last = blame.row_count().saturating_sub(1);
    match key.code {
        KeyCode::Char('j') | KeyCode::Down => blame.cursor = (blame.cursor + 1).min(last),
        KeyCode::Char('k') | KeyCode::Up => blame.cursor = blame.cursor.saturating_sub(1),
        KeyCode::PageDown => blame.cursor = (blame.cursor + page).min(last),
        KeyCode::PageUp => blame.cursor = blame.cursor.saturating_sub(page),
        KeyCode::Char('g') => blame.cursor = 0,
        KeyCode::Char('G') => blame.cursor = last,
        KeyCode::Char(' ') => blame.heatmap = !blame.heatmap,
        KeyCode::Char('/') => blame.find_prompt = Some(String::new()),
        KeyCode::Char('n') => step_find(blame, true),
        KeyCode::Char('N') => step_find(blame, false),
        KeyCode::Char('w') => {
            blame.options.ignore_whitespace = !blame.options.ignore_whitespace;
            return rerun(state);
        }
        KeyCode::Char('i') => {
            if blame.ignore_line().is_none() {
                return Vec::new();
            }
            blame.options.ignore_revs = !blame.options.ignore_revs;
            return rerun(state);
        }
        KeyCode::Enter => return enter(state),
        KeyCode::Char('^') => return caret(state),
        KeyCode::Escape => return escape(state),
        _ => {}
    }
    keep_in_view(state);
    Vec::new()
}

/// The `/` prompt: text goes in, `⏎` finds, `Esc` closes it and nothing
/// else — the graph search's keys and case rule.
fn prompt_key(state: &mut AppState, key: KeyPress) {
    let Some(blame) = state.trail.top_mut() else {
        return;
    };
    let Some(prompt) = blame.find_prompt.as_mut() else {
        return;
    };
    match key.code {
        KeyCode::Char(typed) if !key.modifiers.control && !key.modifiers.alt => prompt.push(typed),
        KeyCode::Backspace => {
            prompt.pop();
        }
        KeyCode::Escape => blame.find_prompt = None,
        KeyCode::Enter => {
            let typed = blame.find_prompt.take().unwrap_or_default();
            // An empty query closes the prompt: nothing to find.
            if !typed.is_empty() {
                blame.find = Some(typed);
                let matches = blame.matches();
                if let Some(&at) = matches
                    .iter()
                    .find(|&&at| at >= blame.cursor)
                    .or_else(|| matches.first())
                {
                    blame.cursor = at;
                }
            }
            keep_in_view(state);
        }
        _ => {}
    }
}

/// `n`/`N`: the next or previous match, wrapping; nothing without a find.
fn step_find(blame: &mut BlameState, forward: bool) {
    if blame.find.is_none() {
        return;
    }
    let matches = blame.matches();
    let next = if forward {
        matches
            .iter()
            .find(|&&at| at > blame.cursor)
            .or_else(|| matches.first())
    } else {
        matches
            .iter()
            .rev()
            .find(|&&at| at < blame.cursor)
            .or_else(|| matches.last())
    };
    if let Some(&at) = next {
        blame.cursor = at;
    }
}

/// `Esc`, in order: an active find, a held failure on the status line,
/// then back along the trail (stopping a read still running).
fn escape(state: &mut AppState) -> Vec<Command> {
    if let Some(blame) = state.trail.top_mut()
        && blame.find.take().is_some()
    {
        return Vec::new();
    }
    if matches!(state.status, Status::Failed(_) | Status::Sticky { .. }) {
        state.status = Status::Ready;
        return Vec::new();
    }
    trail_back(state).unwrap_or_default()
}

/// `w`/`i`: blame again with the new options, the cursor kept on the same
/// final line.
fn rerun(state: &mut AppState) -> Vec<Command> {
    let job = state.local_jobs.next_id();
    let Some(blame) = state.trail.top_mut() else {
        return Vec::new();
    };
    let mut commands = Vec::new();
    // A read still running is asked again whole: `^`'s came-from read too,
    // or its line is lost (review m7-20e-20g Minor 1).
    let ignore_listed = blame.ignore_line().map(|line| line.count);
    let (want, came_from) = match &blame.body {
        BlameLoad::Reading {
            job,
            want,
            came_from,
            ..
        } => {
            commands.push(Command::CancelJob(*job));
            (*want, came_from.clone())
        }
        BlameLoad::Ready { blame: lines, .. } => (
            Want::Line(
                lines
                    .lines
                    .get(blame.cursor)
                    .map_or(1, |line| line.final_line),
            ),
            None,
        ),
        BlameLoad::Failed(_) => (Want::Line(1), None),
    };
    blame.body = BlameLoad::Reading {
        job,
        want,
        came_from: came_from.clone(),
        ignore_listed,
    };
    commands.push(Command::ReadBlame(
        job,
        Box::new(super::BlameRequest {
            rev: blame.rev,
            path: blame.path.clone(),
            options: blame.options,
            came_from,
        }),
    ));
    commands
}

/// The words for a line of a shallow clone's boundary: who wrote it is not
/// known, and nothing older can be shown (20's shallow ruling).
fn shallow_words(line: u32, commit: graft_core::oid::Oid) -> String {
    format!(
        "line {line} is from {} or older · {SHALLOW_END}",
        short(commit)
    )
}

/// `⏎`: land on the commit that last changed the line, its diff's cursor on
/// the line; an uncommitted line lands in the changes pane. The trail
/// stays, hidden, for `Esc`.
fn enter(state: &mut AppState) -> Vec<Command> {
    let Some(blame) = state.trail.top() else {
        return Vec::new();
    };
    let BlameLoad::Ready { blame: lines, .. } = &blame.body else {
        return Vec::new();
    };
    let Some(line) = lines.lines.get(blame.cursor) else {
        return Vec::new();
    };
    let (spec, path, at) = match line.commit {
        None => (DetailSpec::WorkingTree, blame.path.clone(), line.final_line),
        Some(id) => {
            let Some(commit) = lines.commits.get(&id) else {
                return Vec::new();
            };
            if commit.shallow {
                state.status = Status::sticky(shallow_words(line.final_line, id));
                return Vec::new();
            }
            (DetailSpec::Commit(id), commit.path.clone(), line.orig_line)
        }
    };
    let set_aside = state
        .trail
        .origin
        .as_ref()
        .and_then(|origin| origin.base)
        .map(|base| short(base.oid));
    state.trail.shown = false;
    let commands = show_line(
        state,
        spec,
        &path,
        Some(LineRef {
            side: Side::New,
            line: at,
        }),
        // Blame counts in the file it read — on disk, or the commit's
        // version: never a changes-pane section (20m).
        None,
    );
    if let Some(base) = set_aside
        && !matches!(state.status, Status::Sticky { .. })
    {
        // Not `Sticky`: nothing is wrong, and a sticky line would take the
        // `Esc` that goes back to blame (`keys::key_event` clears it first).
        state.status = Status::Waiting(format!(
            "compare base {base} set aside · it comes back when you leave blame"
        ));
    }
    commands
}

/// `^`: blame the parent the line came through, the cursor where the line
/// came from. Refuses, in words, on a shallow boundary, an uncommitted
/// line, or a root.
fn caret(state: &mut AppState) -> Vec<Command> {
    let Some(blame) = state.trail.top() else {
        return Vec::new();
    };
    let BlameLoad::Ready { blame: lines, .. } = &blame.body else {
        return Vec::new();
    };
    let Some(line) = lines.lines.get(blame.cursor) else {
        return Vec::new();
    };
    let Some(id) = line.commit else {
        state.status = Status::sticky(format!(
            "line {} isn't committed yet · nothing before it",
            line.final_line
        ));
        return Vec::new();
    };
    let Some(commit) = lines.commits.get(&id) else {
        return Vec::new();
    };
    if commit.shallow {
        state.status = Status::sticky(shallow_words(line.final_line, id));
        return Vec::new();
    }
    let Some((parent, old_path)) = commit.previous.clone() else {
        state.status = Status::sticky(format!("{} added this file · nothing before it", short(id)));
        return Vec::new();
    };
    let request = CameFromRequest {
        commit: id,
        path: commit.path.clone(),
        old_path: old_path.clone(),
        line: line.orig_line,
    };
    open_place(
        state,
        Rev::Commit(parent),
        old_path,
        graft_ui::blame::BlameRev::Before(short(id)),
        Want::CameFrom,
        Some(request),
    )
}

/// Rows the place draws in: the whole frame but the bar under it
/// (`view::frame::draw_place`, which splits the frame the same way).
fn place_height(state: &AppState) -> u16 {
    state.size.rows.saturating_sub(1)
}

/// Rows above the lines: the title, and the ignore-revs line when there is
/// one. Mirrors `graft_ui::blame`'s `body_area`; plain numbers because
/// `update` stays free of the rendering library (`geometry::Bounds`).
fn header_rows(state: &AppState) -> u16 {
    1 + u16::from(
        state
            .trail
            .top()
            .is_some_and(|blame| blame.ignore_line().is_some()),
    )
}

/// Rows the lines get: the area less the header.
fn body_rows(state: &AppState) -> usize {
    usize::from(place_height(state).saturating_sub(header_rows(state)))
}

/// Scroll just enough to keep the cursor row on screen.
pub(crate) fn keep_in_view(state: &mut AppState) {
    let rows = body_rows(state);
    let Some(blame) = state.trail.top_mut() else {
        return;
    };
    if rows == 0 {
        blame.top = blame.cursor;
        return;
    }
    if blame.cursor < blame.top {
        blame.top = blame.cursor;
    } else if blame.cursor >= blame.top + rows {
        blame.top = blame.cursor + 1 - rows;
    }
}

/// The mouse while a place is on screen: every event is the place's. A
/// click selects a row, a second click on it straight after is `⏎`, the
/// wheel scrolls.
pub(crate) fn place_mouse(state: &mut AppState, mouse: MouseInput) -> Vec<Command> {
    let header = usize::from(header_rows(state));
    let now = state.harness.clock.now_millis();
    let rows = body_rows(state);
    let Some(blame) = state.trail.top_mut() else {
        return Vec::new();
    };
    match mouse.kind {
        MouseKind::ScrollDown => {
            let last = blame.row_count().saturating_sub(rows.max(1));
            blame.top = (blame.top + WHEEL_ROWS).min(last);
        }
        MouseKind::ScrollUp => blame.top = blame.top.saturating_sub(WHEEL_ROWS),
        MouseKind::Down(MouseButton::Left) => {
            // The screen row under the header, if it is a line on screen.
            let Some(row) = usize::from(mouse.row)
                .checked_sub(header)
                .filter(|&below| below < rows)
                .map(|below| blame.top + below)
                .filter(|&row| row < blame.row_count())
            else {
                return Vec::new();
            };
            let double = blame
                .last_click
                .is_some_and(|(at, when)| at == row && now - when <= DOUBLE_CLICK_MS);
            blame.cursor = row;
            if double {
                blame.last_click = None;
                return enter(state);
            }
            blame.last_click = Some((row, now));
        }
        _ => {}
    }
    Vec::new()
}
