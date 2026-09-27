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
use graft_git::trace::{BlobSizes, LineRange, TraceCommit, TraceEnd};
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

mod open;
mod view;

pub(crate) use open::f_key;
pub use open::open_history;
use view::drawn_rows;

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
        /// A binary file's sizes before and after, or why not; `None` for
        /// text (21c).
        sizes: Option<Result<BlobSizes, String>>,
    },
}

/// A binary answer that came without its size read: a wiring bug, shown.
const SIZES_NOT_READ: &str = "sizes not read";

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
    /// `w`: whole-file reads ignore whitespace (git's `-w`). Held as `j`/`k`
    /// step commits (21c).
    pub ignore_whitespace: bool,
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
            ignore_whitespace: self.ignore_whitespace,
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
