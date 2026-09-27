//! The history place (m7-power-tools/21; design note pass 1b "History",
//! pass 2b H1, P1): `L` in blame traces the lines around the cursor, `F` on
//! a file traces the whole file. The commits on the left, the whole file at
//! the selected one on the right with its adds and removes marked.
//!
//! It is a place like blame (`Place::History`): pushed on the trail, `Esc`
//! pops it, `⏎` hides the trail under the graph. Its two jobs are the trace
//! (one answer, ruling a) and the whole file at the selected commit.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::blame::Rev;
use graft_git::trace::{LineRange, TraceCommit, TraceEnd};
use graft_term::sanitize::{sanitize_line, sanitize_path};
use graft_ui::detail::{DiffView, LineKindView, WorkingTreeSection};
use graft_ui::graph_screen::relative_date_long;
use graft_ui::history::{
    FilePane, HistoryFocus, HistoryList, HistoryRow, HistoryView, changed_rows, file_rows,
    row_of_new_line, row_of_old_line,
};
use graft_ui::theme::Theme;

use super::history_answer::{TraceRequest, uncommitted_words};
use super::{Place, TRAIL_MAX, short};
use crate::update::detail::{DetailSpec, LineRef, MAX_LINES, Side, diff_cursor};
use crate::update::events::Command;
use crate::update::state::{AppState, Status};

/// What a history place traces, from where the key was pressed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryFrom {
    /// `L` in blame: the lines around `line` of `path` as of `at`.
    Lines {
        /// The version blamed.
        at: Rev,
        /// The file's path in it.
        path: PathBuf,
        /// The cursor's line, in that version.
        line: u32,
        /// The commit blame showed for the line: the list cursor starts on
        /// it.
        blamed: Option<Oid>,
        /// The cursor's blame block, first and last line: the range when no
        /// function is found (added to 21's signature: the block is blame's,
        /// read where `L` is pressed).
        block: (u32, u32),
    },
    /// `F` on a file: the whole file as of `at`.
    File {
        /// The version shown (`HEAD` for the changes pane).
        at: Rev,
        /// The file's path in it.
        path: PathBuf,
        /// The line he was on, in `at`'s version (`side` says which side of
        /// the diff counts it); `None` from a file row.
        line: Option<u32>,
        /// Which side `line` counts on (added to 21's signature: an
        /// old-side line counts in the parent).
        side: Side,
    },
}

/// The trace, by state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceLoad {
    /// git is tracing; `Esc` cancels it and goes back (ruling a).
    Tracing {
        /// The job.
        job: JobId,
    },
    /// The trace failed: its words (`couldn't trace: …`).
    Failed(String),
    /// The commits, newest first, and why the trace ended.
    Ready {
        /// The commits.
        rows: Vec<TraceCommit>,
        /// Why it ended: the end row's words.
        end: TraceEnd,
    },
}

/// The file pane, by state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileLoad {
    /// Nothing asked for (no commit selected).
    Idle,
    /// Reading `commit`'s whole file.
    Reading {
        /// The job.
        job: JobId,
        /// The commit.
        commit: Oid,
    },
    /// The read failed.
    Failed {
        /// The commit.
        commit: Oid,
        /// git's words.
        words: String,
    },
    /// The whole file at `commit`, its change marked.
    Ready {
        /// The commit.
        commit: Oid,
        /// The file, assembled as the detail pane assembles a diff.
        view: Arc<DiffView>,
    },
}

/// One history place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HistoryState {
    /// The file traced, as of `at`.
    pub path: PathBuf,
    /// The commit traced back from.
    pub at: Oid,
    /// Opened by `L` (a range), not `F`.
    pub lines: bool,
    /// `L`: the commit blame showed for the line.
    pub blamed: Option<Oid>,
    /// The range traced, once the trace says which (`L` only).
    pub range: Option<LineRange>,
    /// The list's source.
    pub trace: TraceLoad,
    /// The selected commit; `None` with none to select.
    pub selected: Option<usize>,
    /// The first commit drawn.
    pub list_top: usize,
    /// The file pane's source.
    pub file: FileLoad,
    /// The file pane's first row (`graft_ui::history::file_rows`' index).
    pub file_top: usize,
    /// The change `n`/`N` last went to (its first row). Apart from
    /// `file_top` because near the file's end a change cannot scroll to the
    /// top: counting from `file_top` there, `n` found the same change again
    /// and never said `no more changes this way`. Scrolling forgets it.
    pub change: Option<usize>,
    /// Which pane has the keys.
    pub focus: HistoryFocus,
    /// `v`: side by side.
    pub side_by_side: bool,
    /// Lines read per file before cutting short (`L` loads them all).
    pub max_lines: usize,
    /// Where the next file to land opens (`F`: the line he was on; `j`/`k`:
    /// the line at the top as he left the last one). `L` places by range.
    pub anchor: Option<LineRef>,
    /// The last click's commit and time (ms), for a double-click.
    pub last_click: Option<(usize, i64)>,
}

/// The commits, if the trace has landed.
impl HistoryState {
    /// The traced commits; empty until the trace lands.
    #[must_use]
    pub fn rows(&self) -> &[TraceCommit] {
        match &self.trace {
            TraceLoad::Ready { rows, .. } => rows,
            TraceLoad::Tracing { .. } | TraceLoad::Failed(_) => &[],
        }
    }

    /// The selected commit's row.
    #[must_use]
    pub fn selected_row(&self) -> Option<&TraceCommit> {
        self.rows().get(self.selected?)
    }

    /// Where the list cursor starts on `rows`: from `L`, the commit blame
    /// showed; from `F` (or when that commit is not traced), the newest.
    /// `None` with no rows (a depth-1 clone): nothing is selected.
    pub(super) fn start_row(&self, rows: &[TraceCommit]) -> Option<usize> {
        if rows.is_empty() {
            return None;
        }
        let blamed = self
            .blamed
            .and_then(|blamed| rows.iter().position(|row| row.id == blamed));
        Some(blamed.unwrap_or(0))
    }

    /// Ask for the selected commit's whole file under `job`, stopping a read
    /// still running. No selection: the pane is left idle.
    pub(super) fn read_selected(&mut self, job: JobId) -> Vec<Command> {
        let mut commands = Vec::new();
        if let FileLoad::Reading { job: running, .. } = self.file {
            commands.push(Command::CancelJob(running));
        }
        let Some(row) = self.selected_row() else {
            self.file = FileLoad::Idle;
            return commands;
        };
        let (commit, path, old_path) = (row.id, row.path.clone(), row.renamed_from.clone());
        self.file = FileLoad::Reading { job, commit };
        commands.push(Command::ReadWholeFile {
            id: job,
            commit,
            path,
            old_path,
            max_lines: self.max_lines,
        });
        commands
    }

    /// The file pane's rows, once the file is read.
    fn file_view(&self) -> Option<&DiffView> {
        match &self.file {
            FileLoad::Ready { view, .. } => Some(view),
            FileLoad::Idle | FileLoad::Reading { .. } | FileLoad::Failed { .. } => None,
        }
    }

    /// Which of the file pane's rows are changes (for `n`/`N`).
    #[must_use]
    pub fn changed(&self) -> Vec<bool> {
        self.file_view()
            .map(|view| changed_rows(&view.hunks))
            .unwrap_or_default()
    }

    /// The file pane's row count.
    #[must_use]
    pub fn file_row_count(&self) -> usize {
        self.file_view()
            .map_or(0, |view| file_rows(&view.hunks).len())
    }

    /// The new-side line at the file pane's top row, or the first kept line
    /// below it (a removed row has none): what `B` blames and where `j`/`k`
    /// leave the next file.
    #[must_use]
    pub fn top_line(&self) -> Option<u32> {
        let view = self.file_view()?;
        let rows = file_rows(&view.hunks);
        rows.iter().skip(self.file_top).find_map(|row| row.new_no)
    }

    /// The range where it sits in the selected commit (`L`; git's `-L`
    /// hunk), for the `┃` bracket.
    #[must_use]
    pub fn range_at(&self) -> Option<(u32, u32)> {
        self.selected_row()?.range_at
    }

    /// A file just landed: open it where it belongs — `L` on the range with
    /// one line above, `F` at the anchor line (the top without one).
    pub(super) fn place_file_pane(&mut self) {
        let row = self.file_view().and_then(|view| {
            if let Some((start, _)) = self.range_at() {
                return row_of_new_line(&view.hunks, start).map(|row| row.saturating_sub(1));
            }
            let anchor = self.anchor?;
            match anchor.side {
                Side::New => row_of_new_line(&view.hunks, anchor.line),
                Side::Old => row_of_old_line(&view.hunks, anchor.line),
            }
        });
        self.file_top = row.unwrap_or(0);
        self.change = None;
    }

    /// Scroll the list just enough to keep the selected commit on screen;
    /// a commit's `↳ was` row counts as its own.
    pub(super) fn keep_list_in_view(&mut self, height: usize) {
        let Some(selected) = self.selected else {
            self.list_top = 0;
            return;
        };
        if selected < self.list_top {
            self.list_top = selected;
            return;
        }
        let rows = self.rows();
        let height = height.max(1);
        let mut top = self.list_top;
        while top < selected && drawn_rows(rows, top, selected) > height {
            top += 1;
        }
        self.list_top = top;
    }

    /// Keep the file pane's top inside the file.
    pub(super) fn keep_file_in_view(&mut self, height: usize) {
        let last = self.file_row_count().saturating_sub(height.max(1));
        self.file_top = self.file_top.min(last);
    }

    /// The picture's view of this place, handed to `draw`.
    pub fn view<R>(&self, now: i64, theme: &Theme, draw: impl FnOnce(&HistoryView) -> R) -> R {
        let texts: Vec<RowText> = self.rows().iter().map(RowText::of).collect();
        let rows: Vec<HistoryRow> = self
            .rows()
            .iter()
            .zip(&texts)
            .map(|(row, text)| HistoryRow {
                sha7: &text.sha7,
                subject: &text.subject,
                who: &text.who,
                when: row.when,
                added: row.added,
                removed: row.removed,
                was: text.was.as_deref(),
            })
            .collect();
        let title = self.title();
        let end = match &self.trace {
            TraceLoad::Ready { end, .. } => Some(end_words(end)),
            TraceLoad::Tracing { .. } | TraceLoad::Failed(_) => None,
        };
        let failed = match &self.trace {
            TraceLoad::Failed(words) => Some(sanitize_line(words)),
            TraceLoad::Tracing { .. } | TraceLoad::Ready { .. } => None,
        };
        let list = match &failed {
            Some(words) => HistoryList::Failed(words.as_str()),
            None => HistoryList::Rows {
                rows: &rows,
                selected: self.selected,
                top: self.list_top,
                end: end.as_deref(),
            },
        };
        let header = self.header(now);
        let file_failed = self.file_failed_words();
        let file = self.file_pane(&header, file_failed.as_deref());
        let view = HistoryView {
            title: &title,
            tracing: matches!(self.trace, TraceLoad::Tracing { .. }),
            list,
            file,
            focus: self.focus,
            now,
            theme,
        };
        draw(&view)
    }

    /// `couldn't read <path> at <sha7>: <why>` when the file's read failed.
    fn file_failed_words(&self) -> Option<String> {
        match &self.file {
            FileLoad::Failed { commit, words } => Some(format!(
                "couldn't read {} at {}: {}",
                sanitize_path(self.selected_row().map_or(&self.path, |row| &row.path)).as_str(),
                short(*commit),
                sanitize_line(words).as_str()
            )),
            FileLoad::Idle | FileLoad::Reading { .. } | FileLoad::Ready { .. } => None,
        }
    }

    /// The file pane's view: what the trace and the file's read allow.
    fn file_pane<'a>(&'a self, header: &'a str, failed: Option<&'a str>) -> FilePane<'a> {
        match (&self.trace, &self.file) {
            (TraceLoad::Tracing { .. } | TraceLoad::Failed(_), _) => FilePane::Empty,
            (TraceLoad::Ready { .. }, FileLoad::Idle) => FilePane::NoChange,
            (TraceLoad::Ready { .. }, FileLoad::Reading { .. }) => FilePane::Reading { header },
            (TraceLoad::Ready { .. }, FileLoad::Failed { .. }) => {
                FilePane::Failed(failed.unwrap_or_default())
            }
            (TraceLoad::Ready { .. }, FileLoad::Ready { view, .. }) if view.binary.is_some() => {
                FilePane::Binary {
                    header,
                    old: None,
                    new: None,
                }
            }
            (TraceLoad::Ready { .. }, FileLoad::Ready { view, .. }) => FilePane::Lines {
                header,
                hunks: &view.hunks,
                top: self.file_top,
                range: self.range_at().map(|(start, end)| start..=end),
                side_by_side: self.side_by_side,
                word_tint: true,
            },
        }
    }

    /// The title's words after `HISTORY · `: the path, and which lines.
    #[must_use]
    pub fn title(&self) -> String {
        let path = sanitize_path(&self.path);
        match &self.range {
            None => path.as_str().to_owned(),
            Some(range) => format!("{} \u{b7} {}", path.as_str(), range_words(range)),
        }
    }

    /// The file pane's header: `sha7 · Name · age · subject · this file +a −r`.
    fn header(&self, now: i64) -> String {
        let Some(row) = self.selected_row() else {
            return String::new();
        };
        let mut header = format!(
            "{} \u{b7} {} \u{b7} {} \u{b7} {}",
            short(row.id),
            sanitize_line(&row.author).as_str(),
            relative_date_long(row.when, now),
            sanitize_line(&row.subject).as_str(),
        );
        if let Some(view) = self.file_view() {
            let lines = file_rows(&view.hunks);
            let added = lines
                .iter()
                .filter(|line| line.kind == LineKindView::Added)
                .count();
            let removed = lines
                .iter()
                .filter(|line| line.kind == LineKindView::Removed)
                .count();
            header.push_str(&format!(" \u{b7} this file +{added} \u{2212}{removed}"));
        }
        header
    }
}

/// Screen rows commits `top..=last` take, a `↳ was` row each for a rename.
fn drawn_rows(rows: &[TraceCommit], top: usize, last: usize) -> usize {
    rows.iter()
        .enumerate()
        .skip(top)
        .take_while(|(at, _)| *at <= last)
        .map(|(_, row)| 1 + usize::from(row.renamed_from.is_some()))
        .sum()
}

/// A row's sanitised words, owned for the view to borrow.
struct RowText {
    sha7: String,
    subject: String,
    who: String,
    was: Option<String>,
}

impl RowText {
    fn of(row: &TraceCommit) -> RowText {
        RowText {
            sha7: short(row.id),
            subject: sanitize_line(&row.subject).as_str().to_owned(),
            who: sanitize_line(&row.author).as_str().to_owned(),
            was: row
                .renamed_from
                .as_ref()
                .map(|was| sanitize_path(was).as_str().to_owned()),
        }
    }
}

/// `fn refresh_token, lines 18–29` / `lines 22–24 · the block blame showed`.
fn range_words(range: &LineRange) -> String {
    let lines = |start: u32, end: u32| {
        if start == end {
            format!("line {start}")
        } else {
            format!("lines {start}\u{2013}{end}")
        }
    };
    match range {
        LineRange::Function { name, start, end } => {
            format!("{}, {}", function_name(name), lines(*start, *end))
        }
        LineRange::Block { start, end } => {
            format!("{} \u{b7} the block blame showed", lines(*start, *end))
        }
    }
}

/// The function's name as the title says it, from git's header line: the
/// word before the first `(`, with the keyword before it when there is one
/// (`pub fn refresh_token(…` → `fn refresh_token`). A header with no `(`
/// is kept whole.
fn function_name(header: &str) -> String {
    const KEYWORDS: [&str; 6] = ["fn", "def", "func", "function", "sub", "class"];
    let header = sanitize_line(header);
    let header = header.as_str();
    let Some((before, _)) = header.split_once('(') else {
        return header.trim().to_owned();
    };
    let words: Vec<&str> = before.split_whitespace().collect();
    match words.as_slice() {
        [.., keyword, name] if KEYWORDS.contains(keyword) => format!("{keyword} {name}"),
        [.., name] => (*name).to_owned(),
        [] => header.trim().to_owned(),
    }
}

/// The end row's words (H1).
fn end_words(end: &TraceEnd) -> String {
    match end {
        TraceEnd::Start { added_by } => {
            format!("the file starts here ({} added it)", short(*added_by))
        }
        TraceEnd::GitStops(reason) => {
            format!("git stops tracing here: {}", sanitize_line(reason).as_str())
        }
        TraceEnd::Capped { shown } => format!("{} shown \u{b7} older not traced", grouped(*shown)),
        // A stopped trace is a cancelled job: its answer never lands. Said
        // plainly if one ever does, rather than drawing nothing.
        TraceEnd::Stopped => "git stops tracing here: stopped".to_owned(),
    }
}

/// `1 000`: thousands set apart by a space, as H1 writes the cap.
fn grouped(count: usize) -> String {
    let digits = count.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(' ');
        }
        out.push(digit);
    }
    out
}

/// Rows the place draws in under its title: the frame less the bar and the
/// title (`view::frame::draw_place`'s split, `graft_ui::history`'s title).
pub(super) fn list_height(state: &AppState) -> usize {
    usize::from(state.size.rows.saturating_sub(2))
}

/// Rows the file pane's lines get: the list's less the pane's header.
pub(super) fn file_height(state: &AppState) -> usize {
    list_height(state).saturating_sub(1)
}

/// **First row of the next run of changed rows**, `next_block`'s contract
/// (`update::resolver::walk`): `from: None` starts outside the file, so it
/// answers the first (or last) run; `None` past the end. A run is changed
/// rows in a row — an add after a remove is one change.
#[must_use]
pub(crate) fn next_change(changed: &[bool], from: Option<usize>, forward: bool) -> Option<usize> {
    let mut starts = changed
        .iter()
        .enumerate()
        .filter(|&(at, &is)| is && (at == 0 || !changed[at - 1]))
        .map(|(at, _)| at);
    match (from, forward) {
        (None, true) => starts.next(),
        (None, false) => starts.next_back(),
        (Some(from), true) => starts.find(|&at| at > from),
        (Some(from), false) => starts.rfind(|&at| at < from),
    }
}

/// Open a history place for `from` on the trail and start its trace.
/// Pushed on top: `F` starts a fresh trail around it (`f_key`), `L` pushes
/// onto blame's.
pub fn open_history(state: &mut AppState, from: HistoryFrom) -> Vec<Command> {
    let head = state.refs.as_ref().and_then(|refs| refs.head());
    let Opening {
        at,
        path,
        lines,
        blamed,
        anchor,
        request,
    } = match opening(from, head) {
        Ok(opening) => opening,
        // Nothing committed to trace: said, and no place opens.
        Err(words) => {
            state.status = Status::sticky(words);
            return Vec::new();
        }
    };
    let job = state.local_jobs.next_id();
    let place = HistoryState {
        path,
        at,
        lines,
        blamed,
        range: None,
        trace: TraceLoad::Tracing { job },
        selected: None,
        list_top: 0,
        file: FileLoad::Idle,
        file_top: 0,
        change: None,
        focus: HistoryFocus::List,
        side_by_side: false,
        max_lines: MAX_LINES,
        anchor,
        last_click: None,
    };
    if state.trail.places.len() >= TRAIL_MAX {
        state.trail.places.remove(0);
    }
    state.trail.places.push(Place::History(Box::new(place)));
    state.trail.shown = true;
    vec![Command::TraceHistory {
        id: job,
        request: Box::new(request),
    }]
}

/// What a history place opens with, worked out from where it was asked.
struct Opening {
    at: Oid,
    path: PathBuf,
    lines: bool,
    blamed: Option<Oid>,
    anchor: Option<LineRef>,
    request: TraceRequest,
}

/// The opening for `from`; a working-tree ask traces from `head`. `Err`
/// carries the words to say when there is no `HEAD` to trace from.
fn opening(from: HistoryFrom, head: Option<Oid>) -> Result<Opening, String> {
    match from {
        HistoryFrom::Lines {
            at,
            path,
            line,
            blamed,
            block,
        } => {
            let (at, from_disk) = match at {
                Rev::Commit(commit) => (commit, false),
                Rev::WorkingTree => (head.ok_or_else(|| uncommitted_words(line))?, true),
            };
            let request = TraceRequest::Lines {
                at,
                path: path.clone(),
                line,
                block: LineRange::Block {
                    start: block.0,
                    end: block.1,
                },
                from_disk,
            };
            Ok(Opening {
                at,
                path,
                lines: true,
                blamed,
                anchor: None,
                request,
            })
        }
        HistoryFrom::File {
            at,
            path,
            line,
            side,
        } => {
            let at = match at {
                Rev::Commit(commit) => commit,
                Rev::WorkingTree => head.ok_or_else(|| not_committed_words(&path))?,
            };
            let request = TraceRequest::File {
                at,
                path: path.clone(),
            };
            Ok(Opening {
                at,
                path,
                lines: false,
                blamed: None,
                anchor: line.map(|line| LineRef { side, line }),
                request,
            })
        }
    }
}

/// `<path> isn't committed yet · no history`.
fn not_committed_words(path: &std::path::Path) -> String {
    format!(
        "{} isn't committed yet \u{b7} no history",
        sanitize_path(path).as_str()
    )
}

/// `F` in the detail pane: the history of the file the cursor is on — its
/// row, or any line of its diff, the changes pane included. Claimed exactly
/// where `B` is (`place::blamable`): the same guard, so the two keys, and
/// the bar that offers them, cannot disagree. `true` when `F` was taken;
/// otherwise it falls through to Focus mode (`update::keys::graph_key`).
pub(crate) fn f_key(next: &mut AppState, commands: &mut Vec<Command>) -> bool {
    if !super::blamable(next) {
        return false;
    }
    let Some(cursor) = diff_cursor(next) else {
        return false;
    };
    if cursor.section == Some(WorkingTreeSection::Untracked) {
        next.status = Status::sticky(not_committed_words(&cursor.path));
        return true;
    }
    // The line he was on, from the diff; a file row's `at` is its first
    // change, which is not a line he chose (F2: the top from a file row).
    let on_a_line = next.detail.focus == graft_ui::detail::DetailFocus::Diff;
    let (at, line) = match cursor.spec {
        DetailSpec::Commit(commit) => (Rev::Commit(commit), cursor.at.filter(|_| on_a_line)),
        DetailSpec::Compare { head, .. } => (
            Rev::Commit(head),
            cursor.at.filter(|at| on_a_line && at.side == Side::New),
        ),
        // The changes pane counts lines on disk or in the index, not in
        // `HEAD`: the file opens at its top.
        DetailSpec::WorkingTree => (Rev::WorkingTree, None),
    };
    let from = HistoryFrom::File {
        at,
        path: cursor.path.clone(),
        line: line.map(|at| at.line),
        side: line.map_or(Side::New, |at| at.side),
    };
    let base = super::begin_trail(next, commands, &cursor);
    let opened = open_history(next, from);
    super::settle_trail(next, base);
    commands.extend(opened);
    true
}
