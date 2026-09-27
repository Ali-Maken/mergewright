//! A history place's two jobs — the trace and the whole file — their
//! requests and answers, and what the place does when one lands.

use std::path::PathBuf;
use std::sync::Arc;

use graft_core::job::JobId;
use graft_core::oid::Oid;
use graft_git::trace::{LineRange, TraceCommit, TraceEnd};

use super::history::{FileLoad, HistoryState, TraceLoad, file_height, list_height};
use super::{Place, leave};
use crate::update::events::{Command, WholeFileAnswer};
use crate::update::state::{AppState, Status};

/// What a trace job reads (m7-power-tools/21). One answer per trace
/// (ruling a): git's runner is buffered, so the rows arrive together.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceRequest {
    /// `L`: the function around `line` as of `at`, else `block`; then the
    /// commits that changed it (`git log -L`).
    Lines {
        /// The version the line counts in.
        at: Oid,
        /// The file's path in it.
        path: PathBuf,
        /// The line.
        line: u32,
        /// The blame block around the line: the range when no function is
        /// found (or it is over [`FUNCTION_MAX`] lines).
        block: LineRange,
        /// `line` and `block` count in the file on disk, not `at` (a blame
        /// of the working tree): the job maps them to `HEAD` first
        /// (`graft_git::trace::map_to_head`), and a line with no `HEAD`
        /// counterpart is [`TraceFailure::Uncommitted`]. Added to 21's
        /// signature: the mapping needs the working-tree diff, which is I/O.
        from_disk: bool,
    },
    /// `F`: every commit that changed the file, following renames.
    File {
        /// The version to trace back from.
        at: Oid,
        /// The file's path in it.
        path: PathBuf,
    },
}

/// A function longer than this is traced as the blame block instead (H1).
pub const FUNCTION_MAX: u32 = 400;

/// At most this many rows; then `1 000 shown · older not traced` (H1).
pub const TRACE_CAP: usize = 1_000;

/// A trace job's answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TraceAnswer {
    /// The range traced (`Lines` only), for the title.
    pub range: Option<LineRange>,
    /// The commits, newest first.
    pub rows: Vec<TraceCommit>,
    /// Why the trace ended.
    pub end: TraceEnd,
}

/// Why a trace job gave no trace. Not 21's `TraceError`: a [`Product`]
/// is `Clone + Eq`, which git's I/O error is not, so its words travel.
///
/// [`Product`]: crate::update::events::Product
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TraceFailure {
    /// A line of the file on disk with no `HEAD` counterpart: nothing
    /// opens (H1).
    Uncommitted(u32),
    /// The trace (or the function lookup before it) failed: its words.
    Failed(String),
}

/// A trace job answered. A stale id (the place was popped, `Esc` while
/// tracing) is dropped: nothing reopens (21 ruling a).
pub(crate) fn trace_arrived(
    state: &mut AppState,
    commands: &mut Vec<Command>,
    id: JobId,
    answer: Result<Arc<TraceAnswer>, TraceFailure>,
) {
    let Some(index) = state.trail.places.iter().position(|place| {
        matches!(place, Place::History(history)
            if matches!(history.trace, TraceLoad::Tracing { job } if job == id))
    }) else {
        return;
    };
    let heights = (list_height(state), file_height(state));
    let job = state.local_jobs.next_id();
    let Place::History(history) = &mut state.trail.places[index] else {
        return;
    };
    match answer {
        Ok(answer) => {
            let answer = Arc::unwrap_or_clone(answer);
            history.range = answer.range;
            history.selected = history.start_row(&answer.rows);
            history.trace = TraceLoad::Ready {
                rows: answer.rows,
                end: answer.end,
            };
            history.list_top = 0;
            history.keep_list_in_view(heights.0);
            commands.extend(history.read_selected(job));
            history.keep_file_in_view(heights.1);
        }
        Err(TraceFailure::Failed(words)) => history.trace = TraceLoad::Failed(words),
        Err(TraceFailure::Uncommitted(line)) => {
            // The words H1 gives; the place comes off as a refused blame
            // does (`answer::refuse`).
            state.trail.places.remove(index);
            if state.trail.is_live() {
                state.trail.shown = true;
            } else {
                leave(state, commands, true);
            }
            state.status = Status::sticky(uncommitted_words(line));
        }
    }
}

/// `line 42 isn't committed yet · no history` (H1).
pub(super) fn uncommitted_words(line: u32) -> String {
    format!("line {line} isn't committed yet \u{b7} no history")
}

/// A whole-file read answered. Only the read the place still waits for
/// lands; `j`/`k` since asked for another.
pub(crate) fn whole_file_arrived(
    state: &mut AppState,
    id: JobId,
    answer: Result<WholeFileAnswer, String>,
) {
    let height = file_height(state);
    let Some(history) = waiting_for(state, id) else {
        return;
    };
    let FileLoad::Reading { commit, .. } = history.file else {
        return;
    };
    history.file = match answer {
        Ok(WholeFileAnswer { view, sizes }) => FileLoad::Ready {
            commit,
            view,
            sizes,
        },
        Err(words) => FileLoad::Failed { commit, words },
    };
    history.place_file_pane();
    history.keep_file_in_view(height);
}

/// The history place whose whole-file read is `id`.
fn waiting_for(state: &mut AppState, id: JobId) -> Option<&mut HistoryState> {
    state.trail.places.iter_mut().find_map(|place| match place {
        Place::History(history)
            if matches!(history.file, FileLoad::Reading { job, .. } if job == id) =>
        {
            Some(history.as_mut())
        }
        Place::History(_) | Place::Blame(_) => None,
    })
}
