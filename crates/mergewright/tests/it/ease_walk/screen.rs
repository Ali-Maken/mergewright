//! Opening a walk's repository and reading its frames as text: the top line,
//! the rows, the branch pane, the bar.

use std::path::Path;

use mergewright::session::Clock;
use mergewright::update::AppState;
use mergewright_core::event::KeyCode;

use super::fixture::NOW;
use crate::app_harness::App;
use crate::captures::{at_rest, frame_at};

pub const ROWS: u16 = 40;

/// Opened at `columns` × [`ROWS`], pumped to rest: the walk complete and
/// promoted, the radar answered, the fold counts and the pane's read in.
pub fn open(repo: &Path, columns: u16) -> (App, AppState) {
    let (app, state) = App::open(repo);
    let state = app.resize(&state, columns, ROWS);
    let state = app.pump(state, "the rows, the refs and the working tree", |state| {
        state.refs.is_some()
            && state.working_tree.is_some()
            && state
                .history
                .as_ref()
                .and_then(|history| history.window.as_ref())
                .is_some_and(|window| !window.rows.is_empty())
    });
    let state = at_rest(&app, state);
    (app, state)
}

/// The frame at the state's own size, against the fixtures' fixed clock.
pub fn drawn(state: &AppState) -> Vec<String> {
    frame_at(
        state,
        state.size.columns,
        state.size.rows,
        Some(Clock::Fixed(NOW)),
    )
}

/// The whole frame as one string, for a failure message.
pub fn shown(frame: &[String]) -> String {
    frame.join("\n")
}

/// The top line.
pub fn top(frame: &[String]) -> &str {
    frame.first().map_or("", String::as_str)
}

/// The bar: the last line.
pub fn bar(frame: &[String]) -> &str {
    frame.last().map_or("", String::as_str)
}

/// The status line: the line above the bar.
pub fn status(frame: &[String]) -> &str {
    frame
        .len()
        .checked_sub(2)
        .map_or("", |at| frame[at].as_str())
}

/// The first line holding `needle`, or a failure showing the frame.
pub fn line_with<'f>(frame: &'f [String], needle: &str) -> &'f str {
    frame
        .iter()
        .find(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}\n{}", shown(frame)))
}

/// The line under the first line holding `needle`.
pub fn line_after<'f>(frame: &'f [String], needle: &str) -> &'f str {
    let at = frame
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}\n{}", shown(frame)));
    frame.get(at + 1).map_or("", String::as_str)
}

/// Each line's part left of the branch pane (the rows).
pub fn rows_side(frame: &[String], columns: u16) -> Vec<String> {
    frame
        .iter()
        .map(|line| crate::row_reads::graph_side(line, columns))
        .collect()
}

/// The branch pane's lines: each line's part right of the pane's rule.
pub fn pane(frame: &[String], columns: u16) -> Vec<String> {
    let slot = usize::from(
        mergewright_ui::branch_pane::pane_width(columns)
            .unwrap_or_else(|| panic!("no pane at {columns} columns"))
            + 2,
    );
    let at = usize::from(columns) - slot;
    frame
        .iter()
        .filter_map(|line| {
            let cells: Vec<char> = line.chars().collect();
            (cells.get(at) == Some(&'│'))
                .then(|| cells[at + 1..].iter().collect::<String>().trim().to_owned())
        })
        .collect()
}

/// The cursor row's commit subject.
pub fn selected_subject(state: &AppState) -> String {
    state
        .history
        .as_ref()
        .and_then(|history| history.selected())
        .map(|row| {
            String::from_utf8_lossy(&row.commit.message)
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned()
        })
        .unwrap_or_default()
}

/// `j` (or `k` past the bottom) until the cursor rests on the row whose
/// subject starts with `subject`; then to rest, so the pane has read it.
pub fn cursor_on(app: &App, state: AppState, subject: &str) -> AppState {
    let mut state = state;
    for key in ['j', 'k'] {
        for _ in 0..60 {
            if selected_subject(&state).starts_with(subject) {
                return at_rest(app, state);
            }
            state = app.key(&state, KeyCode::Char(key));
        }
    }
    panic!("the cursor never reached {subject:?}");
}

/// Type `text` into whatever has focus, one key per character.
pub fn typed(app: &App, state: &AppState, text: &str) -> AppState {
    text.chars()
        .fold(state.clone(), |state, c| app.key(&state, KeyCode::Char(c)))
}

/// The commit `HEAD` names, as a short subject.
pub fn head_subject(repo: &Path) -> String {
    crate::app_harness::git(repo, &["log", "-1", "--format=%s"])
        .trim()
        .to_owned()
}

/// Is the cursor on `HEAD`?
pub fn cursor_on_head(state: &AppState) -> bool {
    let head = state
        .refs
        .as_deref()
        .and_then(mergewright::refs::RefBadges::head);
    head.is_some()
        && state
            .history
            .as_ref()
            .and_then(mergewright::history::History::selected)
            .is_some_and(|row| Some(row.commit.id) == head)
}

/// The drawn buffer, with its cells' styles, at the state's own size.
pub fn buffer(state: &AppState) -> ratatui::buffer::Buffer {
    let session = mergewright::session::Session::settle(
        &mergewright::config::Loaded::default(),
        crate::captures::capabilities(),
    );
    let mut state = state.clone();
    state.harness.clock = Clock::Fixed(NOW);
    let (columns, rows) = (state.size.columns, state.size.rows);
    let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(columns, rows))
        .expect("test backend opens");
    terminal
        .draw(|frame| mergewright::view::view(&session, &state, frame))
        .expect("drawing into a test backend cannot fail");
    terminal.backend().buffer().clone()
}

/// How many cells carry a foreground or background colour.
pub fn coloured_cells(buffer: &ratatui::buffer::Buffer) -> usize {
    use ratatui::style::Color;
    buffer
        .content()
        .iter()
        .filter(|cell| cell.fg != Color::Reset || cell.bg != Color::Reset)
        .count()
}

/// The column of `subject`'s node: the first of `◆●◇○◉` on its line.
pub fn node_column(frame: &[String], subject: &str) -> usize {
    let line = line_with(frame, subject);
    line.chars()
        .position(|cell| "◆●◇○◉".contains(cell))
        .unwrap_or_else(|| panic!("a node on {line:?}"))
}

/// The cell at `column` on `subject`'s line.
pub fn cell(frame: &[String], subject: &str, column: usize) -> char {
    line_with(frame, subject)
        .chars()
        .nth(column)
        .unwrap_or_else(|| panic!("{subject}'s row reaches column {column}"))
}

/// The row index of the first line holding `needle`.
pub fn row_of(frame: &[String], needle: &str) -> usize {
    frame
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("no line holds {needle:?}\n{}", shown(frame)))
}

/// Resize and pump to rest.
pub fn sized(app: &App, state: &AppState, columns: u16, rows: u16) -> AppState {
    at_rest(app, app.resize(state, columns, rows))
}

/// A key, then rest.
pub fn key_at_rest(app: &App, state: &AppState, c: char) -> AppState {
    at_rest(app, app.key(state, KeyCode::Char(c)))
}
